-- Migration 0010: Telegram user interactive sessions with TTL.
-- Replaces fragile reply-marker string matching with an explicit session FSM.

CREATE TABLE IF NOT EXISTS telegram_sessions (
    id TEXT NOT NULL,
    user_id INTEGER NOT NULL,
    chat_id INTEGER NOT NULL,
    flow TEXT NOT NULL,
    step TEXT NOT NULL,
    payload_json TEXT NOT NULL,
    created_at_ms INTEGER NOT NULL,
    expires_at_ms INTEGER NOT NULL,
    PRIMARY KEY (user_id, chat_id)
);

CREATE INDEX IF NOT EXISTS idx_telegram_sessions_expiry
ON telegram_sessions(expires_at_ms);
