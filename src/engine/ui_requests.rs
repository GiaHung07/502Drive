//! Daemon-side consumer for the `ui_requests` queue (migration 0008).
//!
//! The GUI process only *enqueues* requests (see
//! `src-tauri/src/commands/features.rs`); every engine side effect — clone
//! execution, watch creation + initialization, job retry resume — runs here,
//! in the daemon process (`502drive run`), because:
//!
//! * `CloneService::start_one_shot` executes the whole copy inline
//!   (`src/engine/copy.rs`) — there is no jobs-table worker loop that would
//!   pick up a row written by another process.
//! * `recover_initializing_watches` only rescues `initializing` watches at
//!   startup (`src/engine/recovery.rs`), so a watch created at runtime must be
//!   initialized by whoever creates it. The consumer runs
//!   [`crate::watch::run_initial_clone`] itself after accepting a watch
//!   request.
//!
//! Loop cadence: poll every ~2s, claim each pending row atomically via
//! [`crate::state::ui_requests::decide`] (conditional UPDATE on
//! `status='pending'`), so multi-process access stays safe.

use std::time::Duration;

use rusqlite::{OptionalExtension, params};
use serde::{Deserialize, Serialize};
use tokio::task::JoinHandle;
use tracing::{error, info, warn};

use crate::{
    config::AppConfig,
    drive::{client::DriveClient, links::parse_drive_reference, token_manager::TokenManager},
    engine::{
        copy::{CloneRequest, CloneService, validate_clone_request},
        recovery,
    },
    state::{
        db::Database,
        repo,
        ui_requests::{self, UiRequest},
    },
    watch::{self, run_initial_clone},
};

/// Poll interval for the queue consumer.
const POLL_INTERVAL: Duration = Duration::from_secs(2);

// ── Payloads ─────────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CloneRequestPayload {
    pub source_url: String,
    pub name_override: Option<String>,
    pub destination_parent_id: Option<String>,
    pub duplicate_policy: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WatchRequestPayload {
    pub source_url: String,
    /// `None` resolves the configured default destination profile.
    pub destination_url: Option<String>,
    pub exclude_globs: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RetryRequestPayload {
    pub job_id: String,
}

/// GUI-facing owner tag stored in `ui_requests.requested_by`.
pub const REQUESTED_BY_GUI: &str = "gui";

// ── Consumer task ────────────────────────────────────────────────────────────

/// Spawn the UI-request consumer. Called once from `run_bot`; all engine work
/// for GUI requests happens inside this task tree.
pub fn spawn_ui_request_consumer(
    config: AppConfig,
    db: Database,
    drive: DriveClient,
) -> JoinHandle<()> {
    tokio::spawn(async move {
        info!("ui_requests consumer started");
        loop {
            if let Err(err) = process_pending(&config, &db, &drive).await {
                error!(error = %err, "ui_requests consumer cycle failed");
            }
            tokio::time::sleep(POLL_INTERVAL).await;
        }
    })
}

/// Decide helpers that swallow the claim bool (a false means another consumer
/// already decided the row — nothing to do).
async fn reject(db: &Database, id: &str, note: &str) -> anyhow::Result<()> {
    ui_requests::decide(db, id, false, note).await?;
    Ok(())
}

async fn accept(db: &Database, id: &str, note: &str) -> anyhow::Result<()> {
    ui_requests::decide(db, id, true, note).await?;
    Ok(())
}

async fn process_pending(
    config: &AppConfig,
    db: &Database,
    drive: &DriveClient,
) -> anyhow::Result<()> {
    for request in ui_requests::pending(db).await? {
        let result = match request.kind.as_str() {
            ui_requests::KIND_CLONE => process_clone(config, db, drive, &request).await,
            ui_requests::KIND_WATCH => process_watch(config, db, drive, &request).await,
            ui_requests::KIND_RETRY => process_retry(config, db, &request).await,
            other => {
                warn!(
                    request_id = request.id,
                    kind = other,
                    "unknown ui_requests kind"
                );
                Ok(())
            }
        };
        if let Err(err) = result {
            // Failed to record the decision (e.g. transient DB error): leave
            // the row pending so the next cycle retries it.
            warn!(request_id = request.id, error = %err, "ui_request processing error");
        }
    }
    Ok(())
}

// ── Clone requests ───────────────────────────────────────────────────────────

async fn process_clone(
    config: &AppConfig,
    db: &Database,
    drive: &DriveClient,
    request: &UiRequest,
) -> anyhow::Result<()> {
    let payload: CloneRequestPayload = match serde_json::from_str(&request.payload_json) {
        Ok(p) => p,
        Err(err) => {
            return reject(db, &request.id, &format!("invalid clone payload: {err}")).await;
        }
    };

    // Phase 1 — validation, mirroring the telegram /clone pre-flight
    // (parse link, resolve source and explicit destination via DriveClient).
    let source = match parse_drive_reference(&payload.source_url) {
        Ok(source) => source,
        Err(err) => {
            return reject(db, &request.id, &format!("invalid source: {err}")).await;
        }
    };

    let clone_request = CloneRequest {
        chat_id: 0,
        telegram_user_id: 0,
        source,
        progress_message_id: None,
        name_override: payload.name_override.clone(),
        destination_parent_id: payload.destination_parent_id.clone(),
        duplicate_policy: payload.duplicate_policy.clone(),
    };
    if let Err(err) = validate_clone_request(&clone_request) {
        return reject(db, &request.id, &format!("invalid request: {err}")).await;
    }

    let token_manager = TokenManager::new(config.clone(), db.clone());
    let preflight = async {
        let token = token_manager.access_token("default").await?;
        drive
            .get_reference(token.as_str(), &clone_request.source)
            .await?;
        if let Some(dest_id) = clone_request.destination_parent_id.as_deref() {
            drive
                .get_reference(
                    token.as_str(),
                    &crate::drive::links::DriveReference {
                        file_id: dest_id.to_string(),
                        resource_key: None,
                        hinted_kind: None,
                    },
                )
                .await?;
        }
        Ok::<(), anyhow::Error>(())
    };
    if let Err(err) = preflight.await {
        return reject(
            db,
            &request.id,
            &format!("source or destination not usable: {err:#}"),
        )
        .await;
    }

    // Phase 2 — accept and execute in a spawned task so a long clone never
    // blocks the queue. Outcome is written back into `note` (the status stays
    // 'accepted'; the jobs table carries the authoritative job state).
    accept(db, &request.id, "accepted; clone queued").await?;

    let db2 = db.clone();
    let config2 = config.clone();
    let request_id = request.id.clone();
    tokio::spawn(async move {
        let service = CloneService::new(config2.clone(), db2.clone(), drive_client_for(&config2));
        match service.start_one_shot(clone_request).await {
            Ok(outcome) => {
                info!(
                    request_id,
                    job_id = outcome.job_id,
                    "ui clone request finished"
                );
                let _ = ui_requests::set_note(
                    &db2,
                    &request_id,
                    &format!("job {} — {}", outcome.job_id, outcome.message),
                )
                .await;
            }
            Err(err) => {
                error!(request_id, error = %err, "ui clone request failed");
                let _ = ui_requests::set_note(&db2, &request_id, &format!("clone failed: {err:#}"))
                    .await;
            }
        }
    });

    Ok(())
}

// ── Watch requests ───────────────────────────────────────────────────────────

async fn process_watch(
    config: &AppConfig,
    db: &Database,
    drive: &DriveClient,
    request: &UiRequest,
) -> anyhow::Result<()> {
    let payload: WatchRequestPayload = match serde_json::from_str(&request.payload_json) {
        Ok(p) => p,
        Err(err) => {
            return reject(db, &request.id, &format!("invalid watch payload: {err}")).await;
        }
    };

    if !config.watch.enabled {
        return reject(db, &request.id, "watch feature is disabled").await;
    }

    let source = match parse_drive_reference(&payload.source_url) {
        Ok(source) => source,
        Err(err) => {
            return reject(db, &request.id, &format!("invalid source: {err}")).await;
        }
    };
    let destination = match &payload.destination_url {
        Some(url) => match parse_drive_reference(url) {
            Ok(dest) => dest,
            Err(err) => {
                return reject(db, &request.id, &format!("invalid destination: {err}")).await;
            }
        },
        None => match repo::default_destination_profile(db, "default").await? {
            Some(profile) => match parse_drive_reference(&profile.destination_parent_id) {
                Ok(dest) => dest,
                Err(err) => {
                    return reject(
                        db,
                        &request.id,
                        &format!("configured default destination is invalid: {err}"),
                    )
                    .await;
                }
            },
            None => {
                return reject(
                    db,
                    &request.id,
                    "no default destination configured; provide destination_url",
                )
                .await;
            }
        },
    };

    let created = match watch::create_watch(
        config,
        db,
        drive,
        watch::CreateWatchParams {
            source,
            destination,
            telegram_user_id: 0,
            chat_id: 0,
            exclude_globs: payload.exclude_globs.clone(),
        },
    )
    .await
    {
        Ok(created) => created,
        Err(err) => {
            return reject(db, &request.id, &format!("{err:#}")).await;
        }
    };

    accept(
        db,
        &request.id,
        &format!(
            "watch {} created ({} → {}); initial clone running",
            created.watch_id, created.source_name, created.destination_name
        ),
    )
    .await?;

    // Initialize the watch in the daemon process — startup recovery does not
    // rescue runtime-created 'initializing' watches.
    let config2 = config.clone();
    let db2 = db.clone();
    let watch_id = created.watch_id.clone();
    tokio::spawn(async move {
        let watch_id_for_notify = watch_id.clone();
        let notify = move |msg: String| {
            tracing::info!(watch_id = watch_id_for_notify, message = %msg, "watch initialization progress");
        };
        let client = drive_client_for(&config2);
        if let Err(err) = run_initial_clone(config2, db2, client, watch_id.clone(), notify).await {
            error!(watch_id, error = %err, "ui watch initialization failed");
        }
    });

    Ok(())
}

// ── Retry requests ───────────────────────────────────────────────────────────

async fn process_retry(
    config: &AppConfig,
    db: &Database,
    request: &UiRequest,
) -> anyhow::Result<()> {
    let payload: RetryRequestPayload = match serde_json::from_str(&request.payload_json) {
        Ok(p) => p,
        Err(err) => {
            return reject(db, &request.id, &format!("invalid retry payload: {err}")).await;
        }
    };

    // Resolve the owning user so the engine's user-scoped retry applies.
    let job_id = payload.job_id.trim().to_string();
    let owner = job_owner_user(db, &job_id).await?;
    let Some(owner) = owner else {
        return reject(db, &request.id, "job not found").await;
    };

    match repo::retry_failed_job_for_user(db, owner, &job_id).await? {
        Some(summary) => {
            accept(
                db,
                &request.id,
                &format!(
                    "retry queued — folders {}, items {}, operations {}",
                    summary.traversal_folders_requeued,
                    summary.job_items_requeued,
                    summary.operation_intents_replanned
                ),
            )
            .await?;
            // The resume worker must run in the daemon process (it drives the
            // clone engine), which is exactly where this consumer lives.
            recovery::spawn_startup_resume_worker(
                config.clone(),
                db.clone(),
                drive_client_for(config),
            );
        }
        None => {
            reject(db, &request.id, "job not retryable or has no failed items").await?;
        }
    }
    Ok(())
}

async fn job_owner_user(db: &Database, job_id: &str) -> anyhow::Result<Option<i64>> {
    let job_id = job_id.to_string();
    Ok(db
        .conn()
        .call(move |conn| {
            let owner: Option<i64> = conn
                .query_row(
                    "SELECT telegram_user_id FROM jobs WHERE id = ?1",
                    params![job_id],
                    |row| row.get(0),
                )
                .optional()?;
            Ok::<Option<i64>, rusqlite::Error>(owner)
        })
        .await?)
}

fn drive_client_for(config: &AppConfig) -> DriveClient {
    crate::watch::service::drive_client_for(config)
}
