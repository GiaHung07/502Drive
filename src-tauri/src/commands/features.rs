//! GUI↔engine bridge commands.
//!
//! Design: the GUI process NEVER runs engine work (pollers, clone engine,
//! watch initializer live in the daemon `502drive run`). Mutating operations
//! are enqueued into the shared `ui_requests` table (migrations 0008/0009) and
//! the daemon's consumer (`gdclone_bot::engine::ui_requests`) picks them up
//! ~2s later. `browse_drive_children` is the one exception that performs a
//! Drive call in the GUI process: it is a pure read (list folders) through
//! the shared [`gdclone_bot::engine::services::DestinationService`], so the
//! frontend folder picker works even while inspecting a running daemon.

use crate::commands::{get_config_path, get_db_path, open_queue_conn};
use gdclone_bot::{
    config::AppConfig,
    drive::id_parser,
    engine::services::DestinationService,
    engine::ui_requests::{CloneRequestPayload, RetryRequestPayload, WatchRequestPayload},
    state::{db::Database, ui_requests},
};
use rusqlite::{Connection, OpenFlags};
use serde::Serialize;

// ── Request queue commands ───────────────────────────────────────────────────

#[derive(Debug, Serialize)]
pub struct CreateRequestResult {
    pub request_id: String,
}

fn parse_source_id(source_url: &str) -> Result<(), String> {
    id_parser::parse(source_url)
        .map(|_| ())
        .map_err(|e| format!("invalid source URL: {e}"))
}

/// Queue a one-shot clone request. The daemon validates it against Drive and
/// executes it; poll `get_ui_request_status` for the decision.
#[tauri::command]
pub fn create_clone_request(
    source_url: String,
    name_override: Option<String>,
    destination_parent_id: Option<String>,
    duplicate_policy: Option<String>,
) -> Result<CreateRequestResult, String> {
    parse_source_id(&source_url)?;
    if let Some(policy) = duplicate_policy.as_deref().map(str::trim) {
        if !policy.is_empty() && !gdclone_bot::engine::copy::DUPLICATE_POLICIES.contains(&policy) {
            return Err(format!(
                "invalid duplicate_policy '{}'; expected one of: {}",
                policy,
                gdclone_bot::engine::copy::DUPLICATE_POLICIES.join(", ")
            ));
        }
    }
    let payload = CloneRequestPayload {
        source_url: source_url.trim().to_string(),
        name_override,
        destination_parent_id,
        duplicate_policy,
    };
    let payload_json = serde_json::to_string(&payload).map_err(|e| e.to_string())?;
    let conn = open_queue_conn()?;
    let request_id = ui_requests::insert_sync(&conn, "clone", &payload_json, Some("gui"))
        .map_err(|e| e.to_string())?;
    Ok(CreateRequestResult { request_id })
}

/// Queue a watch request. The daemon validates source/destination, creates the
/// watch subscription and runs the initial clone (in the daemon process).
#[tauri::command]
pub fn create_watch_request(
    source_url: String,
    destination_url: Option<String>,
    exclude_globs: Vec<String>,
) -> Result<CreateRequestResult, String> {
    parse_source_id(&source_url)?;
    if let Some(dest) = destination_url.as_deref() {
        id_parser::parse(dest)
            .map(|_| ())
            .map_err(|e| format!("invalid destination URL: {e}"))?;
    }
    for glob in &exclude_globs {
        gdclone_bot::watch::glob::normalize_glob(glob)
            .map_err(|e| format!("invalid exclude glob '{glob}': {e}"))?;
    }
    let payload = WatchRequestPayload {
        source_url: source_url.trim().to_string(),
        destination_url: destination_url.as_deref().map(str::trim).map(String::from),
        exclude_globs,
    };
    let payload_json = serde_json::to_string(&payload).map_err(|e| e.to_string())?;
    let conn = open_queue_conn()?;
    let request_id = ui_requests::insert_sync(&conn, "watch", &payload_json, Some("gui"))
        .map_err(|e| e.to_string())?;
    Ok(CreateRequestResult { request_id })
}

/// Queue a retry of a failed job. The engine's retry must run in the daemon
/// process (it spawns the clone resume worker), so it goes through the queue.
#[tauri::command]
pub fn retry_job(job_id: String) -> Result<CreateRequestResult, String> {
    let job_id = job_id.trim().to_string();
    if job_id.is_empty() {
        return Err("job_id must not be empty".to_string());
    }
    let payload = RetryRequestPayload { job_id };
    let payload_json = serde_json::to_string(&payload).map_err(|e| e.to_string())?;
    let conn = open_queue_conn()?;
    let request_id = ui_requests::insert_sync(&conn, "retry", &payload_json, Some("gui"))
        .map_err(|e| e.to_string())?;
    Ok(CreateRequestResult { request_id })
}

#[derive(Debug, Serialize)]
pub struct UiRequestStatus {
    pub request_id: String,
    pub kind: String,
    pub status: String,
    pub note: Option<String>,
    pub created_at_ms: i64,
    pub decided_at_ms: Option<i64>,
}

/// Poll a queued request's decision. `note` carries the rejection reason on
/// 'rejected' and the job id / watch id on 'accepted'.
#[tauri::command]
pub fn get_ui_request_status(request_id: String) -> Result<Option<UiRequestStatus>, String> {
    let db_path = get_db_path();
    if !db_path.exists() {
        return Err("Database does not exist".to_string());
    }
    let conn = Connection::open_with_flags(&db_path, OpenFlags::SQLITE_OPEN_READ_ONLY)
        .map_err(|e| e.to_string())?;
    Ok(ui_requests::get_sync(&conn, &request_id)
        .map_err(|e| e.to_string())?
        .map(|r| UiRequestStatus {
            request_id: r.id,
            kind: r.kind,
            status: r.status,
            note: r.note,
            created_at_ms: r.created_at_ms,
            decided_at_ms: r.decided_at_ms,
        }))
}

// ── Drive folder browsing (read-only, runs in the GUI process) ───────────────

#[derive(Debug, Serialize)]
pub struct DriveEntry {
    pub id: String,
    pub name: String,
    pub is_folder: bool,
    /// Set on shared-drive entries so the picker can pass it back as
    /// `drive_id` when listing that folder's children.
    pub drive_id: Option<String>,
}

/// List Drive folders for the folder picker. `parent_id = None` lists My Drive
/// root ("root") plus the account's shared drives. Read-only Drive calls,
/// delegated to the shared DestinationService (same paging core the telegram
/// destination browser uses).
#[tauri::command]
pub async fn browse_drive_children(
    parent_id: Option<String>,
    drive_id: Option<String>,
) -> Result<Vec<DriveEntry>, String> {
    let config = AppConfig::load(&get_config_path())
        .map_err(|e| format!("cannot load config (finish the setup wizard first): {e:#}"))?;
    let db = Database::open(&get_db_path())
        .await
        .map_err(|e| format!("cannot open state database: {e:#}"))?;
    let entries =
        DestinationService::browse(&config, &db, parent_id.as_deref(), drive_id.as_deref())
            .await
            .map_err(|e| format!("{e:#}"))?;
    Ok(entries
        .into_iter()
        .map(|entry| DriveEntry {
            id: entry.id,
            name: entry.name,
            is_folder: entry.is_folder,
            drive_id: entry.drive_id,
        })
        .collect())
}
