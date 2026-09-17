use crate::commands::{get_db_path, open_queue_conn, resolve_job_actor};
use gdclone_bot::engine::services::JobService;
use gdclone_bot::engine::ui_requests::{REQUESTED_BY_GUI, ResumeRequestPayload};
use gdclone_bot::state::{db::Database, ui_requests};
use rusqlite::{Connection, OpenFlags, params};
use serde::Serialize;

#[derive(Debug, Serialize, Clone)]
pub struct JobSummary {
    pub id: String,
    pub short_id: String,
    pub kind: String,
    pub status: String,
    pub source_root_id: String,
    pub destination_parent_id: String,
    pub total_discovered: i64,
    pub completed_items: i64,
    pub failed_items: i64,
    pub skipped_items: i64,
    pub progress_pct: f64,
    pub error_summary: Option<String>,
    pub created_at_ms: i64,
    pub updated_at_ms: i64,
    pub speed_bytes_per_sec: Option<f64>,
    pub eta_seconds: Option<f64>,
}

#[tauri::command]
pub async fn list_jobs(limit: Option<usize>) -> Result<Vec<JobSummary>, String> {
    let db_path = get_db_path();
    if !db_path.exists() {
        return Ok(Vec::new());
    }

    let conn = Connection::open_with_flags(&db_path, OpenFlags::SQLITE_OPEN_READ_ONLY)
        .map_err(|e| e.to_string())?;

    let max_rows = limit.unwrap_or(30);
    let mut stmt = conn
        .prepare(
            "SELECT id, kind, status, source_root_id, destination_parent_id,
                total_discovered, completed_items, failed_items, skipped_items,
                error_summary, created_at_ms, updated_at_ms
         FROM jobs
         ORDER BY updated_at_ms DESC
         LIMIT ?1;",
        )
        .map_err(|e| e.to_string())?;

    let rows = stmt
        .query_map(params![max_rows as i64], |row| {
            let id: String = row.get(0)?;
            let short_id = if id.len() > 8 {
                id[..8].to_string()
            } else {
                id.clone()
            };
            let total: i64 = row.get(5)?;
            let completed: i64 = row.get(6)?;
            let progress_pct = if total > 0 {
                (completed as f64 / total as f64 * 100.0).clamp(0.0, 100.0)
            } else {
                0.0
            };

            Ok(JobSummary {
                id,
                short_id,
                kind: row.get(1)?,
                status: row.get(2)?,
                source_root_id: row.get(3)?,
                destination_parent_id: row.get(4)?,
                total_discovered: total,
                completed_items: completed,
                failed_items: row.get(7)?,
                skipped_items: row.get(8)?,
                progress_pct,
                error_summary: row.get(9)?,
                created_at_ms: row.get(10)?,
                updated_at_ms: row.get(11)?,
                speed_bytes_per_sec: None,
                eta_seconds: None,
            })
        })
        .map_err(|e| e.to_string())?;

    let mut jobs = Vec::new();
    for job in rows {
        if let Ok(j) = job {
            jobs.push(j);
        }
    }
    Ok(jobs)
}

/// Shared setup for the job-control commands: open the database and resolve
/// the acting telegram user id (see `resolve_job_actor`).
async fn open_job_db(job_id: &str) -> Result<(Database, i64), String> {
    let db_path = get_db_path();
    if !db_path.exists() {
        return Err("Database does not exist".to_string());
    }
    let actor = {
        let conn = Connection::open_with_flags(&db_path, OpenFlags::SQLITE_OPEN_READ_ONLY)
            .map_err(|e| e.to_string())?;
        resolve_job_actor(&conn, job_id)?
    };
    let db = Database::open(&db_path)
        .await
        .map_err(|e| format!("cannot open state database: {e:#}"))?;
    Ok((db, actor))
}

/// Pause a job through the engine's cooperative handshake
/// (`discovering|running|recovering` → `pausing`); the engine's
/// `check_job_control` completes the transition to `paused`. Never writes
/// `paused` directly — that used to skip the handshake and stall running jobs.
#[tauri::command]
pub async fn pause_job(job_id: String) -> Result<(), String> {
    let (db, actor) = open_job_db(&job_id).await?;
    let changed = JobService::pause(&db, actor, &job_id)
        .await
        .map_err(|e| format!("{e:#}"))?;
    if !changed {
        return Err("job cannot be paused from its current state".to_string());
    }
    Ok(())
}

/// Resume a paused job by enqueueing a `resume` ui_request (migration 0009).
/// The repo transition (`paused` → `recovering`) and the resume worker must
/// both run in the daemon process — writing `queued` from the GUI used to
/// leave the job in a dead state nothing picks up.
#[tauri::command]
pub fn resume_job(job_id: String) -> Result<(), String> {
    let job_id = job_id.trim().to_string();
    if job_id.is_empty() {
        return Err("job_id must not be empty".to_string());
    }
    let payload = ResumeRequestPayload { job_id };
    let payload_json = serde_json::to_string(&payload).map_err(|e| e.to_string())?;
    let conn = open_queue_conn()?;
    ui_requests::insert_sync(
        &conn,
        ui_requests::KIND_RESUME,
        &payload_json,
        Some(REQUESTED_BY_GUI),
    )
    .map_err(|e| e.to_string())?;
    Ok(())
}

/// Cancel a job with the engine's two-step guarded semantics:
/// `queued|paused` → `cancelled`, `discovering|running|pausing|recovering` →
/// `cancelling` (cooperative stop). Never force-writes `cancelled` over a
/// running job — that used to bypass the engine's stop handshake.
#[tauri::command]
pub async fn cancel_job(job_id: String) -> Result<(), String> {
    let (db, actor) = open_job_db(&job_id).await?;
    let changed = JobService::cancel(&db, actor, &job_id)
        .await
        .map_err(|e| format!("{e:#}"))?;
    if !changed {
        return Err("job cannot be cancelled from its current state".to_string());
    }
    Ok(())
}
