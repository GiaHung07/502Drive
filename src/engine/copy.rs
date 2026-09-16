use std::time::Duration;

use anyhow::{Context, bail};
use serde_json::json;
use thiserror::Error;
use tokio::task::JoinSet;
use tracing::warn;

use crate::{
    config::AppConfig,
    drive::{
        client::{DriveApiError, DriveClient},
        links::DriveReference,
        token_manager::TokenManager,
        types::{DriveFile, FOLDER_MIME_TYPE, FileList, SHORTCUT_MIME_TYPE},
    },
    engine::{
        mapping::{MappingRecord, record_mapping},
        operation::{AppProperties, OperationType, ScopeType},
        retry::{RetryDecision, backoff_delay, classify_http},
        scheduler::CopyLimiter,
    },
    report,
    state::{
        db::Database,
        repo::{self, JobStatusValue, NewItem, NewOperationIntent, TraversalFolder},
    },
};

/// Maximum length (in characters) accepted for `CloneRequest::name_override`.
pub const MAX_NAME_OVERRIDE_LEN: usize = 255;

/// Duplicate policies accepted on `CloneRequest` (mirrors the CHECK constraint
/// on `jobs.duplicate_policy`).
pub const DUPLICATE_POLICIES: &[&str] = &["keep_both", "skip_same_source", "replace_safe"];

#[derive(Debug, Clone)]
pub struct CloneRequest {
    pub chat_id: i64,
    pub telegram_user_id: i64,
    pub source: DriveReference,
    pub progress_message_id: Option<i32>,
    /// Destination root folder/file name override. `None` keeps the source
    /// item's own name.
    pub name_override: Option<String>,
    /// Explicit destination parent (a Drive folder id). `None` resolves the
    /// configured default destination profile.
    pub destination_parent_id: Option<String>,
    /// Per-job duplicate policy. `None` falls back to
    /// `engine.default_duplicate_policy`.
    pub duplicate_policy: Option<String>,
}

/// Validate the optional overrides on a [`CloneRequest`] before any Drive I/O.
pub fn validate_clone_request(request: &CloneRequest) -> anyhow::Result<()> {
    if let Some(name) = request.name_override.as_deref().map(str::trim) {
        if name.is_empty() {
            bail!("Tên đích (name_override) không được để trống");
        }
        if name.chars().count() > MAX_NAME_OVERRIDE_LEN {
            bail!("Tên đích quá dài (tối đa {} ký tự)", MAX_NAME_OVERRIDE_LEN);
        }
    }
    if let Some(dest) = request.destination_parent_id.as_deref().map(str::trim) {
        if dest.is_empty() {
            bail!("destination_parent_id không được để trống");
        }
    }
    if let Some(policy) = request.duplicate_policy.as_deref().map(str::trim) {
        if !DUPLICATE_POLICIES.contains(&policy) {
            bail!(
                "duplicate_policy không hợp lệ: '{}'. Các giá trị hợp lệ: {}",
                policy,
                DUPLICATE_POLICIES.join(", ")
            );
        }
    }
    Ok(())
}

/// Validate a destination folder fetched from Drive before planning a job.
fn validate_destination_for_clone(destination: &DriveFile) -> anyhow::Result<()> {
    if destination.trashed == Some(true) {
        bail!("Thư mục đích đang nằm trong thùng rác");
    }
    if !destination.is_folder() {
        bail!("Đích chỉ định không phải là thư mục Drive");
    }
    if destination
        .capabilities
        .as_ref()
        .and_then(|cap| cap.can_add_children)
        != Some(true)
    {
        bail!("Tài khoản Google hiện tại không có quyền ghi vào thư mục đích");
    }
    Ok(())
}

#[derive(Debug, Clone)]
pub struct CloneOutcome {
    pub job_id: String,
    pub message: String,
    pub report_paths: Option<report::ReportPaths>,
}

#[derive(Debug, Error)]
pub enum JobControlStop {
    #[error("Job paused")]
    Paused,
    #[error("Job cancelled")]
    Cancelled,
}

#[derive(Clone)]
pub struct CloneService {
    config: AppConfig,
    db: Database,
    drive: DriveClient,
    token_manager: TokenManager,
    copy_limiter: CopyLimiter,
}

impl CloneService {
    /// `drive` is the process-wide shared client: reqwest pools are
    /// internally Arc'd, so passing a clone reuses the connection pool and
    /// the shared rate-limit pacer instead of building a fresh one per job.
    pub fn new(config: AppConfig, db: Database, drive: DriveClient) -> Self {
        let token_manager = TokenManager::new(config.clone(), db.clone());
        // The effective write concurrency: the GUI writes max_write_concurrency,
        // so it (not initial_write_concurrency) is the knob users actually set.
        let write_concurrency = config.engine.max_write_concurrency;
        Self {
            drive,
            config,
            db,
            token_manager,
            copy_limiter: CopyLimiter::new(write_concurrency),
        }
    }

    pub async fn start_one_shot(&self, request: CloneRequest) -> anyhow::Result<CloneOutcome> {
        validate_clone_request(&request)?;

        let active_jobs = repo::active_job_count(&self.db).await?;
        if active_jobs >= self.config.engine.max_active_jobs as i64 {
            bail!(
                "The bot is already at the active job limit. Use /jobs or wait for a job to finish."
            );
        }

        let active_jobs =
            repo::active_job_count_for_user(&self.db, request.telegram_user_id).await?;
        if active_jobs >= self.config.engine.max_active_jobs_per_user as i64 {
            bail!(
                "You already have an active job. Use /jobs, /status, /pause, /cancel, or wait for it to finish."
            );
        }

        let source = self.get_reference_retry(&request.source).await?;
        validate_source_for_clone(&source)?;

        // Destination: an explicit override is validated (folder + writable)
        // before planning; otherwise fall back to the default profile.
        let (destination_parent_id, destination_drive_id) = if let Some(dest_id) = request
            .destination_parent_id
            .as_deref()
            .map(str::trim)
            .filter(|id| !id.is_empty())
        {
            let destination = self
                .get_reference_retry(&DriveReference {
                    file_id: dest_id.to_string(),
                    resource_key: None,
                    hinted_kind: None,
                })
                .await?;
            validate_destination_for_clone(&destination)?;
            (destination.id.clone(), destination.drive_id.clone())
        } else {
            let profile = repo::default_destination_profile(&self.db, "default")
                .await?
                .context("No default destination configured. Use /set_destination first")?;
            (
                profile.destination_parent_id.clone(),
                profile.destination_drive_id.clone(),
            )
        };

        let job_id = repo::create_job_with_metadata(
            &self.db,
            repo::NewJob {
                chat_id: request.chat_id,
                telegram_user_id: request.telegram_user_id,
                google_account_id: "default".to_string(),
                source_root_id: source.id.clone(),
                source_resource_key: request
                    .source
                    .resource_key
                    .clone()
                    .or(source.resource_key.clone()),
                source_drive_id: source.drive_id.clone(),
                destination_parent_id: destination_parent_id.clone(),
                destination_drive_id,
                progress_message_id: request.progress_message_id,
                duplicate_policy: request
                    .duplicate_policy
                    .clone()
                    .unwrap_or_else(|| self.config.engine.default_duplicate_policy.clone()),
            },
        )
        .await?;

        let root_name = request
            .name_override
            .as_deref()
            .map(str::trim)
            .filter(|name| !name.is_empty())
            .unwrap_or(source.name.as_str())
            .to_string();

        let result = if source.is_folder() {
            self.create_folder_root(
                &job_id,
                &source,
                &destination_parent_id,
                &request.source,
                &root_name,
            )
            .await
        } else {
            self.copy_single_file(
                &job_id,
                &source,
                &destination_parent_id,
                &request.source,
                &root_name,
            )
            .await
        };

        match result {
            Ok(message) => {
                let report_paths = self.write_report_best_effort(&job_id).await;
                Ok(CloneOutcome {
                    job_id,
                    message,
                    report_paths,
                })
            }
            Err(err) => {
                if let Some(stop) = err.downcast_ref::<JobControlStop>() {
                    return Ok(CloneOutcome {
                        job_id,
                        message: stop.to_string(),
                        report_paths: None,
                    });
                }
                let _ = repo::update_job_status(
                    &self.db,
                    &job_id,
                    JobStatusValue::Failed,
                    Some(&err.to_string()),
                )
                .await;
                Err(err)
            }
        }
    }

    /// Like `start_one_shot` but uses an explicit destination parent instead of
    /// the configured default destination profile. Used by the watch initializer.
    ///
    /// `watch_id` is stored as a tag on the job so recovery can associate it.
    pub async fn start_one_shot_with_dest(
        &self,
        request: CloneRequest,
        destination_parent_id: String,
        destination_drive_id: Option<String>,
        _watch_id: Option<String>,
    ) -> anyhow::Result<CloneOutcome> {
        let source = self.get_reference_retry(&request.source).await?;
        validate_source_for_clone(&source)?;

        let job_id = repo::create_job_with_metadata(
            &self.db,
            repo::NewJob {
                chat_id: request.chat_id,
                telegram_user_id: request.telegram_user_id,
                google_account_id: "default".to_string(),
                source_root_id: source.id.clone(),
                source_resource_key: request
                    .source
                    .resource_key
                    .clone()
                    .or(source.resource_key.clone()),
                source_drive_id: source.drive_id.clone(),
                destination_parent_id: destination_parent_id.clone(),
                destination_drive_id,
                progress_message_id: request.progress_message_id,
                duplicate_policy: self.config.engine.default_duplicate_policy.clone(),
            },
        )
        .await?;

        let result = if source.is_folder() {
            self.create_folder_root(
                &job_id,
                &source,
                &destination_parent_id,
                &request.source,
                &source.name,
            )
            .await
        } else {
            self.copy_single_file(
                &job_id,
                &source,
                &destination_parent_id,
                &request.source,
                &source.name,
            )
            .await
        };

        match result {
            Ok(message) => {
                let report_paths = self.write_report_best_effort(&job_id).await;
                Ok(CloneOutcome {
                    job_id,
                    message,
                    report_paths,
                })
            }
            Err(err) => {
                if let Some(stop) = err.downcast_ref::<JobControlStop>() {
                    return Ok(CloneOutcome {
                        job_id,
                        message: stop.to_string(),
                        report_paths: None,
                    });
                }
                let _ = repo::update_job_status(
                    &self.db,
                    &job_id,
                    JobStatusValue::Failed,
                    Some(&err.to_string()),
                )
                .await;
                Err(err)
            }
        }
    }

    pub async fn resume_one_shot_job(
        &self,
        job: &repo::ResumableOneShotJob,
    ) -> anyhow::Result<CloneOutcome> {
        if job.google_account_id != "default" {
            bail!(
                "Unsupported Google account '{}' for one-shot resume",
                job.google_account_id
            );
        }

        let source_reference = DriveReference {
            file_id: job.source_root_id.clone(),
            resource_key: job.source_resource_key.clone(),
            hinted_kind: None,
        };
        let source = self.get_reference_retry(&source_reference).await?;
        validate_source_for_clone(&source)?;

        let message = if source.is_folder() {
            if repo::has_traversal_rows(&self.db, &job.id).await? {
                repo::update_job_status(&self.db, &job.id, JobStatusValue::Discovering, None)
                    .await?;
                self.process_traversal(&job.id).await?;
                self.finish_folder_job(&job.id).await?
            } else {
                self.create_folder_root(
                    &job.id,
                    &source,
                    &job.destination_parent_id,
                    &source_reference,
                    &source.name,
                )
                .await?
            }
        } else if repo::existing_dest_for_source(
            &self.db,
            ScopeType::Job.as_str(),
            &job.id,
            &source.id,
        )
        .await?
        .is_some()
        {
            repo::update_job_status(&self.db, &job.id, JobStatusValue::Completed, None).await?;
            format!(
                "Job {} completed. Existing copied file mapping reused.",
                job.id
            )
        } else {
            self.copy_single_file(
                &job.id,
                &source,
                &job.destination_parent_id,
                &source_reference,
                &source.name,
            )
            .await?
        };

        Ok(CloneOutcome {
            job_id: job.id.clone(),
            message,
            report_paths: self.write_report_best_effort(&job.id).await,
        })
    }

    /// `root_name` is the name used for the destination root item. It equals
    /// the source name unless `CloneRequest::name_override` was provided.
    async fn create_folder_root(
        &self,
        job_id: &str,
        source: &DriveFile,
        destination_parent_id: &str,
        source_reference: &DriveReference,
        root_name: &str,
    ) -> anyhow::Result<String> {
        repo::update_job_status(&self.db, job_id, JobStatusValue::Running, None).await?;
        self.check_job_control(job_id).await?;

        let props =
            AppProperties::new(ScopeType::Job, job_id, &source.id, destination_parent_id, 1);
        repo::plan_operation_intent(
            &self.db,
            NewOperationIntent {
                idempotency_key: props.gdclone_copy_key.clone(),
                job_id: Some(job_id.to_string()),
                watch_id: None,
                operation_type: OperationType::CreateFolder,
                source_item_id: Some(source.id.clone()),
                destination_parent_id: Some(destination_parent_id.to_string()),
                destination_item_id: None,
                request_json: create_folder_request_json(root_name, destination_parent_id, &props),
            },
        )
        .await?;

        let destination = self
            .create_folder_idempotent(root_name, destination_parent_id, &props)
            .await?;
        record_mapping(
            &self.db,
            MappingRecord {
                scope_type: ScopeType::Job.as_str().to_string(),
                scope_id: job_id.to_string(),
                source_item_id: source.id.clone(),
                destination_item_id: destination.id.clone(),
                source_parent_id: source.parents.first().cloned(),
                destination_parent_id: Some(destination_parent_id.to_string()),
                mime_type: FOLDER_MIME_TYPE.to_string(),
                source_name: source.name.clone(),
                source_version: source.version.clone(),
                source_modified_time: source.modified_time.clone(),
                source_md5_checksum: source.md5_checksum.clone(),
            },
        )
        .await?;
        repo::record_traversal_folder(
            &self.db,
            job_id,
            &source.id,
            source_reference
                .resource_key
                .as_deref()
                .or(source.resource_key.as_deref()),
            &destination.id,
            source.parents.first().map(String::as_str),
        )
        .await?;
        repo::update_job_counts(&self.db, job_id, 1, 1, 0, 0).await?;
        repo::update_job_status(&self.db, job_id, JobStatusValue::Discovering, None).await?;
        self.process_traversal(job_id).await?;

        self.finish_folder_job(job_id).await
    }

    async fn finish_folder_job(&self, job_id: &str) -> anyhow::Result<String> {
        let counts = repo::job_counts(&self.db, job_id).await?;
        let final_status = if counts.failed_items > 0 {
            JobStatusValue::PartiallyCompleted
        } else {
            JobStatusValue::Completed
        };
        repo::update_job_status(&self.db, job_id, final_status, None).await?;

        Ok(format!(
            "Job {job_id} đã hoàn tất. Đã quét: {} Hoàn tất: {} Lỗi: {} Bỏ qua: {}",
            counts.total_discovered,
            counts.completed_items,
            counts.failed_items,
            counts.skipped_items
        ))
    }

    async fn copy_single_file(
        &self,
        job_id: &str,
        source: &DriveFile,
        destination_parent_id: &str,
        source_reference: &DriveReference,
        root_name: &str,
    ) -> anyhow::Result<String> {
        repo::update_job_status(&self.db, job_id, JobStatusValue::Running, None).await?;
        self.check_job_control(job_id).await?;

        let props =
            AppProperties::new(ScopeType::Job, job_id, &source.id, destination_parent_id, 1);
        repo::insert_item(
            &self.db,
            NewItem {
                job_id: job_id.to_string(),
                source_item_id: source.id.clone(),
                destination_parent_id: destination_parent_id.to_string(),
                mime_type: source.mime_type.clone(),
                item_kind: item_kind(source).to_string(),
                source_name: source.name.clone(),
                size_bytes: source.size.as_deref().and_then(|size| size.parse().ok()),
            },
        )
        .await?;
        let operation_type = if source.is_shortcut() {
            OperationType::CreateShortcut
        } else {
            OperationType::CopyFile
        };
        let request_json = if source.is_shortcut() {
            let details = source
                .shortcut_details
                .as_ref()
                .context("shortcut is missing shortcutDetails")?;
            create_shortcut_request_json(
                root_name,
                destination_parent_id,
                &details.target_id,
                details.target_resource_key.as_deref(),
                &props,
            )
        } else {
            copy_file_request_json(root_name, destination_parent_id, &props)
        };
        repo::plan_operation_intent(
            &self.db,
            NewOperationIntent {
                idempotency_key: props.gdclone_copy_key.clone(),
                job_id: Some(job_id.to_string()),
                watch_id: None,
                operation_type,
                source_item_id: Some(source.id.clone()),
                destination_parent_id: Some(destination_parent_id.to_string()),
                destination_item_id: None,
                request_json,
            },
        )
        .await?;

        let copied = if source.is_shortcut() {
            self.create_shortcut_idempotent(source, destination_parent_id, root_name, &props)
                .await?
        } else {
            self.copy_file_idempotent(source_reference, root_name, destination_parent_id, &props)
                .await?
        };
        repo::mark_item_done(&self.db, job_id, &source.id, &copied.id).await?;
        record_mapping(
            &self.db,
            MappingRecord {
                scope_type: ScopeType::Job.as_str().to_string(),
                scope_id: job_id.to_string(),
                source_item_id: source.id.clone(),
                destination_item_id: copied.id.clone(),
                source_parent_id: source.parents.first().cloned(),
                destination_parent_id: Some(destination_parent_id.to_string()),
                mime_type: source.mime_type.clone(),
                source_name: source.name.clone(),
                source_version: source.version.clone(),
                source_modified_time: source.modified_time.clone(),
                source_md5_checksum: source.md5_checksum.clone(),
            },
        )
        .await?;
        repo::update_job_counts(&self.db, job_id, 1, 1, 0, 0).await?;
        repo::update_job_status(&self.db, job_id, JobStatusValue::Completed, None).await?;

        Ok(format!(
            "Job {job_id} đã hoàn tất. Đã copy file: {}",
            copied.name
        ))
    }

    async fn process_traversal(&self, job_id: &str) -> anyhow::Result<()> {
        while let Some(folder) = repo::next_pending_traversal_folder(&self.db, job_id).await? {
            self.check_job_control(job_id).await?;
            let result = self.process_folder_page(job_id, &folder).await;
            if let Err(err) = result {
                if err.downcast_ref::<JobControlStop>().is_some() {
                    return Err(err);
                }
                repo::fail_traversal_folder(
                    &self.db,
                    job_id,
                    &folder.source_folder_id,
                    &err.to_string(),
                )
                .await?;
                repo::increment_job_counts(&self.db, job_id, 0, 0, 1, 0).await?;
            }
        }
        Ok(())
    }

    async fn process_folder_page(
        &self,
        job_id: &str,
        folder: &TraversalFolder,
    ) -> anyhow::Result<()> {
        let page = self
            .list_children_retry(
                &folder.source_folder_id,
                folder.source_resource_key.as_deref(),
                folder.next_page_token.as_deref(),
            )
            .await?;

        let mut file_tasks = JoinSet::new();

        for child in page.files {
            self.check_job_control(job_id).await?;
            if child.is_folder() {
                self.create_child_folder(job_id, folder, &child).await?;
            } else {
                let service = self.clone();
                let job_id = job_id.to_string();
                let folder = folder.clone();
                file_tasks.spawn(async move {
                    let result = async {
                        service.check_job_control(&job_id).await?;
                        service.copy_child_file(&job_id, &folder, &child).await
                    }
                    .await;
                    (child, result)
                });
            }
        }

        while let Some(joined) = file_tasks.join_next().await {
            let (child, result) = joined.context("copy task panicked or was cancelled")?;
            if let Err(err) = result {
                if err.downcast_ref::<JobControlStop>().is_some() {
                    return Err(err);
                }
                repo::mark_item_failed(
                    &self.db,
                    job_id,
                    &child.id,
                    "copy_failed",
                    &err.to_string(),
                )
                .await?;
                repo::increment_job_counts(&self.db, job_id, 1, 0, 1, 0).await?;
            }
        }

        if let Some(next_page_token) = page.next_page_token {
            repo::requeue_traversal_folder_page(
                &self.db,
                job_id,
                &folder.source_folder_id,
                &next_page_token,
            )
            .await?;
        } else {
            repo::finish_traversal_folder(&self.db, job_id, &folder.source_folder_id).await?;
        }
        Ok(())
    }

    async fn check_job_control(&self, job_id: &str) -> anyhow::Result<()> {
        match repo::job_status(&self.db, job_id).await?.as_deref() {
            Some("pausing" | "paused") => {
                repo::update_job_status(&self.db, job_id, JobStatusValue::Paused, None).await?;
                Err(JobControlStop::Paused.into())
            }
            Some("cancelling" | "cancelled") => {
                repo::update_job_status(&self.db, job_id, JobStatusValue::Cancelled, None).await?;
                Err(JobControlStop::Cancelled.into())
            }
            _ => Ok(()),
        }
    }

    async fn create_child_folder(
        &self,
        job_id: &str,
        parent: &TraversalFolder,
        source: &DriveFile,
    ) -> anyhow::Result<()> {
        if repo::existing_dest_for_source(&self.db, ScopeType::Job.as_str(), job_id, &source.id)
            .await?
            .is_some()
        {
            return Ok(());
        }
        validate_source_for_clone(source)?;
        let props = AppProperties::new(
            ScopeType::Job,
            job_id,
            &source.id,
            &parent.destination_folder_id,
            1,
        );
        repo::plan_operation_intent(
            &self.db,
            NewOperationIntent {
                idempotency_key: props.gdclone_copy_key.clone(),
                job_id: Some(job_id.to_string()),
                watch_id: None,
                operation_type: OperationType::CreateFolder,
                source_item_id: Some(source.id.clone()),
                destination_parent_id: Some(parent.destination_folder_id.clone()),
                destination_item_id: None,
                request_json: create_folder_request_json(
                    &source.name,
                    &parent.destination_folder_id,
                    &props,
                ),
            },
        )
        .await?;

        let destination = self
            .create_folder_idempotent(&source.name, &parent.destination_folder_id, &props)
            .await?;
        record_mapping(
            &self.db,
            MappingRecord {
                scope_type: ScopeType::Job.as_str().to_string(),
                scope_id: job_id.to_string(),
                source_item_id: source.id.clone(),
                destination_item_id: destination.id.clone(),
                source_parent_id: Some(parent.source_folder_id.clone()),
                destination_parent_id: Some(parent.destination_folder_id.clone()),
                mime_type: FOLDER_MIME_TYPE.to_string(),
                source_name: source.name.clone(),
                source_version: source.version.clone(),
                source_modified_time: source.modified_time.clone(),
                source_md5_checksum: source.md5_checksum.clone(),
            },
        )
        .await?;
        repo::record_traversal_folder(
            &self.db,
            job_id,
            &source.id,
            source.resource_key.as_deref(),
            &destination.id,
            Some(&parent.source_folder_id),
        )
        .await?;
        repo::increment_job_counts(&self.db, job_id, 1, 1, 0, 0).await?;
        Ok(())
    }

    async fn copy_child_file(
        &self,
        job_id: &str,
        parent: &TraversalFolder,
        source: &DriveFile,
    ) -> anyhow::Result<()> {
        if repo::existing_dest_for_source(&self.db, ScopeType::Job.as_str(), job_id, &source.id)
            .await?
            .is_some()
        {
            return Ok(());
        }
        repo::insert_item(
            &self.db,
            NewItem {
                job_id: job_id.to_string(),
                source_item_id: source.id.clone(),
                destination_parent_id: parent.destination_folder_id.clone(),
                mime_type: source.mime_type.clone(),
                item_kind: item_kind(source).to_string(),
                source_name: source.name.clone(),
                size_bytes: source.size.as_deref().and_then(|size| size.parse().ok()),
            },
        )
        .await?;
        validate_source_for_clone(source)?;
        let source_reference = DriveReference {
            file_id: source.id.clone(),
            resource_key: source.resource_key.clone(),
            hinted_kind: None,
        };
        let props = AppProperties::new(
            ScopeType::Job,
            job_id,
            &source.id,
            &parent.destination_folder_id,
            1,
        );
        let operation_type = if source.is_shortcut() {
            OperationType::CreateShortcut
        } else {
            OperationType::CopyFile
        };
        let request_json = if source.is_shortcut() {
            let details = source
                .shortcut_details
                .as_ref()
                .context("shortcut is missing shortcutDetails")?;
            create_shortcut_request_json(
                &source.name,
                &parent.destination_folder_id,
                &details.target_id,
                details.target_resource_key.as_deref(),
                &props,
            )
        } else {
            copy_file_request_json(&source.name, &parent.destination_folder_id, &props)
        };
        repo::plan_operation_intent(
            &self.db,
            NewOperationIntent {
                idempotency_key: props.gdclone_copy_key.clone(),
                job_id: Some(job_id.to_string()),
                watch_id: None,
                operation_type,
                source_item_id: Some(source.id.clone()),
                destination_parent_id: Some(parent.destination_folder_id.clone()),
                destination_item_id: None,
                request_json,
            },
        )
        .await?;

        let copied = if source.is_shortcut() {
            self.create_shortcut_idempotent(
                source,
                &parent.destination_folder_id,
                &source.name,
                &props,
            )
            .await?
        } else {
            self.copy_file_idempotent(
                &source_reference,
                &source.name,
                &parent.destination_folder_id,
                &props,
            )
            .await?
        };
        repo::mark_item_done(&self.db, job_id, &source.id, &copied.id).await?;
        record_mapping(
            &self.db,
            MappingRecord {
                scope_type: ScopeType::Job.as_str().to_string(),
                scope_id: job_id.to_string(),
                source_item_id: source.id.clone(),
                destination_item_id: copied.id.clone(),
                source_parent_id: Some(parent.source_folder_id.clone()),
                destination_parent_id: Some(parent.destination_folder_id.clone()),
                mime_type: source.mime_type.clone(),
                source_name: source.name.clone(),
                source_version: source.version.clone(),
                source_modified_time: source.modified_time.clone(),
                source_md5_checksum: source.md5_checksum.clone(),
            },
        )
        .await?;
        repo::increment_job_counts(&self.db, job_id, 1, 1, 0, 0).await?;
        Ok(())
    }

    async fn create_folder_idempotent(
        &self,
        name: &str,
        destination_parent_id: &str,
        props: &AppProperties,
    ) -> anyhow::Result<DriveFile> {
        if let Some(existing) = self
            .find_existing_by_copy_key(destination_parent_id, props)
            .await?
        {
            return Ok(existing);
        }
        repo::mark_operation_executing(&self.db, &props.gdclone_copy_key).await?;
        let created = self
            .copy_limiter
            .run(self.create_folder_retry(name, destination_parent_id, props))
            .await?;
        repo::mark_operation_applied(&self.db, &props.gdclone_copy_key, &created.id).await?;
        Ok(created)
    }

    async fn copy_file_idempotent(
        &self,
        source_reference: &DriveReference,
        name: &str,
        destination_parent_id: &str,
        props: &AppProperties,
    ) -> anyhow::Result<DriveFile> {
        if let Some(existing) = self
            .find_existing_by_copy_key(destination_parent_id, props)
            .await?
        {
            return Ok(existing);
        }
        repo::mark_operation_executing(&self.db, &props.gdclone_copy_key).await?;
        let copied = self
            .copy_limiter
            .run(self.copy_file_retry(source_reference, name, destination_parent_id, props))
            .await?;
        repo::mark_operation_applied(&self.db, &props.gdclone_copy_key, &copied.id).await?;
        Ok(copied)
    }

    async fn create_shortcut_idempotent(
        &self,
        source: &DriveFile,
        destination_parent_id: &str,
        name: &str,
        props: &AppProperties,
    ) -> anyhow::Result<DriveFile> {
        if let Some(existing) = self
            .find_existing_by_copy_key(destination_parent_id, props)
            .await?
        {
            return Ok(existing);
        }
        let details = source
            .shortcut_details
            .as_ref()
            .context("shortcut is missing shortcutDetails")?;
        repo::mark_operation_executing(&self.db, &props.gdclone_copy_key).await?;
        let shortcut = self
            .copy_limiter
            .run(self.create_shortcut_retry(
                name,
                destination_parent_id,
                &details.target_id,
                details.target_resource_key.as_deref(),
                props,
            ))
            .await?;
        repo::mark_operation_applied(&self.db, &props.gdclone_copy_key, &shortcut.id).await?;
        Ok(shortcut)
    }

    async fn find_existing_by_copy_key(
        &self,
        destination_parent_id: &str,
        props: &AppProperties,
    ) -> anyhow::Result<Option<DriveFile>> {
        if let Some(intent) =
            repo::find_operation_intent_by_key(&self.db, &props.gdclone_copy_key).await?
            && intent.status == "applied"
        {
            if let Some(destination_item_id) = intent.destination_item_id
                && let Ok(existing) = self.get_file_retry(&destination_item_id).await
            {
                return Ok(Some(existing));
            }
            let files = self
                .find_by_copy_key_retry(destination_parent_id, &props.gdclone_copy_key)
                .await?;
            return match files.as_slice() {
                [one] => Ok(Some(one.clone())),
                [] => Ok(None),
                _ => bail!("Ambiguous recovery: multiple destination items share idempotency key"),
            };
        }

        let files = self
            .find_by_copy_key_retry(destination_parent_id, &props.gdclone_copy_key)
            .await?;
        match files.as_slice() {
            [one] => {
                repo::mark_operation_applied(&self.db, &props.gdclone_copy_key, &one.id).await?;
                Ok(Some(one.clone()))
            }
            [] => Ok(None),
            _ => bail!("Ambiguous recovery: multiple destination items share idempotency key"),
        }
    }

    async fn get_reference_retry(&self, reference: &DriveReference) -> anyhow::Result<DriveFile> {
        let mut refreshed = false;
        for attempt in 0..=self.config.engine.max_retry_attempts {
            let access_token = self.token_manager.access_token("default").await?;
            match self
                .drive
                .get_reference(access_token.as_str(), reference)
                .await
            {
                Ok(value) => return Ok(value),
                Err(err) => {
                    self.handle_drive_retry(err, attempt, &mut refreshed)
                        .await?
                }
            }
        }
        unreachable!("retry loop returns on final non-retryable error")
    }

    async fn get_file_retry(&self, file_id: &str) -> anyhow::Result<DriveFile> {
        let mut refreshed = false;
        for attempt in 0..=self.config.engine.max_retry_attempts {
            let access_token = self.token_manager.access_token("default").await?;
            match self.drive.get_file(access_token.as_str(), file_id).await {
                Ok(value) => return Ok(value),
                Err(err) => {
                    self.handle_drive_retry(err, attempt, &mut refreshed)
                        .await?
                }
            }
        }
        unreachable!("retry loop returns on final non-retryable error")
    }

    async fn list_children_retry(
        &self,
        parent_id: &str,
        parent_resource_key: Option<&str>,
        page_token: Option<&str>,
    ) -> anyhow::Result<FileList> {
        let mut refreshed = false;
        for attempt in 0..=self.config.engine.max_retry_attempts {
            let access_token = self.token_manager.access_token("default").await?;
            match self
                .drive
                .list_children(
                    access_token.as_str(),
                    parent_id,
                    parent_resource_key,
                    page_token,
                )
                .await
            {
                Ok(value) => return Ok(value),
                Err(err) => {
                    self.handle_drive_retry(err, attempt, &mut refreshed)
                        .await?
                }
            }
        }
        unreachable!("retry loop returns on final non-retryable error")
    }

    async fn find_by_copy_key_retry(
        &self,
        destination_parent_id: &str,
        copy_key: &str,
    ) -> anyhow::Result<Vec<DriveFile>> {
        let mut refreshed = false;
        for attempt in 0..=self.config.engine.max_retry_attempts {
            let access_token = self.token_manager.access_token("default").await?;
            match self
                .drive
                .find_by_copy_key(access_token.as_str(), destination_parent_id, copy_key)
                .await
            {
                Ok(value) => return Ok(value),
                Err(err) => {
                    self.handle_drive_retry(err, attempt, &mut refreshed)
                        .await?
                }
            }
        }
        unreachable!("retry loop returns on final non-retryable error")
    }

    async fn create_folder_retry(
        &self,
        name: &str,
        destination_parent_id: &str,
        props: &AppProperties,
    ) -> anyhow::Result<DriveFile> {
        let mut refreshed = false;
        for attempt in 0..=self.config.engine.max_retry_attempts {
            let access_token = self.token_manager.access_token("default").await?;
            match self
                .drive
                .create_folder(
                    access_token.as_str(),
                    name,
                    destination_parent_id,
                    Some(props),
                )
                .await
            {
                Ok(value) => return Ok(value),
                Err(err) => {
                    self.handle_drive_retry(err, attempt, &mut refreshed)
                        .await?
                }
            }
        }
        unreachable!("retry loop returns on final non-retryable error")
    }

    async fn copy_file_retry(
        &self,
        source_reference: &DriveReference,
        name: &str,
        destination_parent_id: &str,
        props: &AppProperties,
    ) -> anyhow::Result<DriveFile> {
        let mut refreshed = false;
        for attempt in 0..=self.config.engine.max_retry_attempts {
            let access_token = self.token_manager.access_token("default").await?;
            match self
                .drive
                .copy_file(
                    access_token.as_str(),
                    source_reference,
                    name,
                    destination_parent_id,
                    Some(props),
                )
                .await
            {
                Ok(value) => return Ok(value),
                Err(err) => {
                    self.handle_drive_retry(err, attempt, &mut refreshed)
                        .await?
                }
            }
        }
        unreachable!("retry loop returns on final non-retryable error")
    }

    async fn create_shortcut_retry(
        &self,
        name: &str,
        destination_parent_id: &str,
        target_id: &str,
        target_resource_key: Option<&str>,
        props: &AppProperties,
    ) -> anyhow::Result<DriveFile> {
        let mut refreshed = false;
        for attempt in 0..=self.config.engine.max_retry_attempts {
            let access_token = self.token_manager.access_token("default").await?;
            match self
                .drive
                .create_shortcut(
                    access_token.as_str(),
                    name,
                    destination_parent_id,
                    target_id,
                    target_resource_key,
                    Some(props),
                )
                .await
            {
                Ok(value) => return Ok(value),
                Err(err) => {
                    self.handle_drive_retry(err, attempt, &mut refreshed)
                        .await?
                }
            }
        }
        unreachable!("retry loop returns on final non-retryable error")
    }

    async fn handle_drive_retry(
        &self,
        err: DriveApiError,
        attempt: u32,
        refreshed: &mut bool,
    ) -> anyhow::Result<()> {
        let decision = match &err {
            DriveApiError::Api { status, reason, .. } => classify_http(*status, reason.as_deref()),
            DriveApiError::Transport(_) => RetryDecision::Retry,
        };

        match decision {
            RetryDecision::RefreshTokenOnce if !*refreshed => {
                *refreshed = true;
                self.token_manager.invalidate().await;
                Ok(())
            }
            RetryDecision::Retry if attempt < self.config.engine.max_retry_attempts => {
                let backoff = backoff_delay(
                    attempt,
                    Duration::from_millis(self.config.engine.retry_base_delay_ms),
                    Duration::from_millis(self.config.engine.retry_max_delay_ms),
                );
                // Honor the server's Retry-After when present: pause every
                // worker through the shared pacer and wait at least that long
                // ourselves instead of blindly re-hitting the quota.
                let delay = match err.retry_after() {
                    Some(retry_after) => {
                        self.drive.pacer().force_open(retry_after);
                        retry_after.max(backoff)
                    }
                    None => backoff,
                };
                tokio::time::sleep(delay).await;
                Ok(())
            }
            _ => Err(err.into()),
        }
    }

    async fn write_report_best_effort(&self, job_id: &str) -> Option<report::ReportPaths> {
        match report::write_job_reports(&self.db, &self.config.storage.report_dir, job_id).await {
            Ok(paths) => Some(paths),
            Err(err) => {
                warn!(job_id, error = %err, "write job report failed");
                None
            }
        }
    }
}

pub fn create_folder_request_json(
    name: &str,
    destination_parent_id: &str,
    app_properties: &AppProperties,
) -> String {
    json!({
        "name": name,
        "mimeType": FOLDER_MIME_TYPE,
        "parents": [destination_parent_id],
        "appProperties": app_properties,
    })
    .to_string()
}

pub fn copy_file_request_json(
    name: &str,
    destination_parent_id: &str,
    app_properties: &AppProperties,
) -> String {
    json!({
        "name": name,
        "parents": [destination_parent_id],
        "appProperties": app_properties,
    })
    .to_string()
}

pub fn create_shortcut_request_json(
    name: &str,
    destination_parent_id: &str,
    target_id: &str,
    target_resource_key: Option<&str>,
    app_properties: &AppProperties,
) -> String {
    let mut body = json!({
        "name": name,
        "mimeType": SHORTCUT_MIME_TYPE,
        "parents": [destination_parent_id],
        "shortcutDetails": {
            "targetId": target_id,
        },
        "appProperties": app_properties,
    });
    if let Some(resource_key) = target_resource_key {
        body["shortcutDetails"]["targetResourceKey"] = json!(resource_key);
    }
    body.to_string()
}

fn validate_source_for_clone(source: &DriveFile) -> anyhow::Result<()> {
    if source.trashed == Some(true) {
        bail!("File/folder nguồn đang nằm trong thùng rác");
    }
    if source.is_folder() {
        if source
            .capabilities
            .as_ref()
            .and_then(|cap| cap.can_list_children)
            != Some(true)
        {
            bail!("Tài khoản Google hiện tại không có quyền đọc folder nguồn");
        }
    } else if source.is_shortcut() {
        if source.shortcut_details.is_none() {
            bail!("Shortcut nguồn thiếu thông tin đích");
        }
    } else if source.capabilities.as_ref().and_then(|cap| cap.can_copy) != Some(true) {
        bail!("Tài khoản Google hiện tại không có quyền copy item nguồn");
    }
    Ok(())
}

fn item_kind(source: &DriveFile) -> &'static str {
    if source.is_folder() {
        "folder"
    } else if source.is_shortcut() {
        "shortcut"
    } else if source.mime_type.starts_with("application/vnd.google-apps.") {
        "google_native"
    } else if source.mime_type == SHORTCUT_MIME_TYPE {
        "shortcut"
    } else {
        "binary"
    }
}

#[cfg(test)]
mod tests {
    use super::{
        CloneRequest, DUPLICATE_POLICIES, DriveReference, MAX_NAME_OVERRIDE_LEN,
        validate_clone_request,
    };

    fn base_request() -> CloneRequest {
        CloneRequest {
            chat_id: 1,
            telegram_user_id: 1,
            source: DriveReference {
                file_id: "src".into(),
                resource_key: None,
                hinted_kind: None,
            },
            progress_message_id: None,
            name_override: None,
            destination_parent_id: None,
            duplicate_policy: None,
        }
    }

    #[test]
    fn accepts_defaults_and_valid_overrides() {
        assert!(validate_clone_request(&base_request()).is_ok());

        let mut request = base_request();
        request.name_override = Some("  Bản sao đích  ".into());
        request.destination_parent_id = Some(" folder-abc ".into());
        request.duplicate_policy = Some("replace_safe".into());
        assert!(validate_clone_request(&request).is_ok());
    }

    #[test]
    fn rejects_blank_name_override() {
        let mut request = base_request();
        request.name_override = Some("   ".into());
        assert!(validate_clone_request(&request).is_err());
    }

    #[test]
    fn rejects_too_long_name_override() {
        let mut request = base_request();
        request.name_override = Some("x".repeat(MAX_NAME_OVERRIDE_LEN + 1));
        assert!(validate_clone_request(&request).is_err());
        request.name_override = Some("x".repeat(MAX_NAME_OVERRIDE_LEN));
        assert!(validate_clone_request(&request).is_ok());
    }

    #[test]
    fn rejects_blank_destination_parent() {
        let mut request = base_request();
        request.destination_parent_id = Some("  ".into());
        assert!(validate_clone_request(&request).is_err());
    }

    #[test]
    fn rejects_unknown_duplicate_policy() {
        let mut request = base_request();
        request.duplicate_policy = Some("overwrite_everything".into());
        assert!(validate_clone_request(&request).is_err());

        for policy in DUPLICATE_POLICIES {
            request.duplicate_policy = Some((*policy).to_string());
            assert!(validate_clone_request(&request).is_ok());
        }
    }
}
