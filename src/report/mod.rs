use std::{
    fs::File,
    path::{Path, PathBuf},
};

use rusqlite::params;
use serde::Serialize;

use crate::{drive::types::FOLDER_MIME_TYPE, state::db::Database};

#[derive(Debug, Serialize)]
pub struct ReportItem {
    pub source_item_id: String,
    pub dest_item_id: Option<String>,
    pub name: String,
    pub status: String,
    pub attempts: u32,
    pub last_error: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ReportPaths {
    pub json: PathBuf,
    pub csv: PathBuf,
}

pub async fn write_job_reports(
    db: &Database,
    report_dir: &Path,
    job_id: &str,
) -> anyhow::Result<ReportPaths> {
    let items = collect_job_items(db, job_id).await?;
    let json = report_dir.join(format!("{job_id}.json"));
    let csv = report_dir.join(format!("{job_id}.csv"));
    write_json(&json, &items)?;
    write_csv(&csv, &items)?;
    Ok(ReportPaths { json, csv })
}

pub async fn collect_job_items(db: &Database, job_id: &str) -> anyhow::Result<Vec<ReportItem>> {
    let job_id = job_id.to_string();
    Ok(db
        .conn()
        .call(move |conn| {
            let mut items = Vec::new();
            let mut stmt = conn.prepare(
                "SELECT source_item_id, destination_item_id, source_name, status,
                        attempts, last_error_code, last_error_message
                 FROM job_items
                 WHERE job_id = ?1
                 ORDER BY source_name, source_item_id",
            )?;
            let rows = stmt.query_map(params![job_id], |row| {
                let error_code: Option<String> = row.get(5)?;
                let error_message: Option<String> = row.get(6)?;
                Ok(ReportItem {
                    source_item_id: row.get(0)?,
                    dest_item_id: row.get(1)?,
                    name: row.get(2)?,
                    status: row.get(3)?,
                    attempts: row.get::<_, i64>(4)? as u32,
                    last_error: join_error(error_code, error_message),
                })
            })?;
            for row in rows {
                items.push(row?);
            }

            let mut stmt = conn.prepare(
                "SELECT source_item_id, destination_item_id, source_name, mapping_state
                 FROM source_mappings
                 WHERE scope_type = 'job'
                   AND scope_id = ?1
                   AND mime_type = ?2
                   AND source_item_id NOT IN (
                       SELECT source_item_id FROM job_items WHERE job_id = ?1
                   )
                 ORDER BY source_name, source_item_id",
            )?;
            let rows = stmt.query_map(params![job_id, FOLDER_MIME_TYPE], |row| {
                Ok(ReportItem {
                    source_item_id: row.get(0)?,
                    dest_item_id: row.get(1)?,
                    name: row.get(2)?,
                    status: row.get(3)?,
                    attempts: 0,
                    last_error: None,
                })
            })?;
            for row in rows {
                items.push(row?);
            }
            Ok::<Vec<ReportItem>, rusqlite::Error>(items)
        })
        .await?)
}

pub fn write_json(path: &Path, items: &[ReportItem]) -> anyhow::Result<()> {
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    let file = File::create(path)?;
    serde_json::to_writer_pretty(file, items)?;
    Ok(())
}

pub fn write_csv(path: &Path, items: &[ReportItem]) -> anyhow::Result<()> {
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    let mut writer = csv::Writer::from_path(path)?;
    for item in items {
        writer.serialize(item)?;
    }
    writer.flush()?;
    Ok(())
}

fn join_error(code: Option<String>, message: Option<String>) -> Option<String> {
    match (code, message) {
        (Some(code), Some(message)) => Some(format!("{code}: {message}")),
        (Some(code), None) => Some(code),
        (None, Some(message)) => Some(message),
        (None, None) => None,
    }
}
