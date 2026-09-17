pub mod admin;
pub mod config;
pub mod doctor;
pub mod features;
pub mod jobs;
pub mod logs;
pub mod service;
pub mod status;
pub mod watches;

use directories::ProjectDirs;
use gdclone_bot::config::AppConfig;
use rusqlite::{Connection, OpenFlags, OptionalExtension};
use std::path::PathBuf;

pub fn get_project_dirs() -> Option<ProjectDirs> {
    ProjectDirs::from("dev", "gdclone", "gdclone-bot")
}

pub fn get_db_path() -> PathBuf {
    if let Some(dirs) = get_project_dirs() {
        dirs.data_dir().join("state.db")
    } else {
        PathBuf::from("state.db")
    }
}

pub fn get_config_path() -> PathBuf {
    if let Some(dirs) = get_project_dirs() {
        dirs.config_dir().join("config.toml")
    } else {
        PathBuf::from("config.toml")
    }
}

pub fn get_log_path() -> PathBuf {
    if let Some(dirs) = get_project_dirs() {
        dirs.data_dir().join("logs").join("502drive.log")
    } else {
        PathBuf::from("502drive.log")
    }
}

/// Open the shared database for the `ui_requests` queue. The table only
/// exists after the daemon has applied migrations at least once.
pub(crate) fn open_queue_conn() -> Result<Connection, String> {
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

/// Resolve the acting telegram user id for the local GUI: the configured
/// owner (`telegram.owner_telegram_id`) wins; without a usable config, fall
/// back to the enabled owner row in `authorized_users`.
pub(crate) fn resolve_owner_id() -> Result<i64, String> {
    if let Ok(config) = AppConfig::load(&get_config_path()) {
        if config.telegram.owner_telegram_id > 0 {
            return Ok(config.telegram.owner_telegram_id);
        }
    }
    let db_path = get_db_path();
    if !db_path.exists() {
        return Err("Database does not exist".to_string());
    }
    let conn = Connection::open_with_flags(&db_path, OpenFlags::SQLITE_OPEN_READ_ONLY)
        .map_err(|e| e.to_string())?;
    let owner: Option<i64> = conn
        .query_row(
            "SELECT telegram_user_id FROM authorized_users
             WHERE role = 'owner' AND enabled = 1
             ORDER BY telegram_user_id LIMIT 1",
            [],
            |row| row.get(0),
        )
        .optional()
        .map_err(|e| e.to_string())?;
    owner.ok_or_else(|| "no owner configured — finish the setup wizard first".to_string())
}

/// Resolve the actor for a job row. Jobs created by the GUI queue are owned
/// by `telegram_user_id = 0`; jobs created by the telegram bot are owned by
/// the requesting user. The local GUI is trusted to act as the configured
/// owner for those rows, and as user `0` for its own.
pub(crate) fn resolve_job_actor(conn: &Connection, job_id: &str) -> Result<i64, String> {
    let owner: Option<i64> = conn
        .query_row(
            "SELECT telegram_user_id FROM jobs WHERE id = ?1",
            [job_id],
            |row| row.get(0),
        )
        .optional()
        .map_err(|e| e.to_string())?;
    match owner {
        None => Err(format!("job {job_id} not found")),
        Some(0) => Ok(0),
        Some(_) => resolve_owner_id(),
    }
}

/// [`resolve_job_actor`] for watch rows.
pub(crate) fn resolve_watch_actor(conn: &Connection, watch_id: &str) -> Result<i64, String> {
    let owner: Option<i64> = conn
        .query_row(
            "SELECT telegram_user_id FROM watch_subscriptions WHERE id = ?1",
            [watch_id],
            |row| row.get(0),
        )
        .optional()
        .map_err(|e| e.to_string())?;
    match owner {
        None => Err(format!("watch {watch_id} not found")),
        Some(0) => Ok(0),
        Some(_) => resolve_owner_id(),
    }
}
