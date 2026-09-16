use std::time::Duration;

use anyhow::Context;
use tracing::{info, warn};

use crate::{
    config::AppConfig,
    drive::{client::DriveClient, token_manager::TokenManager, types::FOLDER_MIME_TYPE},
    engine::{
        mapping::{MappingRecord, record_mapping},
        operation::ScopeType,
        retry::{RetryDecision, backoff_delay, classify_http},
    },
    state::{
        db::Database,
        repo::{
            self, WatchSubscription, active_watches, advance_watch_consumed_sequence,
            pending_events_for_watch, upsert_event_application,
        },
    },
    watch::poller::NotifySender,
};

use super::classifier::{Classification, ItemFingerprint, classify};

const DISPATCH_BATCH: usize = 200;

/// Process pending change events for every active watch subscription.
/// Called once per dispatch cycle from the top-level poll loop.
pub async fn dispatch_pending(
    config: &AppConfig,
    db: &Database,
    notify_tx: &NotifySender,
) -> anyhow::Result<()> {
    let watches = active_watches(db).await?;
    for watch in watches {
        if let Err(err) = dispatch_one(config, db, &watch, notify_tx).await {
            warn!(
                watch_id = watch.id,
                error = %err,
                "dispatch error for watch (will retry next cycle)"
            );
        }
    }
    Ok(())
}

async fn dispatch_one(
    config: &AppConfig,
    db: &Database,
    watch: &WatchSubscription,
    notify_tx: &NotifySender,
) -> anyhow::Result<()> {
    let token_manager = TokenManager::new(config.clone(), db.clone());
    let drive =
        DriveClient::with_timeout(Duration::from_secs(config.engine.request_timeout_seconds));

    let cursor = repo::cursor_by_id(db, &watch.cursor_id).await?;
    if backlog_exceeds_limit(
        watch.last_consumed_sequence,
        cursor.last_event_sequence,
        config.watch.max_backlog_events_per_watch,
    ) {
        repo::update_watch_status(db, &watch.id, "needs_reconcile").await?;
        let msg = format!(
            "⚠️ Watch `{}` cần đồng bộ lại vì backlog vượt {} events.",
            &watch.id[..8.min(watch.id.len())],
            config.watch.max_backlog_events_per_watch
        );
        let _ = notify_tx.try_send((watch.chat_id, msg));
        return Ok(());
    }

    let events =
        pending_events_for_watch(db, &watch.id, watch.last_consumed_sequence, DISPATCH_BATCH)
            .await?;

    if events.is_empty() {
        finish_catch_up_if_current(db, watch, watch.last_consumed_sequence).await?;
        scan_missing_children(config, db, &drive, &token_manager, watch, notify_tx).await?;
        return Ok(());
    }

    let mut last_consumed = watch.last_consumed_sequence;

    for event in &events {
        // Determine membership of this file_id in the watch tree.
        let in_tree = repo::is_in_watch_tree(db, &watch.id, &event.file_id).await?;

        // Parse current file metadata from the event JSON (no extra Drive call yet).
        let file = event
            .file_json
            .as_deref()
            .and_then(|j| serde_json::from_str::<crate::drive::types::DriveFile>(j).ok());
        let prior = repo::watch_mapping_fingerprint(db, &watch.id, &event.file_id)
            .await?
            .map(|f| ItemFingerprint {
                parents: f.parents,
                name: f.name,
                md5: f.md5_checksum,
                version: f.version,
                trashed: false,
            });

        // Determine which parents are in-tree.
        let parents_in_tree: Vec<bool> = if let Some(f) = &file {
            let mut results = Vec::with_capacity(f.parents.len());
            for parent in &f.parents {
                let r = repo::is_parent_in_watch_tree(db, &watch.id, parent).await?;
                results.push(r);
            }
            results
        } else {
            vec![]
        };

        let classification = classify(
            watch,
            &event.file_id,
            in_tree,
            &parents_in_tree,
            prior.as_ref(),
            file.as_ref(),
            event.removed,
        );
        if classification == Classification::Ambiguous {
            repo::update_watch_status(db, &watch.id, "needs_reconcile").await?;
            upsert_event_application(
                db,
                &watch.id,
                event.sequence,
                classification.as_str(),
                "failed",
            )
            .await?;
            let msg = format!(
                "⚠️ Watch `{}` cần đồng bộ lại vì event `{}` không đủ metadata để áp an toàn.",
                &watch.id[..8.min(watch.id.len())],
                event.sequence
            );
            let _ = notify_tx.try_send((watch.chat_id, msg));
            break;
        }

        // Record the application row as pending before trying to apply.
        upsert_event_application(
            db,
            &watch.id,
            event.sequence,
            classification.as_str(),
            "applying",
        )
        .await?;

        let apply_result = apply_classification(
            config,
            db,
            &drive,
            &token_manager,
            watch,
            &event.file_id,
            &classification,
            file.as_ref(),
            notify_tx,
        )
        .await;

        let final_status = match apply_result {
            Ok(()) => "applied",
            Err(ref err) => {
                warn!(
                    watch_id = watch.id,
                    file_id = event.file_id,
                    sequence = event.sequence,
                    error = %err,
                    "event application failed"
                );
                "failed"
            }
        };

        upsert_event_application(
            db,
            &watch.id,
            event.sequence,
            classification.as_str(),
            if classification == Classification::Irrelevant {
                "ignored"
            } else {
                final_status
            },
        )
        .await?;

        let stored_status = if classification == Classification::Irrelevant {
            "ignored"
        } else {
            final_status
        };
        if should_advance_consumed(stored_status) {
            last_consumed = last_consumed.max(event.sequence);
        } else {
            break;
        }
    }

    if last_consumed > watch.last_consumed_sequence {
        advance_watch_consumed_sequence(db, &watch.id, last_consumed).await?;
    }
    finish_catch_up_if_current(db, watch, last_consumed).await?;
    scan_missing_children(config, db, &drive, &token_manager, watch, notify_tx).await?;

    Ok(())
}

fn should_advance_consumed(status: &str) -> bool {
    matches!(status, "applied" | "ignored")
}

async fn finish_catch_up_if_current(
    db: &Database,
    watch: &WatchSubscription,
    last_consumed: i64,
) -> anyhow::Result<()> {
    if watch.status != "catching_up" {
        return Ok(());
    }
    let cursor = repo::cursor_by_id(db, &watch.cursor_id).await?;
    if should_finish_catch_up(&watch.status, last_consumed, cursor.last_event_sequence) {
        repo::update_watch_status(db, &watch.id, "active").await?;
    }
    Ok(())
}

fn should_finish_catch_up(status: &str, last_consumed: i64, cursor_last_sequence: i64) -> bool {
    status == "catching_up" && last_consumed >= cursor_last_sequence
}

fn backlog_exceeds_limit(last_consumed: i64, cursor_last: i64, limit: u64) -> bool {
    cursor_last > last_consumed && (cursor_last - last_consumed) as u64 > limit
}

async fn scan_missing_children(
    config: &AppConfig,
    db: &Database,
    drive: &DriveClient,
    token_manager: &TokenManager,
    watch: &WatchSubscription,
    notify_tx: &NotifySender,
) -> anyhow::Result<()> {
    let folders = repo::active_watch_folder_mappings(db, &watch.id).await?;
    let mut copied = 0usize;
    for folder in folders {
        let resource_key = if folder.source_item_id == watch.source_root_id {
            watch.source_resource_key.as_deref()
        } else {
            None
        };
        let mut page_token = None;
        loop {
            let access_token = token_manager.access_token("default").await?;
            let page = drive
                .list_children(
                    access_token.as_str(),
                    &folder.source_item_id,
                    resource_key,
                    page_token.as_deref(),
                )
                .await
                .context("scan watched source folder")?;

            for child in page.files {
                if child.trashed == Some(true)
                    || repo::watch_mapping_destination(db, &watch.id, &child.id)
                        .await?
                        .is_some()
                {
                    continue;
                }
                let destination = if child.mime_type == FOLDER_MIME_TYPE {
                    let access_token = token_manager.access_token("default").await?;
                    drive
                        .create_folder(
                            access_token.as_str(),
                            &child.name,
                            &folder.destination_item_id,
                            None,
                        )
                        .await
                        .context("create missing watched folder")?
                } else {
                    let access_token = token_manager.access_token("default").await?;
                    drive_copy_with_retry(
                        drive,
                        access_token.as_str(),
                        &child.id,
                        &child.name,
                        &folder.destination_item_id,
                        config,
                    )
                    .await?
                };
                record_mapping(
                    db,
                    MappingRecord {
                        scope_type: ScopeType::Watch.as_str().to_string(),
                        scope_id: watch.id.clone(),
                        source_item_id: child.id.clone(),
                        destination_item_id: destination.id.clone(),
                        source_parent_id: Some(folder.source_item_id.clone()),
                        destination_parent_id: Some(folder.destination_item_id.clone()),
                        mime_type: child.mime_type,
                        source_name: child.name,
                        source_version: child.version,
                        source_modified_time: child.modified_time,
                        source_md5_checksum: child.md5_checksum,
                    },
                )
                .await?;
                copied += 1;
            }

            match page.next_page_token {
                Some(next) => page_token = Some(next),
                None => break,
            }
        }
    }

    if copied > 0 {
        info!(
            watch_id = watch.id,
            copied, "fallback source scan copied missing children"
        );
        let msg = format!(
            "✅ Watch `{}`: quét bù đã copy {} item mới từ nguồn.",
            &watch.id[..8.min(watch.id.len())],
            copied
        );
        let _ = notify_tx.try_send((watch.chat_id, msg));
    }
    Ok(())
}

#[allow(clippy::too_many_arguments)]
async fn apply_classification(
    config: &AppConfig,
    db: &Database,
    drive: &DriveClient,
    token_manager: &TokenManager,
    watch: &WatchSubscription,
    file_id: &str,
    classification: &Classification,
    file: Option<&crate::drive::types::DriveFile>,
    notify_tx: &NotifySender,
) -> anyhow::Result<()> {
    match classification {
        Classification::Irrelevant | Classification::Ambiguous => Ok(()),

        Classification::TrashedOrRemoved => {
            match watch.deletion_policy.as_str() {
                "preserve_destination" => {
                    repo::mark_watch_mapping_source_removed(db, &watch.id, file_id).await?;
                    info!(
                        watch_id = watch.id,
                        file_id, "source removed — destination preserved"
                    );
                    // Notify once (spec: notify one time, not repeatedly).
                    let msg = format!(
                        "⚠️ Watch `{}`: source item `{}` removed/trashed. Destination preserved.",
                        &watch.id[..8.min(watch.id.len())],
                        file_id
                    );
                    let _ = notify_tx.try_send((watch.chat_id, msg));
                }
                "manual_confirmation" => {
                    repo::update_watch_status(db, &watch.id, "needs_reconcile").await?;
                    let msg = format!(
                        "❓ Watch `{}`: source `{}` removed/trashed. Reply with action.",
                        &watch.id[..8.min(watch.id.len())],
                        file_id
                    );
                    let _ = notify_tx.try_send((watch.chat_id, msg));
                    info!(
                        watch_id = watch.id,
                        file_id, "source removed — manual confirmation required (notified)"
                    );
                    return Err(anyhow::anyhow!(
                        "manual confirmation required for removed source"
                    ));
                }
                other => warn!(
                    watch_id = watch.id,
                    policy = other,
                    "unknown deletion_policy"
                ),
            }
            Ok(())
        }

        Classification::MovedOutside => {
            let is_folder = file.map(|f| f.is_folder()).unwrap_or(false);
            match watch.move_out_policy.as_str() {
                "detach" | "keep_following" => {
                    if is_folder {
                        // Spec §12.6: recursively detach the whole subtree.
                        let count =
                            repo::detach_watch_folder_subtree(db, &watch.id, file_id).await?;
                        info!(
                            watch_id = watch.id,
                            file_id,
                            rows = count,
                            "folder moved outside — subtree detached"
                        );
                    } else {
                        repo::mark_watch_mapping_detached(db, &watch.id, file_id).await?;
                        info!(
                            watch_id = watch.id,
                            file_id, "source moved outside — detached"
                        );
                    }
                }
                other => warn!(
                    watch_id = watch.id,
                    policy = other,
                    "unknown move_out_policy"
                ),
            }
            Ok(())
        }

        Classification::NewItem => {
            // Find the destination parent by looking up the first in-tree source parent.
            let Some(file) = file else { return Ok(()) };
            for parent_id in &file.parents {
                if let Some(dest_parent) =
                    repo::active_watch_mapping_destination(db, &watch.id, parent_id).await?
                {
                    let access_token = token_manager.access_token("default").await?;
                    let copied = drive_copy_with_retry(
                        drive,
                        access_token.as_str(),
                        file_id,
                        &file.name,
                        &dest_parent,
                        config,
                    )
                    .await?;
                    record_mapping(
                        db,
                        MappingRecord {
                            scope_type: ScopeType::Watch.as_str().to_string(),
                            scope_id: watch.id.clone(),
                            source_item_id: file_id.into(),
                            destination_item_id: copied.id.clone(),
                            source_parent_id: Some(parent_id.clone()),
                            destination_parent_id: Some(dest_parent),
                            mime_type: file.mime_type.clone(),
                            source_name: file.name.clone(),
                            source_version: file.version.clone(),
                            source_modified_time: file.modified_time.clone(),
                            source_md5_checksum: file.md5_checksum.clone(),
                        },
                    )
                    .await?;
                    info!(
                        watch_id = watch.id,
                        file_id,
                        dest_id = copied.id,
                        "new item cloned"
                    );
                    break;
                }
            }
            Ok(())
        }

        Classification::Renamed => {
            let Some(file) = file else { return Ok(()) };
            let Some(dest_id) = repo::watch_mapping_destination(db, &watch.id, file_id).await?
            else {
                return Ok(());
            };
            let access_token = token_manager.access_token("default").await?;
            drive
                .rename_file(
                    access_token.as_str(),
                    &dest_id,
                    &file.name,
                    Duration::from_secs(config.engine.request_timeout_seconds),
                )
                .await
                .context("rename destination item")?;
            info!(
                watch_id = watch.id,
                file_id,
                dest_id,
                new_name = file.name,
                "item renamed"
            );
            Ok(())
        }

        Classification::ContentChanged => {
            match watch.content_update_policy.as_str() {
                "versioned_copy" => {
                    let Some(file) = file else { return Ok(()) };
                    if let Some(dest_parent) = get_dest_parent(db, watch, file_id, file).await? {
                        let access_token = token_manager.access_token("default").await?;
                        let new_copy = drive_copy_with_retry(
                            drive,
                            access_token.as_str(),
                            file_id,
                            &file.name,
                            &dest_parent,
                            config,
                        )
                        .await?;
                        // Update mapping to point at the new copy.
                        record_mapping(
                            db,
                            MappingRecord {
                                scope_type: ScopeType::Watch.as_str().to_string(),
                                scope_id: watch.id.clone(),
                                source_item_id: file_id.into(),
                                destination_item_id: new_copy.id.clone(),
                                source_parent_id: file.parents.first().cloned(),
                                destination_parent_id: Some(dest_parent),
                                mime_type: file.mime_type.clone(),
                                source_name: file.name.clone(),
                                source_version: file.version.clone(),
                                source_modified_time: file.modified_time.clone(),
                                source_md5_checksum: file.md5_checksum.clone(),
                            },
                        )
                        .await?;
                        info!(
                            watch_id = watch.id,
                            file_id,
                            new_copy_id = new_copy.id,
                            "versioned_copy: new copy created, mapping updated"
                        );
                    }
                }
                "replace_copy" => {
                    let Some(file) = file else { return Ok(()) };
                    if let Some(dest_parent) = get_dest_parent(db, watch, file_id, file).await? {
                        let access_token = token_manager.access_token("default").await?;
                        let old_dest =
                            repo::watch_mapping_destination(db, &watch.id, file_id).await?;
                        let copied = drive_copy_with_retry(
                            drive,
                            access_token.as_str(),
                            file_id,
                            &file.name,
                            &dest_parent,
                            config,
                        )
                        .await?;
                        let new_dest_id = copied.id.clone();
                        record_mapping(
                            db,
                            MappingRecord {
                                scope_type: ScopeType::Watch.as_str().to_string(),
                                scope_id: watch.id.clone(),
                                source_item_id: file_id.into(),
                                destination_item_id: new_dest_id.clone(),
                                source_parent_id: file.parents.first().cloned(),
                                destination_parent_id: Some(dest_parent),
                                mime_type: file.mime_type.clone(),
                                source_name: file.name.clone(),
                                source_version: file.version.clone(),
                                source_modified_time: file.modified_time.clone(),
                                source_md5_checksum: file.md5_checksum.clone(),
                            },
                        )
                        .await?;
                        if let Some(old_dest_id) = old_dest
                            && old_dest_id != new_dest_id
                            && let Err(err) =
                                drive.trash_file(access_token.as_str(), &old_dest_id).await
                        {
                            warn!(
                                watch_id = watch.id,
                                file_id,
                                old_dest_id,
                                error = %err,
                                "replace_copy: old destination trash failed after mapping update"
                            );
                        }
                        info!(
                            watch_id = watch.id,
                            file_id,
                            new_copy_id = new_dest_id,
                            "replace_copy: new copy created, mapping updated"
                        );
                    }
                }
                "manual_confirmation" => {
                    repo::update_watch_status(db, &watch.id, "needs_reconcile").await?;
                    let msg = format!(
                        "❓ Watch `{}`: content changed in `{}`. Choose action: versioned_copy / skip.",
                        &watch.id[..8.min(watch.id.len())],
                        file_id
                    );
                    let _ = notify_tx.try_send((watch.chat_id, msg));
                    info!(
                        watch_id = watch.id,
                        file_id, "content changed — manual confirmation notified"
                    );
                    return Err(anyhow::anyhow!(
                        "manual confirmation required for content change"
                    ));
                }
                other => warn!(
                    watch_id = watch.id,
                    policy = other,
                    "unknown content_update_policy"
                ),
            }
            Ok(())
        }

        Classification::MovedInside | Classification::MovedBack => {
            // Re-parent the destination item to follow the source.
            let Some(file) = file else { return Ok(()) };
            let dest_id = repo::watch_mapping_destination(db, &watch.id, file_id).await?;

            // On MovedBack, reactivate the mapping even if we can't find dest yet.
            if matches!(classification, Classification::MovedBack) {
                repo::mark_watch_mapping_active(db, &watch.id, file_id).await?;
            }

            let Some(dest_id) = dest_id else {
                // Lost mapping; treat as new item — find an in-tree parent and clone.
                for parent_id in &file.parents {
                    if let Some(dest_parent) =
                        repo::active_watch_mapping_destination(db, &watch.id, parent_id).await?
                    {
                        let access_token = token_manager.access_token("default").await?;
                        let copied = drive_copy_with_retry(
                            drive,
                            access_token.as_str(),
                            file_id,
                            &file.name,
                            &dest_parent,
                            config,
                        )
                        .await?;
                        record_mapping(
                            db,
                            MappingRecord {
                                scope_type: ScopeType::Watch.as_str().to_string(),
                                scope_id: watch.id.clone(),
                                source_item_id: file_id.into(),
                                destination_item_id: copied.id.clone(),
                                source_parent_id: Some(parent_id.clone()),
                                destination_parent_id: Some(dest_parent),
                                mime_type: file.mime_type.clone(),
                                source_name: file.name.clone(),
                                source_version: file.version.clone(),
                                source_modified_time: file.modified_time.clone(),
                                source_md5_checksum: file.md5_checksum.clone(),
                            },
                        )
                        .await?;
                        info!(
                            watch_id = watch.id,
                            file_id,
                            dest_id = copied.id,
                            "moved-back/lost mapping — re-cloned"
                        );
                        break;
                    }
                }
                return Ok(());
            };
            for parent_id in &file.parents {
                if let Some(dest_parent) =
                    repo::active_watch_mapping_destination(db, &watch.id, parent_id).await?
                {
                    let access_token = token_manager.access_token("default").await?;
                    drive
                        .move_file(
                            access_token.as_str(),
                            &dest_id,
                            &dest_parent,
                            Duration::from_secs(config.engine.request_timeout_seconds),
                        )
                        .await
                        .context("move destination item")?;
                    info!(
                        watch_id = watch.id,
                        file_id, dest_id, dest_parent, "item moved inside tree"
                    );
                    break;
                }
            }
            Ok(())
        }
    }
}

async fn get_dest_parent(
    db: &Database,
    watch: &WatchSubscription,
    file_id: &str,
    file: &crate::drive::types::DriveFile,
) -> anyhow::Result<Option<String>> {
    for parent_id in &file.parents {
        if let Some(dest) = repo::active_watch_mapping_destination(db, &watch.id, parent_id).await?
        {
            return Ok(Some(dest));
        }
    }
    // Fall back to checking the file's own dest parent via its mapping.
    repo::watch_mapping_destination(db, &watch.id, file_id).await
}

async fn drive_copy_with_retry(
    drive: &DriveClient,
    access_token: &str,
    source_id: &str,
    name: &str,
    dest_parent: &str,
    config: &AppConfig,
) -> anyhow::Result<crate::drive::types::DriveFile> {
    let source_ref = crate::drive::links::DriveReference {
        file_id: source_id.into(),
        resource_key: None,
        hinted_kind: None,
    };
    for attempt in 0..=config.engine.max_retry_attempts {
        match drive
            .copy_file(access_token, &source_ref, name, dest_parent, None)
            .await
        {
            Ok(f) => return Ok(f),
            Err(err) => {
                let decision = match &err {
                    crate::drive::client::DriveApiError::Api { status, reason, .. } => {
                        classify_http(*status, reason.as_deref())
                    }
                    crate::drive::client::DriveApiError::Transport(_) => RetryDecision::Retry,
                };
                if matches!(decision, RetryDecision::Retry)
                    && attempt < config.engine.max_retry_attempts
                {
                    tokio::time::sleep(backoff_delay(
                        attempt,
                        Duration::from_millis(config.engine.retry_base_delay_ms),
                        Duration::from_millis(config.engine.retry_max_delay_ms),
                    ))
                    .await;
                } else {
                    return Err(err.into());
                }
            }
        }
    }
    unreachable!("loop exits via return or error")
}

#[cfg(test)]
mod tests {
    use super::{backlog_exceeds_limit, should_advance_consumed, should_finish_catch_up};

    #[test]
    fn failed_watch_event_does_not_advance_cursor() {
        assert!(should_advance_consumed("applied"));
        assert!(should_advance_consumed("ignored"));
        assert!(!should_advance_consumed("failed"));
        assert!(!should_advance_consumed("applying"));
        assert!(!should_advance_consumed("pending"));
    }

    #[test]
    fn catch_up_finishes_only_at_cursor_head() {
        assert!(should_finish_catch_up("catching_up", 10, 10));
        assert!(should_finish_catch_up("catching_up", 11, 10));
        assert!(!should_finish_catch_up("catching_up", 9, 10));
        assert!(!should_finish_catch_up("active", 10, 10));
    }

    #[test]
    fn backlog_limit_is_strictly_greater_than_limit() {
        assert!(!backlog_exceeds_limit(10, 10, 0));
        assert!(!backlog_exceeds_limit(10, 15, 5));
        assert!(backlog_exceeds_limit(10, 16, 5));
        assert!(!backlog_exceeds_limit(20, 10, 5));
    }
}
