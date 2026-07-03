CREATE TABLE IF NOT EXISTS schema_migrations (
    version TEXT PRIMARY KEY,
    applied_at_ms INTEGER NOT NULL
);

CREATE TABLE IF NOT EXISTS authorized_users (
    telegram_user_id INTEGER PRIMARY KEY,
    role TEXT NOT NULL CHECK(role IN ('owner', 'operator')),
    enabled INTEGER NOT NULL DEFAULT 1 CHECK(enabled IN (0, 1)),
    created_at_ms INTEGER NOT NULL
);

CREATE TABLE IF NOT EXISTS google_accounts (
    id TEXT PRIMARY KEY,
    label TEXT NOT NULL,
    email TEXT,
    refresh_token_ciphertext BLOB NOT NULL,
    refresh_token_nonce BLOB,
    secret_backend TEXT NOT NULL,
    scopes_json TEXT NOT NULL,
    status TEXT NOT NULL CHECK(status IN
        ('connected', 'reconnect_required', 'revoked', 'disabled')),
    created_at_ms INTEGER NOT NULL,
    updated_at_ms INTEGER NOT NULL
);

CREATE TABLE IF NOT EXISTS destination_profiles (
    id TEXT PRIMARY KEY,
    google_account_id TEXT NOT NULL REFERENCES google_accounts(id),
    label TEXT NOT NULL,
    destination_parent_id TEXT NOT NULL,
    destination_drive_id TEXT,
    destination_resource_key TEXT,
    is_default INTEGER NOT NULL DEFAULT 0 CHECK(is_default IN (0, 1)),
    last_validated_at_ms INTEGER NOT NULL,
    created_at_ms INTEGER NOT NULL,
    updated_at_ms INTEGER NOT NULL
);

CREATE TABLE IF NOT EXISTS jobs (
    id TEXT PRIMARY KEY,
    kind TEXT NOT NULL CHECK(kind IN
        ('one_shot', 'watch_initial_clone', 'watch_apply', 'reconcile')),
    telegram_user_id INTEGER NOT NULL,
    chat_id INTEGER NOT NULL,
    google_account_id TEXT NOT NULL REFERENCES google_accounts(id),
    source_root_id TEXT NOT NULL,
    source_resource_key TEXT,
    source_drive_id TEXT,
    destination_parent_id TEXT NOT NULL,
    destination_drive_id TEXT,
    status TEXT NOT NULL CHECK(status IN (
        'queued', 'discovering', 'running', 'pausing', 'paused',
        'cancelling', 'cancelled', 'recovering',
        'completed', 'partially_completed', 'failed'
    )),
    duplicate_policy TEXT NOT NULL CHECK(duplicate_policy IN
        ('keep_both', 'skip_same_source', 'replace_safe')),
    shortcut_policy TEXT NOT NULL CHECK(shortcut_policy IN
        ('preserve', 'remap_internal', 'resolve')),
    total_discovered INTEGER NOT NULL DEFAULT 0,
    completed_items INTEGER NOT NULL DEFAULT 0,
    failed_items INTEGER NOT NULL DEFAULT 0,
    skipped_items INTEGER NOT NULL DEFAULT 0,
    progress_message_id INTEGER,
    error_summary TEXT,
    created_at_ms INTEGER NOT NULL,
    updated_at_ms INTEGER NOT NULL
);

CREATE TABLE IF NOT EXISTS traversal_folders (
    job_id TEXT NOT NULL REFERENCES jobs(id) ON DELETE CASCADE,
    source_folder_id TEXT NOT NULL,
    source_resource_key TEXT,
    destination_folder_id TEXT NOT NULL,
    source_parent_id TEXT,
    scan_state TEXT NOT NULL CHECK(scan_state IN
        ('pending', 'listing', 'done', 'failed')),
    next_page_token TEXT,
    last_error TEXT,
    updated_at_ms INTEGER NOT NULL,
    PRIMARY KEY(job_id, source_folder_id)
);

CREATE TABLE IF NOT EXISTS job_items (
    id TEXT PRIMARY KEY,
    job_id TEXT NOT NULL REFERENCES jobs(id) ON DELETE CASCADE,
    source_item_id TEXT NOT NULL,
    source_resource_key TEXT,
    source_parent_id TEXT,
    destination_parent_id TEXT NOT NULL,
    destination_item_id TEXT,
    mime_type TEXT NOT NULL,
    item_kind TEXT NOT NULL CHECK(item_kind IN
        ('binary', 'google_native', 'folder', 'shortcut', 'unknown')),
    source_name TEXT NOT NULL,
    size_bytes INTEGER,
    source_version TEXT,
    source_modified_time TEXT,
    source_md5_checksum TEXT,
    generation INTEGER NOT NULL DEFAULT 1,
    status TEXT NOT NULL CHECK(status IN (
        'discovered', 'ready', 'copying', 'done',
        'failed', 'skipped_duplicate', 'skipped_unsupported',
        'cancelled'
    )),
    attempts INTEGER NOT NULL DEFAULT 0,
    last_error_code TEXT,
    last_error_message TEXT,
    completed_at_ms INTEGER,
    UNIQUE(job_id, source_item_id, generation)
);

CREATE TABLE IF NOT EXISTS source_mappings (
    scope_type TEXT NOT NULL CHECK(scope_type IN ('job', 'watch')),
    scope_id TEXT NOT NULL,
    source_item_id TEXT NOT NULL,
    destination_item_id TEXT NOT NULL,
    source_parent_id TEXT,
    destination_parent_id TEXT,
    mime_type TEXT NOT NULL,
    source_name TEXT NOT NULL,
    source_version TEXT,
    source_modified_time TEXT,
    source_md5_checksum TEXT,
    mapping_state TEXT NOT NULL CHECK(mapping_state IN
        ('active', 'detached', 'source_removed', 'replaced')),
    updated_at_ms INTEGER NOT NULL,
    PRIMARY KEY(scope_type, scope_id, source_item_id)
);

CREATE TABLE IF NOT EXISTS operation_intents (
    id TEXT PRIMARY KEY,
    idempotency_key TEXT NOT NULL UNIQUE,
    job_id TEXT REFERENCES jobs(id) ON DELETE CASCADE,
    watch_id TEXT,
    operation_type TEXT NOT NULL CHECK(operation_type IN (
        'create_folder', 'copy_file', 'create_shortcut',
        'rename', 'move', 'trash', 'restore_mapping'
    )),
    source_item_id TEXT,
    destination_parent_id TEXT,
    destination_item_id TEXT,
    request_json TEXT NOT NULL,
    status TEXT NOT NULL CHECK(status IN (
        'planned', 'executing', 'applied',
        'cleanup_pending', 'failed'
    )),
    attempts INTEGER NOT NULL DEFAULT 0,
    last_error TEXT,
    created_at_ms INTEGER NOT NULL,
    updated_at_ms INTEGER NOT NULL
);
