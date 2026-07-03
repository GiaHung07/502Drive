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

#[derive(Debug, Clone)]
pub struct CloneRequest {
    pub chat_id: i64,
    pub telegram_user_id: i64,
    pub source: DriveReference,
    pub progress_message_id: Option<i32>,
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
    pub fn new(config: AppConfig, db: Database) -> Self {
        let token_manager = TokenManager::new(config.clone(), db.clone());
        let initial_write_concurrency = config.engine.initial_write_concurrency;
        Self {
            drive: DriveClient::with_timeout(Duration::from_secs(
                config.engine.request_timeout_seconds,
            )),
            config,
            db,
            token_manager,
            copy_limiter: CopyLimiter::new(initial_write_concurrency),
        }
    }

    pub async fn start_one_shot(&self, request: CloneRequest) -> anyhow::Result<CloneOutcome> {
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

        let destination = repo::default_destination_profile(&self.db, "default")
            .await?
            .context("No default destination configured. Use /set_destination first")?;
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
                destination_parent_id: destination.destination_parent_id.clone(),
                destination_drive_id: destination.destination_drive_id.clone(),
                progress_message_id: request.progress_message_id,
            },
        )
        .await?;

        let result = if source.is_folder() {
            self.create_folder_root(
                &job_id,
                &source,
                &destination.destination_parent_id,
                &request.source,
            )
            .await
        } else {
            self.copy_single_file(
                &job_id,
                &source,
                &destination.destination_parent_id,
                &request.source,
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
            },
        )
        .await?;

        let result = if source.is_folder() {
            self.create_folder_root(&job_id, &source, &destination_parent_id, &request.source)
                .await
        } else {
            self.copy_single_file(&job_id, &source, &destination_parent_id, &request.source)
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
            )
            .await?
        };

        Ok(CloneOutcome {
            job_id: job.id.clone(),
            message,
            report_paths: self.write_report_best_effort(&job.id).await,
        })
    }

    async fn create_folder_root(
        &self,
        job_id: &str,
        source: &DriveFile,
        destination_parent_id: &str,
        source_reference: &DriveReference,
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
                request_json: create_folder_request_json(
                    &source.name,
                    destination_parent_id,
                    &props,
                ),
            },
        )
        .await?;

        let destination = self
            .create_folder_idempotent(&source.name, destination_parent_id, &props)
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
            "Job {job_id} completed. Discovered: {} Completed: {} Failed: {} Skipped: {}",
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
                &source.name,
                destination_parent_id,
                &details.target_id,
                details.target_resource_key.as_deref(),
                &props,
            )
        } else {
            copy_file_request_json(&source.name, destination_parent_id, &props)
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
            self.create_shortcut_idempotent(source, destination_parent_id, &props)
                .await?
        } else {
            self.copy_file_idempotent(
                source_reference,
                &source.name,
                destination_parent_id,
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
                source_parent_id: source.parents.first().cloned(),
                destination_parent_id: Some(destination_parent_id.to_string()),
                mime_type: source.mime_type.clone(),
                source_name: source.name.clone(),
            },
        )
        .await?;
        repo::update_job_counts(&self.db, job_id, 1, 1, 0, 0).await?;
        repo::update_job_status(&self.db, job_id, JobStatusValue::Completed, None).await?;

        Ok(format!(
            "Job {job_id} completed. Copied file: {}",
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
            self.create_shortcut_idempotent(source, &parent.destination_folder_id, &props)
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
                &source.name,
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
                tokio::time::sleep(backoff_delay(
                    attempt,
                    Duration::from_millis(self.config.engine.retry_base_delay_ms),
                    Duration::from_millis(self.config.engine.retry_max_delay_ms),
                ))
                .await;
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
        bail!("Source item is trashed");
    }
    if source.is_folder() {
        if source
            .capabilities
            .as_ref()
            .and_then(|cap| cap.can_list_children)
            != Some(true)
        {
            bail!("Authenticated account cannot list this source folder");
        }
    } else if source.is_shortcut() {
        if source.shortcut_details.is_none() {
            bail!("Shortcut is missing target details");
        }
    } else if source.capabilities.as_ref().and_then(|cap| cap.can_copy) != Some(true) {
        bail!("Authenticated account cannot copy this source item");
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
