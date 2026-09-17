-- Migration 0011: Cache source and destination names to eliminate repeated Drive API lookups,
-- and track last_consumed_at_ms for precise sync time calculation.

ALTER TABLE jobs ADD COLUMN source_name TEXT;
ALTER TABLE jobs ADD COLUMN destination_name TEXT;

ALTER TABLE watch_subscriptions ADD COLUMN source_name TEXT;
ALTER TABLE watch_subscriptions ADD COLUMN destination_name TEXT;
ALTER TABLE watch_subscriptions ADD COLUMN last_consumed_at_ms INTEGER;
