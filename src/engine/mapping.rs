use rusqlite::params;

use crate::state::{
    db::{Database, now_ms},
    repo,
};

#[derive(Debug, Clone)]
pub struct MappingRecord {
    pub scope_type: String,
    pub scope_id: String,
    pub source_item_id: String,
    pub destination_item_id: String,
    pub source_parent_id: Option<String>,
    pub destination_parent_id: Option<String>,
    pub mime_type: String,
    pub source_name: String,
}

pub async fn record_mapping(db: &Database, record: MappingRecord) -> anyhow::Result<()> {
    db.conn()
        .call(move |conn| {
            conn.execute(
                "INSERT INTO source_mappings (
                    scope_type, scope_id, source_item_id, destination_item_id,
                    source_parent_id, destination_parent_id, mime_type, source_name,
                    mapping_state, updated_at_ms
                 ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, 'active', ?9)
                 ON CONFLICT(scope_type, scope_id, source_item_id) DO UPDATE SET
                    destination_item_id = excluded.destination_item_id,
                    source_parent_id = excluded.source_parent_id,
                    destination_parent_id = excluded.destination_parent_id,
                    mime_type = excluded.mime_type,
                    source_name = excluded.source_name,
                    mapping_state = 'active',
                    updated_at_ms = excluded.updated_at_ms",
                params![
                    record.scope_type,
                    record.scope_id,
                    record.source_item_id,
                    record.destination_item_id,
                    record.source_parent_id,
                    record.destination_parent_id,
                    record.mime_type,
                    record.source_name,
                    now_ms(),
                ],
            )?;
            Ok::<(), rusqlite::Error>(())
        })
        .await?;
    Ok(())
}

pub async fn lookup_mapping(
    db: &Database,
    scope_type: &str,
    scope_id: &str,
    source_item_id: &str,
) -> anyhow::Result<Option<String>> {
    repo::existing_dest_for_source(db, scope_type, scope_id, source_item_id).await
}
