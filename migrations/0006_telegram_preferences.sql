CREATE TABLE IF NOT EXISTS telegram_user_preferences (
    telegram_user_id INTEGER PRIMARY KEY,
    language TEXT NOT NULL,
    updated_at_ms INTEGER NOT NULL
);
