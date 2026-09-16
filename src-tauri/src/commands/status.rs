use crate::commands::get_db_path;
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
        if let Ok(conn) = Connection::open_with_flags(&db_path, OpenFlags::SQLITE_OPEN_READ_ONLY) {
            // Check integrity
            if let Ok(mut stmt) = conn.prepare("PRAGMA integrity_check;") {
                let integrity: Result<String, _> = stmt.query_row([], |row| row.get(0));
                if let Ok(val) = integrity {
                    db_integrity = val;
                }
            }

            // Google account
            if let Ok(mut stmt) = conn.prepare(
                "SELECT email, status FROM google_accounts ORDER BY updated_at_ms DESC LIMIT 1;",
            ) {
                if let Ok(row) = stmt.query_row([], |r| {
                    Ok((r.get::<_, Option<String>>(0)?, r.get::<_, String>(1)?))
                }) {
                    google_account = row.0;
                    account_status = row.1;
                }
            }

            // Destination profile
            if let Ok(mut stmt) = conn.prepare(
                "SELECT label, destination_parent_id FROM destination_profiles WHERE is_default = 1 LIMIT 1;"
            ) {
                if let Ok(row) = stmt.query_row([], |r| {
                    Ok((r.get::<_, String>(0)?, r.get::<_, String>(1)?))
                }) {
                    destination_label = Some(row.0);
                    destination_id = Some(row.1);
                }
            }

            // Job stats
            if let Ok(mut stmt) = conn.prepare("SELECT COUNT(*) FROM jobs;") {
                total_jobs = stmt.query_row([], |r| r.get(0)).unwrap_or(0);
            }
            if let Ok(mut stmt) = conn.prepare(
                "SELECT COUNT(*) FROM jobs WHERE status IN ('running', 'discovering', 'queued', 'pausing');"
            ) {
                active_jobs = stmt.query_row([], |r| r.get(0)).unwrap_or(0);
            }
            if let Ok(mut stmt) =
                conn.prepare("SELECT COUNT(*) FROM jobs WHERE status = 'completed';")
            {
                completed_jobs = stmt.query_row([], |r| r.get(0)).unwrap_or(0);
            }

            // File & bytes stats
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
    } else {
        db_integrity = "no_db_yet".to_string();
    }

    Ok(SystemStatus {
        google_account,
        account_status,
        service_active,
        service_name: "gdclone-bot.service".to_string(),
        db_integrity,
        app_version: "v0.1.0".to_string(),
        bot_username: Some("Drive502_Bot".to_string()),
        destination_label: destination_label.or_else(|| Some("My Drive / Backup 502".to_string())),
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
