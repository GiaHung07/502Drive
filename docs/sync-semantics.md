# Sync Semantics (One-Way Watch)

This document describes what the watch subsystem does and does not do. Sources of truth: `src/watch/poller.rs`, `src/watch/dispatcher.rs`, `src/watch/classifier.rs`, `docs/architecture.md`, and `migrations/0002_watch.sql`.

Related docs: [architecture.md](architecture.md), [oauth-setup.md](oauth-setup.md), [troubleshooting.md](troubleshooting.md).

## 1. Direction of sync: source → destination only

A watch subscription maps a **source** folder (or shared drive) to a **destination** folder. Changes are propagated in one direction only:

- Changes made **in the source** are applied to the destination copy.
- Changes made **directly in the destination** are never propagated back to the source. There is no destination → source sync path anywhere in the codebase.
- Because the default deletion policy is `preserve_destination` (see §4), the destination is *additive*: it accumulates copies of source content and keeps them even after the source item disappears.

The `/sync` Telegram command and `watch.enabled` watches share this same engine; watches are disabled by default (`watch.enabled = false` in `config.sample.toml`). Set it to `true` in your config to start pollers.

## 2. What triggers a change: Drive Changes API

Watches do not poll each watched folder. A **single change cursor per Google account/corpus** (`change_cursors` table) polls Drive `changes.list` (see ADR 0005 in [architecture.md](architecture.md)):

1. `poll_one_cursor` fetches one page of changes and durably inserts raw `change_events` rows **in the same SQLite transaction** that advances the cursor's page token. A crash between poll and commit therefore replays the page, never loses it.
2. Paging continues immediately (no sleep) until the feed head (`newStartPageToken`) is reached.
3. A separate dispatch loop drains `change_events` for every active watch subscription (`dispatch_pending`), classifies each event, and applies it.

Classifications produced by `src/watch/classifier.rs`:

| Classification | Trigger | Default action |
| :--- | :--- | :--- |
| `new_item` | File appears under a tracked parent, not yet mapped | Server-side copy into the mapped destination parent |
| `content_changed` | `md5Checksum` or `version` differs from last fingerprint | Per `content_update_policy` (§3) |
| `renamed` | Name differs, content/parents unchanged | Rename the destination item |
| `moved_inside` | Parents changed but still inside the tracked tree | Re-parent the destination item to follow |
| `moved_outside` | Item left every tracked parent | Per `move_out_policy` (§5) |
| `moved_back` | Previously moved-out item reappears in tree | Reactivate mapping and reuse the existing destination item; if the mapping was lost, re-clone |
| `trashed_or_removed` | `removed` flag set or `trashed == true` | Per `deletion_policy` (§4) |
| `irrelevant` | Change outside the tracked subtree | Ignored |
| `ambiguous` | Change in scope but metadata is insufficient to apply safely | Watch moves to `needs_reconcile`; event is **not** consumed (cursor does not advance past it) |

Fingerprints (parents, name, md5, version) come from stored per-item mappings; `md5Checksum` covers binary files, Google-native files are detected via the `version` field.

## 3. Content-update policies (`content_update_policy`)

| Policy | Behavior |
| :--- | :--- |
| `versioned_copy` (default) | Copy the updated source item as a **new** destination item and point the mapping at it. Old copies are left in place. |
| `replace_copy` | Copy the new version, update the mapping, then **trash the old destination item**. If trashing fails after the mapping is updated, the error is logged and the sync continues (mapping already points at the new copy). |
| `manual_confirmation` | The watch pauses into `needs_reconcile` and sends an interactive conflict card via Telegram showing the file name, modification time, and size diff. You choose: `[New version]` (applies versioned copy), `[Replace old]` (replaces and trashes old copy), or `[Skip]` (ignores change). Once all pending conflicts are resolved, the watch automatically transitions back to `active`. |

Per-watch override via Telegram: `/watch_policy <watch_id> <policy>`. You can also inspect pending conflicts at any time with `/watch_status <id>` or by clicking the *Resolve conflicts* button.

## 4. Deletion policy (`deletion_policy`)

| Policy | Behavior |
| :--- | :--- |
| `preserve_destination` (default) | Source removal/trash/permission loss **keeps the destination copy**. The mapping is marked `source_removed`, and a single Telegram notification is sent (not repeated for the same event). |
| `manual_confirmation` | Watch enters `needs_reconcile`; you decide the action via Telegram. |

Notes:

- Destination deletion is never propagated to the source, and the engine never deletes a destination item on its own under the default policy. The only self-initiated trash in the system is `replace_copy` trashing the superseded old copy.
- Destination files that the user deletes manually are simply no longer mapped as active; the fallback scan (§6) copies only items the watch has never mapped before, so a manual destination delete is respected (not immediately re-copied) unless the source item was never mapped.

## 5. Move-out behavior (`move_out_policy`)

| Policy | Behavior |
| :--- | :--- |
| `detach` (default) | The mapping is detached. The destination copy remains where it is, frozen at the last synced state. |
| `keep_following` | Same detach handling in the current dispatcher (the mapping is detached; the item is not deleted). |

For a **moved-out folder**, the entire subtree of mappings is detached recursively. Move-back reuses the existing mapping and destination item rather than making a second copy (see classifier table above).

## 6. Backlog and fallback scan

- Each watch tracks `last_consumed_sequence` against the cursor head. If the backlog exceeds `watch.max_backlog_events_per_watch` (default 100000), the watch is marked `needs_reconcile`, notified once, and stops consuming events until reconciled.
- Events whose application failed do **not** advance the consumed sequence; the dispatcher retries them on the next cycle (in batch order). One ambiguous event stops the batch for that watch.
- After every dispatch cycle, a **fallback source scan** (`scan_missing_children`) lists each mapped source folder and copies any child that has never been mapped (skipping trashed and already-mapped items), covering missed events.

## 7. Adaptive polling (backoff)

The cursor schedule (`compute_next_poll_ms` in `src/watch/poller.rs`):

| Condition | Interval |
| :--- | :--- |
| Events seen in the last page, or feed recently active | `watch.active_poll_seconds` (default 20 s) |
| Idle ≥ `warm_idle_after_seconds` (default 600 s) | `watch.active_poll_seconds` → `watch.warm_idle_poll_seconds` (60 s) |
| Idle ≥ `cold_idle_after_seconds` (default 3600 s) | `watch.cold_idle_poll_seconds` (300 s) |
| Consecutive errors | Exponential backoff: `active_poll_seconds << min(errors, 6)`, capped at `cold_idle_poll_seconds` |

API-level retries use the shared engine policy (`src/engine/retry.rs`): 429 and 5xx are retried with exponential backoff plus jitter (`retry_base_delay_ms` → `retry_max_delay_ms`); 401 triggers a single token refresh before retry. A shared token-bucket pacer with a circuit breaker (`src/drive/pacer.rs`) throttles all Drive requests and opens its circuit after consecutive 429/500/503 errors.

Raw `change_events` older than `watch.raw_event_retention_days` (default 30) are pruned hourly.

## 8. Data integrity

- **WAL mode**: the SQLite database is opened with WAL, foreign keys ON, normal synchronous mode, and a busy timeout; all access is serialized through one connection actor (`tokio-rusqlite`, ADR 0002).
- **Idempotency keys**: every Drive write is preceded by a durable `operation_intents` row with a unique `idempotency_key` (ADR 0003). Destination objects carry private `appProperties` (`gdclone_source_id`, `gdclone_scope_id`, `gdclone_copy_key`, `gdclone_generation`) so a retried write can recognize its own output.
- **Cursor atomicity**: change events and the cursor page token advance in one transaction (§2), so a crash replays pages instead of skipping them.
- **Crash recovery**: on startup the daemon runs `recovery::recover_on_startup`, reconciles pending `operation_intents` before retrying writes, resumes interrupted jobs, and resets watches that were stuck in `initializing` when the process died.
- **Event applications** are recorded per `(watch_id, event_sequence)` with status `pending → applying → applied/ignored/failed`, giving an auditable trail of what was applied and why.

## 9. What watch is not

- Not bidirectional. Destination edits, renames, and deletes never touch the source.
- Not a real-time push system. It is change-feed polling with adaptive intervals; latency is bounded by the active poll interval (default 20 s).
- Not a permissions mirror. Sharing changes are only acted on when they cause removal (`preserve_destination`), not re-applied to the destination.
