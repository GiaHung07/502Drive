use std::time::Duration;

use anyhow::Context;
use tracing::{info, warn};

use crate::{
    config::AppConfig,
    drive::{client::DriveClient, token_manager::TokenManager},
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

    let events =
        pending_events_for_watch(db, &watch.id, watch.last_consumed_sequence, DISPATCH_BATCH)
            .await?;

    if events.is_empty() {
        return Ok(());
    }

    let mut last_consumed = watch.last_consumed_sequence;

    for event in &events {
        // Determine membership of this file_id in the watch tree.
        let in_tree = repo::is_in_watch_tree(db, &watch.id, &event.file_id).await?;

        // Parse current file metadata from the event JSON (no extra Drive call yet).
        let prior = event
            .file_json
            .as_deref()
            .and_then(ItemFingerprint::from_json);
        let file = event
            .file_json
            .as_deref()
            .and_then(|j| serde_json::from_str::<crate::drive::types::DriveFile>(j).ok());

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

        last_consumed = last_consumed.max(event.sequence);
    }

    if last_consumed > watch.last_consumed_sequence {
        advance_watch_consumed_sequence(db, &watch.id, last_consumed).await?;
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
                    repo::watch_mapping_destination(db, &watch.id, parent_id).await?
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
                        repo::watch_mapping_destination(db, &watch.id, parent_id).await?
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
                    repo::watch_mapping_destination(db, &watch.id, parent_id).await?
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
        if let Some(dest) = repo::watch_mapping_destination(db, &watch.id, parent_id).await? {
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
