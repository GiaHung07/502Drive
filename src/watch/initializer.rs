//! Watch initializer — implements the race-free watch creation flow (spec §12.4).
//!
//! ## Flow
//!
//! 1. Subscription already created in `initializing` state with `baseline_sequence`.
//! 2. This module runs an initial clone of the source tree into the destination:
//!    - Creates a synthetic "watch_initial_clone" job in the DB so that the
//!      existing clone engine (BFS traversal + copy) can be reused.
//!    - After the clone job succeeds, the source_mappings for scope_type='watch'
//!      are populated by copying the job-scoped mappings.
//! 3. After initial clone:
//!    - Switch subscription to `catching_up`.
//!    - Replay all change_events with sequence > baseline_sequence.
//!    - Advance last_consumed_sequence to current cursor's last_event_sequence.
//!    - Switch to `active`.
//!
//! The initial clone job uses `scope_type='job'` for interim mappings.
//! On completion the job mappings are promoted to `scope_type='watch'`.
//! This keeps crash recovery simple: if the initializer crashes, on restart
//! recovery replays the clone job, then re-runs catch-up.

use std::sync::Arc;

use anyhow::{Context, bail};
use tokio::sync::mpsc;
use tracing::{info, warn};

use crate::{
    config::AppConfig,
    drive::links::DriveReference,
    engine::copy::{CloneRequest, CloneService},
    state::{
        db::Database,
        repo::{self, WatchSubscription},
    },
    watch::poller::NotifySender,
};

use super::dispatcher::dispatch_pending;

/// Notification callback type — must be Send+Sync so it can cross await points
/// inside a tokio::spawn task.
type NotifyFn = Arc<dyn Fn(String) + Send + Sync>;

/// Run the full watch initialization flow for a newly created subscription.
///
/// Must be called from a spawned task, not directly from the Telegram handler.
/// Reports progress back via the provided `notify_fn`.
pub async fn run_initial_clone(
    config: AppConfig,
    db: Database,
    watch_id: String,
    notify_fn: impl Fn(String) + Send + Sync + 'static,
) -> anyhow::Result<()> {
    // Wrap in Arc so it can cross await points.
    let notify: NotifyFn = Arc::new(notify_fn);
    // Local mpsc channel for catch-up dispatch_pending calls.
    let (notify_tx, _notify_rx) = mpsc::channel::<(i64, String)>(32);
    run_initial_clone_inner(config, db, watch_id, notify, notify_tx).await
}

async fn run_initial_clone_inner(
    config: AppConfig,
    db: Database,
    watch_id: String,
    notify: NotifyFn,
    notify_tx: NotifySender,
) -> anyhow::Result<()> {
    let watch = repo::watch_for_user_unchecked(&db, &watch_id)
        .await?
        .with_context(|| format!("Watch {watch_id} not found for initialization"))?;

    if watch.status != "initializing" {
        info!(
            watch_id,
            status = watch.status,
            "skipping init — not in initializing state"
        );
        return Ok(());
    }

    notify(format!(
        "⏳ Starting initial clone for watch `{}`...",
        &watch_id[..8.min(watch_id.len())]
    ));

    // ── Step 1: Run initial clone job ─────────────────────────────────────────
    let source_ref = DriveReference {
        file_id: watch.source_root_id.clone(),
        resource_key: watch.source_resource_key.clone(),
        hinted_kind: None,
    };
    // source is already validated by the /watch handler; no need to re-fetch here.

    // Create the clone job using the destination_root_id as the destination parent.
    let clone_service = CloneService::new(config.clone(), db.clone());
    let job_id = run_watch_clone_job(&clone_service, &watch, &source_ref, &notify)
        .await
        .with_context(|| "Watch initial clone job failed")?;

    info!(watch_id, job_id, "initial clone job complete");

    // ── Step 2: Promote job mappings to watch scope ───────────────────────────
    promote_job_mappings_to_watch(&db, &job_id, &watch_id).await?;

    // ── Step 3: Switch to catching_up ─────────────────────────────────────────
    repo::update_watch_status(&db, &watch_id, "catching_up").await?;
    notify(format!(
        "⏩ Initial clone done — catching up on changes since baseline for watch `{}`",
        &watch_id[..8.min(watch_id.len())]
    ));

    // ── Step 4: Catch-up: dispatch all events from baseline to current ─────────
    // Drive up to 10 catch-up dispatch cycles to drain the backlog.
    for cycle in 0..10u32 {
        let watch = match repo::watch_for_user_unchecked(&db, &watch_id).await? {
            Some(w) => w,
            None => bail!("Watch {watch_id} disappeared during catch-up"),
        };
        if watch.status != "catching_up" {
            break;
        }

        let cursor = repo::cursor_by_id(&db, &watch.cursor_id).await?;
        if watch.last_consumed_sequence >= cursor.last_event_sequence {
            // Caught up — promote to active.
            repo::advance_watch_consumed_sequence(&db, &watch_id, cursor.last_event_sequence)
                .await?;
            repo::update_watch_status(&db, &watch_id, "active").await?;
            info!(watch_id, cycles = cycle, "watch caught up → active");
            break;
        }

        // Dispatch one batch of pending events.
        if let Err(err) = dispatch_pending(&config, &db, &notify_tx).await {
            warn!(watch_id, error = %err, "catch-up dispatch error (will retry)");
        }
    }

    // If still catching_up after 10 cycles (huge backlog), that's okay —
    // the regular dispatch loop will continue draining.
    let final_status = repo::watch_for_user_unchecked(&db, &watch_id)
        .await?
        .map(|w| w.status)
        .unwrap_or_default();

    notify(format!(
        "✅ Watch `{}` initialized (status: {}). Use /watches to monitor.",
        &watch_id[..8.min(watch_id.len())],
        final_status
    ));
    Ok(())
}

async fn run_watch_clone_job(
    svc: &CloneService,
    watch: &WatchSubscription,
    source_ref: &DriveReference,
    notify: &NotifyFn,
) -> anyhow::Result<String> {
    let outcome = svc
        .start_one_shot_with_dest(
            CloneRequest {
                chat_id: watch.chat_id,
                telegram_user_id: watch.telegram_user_id,
                source: source_ref.clone(),
                progress_message_id: None,
            },
            watch.destination_root_id.clone(),
            watch.destination_drive_id.clone(),
            Some(watch.id.clone()),
        )
        .await?;

    notify(format!(
        "📋 Initial clone job `{}`: {}",
        &outcome.job_id[..8.min(outcome.job_id.len())],
        outcome.message
    ));

    Ok(outcome.job_id)
}

/// Copy source_mappings from job scope to watch scope.
///
/// After a successful initial clone, all mappings are under `scope_type='job'`.
/// We promote them to `scope_type='watch'` so the dispatcher can find them.
/// The job-scoped rows are kept for recovery purposes and will be GC'd with the job.
async fn promote_job_mappings_to_watch(
    db: &Database,
    job_id: &str,
    watch_id: &str,
) -> anyhow::Result<()> {
    // Log before moving into closure.
    info!(%job_id, %watch_id, "promoting job mappings to watch scope");
    let job_id_owned = job_id.to_string();
    let watch_id_owned = watch_id.to_string();
    db.conn()
        .call(move |conn| {
            conn.execute(
                "INSERT INTO source_mappings (
                     scope_type, scope_id, source_item_id, destination_item_id,
                     source_parent_id, destination_parent_id, mime_type, source_name,
                     source_version, source_modified_time, source_md5_checksum,
                     mapping_state, updated_at_ms
                 )
                 SELECT 'watch', ?2, source_item_id, destination_item_id,
                        source_parent_id, destination_parent_id, mime_type, source_name,
                        source_version, source_modified_time, source_md5_checksum,
                        mapping_state, updated_at_ms
                 FROM source_mappings
                 WHERE scope_type = 'job' AND scope_id = ?1
                 ON CONFLICT(scope_type, scope_id, source_item_id) DO UPDATE SET
                     destination_item_id = excluded.destination_item_id,
                     source_parent_id    = excluded.source_parent_id,
                     destination_parent_id = excluded.destination_parent_id,
                     mime_type           = excluded.mime_type,
                     source_name         = excluded.source_name,
                     source_version      = excluded.source_version,
                     source_modified_time = excluded.source_modified_time,
                     source_md5_checksum = excluded.source_md5_checksum,
                     mapping_state       = excluded.mapping_state,
                     updated_at_ms       = excluded.updated_at_ms",
                rusqlite::params![job_id_owned, watch_id_owned],
            )?;
            Ok::<(), rusqlite::Error>(())
        })
        .await?;
    Ok(())
}
