//! GUI↔engine bridge commands.
//!
//! Design: the GUI process NEVER runs engine work (pollers, clone engine,
//! watch initializer live in the daemon `502drive run`). Mutating operations
//! are enqueued into the shared `ui_requests` table (migration 0008) and the
//! daemon's consumer (`gdclone_bot::engine::ui_requests`) picks them up ~2s
//! later. `browse_drive_children` is the one exception that performs a Drive
//! call in the GUI process: it is a pure read (list folders) through the
//! engine library, so the frontend folder picker works even while inspecting
//! a running daemon.

use crate::commands::{get_config_path, get_db_path};
use gdclone_bot::{
    config::AppConfig,
    drive::{client::DriveClient, id_parser, token_manager::TokenManager},
    engine::ui_requests::{CloneRequestPayload, RetryRequestPayload, WatchRequestPayload},
    state::{db::Database, ui_requests},
};
use rusqlite::{Connection, OpenFlags};
use serde::Serialize;

fn open_queue_conn() -> Result<Connection, String> {
    let db_path = get_db_path();
    if !db_path.exists() {
        return Err("Database does not exist".to_string());
    }
    let conn = Connection::open(&db_path).map_err(|e| e.to_string())?;
    let table_exists: bool = conn
        .query_row(
            "SELECT EXISTS(SELECT 1 FROM sqlite_master WHERE type='table' AND name='ui_requests')",
            [],
            |row| row.get(0),
        )
        .map_err(|e| e.to_string())?;
    if !table_exists {
        return Err(
            "ui_requests table missing — start the 502drive daemon once so it applies migrations"
                .to_string(),
        );
    }
    Ok(conn)
}

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
/// root ("root") plus the account's shared drives. Read-only Drive calls.
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
    let token_manager = TokenManager::new(config, db);
    let token = token_manager
        .access_token("default")
        .await
        .map_err(|e| format!("cannot get Drive access token: {e:#}"))?;
    let drive = DriveClient::new();

    match parent_id.as_deref() {
        None => {
            // Top level: shared drives first, then My Drive root folders.
            let mut entries = Vec::new();
            match drive
                .list_shared_drives_page_size(token.as_str(), None, 200)
                .await
            {
                Ok(drives) => {
                    for d in drives.drives {
                        let id = d.id;
                        entries.push(DriveEntry {
                            name: d.name,
                            is_folder: true,
                            drive_id: Some(id.clone()),
                            id,
                        });
                    }
                }
                Err(err) => {
                    // Shared drives can 403 on plain Gmail accounts — degrade
                    // gracefully and still show My Drive.
                    if err.status().map(|s| s.as_u16()) != Some(403) {
                        return Err(format!("list shared drives failed: {err}"));
                    }
                }
            }
            let page = drive
                .list_child_folders_page_size(token.as_str(), "root", None, None, 200, None)
                .await
                .map_err(|e| format!("list My Drive folders failed: {e}"))?;
            for f in page.files {
                let is_folder = f.is_folder();
                entries.push(DriveEntry {
                    id: f.id,
                    name: f.name,
                    is_folder,
                    drive_id: f.drive_id,
                });
            }
            Ok(entries)
        }
        Some(parent) => {
            let page = drive
                .list_child_folders_page_size(
                    token.as_str(),
                    parent,
                    None,
                    None,
                    200,
                    drive_id.as_deref(),
                )
                .await
                .map_err(|e| format!("list folders failed: {e}"))?;
            Ok(page
                .files
                .into_iter()
                .map(|f| {
                    let is_folder = f.is_folder();
                    DriveEntry {
                        id: f.id,
                        name: f.name,
                        is_folder,
                        drive_id: f.drive_id,
                    }
                })
                .collect())
        }
    }
}
