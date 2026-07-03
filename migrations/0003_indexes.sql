CREATE INDEX IF NOT EXISTS idx_jobs_status ON jobs(status, updated_at_ms);
CREATE INDEX IF NOT EXISTS idx_jobs_user ON jobs(telegram_user_id, created_at_ms);
CREATE INDEX IF NOT EXISTS idx_job_items_work ON job_items(job_id, status, id);
CREATE INDEX IF NOT EXISTS idx_mapping_destination ON source_mappings(destination_item_id);
CREATE INDEX IF NOT EXISTS idx_change_events_cursor_sequence ON change_events(cursor_id, sequence);
CREATE UNIQUE INDEX IF NOT EXISTS idx_one_default_destination
ON destination_profiles(google_account_id)
WHERE is_default = 1;
