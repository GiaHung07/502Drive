-- Migration 0012: Persistent retry state on job_items and durable telegram_updates deduplication table.

ALTER TABLE job_items ADD COLUMN next_attempt_at_ms INTEGER;
ALTER TABLE job_items ADD COLUMN last_error_reason TEXT;
ALTER TABLE job_items ADD COLUMN retry_class TEXT;

CREATE INDEX IF NOT EXISTS idx_job_items_retry ON job_items(job_id, status, next_attempt_at_ms);

CREATE TABLE IF NOT EXISTS telegram_updates (
    update_id INTEGER PRIMARY KEY,
    received_at_ms INTEGER NOT NULL,
    payload_json TEXT,
    processed_at_ms INTEGER
);

CREATE INDEX IF NOT EXISTS idx_telegram_updates_received ON telegram_updates(received_at_ms);
