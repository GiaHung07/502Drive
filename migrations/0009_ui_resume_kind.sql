-- Widen ui_requests.kind to allow 'resume': the GUI cannot spawn the daemon's
-- job resume worker, so resuming a paused job is enqueued as a 'resume'
-- request and the daemon's ui_requests consumer performs the repo transition
-- ('paused' → 'recovering') and spawns the worker in-daemon.
--
-- SQLite cannot alter a CHECK constraint, so the table is rebuilt and the
-- rows are copied over (migration 0008's table has no foreign keys, and the
-- queue is transient state, so a plain INSERT SELECT is sufficient).

CREATE TABLE ui_requests_v2 (
    id TEXT PRIMARY KEY,
    kind TEXT NOT NULL CHECK(kind IN ('clone', 'watch', 'retry', 'resume')),
    payload_json TEXT NOT NULL,
    status TEXT NOT NULL DEFAULT 'pending' CHECK(status IN ('pending', 'accepted', 'rejected')),
    requested_by TEXT,
    created_at_ms INTEGER NOT NULL,
    decided_at_ms INTEGER,
    note TEXT
);

INSERT INTO ui_requests_v2 (id, kind, payload_json, status, requested_by,
                            created_at_ms, decided_at_ms, note)
    SELECT id, kind, payload_json, status, requested_by,
           created_at_ms, decided_at_ms, note
    FROM ui_requests;

DROP TABLE ui_requests;
ALTER TABLE ui_requests_v2 RENAME TO ui_requests;

CREATE INDEX IF NOT EXISTS idx_ui_requests_status_created
    ON ui_requests(status, created_at_ms);
