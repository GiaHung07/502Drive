use crate::commands::get_db_path;
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

#[tauri::command]
pub async fn pause_job(job_id: String) -> Result<(), String> {
    let db_path = get_db_path();
    if !db_path.exists() {
        return Err("Database does not exist".to_string());
    }

    let conn = Connection::open(&db_path).map_err(|e| e.to_string())?;
    let now = chrono::Utc::now().timestamp_millis();
    conn.execute(
        "UPDATE jobs SET status = 'paused', updated_at_ms = ?1 WHERE id = ?2 AND status IN ('running', 'queued', 'discovering');",
        params![now, job_id]
    ).map_err(|e| e.to_string())?;

    Ok(())
}

#[tauri::command]
pub async fn resume_job(job_id: String) -> Result<(), String> {
    let db_path = get_db_path();
    if !db_path.exists() {
        return Err("Database does not exist".to_string());
    }

    let conn = Connection::open(&db_path).map_err(|e| e.to_string())?;
    let now = chrono::Utc::now().timestamp_millis();
    conn.execute(
        "UPDATE jobs SET status = 'queued', updated_at_ms = ?1 WHERE id = ?2 AND status = 'paused';",
        params![now, job_id]
    ).map_err(|e| e.to_string())?;

    Ok(())
}

#[tauri::command]
pub async fn cancel_job(job_id: String) -> Result<(), String> {
    let db_path = get_db_path();
    if !db_path.exists() {
        return Err("Database does not exist".to_string());
    }

    let conn = Connection::open(&db_path).map_err(|e| e.to_string())?;
    let now = chrono::Utc::now().timestamp_millis();
    conn.execute(
        "UPDATE jobs SET status = 'cancelled', updated_at_ms = ?1 WHERE id = ?2;",
        params![now, job_id],
    )
    .map_err(|e| e.to_string())?;

    Ok(())
}
