# Architecture Notes and ADRs

## ADR 0001: OAuth Is Local CLI First

Telegram `/connect` cannot safely complete a desktop loopback OAuth flow when the user opens the URL on a phone. `gdclone-bot auth login` owns the loopback listener, PKCE verifier, browser open/printed URL, token exchange, and credential validation. Telegram `/account` reports state and points users to the CLI.

## ADR 0002: SQLite Access Through an Actor

SQLite calls are blocking. Application code uses `tokio-rusqlite`, which serializes work onto a dedicated connection thread and keeps Tokio workers free. The DB is opened with foreign keys, WAL, normal synchronous mode, and a busy timeout.

## ADR 0003: Idempotency Before Drive Writes

Every Drive write is preceded by a durable `operation_intents` row. Destination objects created by this application carry private `appProperties` with `gdclone_source_id`, `gdclone_scope_id`, `gdclone_copy_key`, and `gdclone_generation`. Recovery reconciles intents before retrying writes.

## ADR 0004: Shortcut Policy Defaults to Preserve

Shortcut resolution can pull data outside the selected tree and can introduce cycles. The default policy is `preserve`; `remap_internal` and `resolve` require explicit user selection and separate safeguards.

## ADR 0005: Watch Uses Raw Change Event Log

Watch subscriptions are long-lived objects separate from finite jobs. A single poller per Google account and corpus writes raw `change_events` durably before advancing a cursor. Each watch consumes those events independently.

## ADR 0006: Windows Startup Defaults to Task Scheduler

The default Windows startup mechanism is a logon task under the same user that ran OAuth. Windows Service support is advanced because service account context must match the credential/secret backend.

