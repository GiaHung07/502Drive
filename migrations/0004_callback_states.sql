CREATE TABLE IF NOT EXISTS telegram_callback_states (
    id TEXT PRIMARY KEY,
    telegram_user_id INTEGER NOT NULL,
    chat_id INTEGER NOT NULL,
    action TEXT NOT NULL,
    payload TEXT NOT NULL,
    expires_at_ms INTEGER NOT NULL,
    created_at_ms INTEGER NOT NULL
);

CREATE INDEX IF NOT EXISTS idx_callback_states_expiry
ON telegram_callback_states(expires_at_ms);
