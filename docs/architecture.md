# Architecture Notes and ADRs

Related docs: [sync-semantics.md](sync-semantics.md), [threat-model.md](threat-model.md), [troubleshooting.md](troubleshooting.md).

## ADR 0001: OAuth Is Local CLI First

Telegram `/connect` cannot safely complete a desktop loopback OAuth flow when the user opens the URL on a phone. `502drive auth login` owns the loopback listener, PKCE verifier, browser open/printed URL, token exchange, and credential validation. Telegram `/account` reports state and points users to the CLI. See [oauth-setup.md](oauth-setup.md).

## ADR 0002: SQLite Access Through an Actor

SQLite calls are blocking. Application code uses `tokio-rusqlite`, which serializes work onto a dedicated connection thread and keeps Tokio workers free. The DB is opened with foreign keys, WAL, normal synchronous mode, and a busy timeout.

## ADR 0003: Idempotency Before Drive Writes

Every Drive write is preceded by a durable `operation_intents` row. Destination objects created by this application carry private `appProperties` with `gdclone_source_id`, `gdclone_scope_id`, `gdclone_copy_key`, and `gdclone_generation`. Recovery reconciles intents before retrying writes.

## ADR 0004: Shortcut Policy Defaults to Preserve

Shortcut resolution can pull data outside the selected tree and can introduce cycles. The default policy is `preserve`; `remap_internal` and `resolve` require explicit user selection and separate safeguards.

## ADR 0005: Watch Uses Raw Change Event Log

Watch subscriptions are long-lived objects separate from finite jobs. A single poller per Google account and corpus writes raw `change_events` durably before advancing a cursor. Each watch consumes those events independently.

## ADR 0006: Windows Startup Defaults to Task Scheduler

The default Windows startup mechanism is a logon task under the same user that ran OAuth. Windows Service support is advanced because service account context must match the credential/secret backend. See [install-windows.md](install-windows.md).

## Recovery Model

Drive writes are effectively-once, not exactly-once. Recovery examines `operation_intents`, destination IDs, and private `appProperties` before retrying an operation: an intent in `executing` state whose destination object already exists (matched via `appProperties` idempotency keys) is marked `applied` instead of re-executed, so a crash mid-write never produces a duplicate copy.

On startup the daemon (`src/engine/recovery.rs`):

1. runs `recover_on_startup` — reconciles pending `operation_intents` against the destination before any new work;
2. spawns a resume worker that re-queues jobs left in `running`/`recovering` state by the previous process;
3. resets watches stuck in `initializing` when the process died mid-baseline.

Cursor advancement for watch is committed in the same SQLite transaction as the raw change events, so a crash between fetching a Drive changes page and durably storing it replays the page rather than skipping it. Failed watch events do not advance a watch's consumed sequence and are retried on the next dispatch cycle. The full trigger → classification → policy behavior is in [sync-semantics.md](sync-semantics.md).
