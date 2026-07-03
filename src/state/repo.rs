use rusqlite::{OptionalExtension, params};
use uuid::Uuid;

use super::db::{Database, now_ms};
use crate::engine::operation::{OperationStatus, OperationType};

#[derive(Debug, Clone, Copy)]
pub enum JobStatusValue {
    Queued,
    Discovering,
    Running,
    Pausing,
    Paused,
    Cancelling,
    Cancelled,
    Recovering,
    Completed,
    PartiallyCompleted,
    Failed,
}

#[derive(Debug, Clone)]
pub struct TraversalFolder {
    pub job_id: String,
    pub source_folder_id: String,
    pub source_resource_key: Option<String>,
    pub destination_folder_id: String,
    pub source_parent_id: Option<String>,
    pub next_page_token: Option<String>,
}

#[derive(Debug, Clone, Default)]
pub struct JobCounts {
    pub total_discovered: i64,
    pub completed_items: i64,
    pub failed_items: i64,
    pub skipped_items: i64,
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct StartupRecoverySummary {
    pub jobs_marked_recovering: usize,
    pub traversal_folders_requeued: usize,
    pub operation_intents_replanned: usize,
    pub job_items_requeued: usize,
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct RetryFailedSummary {
    pub traversal_folders_requeued: usize,
    pub job_items_requeued: usize,
    pub operation_intents_replanned: usize,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NewCallbackState {
    pub telegram_user_id: i64,
    pub chat_id: i64,
    pub action: String,
    pub payload: String,
    pub ttl_ms: i64,
}

impl RetryFailedSummary {
    pub fn is_empty(&self) -> bool {
        self.traversal_folders_requeued == 0
            && self.job_items_requeued == 0
            && self.operation_intents_replanned == 0
    }
}

impl JobStatusValue {
    pub fn as_str(self) -> &'static str {
        match self {
            JobStatusValue::Queued => "queued",
            JobStatusValue::Discovering => "discovering",
            JobStatusValue::Running => "running",
            JobStatusValue::Pausing => "pausing",
            JobStatusValue::Paused => "paused",
            JobStatusValue::Cancelling => "cancelling",
            JobStatusValue::Cancelled => "cancelled",
            JobStatusValue::Recovering => "recovering",
            JobStatusValue::Completed => "completed",
            JobStatusValue::PartiallyCompleted => "partially_completed",
            JobStatusValue::Failed => "failed",
        }
    }
}

pub async fn create_callback_state(
    db: &Database,
    state: NewCallbackState,
) -> anyhow::Result<String> {
    let id = Uuid::new_v4().to_string();
    let now = now_ms();
    let expires_at_ms = now + state.ttl_ms.max(1);
    let insert_id = id.clone();
    db.conn()
        .call(move |conn| {
            conn.execute(
                "INSERT INTO telegram_callback_states (
                    id, telegram_user_id, chat_id, action, payload,
                    expires_at_ms, created_at_ms
                 ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
                params![
                    insert_id,
                    state.telegram_user_id,
                    state.chat_id,
                    state.action,
                    state.payload,
                    expires_at_ms,
                    now,
                ],
            )?;
            Ok::<(), rusqlite::Error>(())
        })
        .await?;
    Ok(id)
}

pub async fn consume_callback_state(
    db: &Database,
    id: &str,
    telegram_user_id: i64,
    chat_id: i64,
    action: &str,
) -> anyhow::Result<Option<String>> {
    let id = id.to_string();
    let action = action.to_string();
    Ok(db
        .conn()
        .call(move |conn| {
            let tx = conn.transaction()?;
            let now = now_ms();
            let payload = tx
                .query_row(
                    "SELECT payload
                     FROM telegram_callback_states
                     WHERE id = ?1
                       AND telegram_user_id = ?2
                       AND chat_id = ?3
                       AND action = ?4
                       AND expires_at_ms > ?5",
                    params![id, telegram_user_id, chat_id, action, now],
                    |row| row.get::<_, String>(0),
                )
                .optional()?;
            if payload.is_some() {
                tx.execute(
                    "DELETE FROM telegram_callback_states WHERE id = ?1",
                    params![id],
                )?;
            }
            tx.commit()?;
            Ok::<Option<String>, rusqlite::Error>(payload)
        })
        .await?)
}

pub async fn delete_expired_callback_states(db: &Database) -> anyhow::Result<usize> {
    Ok(db
        .conn()
        .call(move |conn| {
            conn.execute(
                "DELETE FROM telegram_callback_states WHERE expires_at_ms <= ?1",
                params![now_ms()],
            )
        })
        .await?)
}

pub async fn job_status(db: &Database, job_id: &str) -> anyhow::Result<Option<String>> {
    let job_id = job_id.to_string();
    Ok(db
        .conn()
        .call(move |conn| {
            conn.query_row(
                "SELECT status FROM jobs WHERE id = ?1",
                params![job_id],
                |row| row.get(0),
            )
            .optional()
        })
        .await?)
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct JobStatusCount {
    pub status: String,
    pub count: i64,
}

pub async fn job_status_counts(db: &Database) -> anyhow::Result<Vec<JobStatusCount>> {
    Ok(db
        .conn()
        .call(move |conn| {
            let mut stmt = conn.prepare(
                "SELECT status, COUNT(*)
                 FROM jobs
                 GROUP BY status
                 ORDER BY status",
            )?;
            let rows = stmt
                .query_map([], |row| {
                    Ok(JobStatusCount {
                        status: row.get(0)?,
                        count: row.get(1)?,
                    })
                })?
                .collect::<Result<Vec<_>, _>>()?;
            Ok::<Vec<JobStatusCount>, rusqlite::Error>(rows)
        })
        .await?)
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct JobSummary {
    pub id: String,
    pub kind: String,
    pub status: String,
    pub total_discovered: i64,
    pub completed_items: i64,
    pub failed_items: i64,
    pub skipped_items: i64,
    pub updated_at_ms: i64,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct JobDetail {
    pub id: String,
    pub kind: String,
    pub status: String,
    pub source_root_id: String,
    pub destination_parent_id: String,
    pub total_discovered: i64,
    pub completed_items: i64,
    pub failed_items: i64,
    pub skipped_items: i64,
    pub error_summary: Option<String>,
    pub created_at_ms: i64,
    pub updated_at_ms: i64,
}

pub async fn list_active_jobs_for_user(
    db: &Database,
    telegram_user_id: i64,
    limit: usize,
) -> anyhow::Result<Vec<JobSummary>> {
    Ok(db
        .conn()
        .call(move |conn| {
            let mut stmt = conn.prepare(
                "SELECT id, kind, status, total_discovered, completed_items,
                        failed_items, skipped_items, updated_at_ms
                 FROM jobs
                 WHERE telegram_user_id = ?1
                   AND status IN (
                       'queued', 'discovering', 'running', 'pausing',
                       'paused', 'cancelling', 'recovering'
                   )
                 ORDER BY updated_at_ms DESC, id
                 LIMIT ?2",
            )?;
            let rows = stmt
                .query_map(params![telegram_user_id, limit as i64], |row| {
                    Ok(JobSummary {
                        id: row.get(0)?,
                        kind: row.get(1)?,
                        status: row.get(2)?,
                        total_discovered: row.get(3)?,
                        completed_items: row.get(4)?,
                        failed_items: row.get(5)?,
                        skipped_items: row.get(6)?,
                        updated_at_ms: row.get(7)?,
                    })
                })?
                .collect::<Result<Vec<_>, _>>()?;
            Ok::<Vec<JobSummary>, rusqlite::Error>(rows)
        })
        .await?)
}

pub async fn latest_reportable_job_for_user(
    db: &Database,
    telegram_user_id: i64,
) -> anyhow::Result<Option<JobSummary>> {
    Ok(db
        .conn()
        .call(move |conn| {
            conn.query_row(
                "SELECT id, kind, status, total_discovered, completed_items,
                        failed_items, skipped_items, updated_at_ms
                 FROM jobs
                 WHERE telegram_user_id = ?1
                   AND status IN ('completed', 'partially_completed', 'failed')
                 ORDER BY updated_at_ms DESC, id
                 LIMIT 1",
                params![telegram_user_id],
                |row| {
                    Ok(JobSummary {
                        id: row.get(0)?,
                        kind: row.get(1)?,
                        status: row.get(2)?,
                        total_discovered: row.get(3)?,
                        completed_items: row.get(4)?,
                        failed_items: row.get(5)?,
                        skipped_items: row.get(6)?,
                        updated_at_ms: row.get(7)?,
                    })
                },
            )
            .optional()
        })
        .await?)
}

pub async fn active_job_count_for_user(
    db: &Database,
    telegram_user_id: i64,
) -> anyhow::Result<i64> {
    Ok(db
        .conn()
        .call(move |conn| {
            conn.query_row(
                "SELECT COUNT(*)
                 FROM jobs
                 WHERE telegram_user_id = ?1
                   AND status IN (
                       'queued', 'discovering', 'running', 'pausing',
                       'paused', 'cancelling', 'recovering'
                   )",
                params![telegram_user_id],
                |row| row.get(0),
            )
        })
        .await?)
}

pub async fn active_job_count(db: &Database) -> anyhow::Result<i64> {
    Ok(db
        .conn()
        .call(move |conn| {
            conn.query_row(
                "SELECT COUNT(*)
                 FROM jobs
                 WHERE status IN (
                     'queued', 'discovering', 'running', 'pausing',
                     'paused', 'cancelling', 'recovering'
                 )",
                [],
                |row| row.get(0),
            )
        })
        .await?)
}

pub async fn job_detail_for_user(
    db: &Database,
    telegram_user_id: i64,
    job_id: &str,
) -> anyhow::Result<Option<JobDetail>> {
    let job_id = job_id.to_string();
    Ok(db
        .conn()
        .call(move |conn| {
            conn.query_row(
                "SELECT id, kind, status, source_root_id, destination_parent_id,
                        total_discovered, completed_items, failed_items, skipped_items,
                        error_summary, created_at_ms, updated_at_ms
                 FROM jobs
                 WHERE telegram_user_id = ?1 AND id = ?2",
                params![telegram_user_id, job_id],
                |row| {
                    Ok(JobDetail {
                        id: row.get(0)?,
                        kind: row.get(1)?,
                        status: row.get(2)?,
                        source_root_id: row.get(3)?,
                        destination_parent_id: row.get(4)?,
                        total_discovered: row.get(5)?,
                        completed_items: row.get(6)?,
                        failed_items: row.get(7)?,
                        skipped_items: row.get(8)?,
                        error_summary: row.get(9)?,
                        created_at_ms: row.get(10)?,
                        updated_at_ms: row.get(11)?,
                    })
                },
            )
            .optional()
        })
        .await?)
}

pub async fn job_detail_for_progress_message(
    db: &Database,
    telegram_user_id: i64,
    progress_message_id: i32,
) -> anyhow::Result<Option<JobDetail>> {
    Ok(db
        .conn()
        .call(move |conn| {
            conn.query_row(
                "SELECT id, kind, status, source_root_id, destination_parent_id,
                        total_discovered, completed_items, failed_items, skipped_items,
                        error_summary, created_at_ms, updated_at_ms
                 FROM jobs
                 WHERE telegram_user_id = ?1 AND progress_message_id = ?2
                 ORDER BY created_at_ms DESC
                 LIMIT 1",
                params![telegram_user_id, progress_message_id],
                |row| {
                    Ok(JobDetail {
                        id: row.get(0)?,
                        kind: row.get(1)?,
                        status: row.get(2)?,
                        source_root_id: row.get(3)?,
                        destination_parent_id: row.get(4)?,
                        total_discovered: row.get(5)?,
                        completed_items: row.get(6)?,
                        failed_items: row.get(7)?,
                        skipped_items: row.get(8)?,
                        error_summary: row.get(9)?,
                        created_at_ms: row.get(10)?,
                        updated_at_ms: row.get(11)?,
                    })
                },
            )
            .optional()
        })
        .await?)
}

pub async fn recover_interrupted_state(db: &Database) -> anyhow::Result<StartupRecoverySummary> {
    Ok(db
        .conn()
        .call(move |conn| {
            let now = now_ms();
            let tx = conn.transaction()?;

            let jobs_marked_recovering = tx.execute(
                "UPDATE jobs
                 SET status = 'recovering', updated_at_ms = ?1
                 WHERE status IN (
                    'discovering', 'running', 'pausing', 'cancelling'
                 )",
                params![now],
            )?;

            let traversal_folders_requeued = tx.execute(
                "UPDATE traversal_folders
                 SET scan_state = 'pending', updated_at_ms = ?1
                 WHERE scan_state = 'listing'",
                params![now],
            )?;

            let operation_intents_replanned = tx.execute(
                "UPDATE operation_intents
                 SET status = 'planned', updated_at_ms = ?1
                 WHERE status = 'executing'",
                params![now],
            )?;

            let job_items_requeued = tx.execute(
                "UPDATE job_items
                 SET status = 'ready'
                 WHERE status = 'copying'",
                [],
            )?;

            tx.commit()?;

            Ok::<StartupRecoverySummary, rusqlite::Error>(StartupRecoverySummary {
                jobs_marked_recovering,
                traversal_folders_requeued,
                operation_intents_replanned,
                job_items_requeued,
            })
        })
        .await?)
}

pub async fn is_authorized(db: &Database, telegram_user_id: i64) -> anyhow::Result<bool> {
    Ok(db
        .conn()
        .call(move |conn| {
            let exists: Option<i64> = conn
                .query_row(
                    "SELECT telegram_user_id
                     FROM authorized_users
                     WHERE telegram_user_id = ?1 AND enabled = 1",
                    [telegram_user_id],
                    |row| row.get(0),
                )
                .optional()?;
            Ok::<bool, rusqlite::Error>(exists.is_some())
        })
        .await?)
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AuthorizedUser {
    pub telegram_user_id: i64,
    pub role: String,
    pub enabled: bool,
}

pub async fn authorized_user(
    db: &Database,
    telegram_user_id: i64,
) -> anyhow::Result<Option<AuthorizedUser>> {
    Ok(db
        .conn()
        .call(move |conn| {
            conn.query_row(
                "SELECT telegram_user_id, role, enabled
                 FROM authorized_users
                 WHERE telegram_user_id = ?1",
                params![telegram_user_id],
                |row| {
                    Ok(AuthorizedUser {
                        telegram_user_id: row.get(0)?,
                        role: row.get(1)?,
                        enabled: row.get::<_, i64>(2)? == 1,
                    })
                },
            )
            .optional()
        })
        .await?)
}

pub async fn is_owner(db: &Database, telegram_user_id: i64) -> anyhow::Result<bool> {
    Ok(matches!(
        authorized_user(db, telegram_user_id).await?,
        Some(user) if user.enabled && user.role == "owner"
    ))
}

pub async fn grant_operator(db: &Database, telegram_user_id: i64) -> anyhow::Result<()> {
    db.conn()
        .call(move |conn| {
            let now = now_ms();
            conn.execute(
                "INSERT INTO authorized_users
                    (telegram_user_id, role, enabled, created_at_ms)
                 VALUES (?1, 'operator', 1, ?2)
                 ON CONFLICT(telegram_user_id) DO UPDATE SET
                    role = CASE
                        WHEN authorized_users.role = 'owner' THEN 'owner'
                        ELSE 'operator'
                    END,
                    enabled = 1",
                params![telegram_user_id, now],
            )?;
            Ok::<(), rusqlite::Error>(())
        })
        .await?;
    Ok(())
}

pub async fn revoke_operator(db: &Database, telegram_user_id: i64) -> anyhow::Result<bool> {
    Ok(db
        .conn()
        .call(move |conn| {
            let changed = conn.execute(
                "UPDATE authorized_users
                 SET enabled = 0
                 WHERE telegram_user_id = ?1 AND role != 'owner' AND enabled = 1",
                params![telegram_user_id],
            )?;
            Ok::<bool, rusqlite::Error>(changed > 0)
        })
        .await?)
}

pub async fn upsert_google_account(
    db: &Database,
    label: &str,
    email: Option<&str>,
    refresh_token_ciphertext: Vec<u8>,
    secret_backend: &str,
    scopes_json: &str,
) -> anyhow::Result<String> {
    let id = "default".to_string();
    let label = label.to_string();
    let email = email.map(ToOwned::to_owned);
    let secret_backend = secret_backend.to_string();
    let scopes_json = scopes_json.to_string();
    let id_for_db = id.clone();
    db.conn()
        .call(move |conn| {
            let now = now_ms();
            conn.execute(
                "INSERT INTO google_accounts (
                    id, label, email, refresh_token_ciphertext, refresh_token_nonce,
                    secret_backend, scopes_json, status, created_at_ms, updated_at_ms
                 ) VALUES (?1, ?2, ?3, ?4, NULL, ?5, ?6, 'connected', ?7, ?7)
                 ON CONFLICT(id) DO UPDATE SET
                    label = excluded.label,
                    email = excluded.email,
                    refresh_token_ciphertext = excluded.refresh_token_ciphertext,
                    secret_backend = excluded.secret_backend,
                    scopes_json = excluded.scopes_json,
                    status = 'connected',
                    updated_at_ms = excluded.updated_at_ms",
                params![
                    id_for_db,
                    label,
                    email,
                    refresh_token_ciphertext,
                    secret_backend,
                    scopes_json,
                    now,
                ],
            )?;
            Ok::<(), rusqlite::Error>(())
        })
        .await?;
    Ok(id)
}

pub async fn mark_account_revoked(db: &Database, account_id: &str) -> anyhow::Result<()> {
    let account_id = account_id.to_string();
    db.conn()
        .call(move |conn| {
            conn.execute(
                "UPDATE google_accounts SET status = 'revoked', updated_at_ms = ?2 WHERE id = ?1",
                params![account_id, now_ms()],
            )?;
            Ok::<(), rusqlite::Error>(())
        })
        .await?;
    Ok(())
}

pub async fn mark_account_reconnect_required(
    db: &Database,
    account_id: &str,
) -> anyhow::Result<()> {
    let account_id = account_id.to_string();
    db.conn()
        .call(move |conn| {
            conn.execute(
                "UPDATE google_accounts
                 SET status = 'reconnect_required', updated_at_ms = ?2
                 WHERE id = ?1",
                params![account_id, now_ms()],
            )?;
            Ok::<(), rusqlite::Error>(())
        })
        .await?;
    Ok(())
}

pub async fn account_status(db: &Database) -> anyhow::Result<Option<String>> {
    Ok(db
        .conn()
        .call(|conn| {
            conn.query_row(
                "SELECT status FROM google_accounts WHERE id = 'default'",
                [],
                |row| row.get(0),
            )
            .optional()
        })
        .await?)
}

#[derive(Debug, Clone)]
pub struct GoogleAccountSecret {
    pub id: String,
    pub label: String,
    pub email: Option<String>,
    pub refresh_token_ciphertext: Vec<u8>,
    pub secret_backend: String,
    pub status: String,
}

pub async fn google_account_secret(
    db: &Database,
    account_id: &str,
) -> anyhow::Result<Option<GoogleAccountSecret>> {
    let account_id = account_id.to_string();
    Ok(db
        .conn()
        .call(move |conn| {
            conn.query_row(
                "SELECT id, label, email, refresh_token_ciphertext, secret_backend, status
                 FROM google_accounts
                 WHERE id = ?1",
                params![account_id],
                |row| {
                    Ok(GoogleAccountSecret {
                        id: row.get(0)?,
                        label: row.get(1)?,
                        email: row.get(2)?,
                        refresh_token_ciphertext: row.get(3)?,
                        secret_backend: row.get(4)?,
                        status: row.get(5)?,
                    })
                },
            )
            .optional()
        })
        .await?)
}

pub async fn create_job(
    db: &Database,
    chat_id: i64,
    telegram_user_id: i64,
    google_account_id: &str,
    source_root_id: &str,
    destination_parent_id: &str,
) -> anyhow::Result<String> {
    create_job_with_metadata(
        db,
        NewJob {
            chat_id,
            telegram_user_id,
            google_account_id: google_account_id.to_string(),
            source_root_id: source_root_id.to_string(),
            source_resource_key: None,
            source_drive_id: None,
            destination_parent_id: destination_parent_id.to_string(),
            destination_drive_id: None,
            progress_message_id: None,
        },
    )
    .await
}

#[derive(Debug, Clone)]
pub struct NewJob {
    pub chat_id: i64,
    pub telegram_user_id: i64,
    pub google_account_id: String,
    pub source_root_id: String,
    pub source_resource_key: Option<String>,
    pub source_drive_id: Option<String>,
    pub destination_parent_id: String,
    pub destination_drive_id: Option<String>,
    pub progress_message_id: Option<i32>,
}

pub async fn create_job_with_metadata(db: &Database, job: NewJob) -> anyhow::Result<String> {
    let id = Uuid::new_v4().to_string();
    let id_for_db = id.clone();
    db.conn()
        .call(move |conn| {
            let now = now_ms();
            conn.execute(
                "INSERT INTO jobs (
                    id, kind, telegram_user_id, chat_id, google_account_id,
                    source_root_id, source_resource_key, source_drive_id,
                    destination_parent_id, destination_drive_id, status,
                    duplicate_policy, shortcut_policy, progress_message_id, created_at_ms, updated_at_ms
                 ) VALUES (
                    ?1, 'one_shot', ?2, ?3, ?4,
                    ?5, ?6, ?7,
                    ?8, ?9, 'queued',
                    'skip_same_source', 'preserve', ?10, ?11, ?11
                 )",
                params![
                    id_for_db,
                    job.telegram_user_id,
                    job.chat_id,
                    job.google_account_id,
                    job.source_root_id,
                    job.source_resource_key,
                    job.source_drive_id,
                    job.destination_parent_id,
                    job.destination_drive_id,
                    job.progress_message_id,
                    now,
                ],
            )?;
            Ok::<(), rusqlite::Error>(())
        })
        .await?;
    Ok(id)
}

#[derive(Debug, Clone)]
pub struct ResumableOneShotJob {
    pub id: String,
    pub telegram_user_id: i64,
    pub chat_id: i64,
    pub google_account_id: String,
    pub source_root_id: String,
    pub source_resource_key: Option<String>,
    pub destination_parent_id: String,
}

pub async fn resumable_one_shot_jobs(db: &Database) -> anyhow::Result<Vec<ResumableOneShotJob>> {
    Ok(db
        .conn()
        .call(move |conn| {
            let mut stmt = conn.prepare(
                "SELECT id, telegram_user_id, chat_id, google_account_id,
                        source_root_id, source_resource_key, destination_parent_id
                 FROM jobs
                 WHERE kind = 'one_shot' AND status IN ('queued', 'recovering')
                 ORDER BY created_at_ms, id",
            )?;
            let rows = stmt
                .query_map([], |row| {
                    Ok(ResumableOneShotJob {
                        id: row.get(0)?,
                        telegram_user_id: row.get(1)?,
                        chat_id: row.get(2)?,
                        google_account_id: row.get(3)?,
                        source_root_id: row.get(4)?,
                        source_resource_key: row.get(5)?,
                        destination_parent_id: row.get(6)?,
                    })
                })?
                .collect::<Result<Vec<_>, _>>()?;
            Ok::<Vec<ResumableOneShotJob>, rusqlite::Error>(rows)
        })
        .await?)
}

pub async fn update_job_status(
    db: &Database,
    job_id: &str,
    status: JobStatusValue,
    error_summary: Option<&str>,
) -> anyhow::Result<()> {
    let job_id = job_id.to_string();
    let error_summary = error_summary.map(ToOwned::to_owned);
    db.conn()
        .call(move |conn| {
            conn.execute(
                "UPDATE jobs
                 SET status = ?1, error_summary = ?2, updated_at_ms = ?3
                 WHERE id = ?4",
                params![status.as_str(), error_summary, now_ms(), job_id],
            )?;
            Ok::<(), rusqlite::Error>(())
        })
        .await?;
    Ok(())
}

pub async fn pause_job_for_user(
    db: &Database,
    telegram_user_id: i64,
    job_id: &str,
) -> anyhow::Result<bool> {
    update_job_status_for_user(
        db,
        telegram_user_id,
        job_id,
        "pausing",
        &["discovering", "running", "recovering"],
    )
    .await
}

pub async fn resume_job_for_user(
    db: &Database,
    telegram_user_id: i64,
    job_id: &str,
) -> anyhow::Result<bool> {
    update_job_status_for_user(db, telegram_user_id, job_id, "recovering", &["paused"]).await
}

pub async fn cancel_job_for_user(
    db: &Database,
    telegram_user_id: i64,
    job_id: &str,
) -> anyhow::Result<bool> {
    let job_id = job_id.to_string();
    Ok(db
        .conn()
        .call(move |conn| {
            let now = now_ms();
            let tx = conn.transaction()?;
            let cancelled = tx.execute(
                "UPDATE jobs
                 SET status = 'cancelled', updated_at_ms = ?1
                 WHERE telegram_user_id = ?2
                   AND id = ?3
                   AND status IN ('queued', 'paused')",
                params![now, telegram_user_id, job_id],
            )?;
            let cancelling = tx.execute(
                "UPDATE jobs
                 SET status = 'cancelling', updated_at_ms = ?1
                 WHERE telegram_user_id = ?2
                   AND id = ?3
                   AND status IN ('discovering', 'running', 'pausing', 'recovering')",
                params![now, telegram_user_id, job_id],
            )?;
            tx.commit()?;
            Ok::<bool, rusqlite::Error>(cancelled + cancelling > 0)
        })
        .await?)
}

pub async fn retry_failed_job_for_user(
    db: &Database,
    telegram_user_id: i64,
    job_id: &str,
) -> anyhow::Result<Option<RetryFailedSummary>> {
    let job_id = job_id.to_string();
    Ok(db
        .conn()
        .call(move |conn| {
            let now = now_ms();
            let tx = conn.transaction()?;
            let status = tx
                .query_row(
                    "SELECT status
                     FROM jobs
                     WHERE telegram_user_id = ?1 AND id = ?2",
                    params![telegram_user_id, job_id],
                    |row| row.get::<_, String>(0),
                )
                .optional()?;
            let Some(status) = status else {
                return Ok::<Option<RetryFailedSummary>, rusqlite::Error>(None);
            };
            if !matches!(
                status.as_str(),
                "paused" | "completed" | "partially_completed" | "failed"
            ) {
                return Ok(None);
            }

            let traversal_folders_requeued = tx.execute(
                "UPDATE traversal_folders
                 SET scan_state = 'pending', last_error = NULL, updated_at_ms = ?1
                 WHERE job_id = ?2 AND scan_state = 'failed'",
                params![now, job_id],
            )?;
            let job_items_requeued = tx.execute(
                "UPDATE job_items
                 SET status = 'ready',
                     attempts = 0,
                     last_error_code = NULL,
                     last_error_message = NULL
                 WHERE job_id = ?1 AND status = 'failed'",
                params![job_id],
            )?;
            let operation_intents_replanned = tx.execute(
                "UPDATE operation_intents
                 SET status = 'planned', last_error = NULL, updated_at_ms = ?1
                 WHERE job_id = ?2 AND status = 'failed'",
                params![now, job_id],
            )?;

            let summary = RetryFailedSummary {
                traversal_folders_requeued,
                job_items_requeued,
                operation_intents_replanned,
            };
            if summary.is_empty() {
                return Ok(None);
            }

            tx.execute(
                "UPDATE jobs
                 SET status = 'recovering',
                     failed_items = 0,
                     error_summary = NULL,
                     updated_at_ms = ?1
                 WHERE id = ?2",
                params![now, job_id],
            )?;
            tx.commit()?;
            Ok(Some(summary))
        })
        .await?)
}

async fn update_job_status_for_user(
    db: &Database,
    telegram_user_id: i64,
    job_id: &str,
    next_status: &str,
    allowed_current: &[&str],
) -> anyhow::Result<bool> {
    let job_id = job_id.to_string();
    let next_status = next_status.to_string();
    let allowed_current = allowed_current
        .iter()
        .map(|status| format!("'{status}'"))
        .collect::<Vec<_>>()
        .join(",");
    Ok(db
        .conn()
        .call(move |conn| {
            let sql = format!(
                "UPDATE jobs
                 SET status = ?1, updated_at_ms = ?2
                 WHERE telegram_user_id = ?3
                   AND id = ?4
                   AND status IN ({allowed_current})"
            );
            let changed = conn.execute(
                &sql,
                params![next_status, now_ms(), telegram_user_id, job_id],
            )?;
            Ok::<bool, rusqlite::Error>(changed > 0)
        })
        .await?)
}

pub async fn update_job_counts(
    db: &Database,
    job_id: &str,
    total_discovered: i64,
    completed_items: i64,
    failed_items: i64,
    skipped_items: i64,
) -> anyhow::Result<()> {
    let job_id = job_id.to_string();
    db.conn()
        .call(move |conn| {
            conn.execute(
                "UPDATE jobs
                 SET total_discovered = ?1,
                     completed_items = ?2,
                     failed_items = ?3,
                     skipped_items = ?4,
                     updated_at_ms = ?5
                 WHERE id = ?6",
                params![
                    total_discovered,
                    completed_items,
                    failed_items,
                    skipped_items,
                    now_ms(),
                    job_id
                ],
            )?;
            Ok::<(), rusqlite::Error>(())
        })
        .await?;
    Ok(())
}

pub async fn job_counts(db: &Database, job_id: &str) -> anyhow::Result<JobCounts> {
    let job_id = job_id.to_string();
    Ok(db
        .conn()
        .call(move |conn| {
            conn.query_row(
                "SELECT total_discovered, completed_items, failed_items, skipped_items
                 FROM jobs
                 WHERE id = ?1",
                params![job_id],
                |row| {
                    Ok(JobCounts {
                        total_discovered: row.get(0)?,
                        completed_items: row.get(1)?,
                        failed_items: row.get(2)?,
                        skipped_items: row.get(3)?,
                    })
                },
            )
        })
        .await?)
}

pub async fn increment_job_counts(
    db: &Database,
    job_id: &str,
    discovered_delta: i64,
    completed_delta: i64,
    failed_delta: i64,
    skipped_delta: i64,
) -> anyhow::Result<()> {
    let job_id = job_id.to_string();
    db.conn()
        .call(move |conn| {
            conn.execute(
                "UPDATE jobs
                 SET total_discovered = total_discovered + ?1,
                     completed_items = completed_items + ?2,
                     failed_items = failed_items + ?3,
                     skipped_items = skipped_items + ?4,
                     updated_at_ms = ?5
                 WHERE id = ?6",
                params![
                    discovered_delta,
                    completed_delta,
                    failed_delta,
                    skipped_delta,
                    now_ms(),
                    job_id
                ],
            )?;
            Ok::<(), rusqlite::Error>(())
        })
        .await?;
    Ok(())
}

#[derive(Debug, Clone)]
pub struct NewItem {
    pub job_id: String,
    pub source_item_id: String,
    pub destination_parent_id: String,
    pub mime_type: String,
    pub item_kind: String,
    pub source_name: String,
    pub size_bytes: Option<i64>,
}

pub async fn insert_item(db: &Database, item: NewItem) -> anyhow::Result<String> {
    let id = Uuid::new_v4().to_string();
    let values = (
        id.clone(),
        item.job_id,
        item.source_item_id,
        item.destination_parent_id,
        item.mime_type,
        item.item_kind,
        item.source_name,
        item.size_bytes,
    );
    db.conn()
        .call(move |conn| {
            conn.execute(
                "INSERT OR IGNORE INTO job_items (
                    id, job_id, source_item_id, destination_parent_id,
                    mime_type, item_kind, source_name, size_bytes, status
                 ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, 'ready')",
                params![
                    values.0, values.1, values.2, values.3, values.4, values.5, values.6, values.7,
                ],
            )?;
            Ok::<(), rusqlite::Error>(())
        })
        .await?;
    Ok(id)
}

pub async fn record_traversal_folder(
    db: &Database,
    job_id: &str,
    source_folder_id: &str,
    source_resource_key: Option<&str>,
    destination_folder_id: &str,
    source_parent_id: Option<&str>,
) -> anyhow::Result<()> {
    let job_id = job_id.to_string();
    let source_folder_id = source_folder_id.to_string();
    let source_resource_key = source_resource_key.map(ToOwned::to_owned);
    let destination_folder_id = destination_folder_id.to_string();
    let source_parent_id = source_parent_id.map(ToOwned::to_owned);
    db.conn()
        .call(move |conn| {
            conn.execute(
                "INSERT INTO traversal_folders (
                    job_id, source_folder_id, source_resource_key,
                    destination_folder_id, source_parent_id,
                    scan_state, updated_at_ms
                 ) VALUES (?1, ?2, ?3, ?4, ?5, 'pending', ?6)
                 ON CONFLICT(job_id, source_folder_id) DO UPDATE SET
                    destination_folder_id = excluded.destination_folder_id,
                    source_resource_key = excluded.source_resource_key,
                    source_parent_id = excluded.source_parent_id,
                    updated_at_ms = excluded.updated_at_ms",
                params![
                    job_id,
                    source_folder_id,
                    source_resource_key,
                    destination_folder_id,
                    source_parent_id,
                    now_ms()
                ],
            )?;
            Ok::<(), rusqlite::Error>(())
        })
        .await?;
    Ok(())
}

pub async fn next_pending_traversal_folder(
    db: &Database,
    job_id: &str,
) -> anyhow::Result<Option<TraversalFolder>> {
    let job_id = job_id.to_string();
    Ok(db
        .conn()
        .call(move |conn| {
            let tx = conn.transaction()?;
            let folder = tx
                .query_row(
                    "SELECT job_id, source_folder_id, source_resource_key,
                            destination_folder_id, source_parent_id, next_page_token
                     FROM traversal_folders
                     WHERE job_id = ?1 AND scan_state = 'pending'
                     ORDER BY updated_at_ms, source_folder_id
                     LIMIT 1",
                    params![job_id],
                    |row| {
                        Ok(TraversalFolder {
                            job_id: row.get(0)?,
                            source_folder_id: row.get(1)?,
                            source_resource_key: row.get(2)?,
                            destination_folder_id: row.get(3)?,
                            source_parent_id: row.get(4)?,
                            next_page_token: row.get(5)?,
                        })
                    },
                )
                .optional()?;
            if let Some(folder) = &folder {
                tx.execute(
                    "UPDATE traversal_folders
                     SET scan_state = 'listing', updated_at_ms = ?3
                     WHERE job_id = ?1 AND source_folder_id = ?2",
                    params![folder.job_id, folder.source_folder_id, now_ms()],
                )?;
            }
            tx.commit()?;
            Ok::<Option<TraversalFolder>, rusqlite::Error>(folder)
        })
        .await?)
}

pub async fn finish_traversal_folder(
    db: &Database,
    job_id: &str,
    source_folder_id: &str,
) -> anyhow::Result<()> {
    update_traversal_folder_state(db, job_id, source_folder_id, "done", None, None).await
}

pub async fn requeue_traversal_folder_page(
    db: &Database,
    job_id: &str,
    source_folder_id: &str,
    next_page_token: &str,
) -> anyhow::Result<()> {
    update_traversal_folder_state(
        db,
        job_id,
        source_folder_id,
        "pending",
        Some(next_page_token),
        None,
    )
    .await
}

pub async fn fail_traversal_folder(
    db: &Database,
    job_id: &str,
    source_folder_id: &str,
    error: &str,
) -> anyhow::Result<()> {
    update_traversal_folder_state(db, job_id, source_folder_id, "failed", None, Some(error)).await
}

async fn update_traversal_folder_state(
    db: &Database,
    job_id: &str,
    source_folder_id: &str,
    scan_state: &str,
    next_page_token: Option<&str>,
    last_error: Option<&str>,
) -> anyhow::Result<()> {
    let job_id = job_id.to_string();
    let source_folder_id = source_folder_id.to_string();
    let scan_state = scan_state.to_string();
    let next_page_token = next_page_token.map(ToOwned::to_owned);
    let last_error = last_error.map(ToOwned::to_owned);
    db.conn()
        .call(move |conn| {
            conn.execute(
                "UPDATE traversal_folders
                 SET scan_state = ?1, next_page_token = ?2, last_error = ?3, updated_at_ms = ?4
                 WHERE job_id = ?5 AND source_folder_id = ?6",
                params![
                    scan_state,
                    next_page_token,
                    last_error,
                    now_ms(),
                    job_id,
                    source_folder_id
                ],
            )?;
            Ok::<(), rusqlite::Error>(())
        })
        .await?;
    Ok(())
}

pub async fn has_pending_traversal(db: &Database, job_id: &str) -> anyhow::Result<bool> {
    let job_id = job_id.to_string();
    Ok(db
        .conn()
        .call(move |conn| {
            conn.query_row(
                "SELECT EXISTS(
                    SELECT 1 FROM traversal_folders
                    WHERE job_id = ?1 AND scan_state IN ('pending', 'listing')
                 )",
                params![job_id],
                |row| row.get::<_, bool>(0),
            )
        })
        .await?)
}

pub async fn has_traversal_rows(db: &Database, job_id: &str) -> anyhow::Result<bool> {
    let job_id = job_id.to_string();
    Ok(db
        .conn()
        .call(move |conn| {
            conn.query_row(
                "SELECT EXISTS(
                    SELECT 1 FROM traversal_folders
                    WHERE job_id = ?1
                 )",
                params![job_id],
                |row| row.get::<_, bool>(0),
            )
        })
        .await?)
}

pub async fn mark_item_done(
    db: &Database,
    job_id: &str,
    source_item_id: &str,
    dest_item_id: &str,
) -> anyhow::Result<()> {
    let job_id = job_id.to_string();
    let source_item_id = source_item_id.to_string();
    let dest_item_id = dest_item_id.to_string();
    db.conn()
        .call(move |conn| {
            conn.execute(
                "UPDATE job_items
                 SET status = 'done', destination_item_id = ?1, completed_at_ms = ?2
                 WHERE job_id = ?3 AND source_item_id = ?4",
                params![dest_item_id, now_ms(), job_id, source_item_id],
            )?;
            Ok::<(), rusqlite::Error>(())
        })
        .await?;
    Ok(())
}

pub async fn mark_item_copying(
    db: &Database,
    job_id: &str,
    source_item_id: &str,
) -> anyhow::Result<()> {
    let job_id = job_id.to_string();
    let source_item_id = source_item_id.to_string();
    db.conn()
        .call(move |conn| {
            conn.execute(
                "UPDATE job_items
                 SET status = 'copying'
                 WHERE job_id = ?1 AND source_item_id = ?2",
                params![job_id, source_item_id],
            )?;
            Ok::<(), rusqlite::Error>(())
        })
        .await?;
    Ok(())
}

pub async fn mark_item_failed(
    db: &Database,
    job_id: &str,
    source_item_id: &str,
    error_code: &str,
    error_message: &str,
) -> anyhow::Result<()> {
    let job_id = job_id.to_string();
    let source_item_id = source_item_id.to_string();
    let error_code = error_code.to_string();
    let error_message = error_message.to_string();
    db.conn()
        .call(move |conn| {
            conn.execute(
                "UPDATE job_items
                 SET status = 'failed',
                     last_error_code = ?1,
                     last_error_message = ?2
                 WHERE job_id = ?3 AND source_item_id = ?4",
                params![error_code, error_message, job_id, source_item_id],
            )?;
            Ok::<(), rusqlite::Error>(())
        })
        .await?;
    Ok(())
}

pub async fn item_status(
    db: &Database,
    job_id: &str,
    source_item_id: &str,
) -> anyhow::Result<Option<String>> {
    let job_id = job_id.to_string();
    let source_item_id = source_item_id.to_string();
    Ok(db
        .conn()
        .call(move |conn| {
            conn.query_row(
                "SELECT status
                 FROM job_items
                 WHERE job_id = ?1 AND source_item_id = ?2",
                params![job_id, source_item_id],
                |row| row.get(0),
            )
            .optional()
        })
        .await?)
}

pub async fn existing_dest_for_source(
    db: &Database,
    scope_type: &str,
    scope_id: &str,
    source_item_id: &str,
) -> anyhow::Result<Option<String>> {
    let scope_type = scope_type.to_string();
    let scope_id = scope_id.to_string();
    let source_item_id = source_item_id.to_string();
    Ok(db
        .conn()
        .call(move |conn| {
            conn.query_row(
                "SELECT destination_item_id
                 FROM source_mappings
                 WHERE scope_type = ?1 AND scope_id = ?2 AND source_item_id = ?3",
                params![scope_type, scope_id, source_item_id],
                |row| row.get(0),
            )
            .optional()
        })
        .await?)
}

#[derive(Debug, Clone)]
pub struct DestinationProfile {
    pub id: String,
    pub google_account_id: String,
    pub label: String,
    pub destination_parent_id: String,
    pub destination_drive_id: Option<String>,
    pub destination_resource_key: Option<String>,
    pub is_default: bool,
}

#[derive(Debug, Clone)]
pub struct NewDestinationProfile {
    pub google_account_id: String,
    pub label: String,
    pub destination_parent_id: String,
    pub destination_drive_id: Option<String>,
    pub destination_resource_key: Option<String>,
    pub is_default: bool,
}

pub async fn upsert_destination_profile(
    db: &Database,
    profile: NewDestinationProfile,
) -> anyhow::Result<String> {
    let id = Uuid::new_v4().to_string();
    let id_for_db = id.clone();
    db.conn()
        .call(move |conn| {
            let now = now_ms();
            let tx = conn.transaction()?;
            if profile.is_default {
                tx.execute(
                    "UPDATE destination_profiles
                     SET is_default = 0, updated_at_ms = ?2
                     WHERE google_account_id = ?1",
                    params![profile.google_account_id, now],
                )?;
            }
            tx.execute(
                "INSERT INTO destination_profiles (
                    id, google_account_id, label, destination_parent_id,
                    destination_drive_id, destination_resource_key, is_default,
                    last_validated_at_ms, created_at_ms, updated_at_ms
                 ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?8, ?8)",
                params![
                    id_for_db,
                    profile.google_account_id,
                    profile.label,
                    profile.destination_parent_id,
                    profile.destination_drive_id,
                    profile.destination_resource_key,
                    i64::from(profile.is_default),
                    now,
                ],
            )?;
            tx.commit()?;
            Ok::<(), rusqlite::Error>(())
        })
        .await?;
    Ok(id)
}

pub async fn default_destination_profile(
    db: &Database,
    google_account_id: &str,
) -> anyhow::Result<Option<DestinationProfile>> {
    let google_account_id = google_account_id.to_string();
    Ok(db
        .conn()
        .call(move |conn| {
            conn.query_row(
                "SELECT id, google_account_id, label, destination_parent_id,
                        destination_drive_id, destination_resource_key, is_default
                 FROM destination_profiles
                 WHERE google_account_id = ?1 AND is_default = 1",
                params![google_account_id],
                |row| {
                    Ok(DestinationProfile {
                        id: row.get(0)?,
                        google_account_id: row.get(1)?,
                        label: row.get(2)?,
                        destination_parent_id: row.get(3)?,
                        destination_drive_id: row.get(4)?,
                        destination_resource_key: row.get(5)?,
                        is_default: row.get::<_, i64>(6)? == 1,
                    })
                },
            )
            .optional()
        })
        .await?)
}

pub async fn clear_default_destination(
    db: &Database,
    google_account_id: &str,
) -> anyhow::Result<usize> {
    let google_account_id = google_account_id.to_string();
    Ok(db
        .conn()
        .call(move |conn| {
            conn.execute(
                "UPDATE destination_profiles
                 SET is_default = 0, updated_at_ms = ?2
                 WHERE google_account_id = ?1 AND is_default = 1",
                params![google_account_id, now_ms()],
            )
        })
        .await?)
}

/// Return up to `limit` destination profiles ordered by most-recently used.
/// The default profile is always first if present.
pub async fn list_recent_destinations(
    db: &Database,
    google_account_id: &str,
    limit: usize,
) -> anyhow::Result<Vec<DestinationProfile>> {
    let google_account_id = google_account_id.to_string();
    let limit = limit as i64;
    Ok(db
        .conn()
        .call(move |conn| {
            let mut stmt = conn.prepare(
                "SELECT id, google_account_id, label, destination_parent_id,
                        destination_drive_id, destination_resource_key, is_default
                 FROM destination_profiles
                 WHERE google_account_id = ?1
                 ORDER BY is_default DESC, updated_at_ms DESC
                 LIMIT ?2",
            )?;
            let rows = stmt
                .query_map(params![google_account_id, limit], |row| {
                    Ok(DestinationProfile {
                        id: row.get(0)?,
                        google_account_id: row.get(1)?,
                        label: row.get(2)?,
                        destination_parent_id: row.get(3)?,
                        destination_drive_id: row.get(4)?,
                        destination_resource_key: row.get(5)?,
                        is_default: row.get::<_, i64>(6)? == 1,
                    })
                })?
                .collect::<Result<Vec<_>, _>>()?;
            Ok::<Vec<DestinationProfile>, rusqlite::Error>(rows)
        })
        .await?)
}

/// Switch the default destination to a specific profile by ID.
pub async fn set_default_destination_by_id(
    db: &Database,
    google_account_id: &str,
    profile_id: &str,
) -> anyhow::Result<bool> {
    let google_account_id = google_account_id.to_string();
    let profile_id = profile_id.to_string();
    Ok(db
        .conn()
        .call(move |conn| {
            let now = now_ms();
            let tx = conn.transaction()?;
            // Clear all defaults for this account.
            tx.execute(
                "UPDATE destination_profiles SET is_default = 0, updated_at_ms = ?2
                 WHERE google_account_id = ?1",
                params![google_account_id, now],
            )?;
            // Set the chosen profile as default.
            let changed = tx.execute(
                "UPDATE destination_profiles SET is_default = 1, updated_at_ms = ?2
                 WHERE id = ?1 AND google_account_id = ?3",
                params![profile_id, now, google_account_id],
            )?;
            tx.commit()?;
            Ok::<bool, rusqlite::Error>(changed > 0)
        })
        .await?)
}

#[derive(Debug, Clone)]
pub struct NewOperationIntent {
    pub idempotency_key: String,
    pub job_id: Option<String>,
    pub watch_id: Option<String>,
    pub operation_type: OperationType,
    pub source_item_id: Option<String>,
    pub destination_parent_id: Option<String>,
    pub destination_item_id: Option<String>,
    pub request_json: String,
}

#[derive(Debug, Clone)]
pub struct OperationIntent {
    pub id: String,
    pub idempotency_key: String,
    pub status: String,
    pub attempts: i64,
    pub destination_item_id: Option<String>,
}

pub async fn plan_operation_intent(
    db: &Database,
    intent: NewOperationIntent,
) -> anyhow::Result<String> {
    let id = Uuid::new_v4().to_string();
    let id_for_db = id.clone();
    db.conn()
        .call(move |conn| {
            let now = now_ms();
            conn.execute(
                "INSERT OR IGNORE INTO operation_intents (
                    id, idempotency_key, job_id, watch_id, operation_type,
                    source_item_id, destination_parent_id, destination_item_id,
                    request_json, status, created_at_ms, updated_at_ms
                 ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?11)",
                params![
                    id_for_db,
                    intent.idempotency_key,
                    intent.job_id,
                    intent.watch_id,
                    intent.operation_type.as_str(),
                    intent.source_item_id,
                    intent.destination_parent_id,
                    intent.destination_item_id,
                    intent.request_json,
                    OperationStatus::Planned.as_str(),
                    now,
                ],
            )?;
            Ok::<(), rusqlite::Error>(())
        })
        .await?;
    Ok(id)
}

pub async fn find_operation_intent_by_key(
    db: &Database,
    idempotency_key: &str,
) -> anyhow::Result<Option<OperationIntent>> {
    let idempotency_key = idempotency_key.to_string();
    Ok(db
        .conn()
        .call(move |conn| {
            conn.query_row(
                "SELECT id, idempotency_key, status, attempts, destination_item_id
                 FROM operation_intents
                 WHERE idempotency_key = ?1",
                params![idempotency_key],
                |row| {
                    Ok(OperationIntent {
                        id: row.get(0)?,
                        idempotency_key: row.get(1)?,
                        status: row.get(2)?,
                        attempts: row.get(3)?,
                        destination_item_id: row.get(4)?,
                    })
                },
            )
            .optional()
        })
        .await?)
}

pub async fn mark_operation_applied(
    db: &Database,
    idempotency_key: &str,
    destination_item_id: &str,
) -> anyhow::Result<()> {
    let idempotency_key = idempotency_key.to_string();
    let destination_item_id = destination_item_id.to_string();
    db.conn()
        .call(move |conn| {
            conn.execute(
                "UPDATE operation_intents
                 SET status = ?1, destination_item_id = ?2, updated_at_ms = ?3
                 WHERE idempotency_key = ?4",
                params![
                    OperationStatus::Applied.as_str(),
                    destination_item_id,
                    now_ms(),
                    idempotency_key,
                ],
            )?;
            Ok::<(), rusqlite::Error>(())
        })
        .await?;
    Ok(())
}

pub async fn mark_operation_executing(db: &Database, idempotency_key: &str) -> anyhow::Result<()> {
    let idempotency_key = idempotency_key.to_string();
    db.conn()
        .call(move |conn| {
            conn.execute(
                "UPDATE operation_intents
                 SET status = ?1, attempts = attempts + 1, updated_at_ms = ?2
                 WHERE idempotency_key = ?3",
                params![
                    OperationStatus::Executing.as_str(),
                    now_ms(),
                    idempotency_key,
                ],
            )?;
            Ok::<(), rusqlite::Error>(())
        })
        .await?;
    Ok(())
}

// ── Watch / cursor repo ──────────────────────────────────────────────────────

#[derive(Debug, Clone)]
pub struct ChangeCursor {
    pub id: String,
    pub google_account_id: String,
    pub corpus_kind: String,
    pub drive_id: Option<String>,
    pub current_page_token: String,
    pub last_event_sequence: i64,
    pub next_poll_at_ms: i64,
    pub consecutive_error_count: i64,
    /// Unix-ms timestamp of the most recent non-empty change page.
    /// NULL when no events have ever been received on this cursor.
    pub last_event_at_ms: Option<i64>,
}

/// Ensure a cursor row exists for this (account, corpus, drive_id) key and
/// return it. Creates with `start_page_token` when absent.
pub async fn upsert_change_cursor(
    db: &Database,
    google_account_id: &str,
    corpus_kind: &str,
    drive_id: Option<&str>,
    start_page_token: &str,
) -> anyhow::Result<ChangeCursor> {
    let google_account_id = google_account_id.to_string();
    let corpus_kind = corpus_kind.to_string();
    let drive_id = drive_id.map(ToOwned::to_owned);
    let start_page_token = start_page_token.to_string();
    Ok(db
        .conn()
        .call(move |conn| {
            let id = Uuid::new_v4().to_string();
            let now = now_ms();
            conn.execute(
                "INSERT INTO change_cursors (
                    id, google_account_id, corpus_kind, drive_id,
                    current_page_token, next_poll_at_ms
                 ) VALUES (?1, ?2, ?3, ?4, ?5, ?6)
                 ON CONFLICT(google_account_id, corpus_kind, drive_id) DO NOTHING",
                params![
                    id,
                    google_account_id,
                    corpus_kind,
                    drive_id,
                    start_page_token,
                    now,
                ],
            )?;
            conn.query_row(
                "SELECT id, google_account_id, corpus_kind, drive_id,
                        current_page_token, last_event_sequence,
                        next_poll_at_ms, consecutive_error_count,
                        last_event_at_ms
                 FROM change_cursors
                 WHERE google_account_id = ?1
                   AND corpus_kind = ?2
                   AND (drive_id = ?3 OR (drive_id IS NULL AND ?3 IS NULL))",
                params![google_account_id, corpus_kind, drive_id],
                |row| {
                    Ok(ChangeCursor {
                        id: row.get(0)?,
                        google_account_id: row.get(1)?,
                        corpus_kind: row.get(2)?,
                        drive_id: row.get(3)?,
                        current_page_token: row.get(4)?,
                        last_event_sequence: row.get(5)?,
                        next_poll_at_ms: row.get(6)?,
                        consecutive_error_count: row.get(7)?,
                        last_event_at_ms: row.get(8)?,
                    })
                },
            )
        })
        .await?)
}

/// Load all cursors (for starting pollers on startup).
pub async fn all_change_cursors(db: &Database) -> anyhow::Result<Vec<ChangeCursor>> {
    Ok(db
        .conn()
        .call(|conn| {
            let mut stmt = conn.prepare(
                "SELECT id, google_account_id, corpus_kind, drive_id,
                        current_page_token, last_event_sequence,
                        next_poll_at_ms, consecutive_error_count,
                        last_event_at_ms
                 FROM change_cursors
                 ORDER BY id",
            )?;
            let rows = stmt
                .query_map([], |row| {
                    Ok(ChangeCursor {
                        id: row.get(0)?,
                        google_account_id: row.get(1)?,
                        corpus_kind: row.get(2)?,
                        drive_id: row.get(3)?,
                        current_page_token: row.get(4)?,
                        last_event_sequence: row.get(5)?,
                        next_poll_at_ms: row.get(6)?,
                        consecutive_error_count: row.get(7)?,
                        last_event_at_ms: row.get(8)?,
                    })
                })?
                .collect::<Result<Vec<_>, _>>()?;
            Ok::<Vec<ChangeCursor>, rusqlite::Error>(rows)
        })
        .await?)
}

#[derive(Debug, Clone)]
pub struct NewChangeEventRow {
    pub cursor_id: String,
    pub request_page_token: String,
    pub ordinal_in_page: i64,
    pub file_id: String,
    pub removed: bool,
    pub file_json: Option<String>,
}

/// Atomically insert all events for one page and advance the cursor token.
///
/// Spec §12.3: fetch page outside transaction; begin transaction; insert raw
/// events with UNIQUE dedupe; update cursor; commit. If crash before commit,
/// re-fetch and dedupe via unique constraint.
pub async fn commit_change_page(
    db: &Database,
    cursor_id: &str,
    events: Vec<NewChangeEventRow>,
    next_page_token: Option<&str>,
    new_start_page_token: Option<&str>,
    next_poll_at_ms: i64,
) -> anyhow::Result<i64> {
    let cursor_id = cursor_id.to_string();
    let next_page_token = next_page_token.map(ToOwned::to_owned);
    let new_start_page_token = new_start_page_token.map(ToOwned::to_owned);
    Ok(db
        .conn()
        .call(move |conn| {
            let now = now_ms();
            let tx = conn.transaction()?;

            // Insert events — IGNORE on duplicate (page re-fetched after crash).
            let mut last_sequence: i64 = 0;
            for event in &events {
                tx.execute(
                    "INSERT OR IGNORE INTO change_events (
                        cursor_id, request_page_token, ordinal_in_page,
                        file_id, removed, file_json, received_at_ms
                     ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
                    params![
                        event.cursor_id,
                        event.request_page_token,
                        event.ordinal_in_page,
                        event.file_id,
                        i64::from(event.removed),
                        event.file_json,
                        now,
                    ],
                )?;
                // Track the max sequence inserted.
                let seq: i64 = tx.query_row(
                    "SELECT sequence FROM change_events
                     WHERE cursor_id = ?1
                       AND request_page_token = ?2
                       AND ordinal_in_page = ?3",
                    params![
                        event.cursor_id,
                        event.request_page_token,
                        event.ordinal_in_page,
                    ],
                    |row| row.get(0),
                )?;
                last_sequence = last_sequence.max(seq);
            }

            // Advance cursor token. Use newStartPageToken only at end of list.
            let new_token = new_start_page_token
                .as_deref()
                .or(next_page_token.as_deref())
                .unwrap_or("");
            if !new_token.is_empty() {
                // Write last_event_at_ms only when this page contained events.
                let new_last_event_at: Option<i64> =
                    if events.is_empty() { None } else { Some(now) };
                tx.execute(
                    "UPDATE change_cursors
                     SET current_page_token = ?1,
                         last_event_sequence = MAX(last_event_sequence, ?2),
                         next_poll_at_ms = ?3,
                         last_success_at_ms = ?4,
                         consecutive_error_count = 0,
                         last_event_at_ms = COALESCE(?6, last_event_at_ms)
                     WHERE id = ?5",
                    params![
                        new_token,
                        last_sequence,
                        next_poll_at_ms,
                        now,
                        cursor_id,
                        new_last_event_at,
                    ],
                )?;
            }
            tx.commit()?;
            Ok::<i64, rusqlite::Error>(last_sequence)
        })
        .await?)
}

pub async fn record_cursor_error(
    db: &Database,
    cursor_id: &str,
    next_poll_at_ms: i64,
) -> anyhow::Result<()> {
    let cursor_id = cursor_id.to_string();
    db.conn()
        .call(move |conn| {
            conn.execute(
                "UPDATE change_cursors
                 SET consecutive_error_count = consecutive_error_count + 1,
                     next_poll_at_ms = ?1
                 WHERE id = ?2",
                params![next_poll_at_ms, cursor_id],
            )?;
            Ok::<(), rusqlite::Error>(())
        })
        .await?;
    Ok(())
}

// ── Watch subscriptions ──────────────────────────────────────────────────────

#[derive(Debug, Clone)]
pub struct WatchSubscription {
    pub id: String,
    pub google_account_id: String,
    pub cursor_id: String,
    pub telegram_user_id: i64,
    pub chat_id: i64,
    pub source_root_id: String,
    pub source_resource_key: Option<String>,
    pub source_drive_id: Option<String>,
    pub destination_root_id: String,
    pub destination_drive_id: Option<String>,
    pub status: String,
    pub content_update_policy: String,
    pub deletion_policy: String,
    pub move_out_policy: String,
    pub baseline_sequence: i64,
    pub last_consumed_sequence: i64,
    pub created_at_ms: i64,
    pub updated_at_ms: i64,
}

#[derive(Debug, Clone)]
pub struct NewWatchSubscription {
    pub google_account_id: String,
    pub cursor_id: String,
    pub telegram_user_id: i64,
    pub chat_id: i64,
    pub source_root_id: String,
    pub source_resource_key: Option<String>,
    pub source_drive_id: Option<String>,
    pub destination_root_id: String,
    pub destination_drive_id: Option<String>,
    pub content_update_policy: String,
    pub deletion_policy: String,
    pub move_out_policy: String,
    pub baseline_sequence: i64,
}

pub async fn create_watch_subscription(
    db: &Database,
    sub: NewWatchSubscription,
) -> anyhow::Result<String> {
    let id = Uuid::new_v4().to_string();
    let id_for_db = id.clone();
    db.conn()
        .call(move |conn| {
            let now = now_ms();
            conn.execute(
                "INSERT INTO watch_subscriptions (
                    id, google_account_id, cursor_id, telegram_user_id, chat_id,
                    source_root_id, source_resource_key, source_drive_id,
                    destination_root_id, destination_drive_id,
                    status, content_update_policy, deletion_policy, move_out_policy,
                    baseline_sequence, last_consumed_sequence, created_at_ms, updated_at_ms
                 ) VALUES (
                    ?1, ?2, ?3, ?4, ?5,
                    ?6, ?7, ?8,
                    ?9, ?10,
                    'initializing', ?11, ?12, ?13,
                    ?14, ?14, ?15, ?15
                 )",
                params![
                    id_for_db,
                    sub.google_account_id,
                    sub.cursor_id,
                    sub.telegram_user_id,
                    sub.chat_id,
                    sub.source_root_id,
                    sub.source_resource_key,
                    sub.source_drive_id,
                    sub.destination_root_id,
                    sub.destination_drive_id,
                    sub.content_update_policy,
                    sub.deletion_policy,
                    sub.move_out_policy,
                    sub.baseline_sequence,
                    now,
                ],
            )?;
            Ok::<(), rusqlite::Error>(())
        })
        .await?;
    Ok(id)
}

fn row_to_watch(row: &rusqlite::Row<'_>) -> rusqlite::Result<WatchSubscription> {
    Ok(WatchSubscription {
        id: row.get(0)?,
        google_account_id: row.get(1)?,
        cursor_id: row.get(2)?,
        telegram_user_id: row.get(3)?,
        chat_id: row.get(4)?,
        source_root_id: row.get(5)?,
        source_resource_key: row.get(6)?,
        source_drive_id: row.get(7)?,
        destination_root_id: row.get(8)?,
        destination_drive_id: row.get(9)?,
        status: row.get(10)?,
        content_update_policy: row.get(11)?,
        deletion_policy: row.get(12)?,
        move_out_policy: row.get(13)?,
        baseline_sequence: row.get(14)?,
        last_consumed_sequence: row.get(15)?,
        created_at_ms: row.get(16)?,
        updated_at_ms: row.get(17)?,
    })
}

const WATCH_COLS: &str = "id, google_account_id, cursor_id, telegram_user_id, chat_id,
    source_root_id, source_resource_key, source_drive_id,
    destination_root_id, destination_drive_id, status,
    content_update_policy, deletion_policy, move_out_policy,
    baseline_sequence, last_consumed_sequence, created_at_ms, updated_at_ms";

pub async fn list_watches_for_user(
    db: &Database,
    telegram_user_id: i64,
) -> anyhow::Result<Vec<WatchSubscription>> {
    Ok(db
        .conn()
        .call(move |conn| {
            let sql = format!(
                "SELECT {WATCH_COLS}
                 FROM watch_subscriptions
                 WHERE telegram_user_id = ?1
                 ORDER BY created_at_ms DESC"
            );
            let mut stmt = conn.prepare(&sql)?;
            let rows = stmt
                .query_map(params![telegram_user_id], row_to_watch)?
                .collect::<Result<Vec<_>, _>>()?;
            Ok::<Vec<WatchSubscription>, rusqlite::Error>(rows)
        })
        .await?)
}

pub async fn watch_for_user(
    db: &Database,
    telegram_user_id: i64,
    watch_id: &str,
) -> anyhow::Result<Option<WatchSubscription>> {
    let watch_id = watch_id.to_string();
    Ok(db
        .conn()
        .call(move |conn| {
            let sql = format!(
                "SELECT {WATCH_COLS}
                 FROM watch_subscriptions
                 WHERE telegram_user_id = ?1 AND id = ?2"
            );
            conn.query_row(&sql, params![telegram_user_id, watch_id], row_to_watch)
                .optional()
        })
        .await?)
}

/// All active watches — used by the dispatcher.
pub async fn active_watches(db: &Database) -> anyhow::Result<Vec<WatchSubscription>> {
    Ok(db
        .conn()
        .call(|conn| {
            let sql = format!(
                "SELECT {WATCH_COLS}
                 FROM watch_subscriptions
                 WHERE status IN ('active', 'catching_up', 'degraded')
                 ORDER BY id"
            );
            let mut stmt = conn.prepare(&sql)?;
            let rows = stmt
                .query_map([], row_to_watch)?
                .collect::<Result<Vec<_>, _>>()?;
            Ok::<Vec<WatchSubscription>, rusqlite::Error>(rows)
        })
        .await?)
}

pub async fn update_watch_status(
    db: &Database,
    watch_id: &str,
    status: &str,
) -> anyhow::Result<()> {
    let watch_id = watch_id.to_string();
    let status = status.to_string();
    db.conn()
        .call(move |conn| {
            conn.execute(
                "UPDATE watch_subscriptions SET status = ?1, updated_at_ms = ?2 WHERE id = ?3",
                params![status, now_ms(), watch_id],
            )?;
            Ok::<(), rusqlite::Error>(())
        })
        .await?;
    Ok(())
}

pub async fn advance_watch_consumed_sequence(
    db: &Database,
    watch_id: &str,
    last_consumed_sequence: i64,
) -> anyhow::Result<()> {
    let watch_id = watch_id.to_string();
    db.conn()
        .call(move |conn| {
            conn.execute(
                "UPDATE watch_subscriptions
                 SET last_consumed_sequence = ?1, updated_at_ms = ?2
                 WHERE id = ?3",
                params![last_consumed_sequence, now_ms(), watch_id],
            )?;
            Ok::<(), rusqlite::Error>(())
        })
        .await?;
    Ok(())
}

pub async fn pause_watch_for_user(
    db: &Database,
    telegram_user_id: i64,
    watch_id: &str,
) -> anyhow::Result<bool> {
    let watch_id = watch_id.to_string();
    Ok(db
        .conn()
        .call(move |conn| {
            let changed = conn.execute(
                "UPDATE watch_subscriptions
                 SET status = 'paused', updated_at_ms = ?1
                 WHERE telegram_user_id = ?2 AND id = ?3
                   AND status IN ('active', 'catching_up', 'degraded')",
                params![now_ms(), telegram_user_id, watch_id],
            )?;
            Ok::<bool, rusqlite::Error>(changed > 0)
        })
        .await?)
}

pub async fn resume_watch_for_user(
    db: &Database,
    telegram_user_id: i64,
    watch_id: &str,
) -> anyhow::Result<bool> {
    let watch_id = watch_id.to_string();
    Ok(db
        .conn()
        .call(move |conn| {
            let changed = conn.execute(
                "UPDATE watch_subscriptions
                 SET status = 'catching_up', updated_at_ms = ?1
                 WHERE telegram_user_id = ?2 AND id = ?3 AND status = 'paused'",
                params![now_ms(), telegram_user_id, watch_id],
            )?;
            Ok::<bool, rusqlite::Error>(changed > 0)
        })
        .await?)
}

pub async fn stop_watch_for_user(
    db: &Database,
    telegram_user_id: i64,
    watch_id: &str,
) -> anyhow::Result<bool> {
    let watch_id = watch_id.to_string();
    Ok(db
        .conn()
        .call(move |conn| {
            let changed = conn.execute(
                "UPDATE watch_subscriptions
                 SET status = 'stopped', updated_at_ms = ?1
                 WHERE telegram_user_id = ?2 AND id = ?3
                   AND status != 'stopped'",
                params![now_ms(), telegram_user_id, watch_id],
            )?;
            Ok::<bool, rusqlite::Error>(changed > 0)
        })
        .await?)
}

pub async fn set_watch_content_update_policy(
    db: &Database,
    telegram_user_id: i64,
    watch_id: &str,
    policy: &str,
) -> anyhow::Result<bool> {
    let watch_id = watch_id.to_string();
    let policy = policy.to_string();
    Ok(db
        .conn()
        .call(move |conn| {
            let changed = conn.execute(
                "UPDATE watch_subscriptions
                 SET content_update_policy = ?1, updated_at_ms = ?2
                 WHERE telegram_user_id = ?3 AND id = ?4",
                params![policy, now_ms(), telegram_user_id, watch_id],
            )?;
            Ok::<bool, rusqlite::Error>(changed > 0)
        })
        .await?)
}

// ── Change event application ─────────────────────────────────────────────────

#[derive(Debug, Clone)]
pub struct PendingChangeEvent {
    pub sequence: i64,
    pub file_id: String,
    pub removed: bool,
    pub file_json: Option<String>,
}

/// Fetch the next batch of change events that `watch` has not yet consumed.
/// Returns at most `limit` rows ordered by sequence ascending.
pub async fn pending_events_for_watch(
    db: &Database,
    watch_id: &str,
    last_consumed_sequence: i64,
    limit: usize,
) -> anyhow::Result<Vec<PendingChangeEvent>> {
    let watch_id = watch_id.to_string();
    Ok(db
        .conn()
        .call(move |conn| {
            // Events whose sequence > last_consumed that have not already been
            // applied or ignored for this watch.
            let mut stmt = conn.prepare(
                "SELECT ce.sequence, ce.file_id, ce.removed, ce.file_json
                 FROM change_events ce
                 JOIN change_cursors cc ON cc.id = ce.cursor_id
                 JOIN watch_subscriptions ws ON ws.cursor_id = cc.id AND ws.id = ?1
                 WHERE ce.sequence > ?2
                   AND NOT EXISTS (
                       SELECT 1 FROM watch_event_applications wea
                       WHERE wea.watch_id = ?1 AND wea.event_sequence = ce.sequence
                         AND wea.status IN ('applied', 'ignored')
                   )
                 ORDER BY ce.sequence
                 LIMIT ?3",
            )?;
            let rows = stmt
                .query_map(
                    params![watch_id, last_consumed_sequence, limit as i64],
                    |row| {
                        Ok(PendingChangeEvent {
                            sequence: row.get(0)?,
                            file_id: row.get(1)?,
                            removed: row.get::<_, i64>(2)? != 0,
                            file_json: row.get(3)?,
                        })
                    },
                )?
                .collect::<Result<Vec<_>, _>>()?;
            Ok::<Vec<PendingChangeEvent>, rusqlite::Error>(rows)
        })
        .await?)
}

pub async fn upsert_event_application(
    db: &Database,
    watch_id: &str,
    event_sequence: i64,
    classification: &str,
    status: &str,
) -> anyhow::Result<()> {
    let watch_id = watch_id.to_string();
    let classification = classification.to_string();
    let status = status.to_string();
    db.conn()
        .call(move |conn| {
            conn.execute(
                "INSERT INTO watch_event_applications (
                    watch_id, event_sequence, classification, status,
                    attempts, updated_at_ms
                 ) VALUES (?1, ?2, ?3, ?4, 0, ?5)
                 ON CONFLICT(watch_id, event_sequence) DO UPDATE SET
                    classification = excluded.classification,
                    status = excluded.status,
                    attempts = watch_event_applications.attempts + 1,
                    updated_at_ms = excluded.updated_at_ms",
                params![watch_id, event_sequence, classification, status, now_ms(),],
            )?;
            Ok::<(), rusqlite::Error>(())
        })
        .await?;
    Ok(())
}

/// Check whether a source file_id is within the watch's mapped subtree.
/// Returns `true` when source_mappings has an active row for this
/// (watch scope, file_id).
pub async fn is_in_watch_tree(
    db: &Database,
    watch_id: &str,
    file_id: &str,
) -> anyhow::Result<bool> {
    let watch_id = watch_id.to_string();
    let file_id = file_id.to_string();
    Ok(db
        .conn()
        .call(move |conn| {
            conn.query_row(
                "SELECT EXISTS(
                    SELECT 1 FROM source_mappings
                    WHERE scope_type = 'watch'
                      AND scope_id = ?1
                      AND source_item_id = ?2
                      AND mapping_state = 'active'
                 )",
                params![watch_id, file_id],
                |row| row.get::<_, bool>(0),
            )
        })
        .await?)
}

/// Look up the mapped destination item for a source within a watch.
pub async fn watch_mapping_destination(
    db: &Database,
    watch_id: &str,
    source_item_id: &str,
) -> anyhow::Result<Option<String>> {
    let watch_id = watch_id.to_string();
    let source_item_id = source_item_id.to_string();
    Ok(db
        .conn()
        .call(move |conn| {
            conn.query_row(
                "SELECT destination_item_id FROM source_mappings
                 WHERE scope_type = 'watch' AND scope_id = ?1 AND source_item_id = ?2",
                params![watch_id, source_item_id],
                |row| row.get(0),
            )
            .optional()
        })
        .await?)
}

/// Check if a source_parent_id is mapped in this watch (i.e. is a tracked folder).
pub async fn is_parent_in_watch_tree(
    db: &Database,
    watch_id: &str,
    parent_id: &str,
) -> anyhow::Result<bool> {
    let watch_id = watch_id.to_string();
    let parent_id = parent_id.to_string();
    Ok(db
        .conn()
        .call(move |conn| {
            conn.query_row(
                "SELECT EXISTS(
                    SELECT 1 FROM source_mappings
                    WHERE scope_type = 'watch'
                      AND scope_id = ?1
                      AND source_item_id = ?2
                      AND mapping_state = 'active'
                 )",
                params![watch_id, parent_id],
                |row| row.get::<_, bool>(0),
            )
        })
        .await?)
}

pub async fn mark_watch_mapping_detached(
    db: &Database,
    watch_id: &str,
    source_item_id: &str,
) -> anyhow::Result<()> {
    let watch_id = watch_id.to_string();
    let source_item_id = source_item_id.to_string();
    db.conn()
        .call(move |conn| {
            conn.execute(
                "UPDATE source_mappings
                 SET mapping_state = 'detached', updated_at_ms = ?1
                 WHERE scope_type = 'watch' AND scope_id = ?2 AND source_item_id = ?3",
                params![now_ms(), watch_id, source_item_id],
            )?;
            Ok::<(), rusqlite::Error>(())
        })
        .await?;
    Ok(())
}

pub async fn mark_watch_mapping_source_removed(
    db: &Database,
    watch_id: &str,
    source_item_id: &str,
) -> anyhow::Result<()> {
    let watch_id = watch_id.to_string();
    let source_item_id = source_item_id.to_string();
    db.conn()
        .call(move |conn| {
            conn.execute(
                "UPDATE source_mappings
                 SET mapping_state = 'source_removed', updated_at_ms = ?1
                 WHERE scope_type = 'watch' AND scope_id = ?2 AND source_item_id = ?3",
                params![now_ms(), watch_id, source_item_id],
            )?;
            Ok::<(), rusqlite::Error>(())
        })
        .await?;
    Ok(())
}

/// Reactivate a detached or source_removed mapping for a watch item.
/// Called when a source item moves back into the watched tree.
pub async fn mark_watch_mapping_active(
    db: &Database,
    watch_id: &str,
    source_item_id: &str,
) -> anyhow::Result<()> {
    let watch_id = watch_id.to_string();
    let source_item_id = source_item_id.to_string();
    db.conn()
        .call(move |conn| {
            conn.execute(
                "UPDATE source_mappings
                 SET mapping_state = 'active', updated_at_ms = ?1
                 WHERE scope_type = 'watch' AND scope_id = ?2 AND source_item_id = ?3",
                params![now_ms(), watch_id, source_item_id],
            )?;
            Ok::<(), rusqlite::Error>(())
        })
        .await?;
    Ok(())
}

// ── Event retention ──────────────────────────────────────────────────────────

/// Delete raw change_events that:
///  1. Are older than `retention_days` days; AND
///  2. Every still-active watch associated with the cursor has already
///     consumed past that sequence (last_consumed_sequence >= event.sequence).
///
/// Returns the number of deleted rows.
pub async fn prune_consumed_events(db: &Database, retention_days: u64) -> anyhow::Result<usize> {
    let cutoff_ms = now_ms() - (retention_days as i64) * 86_400_000;
    Ok(db
        .conn()
        .call(move |conn| {
            let deleted = conn.execute(
                "DELETE FROM change_events
                 WHERE received_at_ms < ?1
                   AND sequence <= (
                       -- minimum last_consumed across all non-stopped watches
                       -- for the same cursor
                       SELECT COALESCE(MIN(ws.last_consumed_sequence), sequence)
                       FROM change_cursors cc
                       JOIN watch_subscriptions ws ON ws.cursor_id = cc.id
                          AND ws.status NOT IN ('stopped')
                       WHERE cc.id = change_events.cursor_id
                   )",
                params![cutoff_ms],
            )?;
            Ok::<usize, rusqlite::Error>(deleted)
        })
        .await?)
}

// ── Cursor helpers ───────────────────────────────────────────────────────────

/// Fetch a change cursor by its primary key ID.
pub async fn cursor_by_id(db: &Database, cursor_id: &str) -> anyhow::Result<ChangeCursor> {
    let cursor_id = cursor_id.to_string();
    Ok(db
        .conn()
        .call(move |conn| {
            conn.query_row(
                "SELECT id, google_account_id, corpus_kind, drive_id,
                        current_page_token, last_event_sequence, next_poll_at_ms,
                        consecutive_error_count, last_event_at_ms
                 FROM change_cursors WHERE id = ?1",
                params![cursor_id],
                |row| {
                    Ok(ChangeCursor {
                        id: row.get(0)?,
                        google_account_id: row.get(1)?,
                        corpus_kind: row.get(2)?,
                        drive_id: row.get(3)?,
                        current_page_token: row.get(4)?,
                        last_event_sequence: row.get(5)?,
                        next_poll_at_ms: row.get(6)?,
                        consecutive_error_count: row.get(7)?,
                        last_event_at_ms: row.get(8)?,
                    })
                },
            )
        })
        .await?)
}

// ── Watch subscription helpers ───────────────────────────────────────────────

/// Fetch a watch subscription by ID, without Telegram-user ownership check.
/// Used internally by the initializer and recovery logic.
pub async fn watch_for_user_unchecked(
    db: &Database,
    watch_id: &str,
) -> anyhow::Result<Option<WatchSubscription>> {
    let watch_id = watch_id.to_string();
    Ok(db
        .conn()
        .call(move |conn| {
            let sql = format!(
                "SELECT {WATCH_COLS}
                 FROM watch_subscriptions WHERE id = ?1"
            );
            conn.query_row(&sql, params![watch_id], row_to_watch)
                .optional()
        })
        .await?)
}

/// Return all subscriptions in `initializing` state (need initial clone).
pub async fn initializing_watches(db: &Database) -> anyhow::Result<Vec<WatchSubscription>> {
    Ok(db
        .conn()
        .call(move |conn| {
            let sql = format!(
                "SELECT {WATCH_COLS}
                 FROM watch_subscriptions
                 WHERE status = 'initializing'
                 ORDER BY created_at_ms"
            );
            let mut stmt = conn.prepare(&sql)?;
            let rows = stmt
                .query_map([], row_to_watch)?
                .collect::<Result<Vec<_>, _>>()?;
            Ok::<Vec<WatchSubscription>, rusqlite::Error>(rows)
        })
        .await?)
}

// ── Subtree detach ───────────────────────────────────────────────────────────

/// Mark an entire mapped subtree as `detached` using a recursive CTE walk.
///
/// Called when a watched folder is moved outside the watched tree (spec §12.6).
/// The root item and every descendant that has a mapping under this watch are
/// updated atomically in a single SQLite transaction.
///
/// Returns the count of rows updated.
pub async fn detach_watch_folder_subtree(
    db: &Database,
    watch_id: &str,
    root_source_id: &str,
) -> anyhow::Result<usize> {
    let watch_id = watch_id.to_string();
    let root_source_id = root_source_id.to_string();
    Ok(db
        .conn()
        .call(move |conn| {
            // Recursive CTE: start from root_source_id, walk via source_parent_id.
            // We treat source_parent_id stored in source_mappings as the parent link.
            let updated = conn.execute(
                "WITH RECURSIVE subtree(source_item_id) AS (
                     -- anchor: the folder that moved out
                     SELECT ?2
                     UNION ALL
                     -- recursive step: children whose parent is in subtree
                     SELECT sm.source_item_id
                     FROM source_mappings sm
                     INNER JOIN subtree ON sm.source_parent_id = subtree.source_item_id
                     WHERE sm.scope_type = 'watch' AND sm.scope_id = ?1
                 )
                 UPDATE source_mappings
                 SET mapping_state = 'detached',
                     updated_at_ms = ?3
                 WHERE scope_type = 'watch'
                   AND scope_id = ?1
                   AND source_item_id IN (SELECT source_item_id FROM subtree)",
                params![watch_id, root_source_id, now_ms()],
            )?;
            Ok::<usize, rusqlite::Error>(updated)
        })
        .await?)
}
