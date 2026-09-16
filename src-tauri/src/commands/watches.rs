use crate::commands::get_db_path;
use gdclone_bot::state::repo;
use rusqlite::{Connection, OpenFlags, params};
use serde::Serialize;

#[derive(Debug, Serialize, Clone)]
pub struct WatchSummary {
    pub id: String,
    pub short_id: String,
    pub source_root_id: String,
    pub destination_root_id: String,
    pub status: String,
    /// Pending change events = cursor's `change_cursors.last_event_sequence`
    /// minus the watch's `last_consumed_sequence`. (Previously this was
    /// computed from `baseline_sequence`, which goes stale after catch-up.)
    pub backlog_count: i64,
    pub baseline_sequence: i64,
    pub last_consumed_sequence: i64,
    pub cursor_last_event_sequence: i64,
    /// JSON string array, e.g. `["*.tmp", "~$*"]`.
    pub exclude_globs: String,
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

    // Shared query with the engine (repo::watch_summaries_sync): LEFT JOIN on
    // change_cursors so the backlog reflects the live change feed.
    let rows = repo::watch_summaries_sync(&conn).map_err(|e| e.to_string())?;

    Ok(rows
        .into_iter()
        .map(|w| WatchSummary {
            short_id: short_id(&w.id),
            id: w.id,
            source_root_id: w.source_root_id,
            destination_root_id: w.destination_root_id,
            status: w.status,
            backlog_count: w.backlog_count,
            baseline_sequence: w.baseline_sequence,
            last_consumed_sequence: w.last_consumed_sequence,
            cursor_last_event_sequence: w.cursor_last_event_sequence,
            exclude_globs: w.exclude_globs,
            updated_at_ms: w.updated_at_ms,
        })
        .collect())
}

fn short_id(id: &str) -> String {
    if id.len() > 8 {
        id[..8].to_string()
    } else {
        id.to_string()
    }
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

/// Stop (unwatch). Safe as a direct write: the engine's own
/// `stop_watch_for_user` is the same unconditional status transition
/// (`status != 'stopped'` → `'stopped'`); the dispatcher ignores stopped
/// watches and the poller/dispatcher re-read state every cycle.
#[tauri::command]
pub async fn unwatch(watch_id: String) -> Result<bool, String> {
    let db_path = get_db_path();
    if !db_path.exists() {
        return Err("Database does not exist".to_string());
    }

    let conn = Connection::open(&db_path).map_err(|e| e.to_string())?;
    let now = chrono::Utc::now().timestamp_millis();
    let changed = conn
        .execute(
            "UPDATE watch_subscriptions SET status = 'stopped', updated_at_ms = ?1
             WHERE id = ?2 AND status != 'stopped';",
            params![now, watch_id],
        )
        .map_err(|e| e.to_string())?;
    Ok(changed > 0)
}

/// Update one policy of a watch. Safe as a direct write: the engine's
/// `set_watch_*_policy` repo functions are plain column UPDATEs with no status
/// guard; the dispatcher reads the policy per event. Values are validated
/// against the same sets as the `watch_subscriptions` CHECK constraints.
#[tauri::command]
pub async fn set_watch_policy(
    watch_id: String,
    policy_kind: String,
    policy_value: String,
) -> Result<(), String> {
    let db_path = get_db_path();
    if !db_path.exists() {
        return Err("Database does not exist".to_string());
    }

    let (column, allowed): (&str, &[&str]) = match policy_kind.as_str() {
        "content_update" => (
            "content_update_policy",
            &["versioned_copy", "replace_copy", "manual_confirmation"],
        ),
        "deletion" => (
            "deletion_policy",
            &["preserve_destination", "manual_confirmation"],
        ),
        "move_out" => ("move_out_policy", &["detach", "keep_following"]),
        other => return Err(format!("unknown policy kind '{other}'")),
    };
    if !allowed.contains(&policy_value.as_str()) {
        return Err(format!(
            "invalid {} policy '{}'; expected one of: {}",
            policy_kind,
            policy_value,
            allowed.join(", ")
        ));
    }

    let conn = Connection::open(&db_path).map_err(|e| e.to_string())?;
    let now = chrono::Utc::now().timestamp_millis();
    let sql =
        format!("UPDATE watch_subscriptions SET {column} = ?1, updated_at_ms = ?2 WHERE id = ?3;");
    conn.execute(&sql, params![policy_value, now, watch_id])
        .map_err(|e| e.to_string())?;
    Ok(())
}
