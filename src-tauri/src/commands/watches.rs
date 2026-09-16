use crate::commands::get_db_path;
use rusqlite::{Connection, OpenFlags, params};
use serde::Serialize;

#[derive(Debug, Serialize, Clone)]
pub struct WatchSummary {
    pub id: String,
    pub short_id: String,
    pub source_root_id: String,
    pub destination_root_id: String,
    pub status: String,
    pub backlog_count: i64,
    pub baseline_sequence: i64,
    pub last_consumed_sequence: i64,
    pub updated_at_ms: i64,
}

#[tauri::command]
pub async fn list_watches() -> Result<Vec<WatchSummary>, String> {
    let db_path = get_db_path();
    if !db_path.exists() {
        return Ok(Vec::new());
    }

    let conn = Connection::open_with_flags(&db_path, OpenFlags::SQLITE_OPEN_READ_ONLY)
        .map_err(|e| e.to_string())?;

    let mut stmt = conn
        .prepare(
            "SELECT id, source_root_id, destination_root_id, status,
                baseline_sequence, last_consumed_sequence, updated_at_ms
         FROM watch_subscriptions
         ORDER BY updated_at_ms DESC;",
        )
        .map_err(|e| e.to_string())?;

    let rows = stmt
        .query_map([], |row| {
            let id: String = row.get(0)?;
            let short_id = if id.len() > 8 {
                id[..8].to_string()
            } else {
                id.clone()
            };
            let baseline: i64 = row.get(4)?;
            let last_consumed: i64 = row.get(5)?;
            let backlog = (baseline - last_consumed).max(0);

            Ok(WatchSummary {
                id,
                short_id,
                source_root_id: row.get(1)?,
                destination_root_id: row.get(2)?,
                status: row.get(3)?,
                backlog_count: backlog,
                baseline_sequence: baseline,
                last_consumed_sequence: last_consumed,
                updated_at_ms: row.get(6)?,
            })
        })
        .map_err(|e| e.to_string())?;

    let mut watches = Vec::new();
    for w in rows {
        if let Ok(watch) = w {
            watches.push(watch);
        }
    }
    Ok(watches)
}

#[tauri::command]
pub async fn pause_watch(watch_id: String) -> Result<(), String> {
    let db_path = get_db_path();
    if !db_path.exists() {
        return Err("Database does not exist".to_string());
    }

    let conn = Connection::open(&db_path).map_err(|e| e.to_string())?;
    let now = chrono::Utc::now().timestamp_millis();
    conn.execute(
        "UPDATE watch_subscriptions SET status = 'paused', updated_at_ms = ?1 WHERE id = ?2;",
        params![now, watch_id],
    )
    .map_err(|e| e.to_string())?;

    Ok(())
}

#[tauri::command]
pub async fn resume_watch(watch_id: String) -> Result<(), String> {
    let db_path = get_db_path();
    if !db_path.exists() {
        return Err("Database does not exist".to_string());
    }

    let conn = Connection::open(&db_path).map_err(|e| e.to_string())?;
    let now = chrono::Utc::now().timestamp_millis();
    conn.execute(
        "UPDATE watch_subscriptions SET status = 'active', updated_at_ms = ?1 WHERE id = ?2;",
        params![now, watch_id],
    )
    .map_err(|e| e.to_string())?;

    Ok(())
}
