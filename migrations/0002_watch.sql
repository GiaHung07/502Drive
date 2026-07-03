CREATE TABLE IF NOT EXISTS change_cursors (
    id TEXT PRIMARY KEY,
    google_account_id TEXT NOT NULL REFERENCES google_accounts(id),
    corpus_kind TEXT NOT NULL CHECK(corpus_kind IN ('user', 'shared_drive')),
    drive_id TEXT,
    current_page_token TEXT NOT NULL,
    last_event_sequence INTEGER NOT NULL DEFAULT 0,
    next_poll_at_ms INTEGER NOT NULL,
    last_success_at_ms INTEGER,
    consecutive_error_count INTEGER NOT NULL DEFAULT 0,
    UNIQUE(google_account_id, corpus_kind, drive_id)
);

CREATE TABLE IF NOT EXISTS change_events (
    sequence INTEGER PRIMARY KEY AUTOINCREMENT,
    cursor_id TEXT NOT NULL REFERENCES change_cursors(id) ON DELETE CASCADE,
    request_page_token TEXT NOT NULL,
    ordinal_in_page INTEGER NOT NULL,
    file_id TEXT NOT NULL,
    removed INTEGER NOT NULL CHECK(removed IN (0, 1)),
    file_json TEXT,
    received_at_ms INTEGER NOT NULL,
    UNIQUE(cursor_id, request_page_token, ordinal_in_page)
);

CREATE TABLE IF NOT EXISTS watch_subscriptions (
    id TEXT PRIMARY KEY,
    google_account_id TEXT NOT NULL REFERENCES google_accounts(id),
    cursor_id TEXT NOT NULL REFERENCES change_cursors(id),
    telegram_user_id INTEGER NOT NULL,
    chat_id INTEGER NOT NULL,
    source_root_id TEXT NOT NULL,
    source_resource_key TEXT,
    source_drive_id TEXT,
    destination_root_id TEXT NOT NULL,
    destination_drive_id TEXT,
    status TEXT NOT NULL CHECK(status IN (
        'initializing', 'catching_up', 'active',
        'paused', 'degraded', 'needs_reconcile', 'stopped'
    )),
    content_update_policy TEXT NOT NULL CHECK(content_update_policy IN (
        'versioned_copy', 'replace_copy', 'manual_confirmation'
    )),
    deletion_policy TEXT NOT NULL CHECK(deletion_policy IN (
        'preserve_destination', 'manual_confirmation'
    )),
    move_out_policy TEXT NOT NULL CHECK(move_out_policy IN (
        'detach', 'keep_following'
    )),
    baseline_sequence INTEGER NOT NULL,
    last_consumed_sequence INTEGER NOT NULL,
    created_at_ms INTEGER NOT NULL,
    updated_at_ms INTEGER NOT NULL
);

CREATE TABLE IF NOT EXISTS watch_event_applications (
    watch_id TEXT NOT NULL REFERENCES watch_subscriptions(id) ON DELETE CASCADE,
    event_sequence INTEGER NOT NULL REFERENCES change_events(sequence) ON DELETE CASCADE,
    classification TEXT NOT NULL CHECK(classification IN (
        'new_item', 'content_changed', 'renamed', 'moved_inside',
        'moved_outside', 'moved_back', 'trashed_or_removed',
        'permission_lost', 'irrelevant', 'ambiguous'
    )),
    status TEXT NOT NULL CHECK(status IN (
        'pending', 'applying', 'applied', 'ignored', 'failed'
    )),
    attempts INTEGER NOT NULL DEFAULT 0,
    last_error TEXT,
    updated_at_ms INTEGER NOT NULL,
    PRIMARY KEY(watch_id, event_sequence)
);
