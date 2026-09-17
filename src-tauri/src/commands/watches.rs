use crate::commands::{get_config_path, get_db_path, resolve_watch_actor};
use gdclone_bot::config::AppConfig;
use gdclone_bot::engine::services::watch::{WatchPolicyKind, WatchService};
use gdclone_bot::state::db::Database;
use gdclone_bot::state::repo::{self, WatchResumeResult};
use rusqlite::{Connection, OpenFlags};
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

/// Shared setup for the watch-control commands: open the database and resolve
/// the acting telegram user id (see `resolve_watch_actor`).
async fn open_watch_db(watch_id: &str) -> Result<(Database, i64), String> {
    let db_path = get_db_path();
    if !db_path.exists() {
        return Err("Database does not exist".to_string());
    }
    let actor = {
        let conn = Connection::open_with_flags(&db_path, OpenFlags::SQLITE_OPEN_READ_ONLY)
            .map_err(|e| e.to_string())?;
        resolve_watch_actor(&conn, watch_id)?
    };
    let db = Database::open(&db_path)
        .await
        .map_err(|e| format!("cannot open state database: {e:#}"))?;
    Ok((db, actor))
}

/// Pause a watch through the engine's guarded transition
/// (`active|catching_up|degraded` → `paused`). Never writes `paused`
/// unconditionally — that used to also stop `initializing`/`needs_reconcile`
/// watches behind the engine's back.
#[tauri::command]
pub async fn pause_watch(watch_id: String) -> Result<(), String> {
    let (db, actor) = open_watch_db(&watch_id).await?;
    let changed = WatchService::pause(&db, actor, &watch_id)
        .await
        .map_err(|e| format!("{e:#}"))?;
    if !changed {
        return Err("watch cannot be paused from its current state".to_string());
    }
    Ok(())
}

/// Resume a paused watch through the engine's backlog-aware transition:
/// within the configured backlog limit → `catching_up`, beyond it →
/// `needs_reconcile`. Never writes `active` directly — the dispatcher owns
/// that promotion.
#[tauri::command]
pub async fn resume_watch(watch_id: String) -> Result<(), String> {
    let (db, actor) = open_watch_db(&watch_id).await?;
    let config =
        AppConfig::load(&get_config_path()).map_err(|e| format!("cannot load config: {e:#}"))?;
    match WatchService::resume(
        &db,
        actor,
        &watch_id,
        config.watch.max_backlog_events_per_watch,
    )
    .await
    .map_err(|e| format!("{e:#}"))?
    {
        WatchResumeResult::Resumed | WatchResumeResult::NeedsReconcile { .. } => Ok(()),
        WatchResumeResult::NotResumable => {
            Err("watch cannot be resumed from its current state".to_string())
        }
    }
}

/// Stop (unwatch) through the engine's `stop_watch_for_user` semantics
/// (any status `!= 'stopped'` → `stopped`); the dispatcher ignores stopped
/// watches and re-reads state every cycle.
#[tauri::command]
pub async fn unwatch(watch_id: String) -> Result<bool, String> {
    let (db, actor) = open_watch_db(&watch_id).await?;
    WatchService::stop(&db, actor, &watch_id)
        .await
        .map_err(|e| format!("{e:#}"))
}

/// Update one policy of a watch through the shared [`WatchService`]
/// validator, which covers all three policy kinds and mirrors the
/// `watch_subscriptions` CHECK constraints. The dispatcher reads the policy
/// per event.
#[tauri::command]
pub async fn set_watch_policy(
    watch_id: String,
    policy_kind: String,
    policy_value: String,
) -> Result<(), String> {
    let kind = WatchPolicyKind::parse(&policy_kind)
        .ok_or_else(|| format!("unknown policy kind '{policy_kind}'"))?;
    let (db, actor) = open_watch_db(&watch_id).await?;
    let changed = WatchService::set_policy(&db, actor, &watch_id, kind, &policy_value)
        .await
        .map_err(|e| e.to_string())?;
    if !changed {
        return Err(format!("watch {watch_id} not found"));
    }
    Ok(())
}
