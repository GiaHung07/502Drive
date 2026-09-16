-- UI request queue: the GUI process enqueues requests here; the daemon
-- (502drive run) consumes and executes them. Multi-process safe: both sides
-- access the shared WAL database, the consumer claims rows atomically by
-- flipping status 'pending' → 'accepted'/'rejected'.
CREATE TABLE IF NOT EXISTS ui_requests (
    id TEXT PRIMARY KEY,
    kind TEXT NOT NULL CHECK(kind IN ('clone', 'watch', 'retry')),
    payload_json TEXT NOT NULL,
    status TEXT NOT NULL DEFAULT 'pending' CHECK(status IN ('pending', 'accepted', 'rejected')),
    requested_by TEXT,
    created_at_ms INTEGER NOT NULL,
    decided_at_ms INTEGER,
    note TEXT
);

CREATE INDEX IF NOT EXISTS idx_ui_requests_status_created
    ON ui_requests(status, created_at_ms);
