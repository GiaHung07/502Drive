-- Migration 0005: add last_event_at_ms to change_cursors for adaptive polling.
-- Existing rows get NULL (treated as "no recent event" = idle).
ALTER TABLE change_cursors
    ADD COLUMN last_event_at_ms INTEGER;
