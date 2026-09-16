//! UI request queue (`ui_requests` table, migration 0008).
//!
//! The GUI process cannot run engine work (pollers, clone engine, watch
//! initializer) — the daemon (`502drive run`) owns all of that. Instead the
//! GUI enqueues requests here and the daemon's consumer
//! ([`crate::engine::ui_requests`]) picks them up.
//!
//! The SQL core is exposed as synchronous functions taking a plain
//! `rusqlite::Connection` so the GUI crate can reuse the exact same statements
//! against its own (read-write or read-only) connection, while the daemon
//! wraps them with the tokio_rusqlite pool. All access stays WAL/multi-process
//! safe; the consumer claims a row atomically by flipping
//! `status='pending' → 'accepted'/'rejected'` in a single conditional UPDATE.

use rusqlite::{Connection, OptionalExtension, params};
use serde::Serialize;

use super::db::{Database, now_ms};

pub const KIND_CLONE: &str = "clone";
pub const KIND_WATCH: &str = "watch";
pub const KIND_RETRY: &str = "retry";

#[derive(Debug, Clone, Serialize)]
pub struct UiRequest {
    pub id: String,
    pub kind: String,
    pub payload_json: String,
    pub status: String,
    pub requested_by: Option<String>,
    pub created_at_ms: i64,
    pub decided_at_ms: Option<i64>,
    pub note: Option<String>,
}

fn row_to_request(row: &rusqlite::Row<'_>) -> rusqlite::Result<UiRequest> {
    Ok(UiRequest {
        id: row.get(0)?,
        kind: row.get(1)?,
        payload_json: row.get(2)?,
        status: row.get(3)?,
        requested_by: row.get(4)?,
        created_at_ms: row.get(5)?,
        decided_at_ms: row.get(6)?,
        note: row.get(7)?,
    })
}

// ── Sync core (shared with the GUI crate) ────────────────────────────────────

/// Insert a pending request. Returns the generated request id.
pub fn insert_sync(
    conn: &Connection,
    kind: &str,
    payload_json: &str,
    requested_by: Option<&str>,
) -> rusqlite::Result<String> {
    let id = uuid::Uuid::new_v4().to_string();
    conn.execute(
        "INSERT INTO ui_requests (id, kind, payload_json, status, requested_by, created_at_ms)
         VALUES (?1, ?2, ?3, 'pending', ?4, ?5)",
        params![id, kind, payload_json, requested_by, now_ms()],
    )?;
    Ok(id)
}

/// Fetch one request by id.
pub fn get_sync(conn: &Connection, id: &str) -> rusqlite::Result<Option<UiRequest>> {
    conn.query_row(
        "SELECT id, kind, payload_json, status, requested_by, created_at_ms, decided_at_ms, note
         FROM ui_requests WHERE id = ?1",
        params![id],
        row_to_request,
    )
    .optional()
}

// ── Daemon-side async wrappers ───────────────────────────────────────────────

/// Insert a pending request through the daemon's connection pool.
pub async fn enqueue(
    db: &Database,
    kind: &str,
    payload_json: &str,
    requested_by: Option<&str>,
) -> anyhow::Result<String> {
    let kind = kind.to_string();
    let payload_json = payload_json.to_string();
    let requested_by = requested_by.map(ToOwned::to_owned);
    Ok(db
        .conn()
        .call(move |conn| insert_sync(conn, &kind, &payload_json, requested_by.as_deref()))
        .await?)
}

/// Fetch one request by id through the daemon's connection pool.
pub async fn get(db: &Database, id: &str) -> anyhow::Result<Option<UiRequest>> {
    let id = id.to_string();
    Ok(db.conn().call(move |conn| get_sync(conn, &id)).await?)
}

/// All pending requests, oldest first.
pub async fn pending(db: &Database) -> anyhow::Result<Vec<UiRequest>> {
    Ok(db
        .conn()
        .call(|conn| {
            let mut stmt = conn.prepare(
                "SELECT id, kind, payload_json, status, requested_by, created_at_ms, decided_at_ms, note
                 FROM ui_requests WHERE status = 'pending'
                 ORDER BY created_at_ms, id",
            )?;
            let rows = stmt
                .query_map([], row_to_request)?
                .collect::<Result<Vec<_>, _>>()?;
            Ok::<Vec<UiRequest>, rusqlite::Error>(rows)
        })
        .await?)
}

/// Atomically decide a pending request. Returns false when the row is gone or
/// was already decided (lost claim), so concurrent consumers never double-run.
pub async fn decide(db: &Database, id: &str, accepted: bool, note: &str) -> anyhow::Result<bool> {
    let id = id.to_string();
    let status = if accepted { "accepted" } else { "rejected" };
    let note = note.to_string();
    let changed = db
        .conn()
        .call(move |conn| {
            let changed = conn.execute(
                "UPDATE ui_requests
                 SET status = ?1, decided_at_ms = ?2, note = ?3
                 WHERE id = ?4 AND status = 'pending'",
                params![status, now_ms(), note, id],
            )?;
            Ok::<usize, rusqlite::Error>(changed)
        })
        .await?;
    Ok(changed > 0)
}

/// Replace the note on an already-decided request (e.g. attaching the job id
/// once the spawned clone finishes). Never touches the status.
pub async fn set_note(db: &Database, id: &str, note: &str) -> anyhow::Result<()> {
    let id = id.to_string();
    let note = note.to_string();
    db.conn()
        .call(move |conn| {
            conn.execute(
                "UPDATE ui_requests SET note = ?1 WHERE id = ?2",
                params![note, id],
            )?;
            Ok::<(), rusqlite::Error>(())
        })
        .await?;
    Ok(())
}
