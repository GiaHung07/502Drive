use crate::commands::get_db_path;
use gdclone_bot::state::{db::Database, repo};
use rusqlite::{Connection, OpenFlags};
use serde::Serialize;
use std::process::Command;

#[derive(Debug, Serialize, Clone)]
pub struct SystemStats {
    pub total_jobs: i64,
    pub active_jobs: i64,
    pub completed_jobs: i64,
    pub total_cloned_files: i64,
    pub total_cloned_bytes: i64,
}

#[derive(Debug, Serialize, Clone)]
pub struct SystemStatus {
    pub google_account: Option<String>,
    pub account_status: String,
    pub service_active: bool,
    pub service_name: String,
    pub db_integrity: String,
    pub app_version: String,
    pub bot_username: Option<String>,
    pub destination_label: Option<String>,
    pub destination_id: Option<String>,
    pub stats: SystemStats,
}

fn check_systemd_service_active(service_name: &str) -> bool {
    #[cfg(target_os = "linux")]
    {
        let output = Command::new("systemctl")
            .args(["--user", "is-active", service_name])
            .output();
        if let Ok(out) = output {
            let status = String::from_utf8_lossy(&out.stdout).trim().to_string();
            return status == "active";
        }
    }
    false
}

#[tauri::command]
pub async fn get_system_status() -> Result<SystemStatus, String> {
    let db_path = get_db_path();
    let service_active = check_systemd_service_active("gdclone-bot");

    let mut google_account = None;
    let mut account_status = "disconnected".to_string();
    let mut db_integrity = "unknown".to_string();
    let mut destination_label = None;
    let mut destination_id = None;

    let mut total_jobs = 0i64;
    let mut active_jobs = 0i64;
    let mut completed_jobs = 0i64;
    let mut total_cloned_files = 0i64;
    let mut total_cloned_bytes = 0i64;

    if db_path.exists() {
        // Read-only connection: integrity check plus the two aggregates that
        // have no repo function yet (byte/file sums over job_items).
        if let Ok(conn) = Connection::open_with_flags(&db_path, OpenFlags::SQLITE_OPEN_READ_ONLY) {
            if let Ok(mut stmt) = conn.prepare("PRAGMA integrity_check;") {
                let integrity: Result<String, _> = stmt.query_row([], |row| row.get(0));
                if let Ok(val) = integrity {
                    db_integrity = val;
                }
            }
            if let Ok(mut stmt) =
                conn.prepare("SELECT COALESCE(SUM(completed_items), 0) FROM jobs;")
            {
                total_cloned_files = stmt.query_row([], |r| r.get(0)).unwrap_or(0);
            }
            if let Ok(mut stmt) = conn.prepare(
                "SELECT COALESCE(SUM(size_bytes), 0) FROM job_items WHERE status = 'done';",
            ) {
                total_cloned_bytes = stmt.query_row([], |r| r.get(0)).unwrap_or(0);
            }
        }

        // Everything else goes through the shared repo layer so the GUI and
        // the daemon always agree on the definitions (active = queued |
        // discovering | running | pausing | paused | cancelling | recovering).
        if let Ok(db) = Database::open(&db_path).await {
            if let Ok(Some(account)) = repo::google_account_secret(&db, "default").await {
                google_account = account.email;
                account_status = account.status;
            }
            // Default destination from destination_profiles; None (empty
            // state) when nothing is configured — no invented fallback label.
            if let Ok(Some(profile)) = repo::default_destination_profile(&db, "default").await {
                destination_label = Some(profile.label);
                destination_id = Some(profile.destination_parent_id);
            }
            if let Ok(counts) = repo::job_status_counts(&db).await {
                for count in counts {
                    total_jobs += count.count;
                    if count.status == "completed" {
                        completed_jobs = count.count;
                    }
                }
            }
            active_jobs = repo::active_job_count(&db).await.unwrap_or(0);
        }
    } else {
        db_integrity = "no_db_yet".to_string();
    }

    Ok(SystemStatus {
        google_account,
        account_status,
        service_active,
        service_name: "gdclone-bot.service".to_string(),
        db_integrity,
        app_version: format!("v{}", env!("CARGO_PKG_VERSION")),
        // No fake bot username: the real handle only exists after a getMe
        // round-trip against the configured bot token, which the status panel
        // does not perform. The frontend already tolerates `None`.
        bot_username: None,
        destination_label,
        destination_id,
        stats: SystemStats {
            total_jobs,
            active_jobs,
            completed_jobs,
            total_cloned_files,
            total_cloned_bytes,
        },
    })
}
