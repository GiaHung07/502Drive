use std::sync::Arc;
use std::time::Duration;

use tokio::{
    sync::{Notify, broadcast, mpsc},
    task::JoinHandle,
    time::Instant,
};
use tracing::{error, info, warn};

use crate::{
    config::AppConfig,
    drive::{client::DriveClient, token_manager::TokenManager},
    engine::retry::{RetryDecision, backoff_delay, classify_http},
    state::{
        db::Database,
        repo::{self, ChangeCursor, NewChangeEventRow, record_cursor_error},
    },
};

use super::dispatcher::{DispatchState, dispatch_pending};
use super::retention::prune_old_events;

/// Shutdown signal sent to all pollers.
pub type StopSignal = broadcast::Sender<()>;

/// (chat_id, message_text) notification sent from the watch subsystem to the
/// Telegram layer. The bot task drains this channel and calls send_message.
pub type NotifySender = mpsc::Sender<(i64, String)>;
pub type NotifyReceiver = mpsc::Receiver<(i64, String)>;

/// Spawn one poller task per cursor already in the DB, plus start a dispatch
/// loop. Returns the stop signal, task handles, and the notification receiver.
pub fn spawn_all_pollers(
    config: AppConfig,
    db: Database,
    drive: DriveClient,
) -> (StopSignal, Vec<JoinHandle<()>>, NotifyReceiver) {
    let (stop_tx, _) = broadcast::channel::<()>(1);
    let (notify_tx, notify_rx) = mpsc::channel::<(i64, String)>(256);
    // Realtime dispatch trigger: the poll loop notifies after committing a
    // page with events; the dispatch loop wakes immediately instead of
    // waiting for its next interval tick.
    let dispatch_notify = Arc::new(Notify::new());
    let handles = vec![
        spawn_poll_loop(
            config.clone(),
            db.clone(),
            drive.clone(),
            stop_tx.clone(),
            Arc::clone(&dispatch_notify),
        ),
        spawn_dispatch_loop(
            config,
            db,
            drive,
            stop_tx.clone(),
            notify_tx,
            dispatch_notify,
        ),
    ];
    (stop_tx, handles, notify_rx)
}

/// One task that loads all cursors from DB and polls them in sequence.
/// This is intentionally single-threaded per cursor to keep Drive API load
/// proportional (one account = one change feed at a time).
fn spawn_poll_loop(
    config: AppConfig,
    db: Database,
    drive: DriveClient,
    stop_tx: StopSignal,
    dispatch_notify: Arc<Notify>,
) -> JoinHandle<()> {
    tokio::spawn(async move {
        let mut stop_rx = stop_tx.subscribe();
        // Constructed once and reused for every cursor and every cycle: the
        // TokenManager caches the access token (Arc'd) and the DriveClient
        // shares one reqwest connection pool + pacer.
        let token_manager = TokenManager::new(config.clone(), db.clone());
        loop {
            // Re-read cursors on every cycle so newly created ones are picked up.
            let cursors = match repo::all_change_cursors(&db).await {
                Ok(c) => c,
                Err(err) => {
                    error!(error = %err, "failed to load change cursors");
                    tokio::select! {
                        _ = stop_rx.recv() => return,
                        _ = tokio::time::sleep(Duration::from_secs(30)) => continue,
                    }
                }
            };

            let now_ms = crate::state::db::now_ms();
            let mut soonest_next: Option<i64> = None;

            for cursor in &cursors {
                if cursor.next_poll_at_ms > now_ms {
                    soonest_next = Some(match soonest_next {
                        None => cursor.next_poll_at_ms,
                        Some(s) => s.min(cursor.next_poll_at_ms),
                    });
                    continue;
                }
                poll_one_cursor(
                    &config,
                    &db,
                    &token_manager,
                    &drive,
                    cursor,
                    &dispatch_notify,
                )
                .await;
            }

            // Sleep until the soonest cursor needs to run again.
            let sleep_ms = soonest_next
                .map(|t| (t - crate::state::db::now_ms()).max(0) as u64)
                .unwrap_or(config.watch.active_poll_seconds * 1000);

            tokio::select! {
                _ = stop_rx.recv() => return,
                _ = tokio::time::sleep(Duration::from_millis(sleep_ms)) => {}
            }
        }
    })
}

/// Separate loop for dispatching pending events to active watches. Wakes
/// immediately when the poll loop commits fresh events (Notify) and otherwise
/// ticks on a fallback heartbeat interval.
fn spawn_dispatch_loop(
    config: AppConfig,
    db: Database,
    drive: DriveClient,
    stop_tx: StopSignal,
    notify_tx: NotifySender,
    dispatch_notify: Arc<Notify>,
) -> JoinHandle<()> {
    tokio::spawn(async move {
        let mut stop_rx = stop_tx.subscribe();
        let retention_days = config.watch.raw_event_retention_days;
        // Fallback heartbeat only: fresh events trigger dispatch via the
        // shared Notify, so a change waits at most one page commit. The
        // default active_poll_seconds is 10 (previously 20).
        let dispatch_interval = Duration::from_secs(config.watch.active_poll_seconds.max(2));
        let prune_interval = Duration::from_secs(3600);
        let mut last_prune = Instant::now();
        let mut dispatch_state = DispatchState::new();

        loop {
            tokio::select! {
                _ = stop_rx.recv() => return,
                _ = tokio::time::sleep(dispatch_interval) => {}
                _ = dispatch_notify.notified() => {}
            }

            if let Err(err) =
                dispatch_pending(&config, &db, &drive, &notify_tx, &mut dispatch_state).await
            {
                warn!(error = %err, "dispatch_pending error");
            }

            if last_prune.elapsed() >= prune_interval {
                match prune_old_events(&db, retention_days).await {
                    Ok(n) if n > 0 => info!(pruned = n, "pruned old change events"),
                    Ok(_) => {}
                    Err(err) => warn!(error = %err, "prune_old_events error"),
                }
                last_prune = Instant::now();
            }
        }
    })
}

/// Fetch one page of changes for `cursor`, commit atomically, repeat until
/// we exhaust the current feed (got a `newStartPageToken`).
async fn poll_one_cursor(
    config: &AppConfig,
    db: &Database,
    token_manager: &TokenManager,
    drive: &DriveClient,
    cursor: &ChangeCursor,
    dispatch_notify: &Notify,
) {
    let mut page_token = cursor.current_page_token.clone();
    let drive_id = cursor.drive_id.as_deref();
    let mut error_count = 0u32;
    // Track whether this cycle had any events for adaptive poll scheduling.
    let mut had_events_this_cycle = false;
    // Use the persisted timestamp of the last event page so compute_next_poll_ms
    // can back off correctly when the feed has been quiet for a long time.
    let idle_since_ms: i64 = cursor.last_event_at_ms.unwrap_or(0);

    loop {
        let access_token = match token_manager.access_token("default").await {
            Ok(t) => t,
            Err(err) => {
                warn!(cursor_id = cursor.id, error = %err, "token fetch failed");
                let next = compute_next_poll_ms(
                    config,
                    cursor.consecutive_error_count + 1,
                    false,
                    idle_since_ms,
                );
                let _ = record_cursor_error(db, &cursor.id, next).await;
                return;
            }
        };

        match drive
            .list_changes(access_token.as_str(), &page_token, drive_id)
            .await
        {
            Ok(page) => {
                let request_page_token = page_token.clone();
                let events: Vec<NewChangeEventRow> = page
                    .changes
                    .iter()
                    .enumerate()
                    .map(|(i, change)| NewChangeEventRow {
                        cursor_id: cursor.id.clone(),
                        request_page_token: request_page_token.clone(),
                        ordinal_in_page: i as i64,
                        file_id: change.file_id.clone(),
                        removed: change.removed,
                        // Slim projection: only the fields the dispatcher
                        // reads, nulls/empties omitted (see
                        // DriveFile::event_projection_json).
                        file_json: change.file.as_ref().map(|f| f.event_projection_json()),
                    })
                    .collect();

                if !events.is_empty() {
                    had_events_this_cycle = true;
                    // Realtime trigger: wake the dispatch loop right away so
                    // fresh events don't wait for the next heartbeat tick.
                    dispatch_notify.notify_one();
                }
                let next_poll_at =
                    compute_next_poll_ms(config, 0, had_events_this_cycle, idle_since_ms);

                if let Err(err) = repo::commit_change_page(
                    db,
                    &cursor.id,
                    events,
                    page.next_page_token.as_deref(),
                    page.new_start_page_token.as_deref(),
                    next_poll_at,
                )
                .await
                {
                    let next = compute_next_poll_ms(
                        config,
                        cursor.consecutive_error_count + 1,
                        false,
                        idle_since_ms,
                    );
                    let _ = record_cursor_error(db, &cursor.id, next).await;
                    warn!(
                        cursor_id = cursor.id,
                        error = %err,
                        "failed to commit changes page, will retry page later"
                    );
                    return;
                }
                match repo::mark_paused_watches_over_backlog_limit(
                    db,
                    &cursor.id,
                    config.watch.max_backlog_events_per_watch,
                )
                .await
                {
                    Ok(n) if n > 0 => {
                        warn!(
                            cursor_id = cursor.id,
                            watches = n,
                            "paused watches exceeded backlog limit and now need reconcile"
                        );
                    }
                    Ok(_) => {}
                    Err(err) => warn!(
                        cursor_id = cursor.id,
                        error = %err,
                        "failed to mark paused watches over backlog limit"
                    ),
                }

                if let Some(next_token) = page.next_page_token {
                    // More pages — continue immediately (no sleep).
                    page_token = next_token;
                } else {
                    // Reached head of feed.
                    return;
                }
            }
            Err(err) => {
                error_count += 1;
                let decision = match &err {
                    crate::drive::client::DriveApiError::Api { status, reason, .. } => {
                        classify_http(*status, reason.as_deref())
                    }
                    crate::drive::client::DriveApiError::Transport(_) => RetryDecision::Retry,
                };
                match decision {
                    RetryDecision::Retry if error_count < 3 => {
                        let delay = backoff_delay(
                            error_count - 1,
                            Duration::from_millis(config.engine.retry_base_delay_ms),
                            Duration::from_millis(config.engine.retry_max_delay_ms),
                        );
                        warn!(
                            cursor_id = cursor.id,
                            attempt = error_count,
                            delay_ms = delay.as_millis(),
                            error = %err,
                            "changes.list retryable error"
                        );
                        tokio::time::sleep(delay).await;
                    }
                    RetryDecision::RefreshTokenOnce => {
                        token_manager.invalidate().await;
                    }
                    _ => {
                        let next = compute_next_poll_ms(
                            config,
                            cursor.consecutive_error_count + error_count as i64,
                            false,
                            idle_since_ms,
                        );
                        let _ = record_cursor_error(db, &cursor.id, next).await;
                        warn!(
                            cursor_id = cursor.id,
                            error = %err,
                            "changes.list non-retryable error, backing off"
                        );
                        return;
                    }
                }
            }
        }
    }
}

/// Compute the next poll timestamp for a cursor.
///
/// Adaptive intervals (spec §12.9):
/// - recent activity (events_in_last_page > 0)   → active_poll_seconds
/// - idle >= warm_idle_after_seconds             → warm_idle_poll_seconds
/// - idle >= cold_idle_after_seconds             → cold_idle_poll_seconds
/// - consecutive errors                          → exponential backoff
///
/// `idle_since_ms` is the timestamp of the last page that contained events,
/// or 0 if always idle.
fn compute_next_poll_ms(
    config: &AppConfig,
    consecutive_errors: i64,
    had_events: bool,
    idle_since_ms: i64,
) -> i64 {
    if consecutive_errors > 0 {
        // Exponential back-off, capped at cold_idle_poll_seconds.
        let shifted = consecutive_errors.min(6) as u32;
        let secs =
            (config.watch.active_poll_seconds << shifted).min(config.watch.cold_idle_poll_seconds);
        return crate::state::db::now_ms() + (secs as i64) * 1000;
    }

    let now_ms = crate::state::db::now_ms();
    let secs = if had_events || idle_since_ms == 0 {
        config.watch.active_poll_seconds
    } else {
        let idle_secs = ((now_ms - idle_since_ms) / 1000).max(0);
        if idle_secs >= config.watch.cold_idle_after_seconds as i64 {
            config.watch.cold_idle_poll_seconds
        } else if idle_secs >= config.watch.warm_idle_after_seconds as i64 {
            config.watch.warm_idle_poll_seconds
        } else {
            config.watch.active_poll_seconds
        }
    };
    now_ms + (secs as i64) * 1000
}
