//! Shared watch-creation service (extracted from
//! `src/telegram/handlers.rs::start_watch`) so both the Telegram flow and the
//! daemon's UI-request consumer create watches through one code path.
//!
//! Callers resolve the destination `DriveReference` themselves (telegram uses
//! the user-supplied link or the default destination profile; the UI consumer
//! does the same), then call [`create_watch`]. The returned subscription is in
//! `initializing` state — the caller is responsible for running
//! [`crate::watch::run_initial_clone`] **in the daemon process**, because
//! startup recovery only rescues `initializing` watches at boot, not at
//! runtime.

use std::time::Duration;

use anyhow::Context;
use thiserror::Error;

use crate::{
    config::AppConfig,
    drive::{client::DriveClient, links::DriveReference, token_manager::TokenManager},
    state::{db::Database, repo},
};

#[derive(Debug, Error)]
pub enum CreateWatchError {
    #[error("watch source must be a Drive folder")]
    SourceNotFolder,
    #[error("watch destination must be a Drive folder")]
    DestinationNotFolder,
    #[error("watch destination is not writable (can_add_children=false)")]
    DestinationNotWritable,
    #[error(transparent)]
    Other(#[from] anyhow::Error),
}

pub struct CreateWatchParams {
    pub source: DriveReference,
    pub destination: DriveReference,
    /// Owner scope for the subscription. GUI-initiated watches use
    /// `telegram_user_id = 0` / `chat_id = 0` (no Telegram notifications).
    pub telegram_user_id: i64,
    pub chat_id: i64,
    /// Exclude globs applied to the watch (JSON-serialized into the
    /// `exclude_globs` column). Invalid globs are rejected.
    pub exclude_globs: Vec<String>,
    /// Optional content update policy override (defaults to config default).
    pub content_update_policy: Option<String>,
}

pub struct CreatedWatch {
    pub watch_id: String,
    pub source_name: String,
    pub source_id: String,
    pub destination_name: String,
    pub destination_id: String,
}

/// Validate source/destination against Drive, upsert the change cursor and
/// create the watch subscription in `initializing` state.
pub async fn create_watch(
    config: &AppConfig,
    db: &Database,
    drive: &DriveClient,
    params: CreateWatchParams,
) -> Result<CreatedWatch, CreateWatchError> {
    // Reject invalid glob patterns before any Drive I/O.
    for glob in &params.exclude_globs {
        super::glob::normalize_glob(glob).map_err(|reason| {
            CreateWatchError::Other(anyhow::anyhow!("invalid exclude glob '{glob}': {reason}"))
        })?;
    }

    let token_manager = TokenManager::new(config.clone(), db.clone());
    let access_token = token_manager
        .access_token("default")
        .await
        .context("fetch Drive access token")?;

    let source = drive
        .get_reference(access_token.as_str(), &params.source)
        .await
        .context("resolve watch source")?;
    if !source.is_folder() {
        return Err(CreateWatchError::SourceNotFolder);
    }
    let dest = drive
        .get_reference(access_token.as_str(), &params.destination)
        .await
        .context("resolve watch destination")?;
    if !dest.is_folder() {
        return Err(CreateWatchError::DestinationNotFolder);
    }
    if dest.capabilities.as_ref().and_then(|c| c.can_add_children) != Some(true) {
        return Err(CreateWatchError::DestinationNotWritable);
    }

    let corpus_kind = if source.drive_id.is_some() {
        "shared_drive"
    } else {
        "user"
    };
    let start_token = drive
        .get_start_page_token(access_token.as_str(), source.drive_id.as_deref())
        .await
        .context("fetch start page token")?;
    let cursor = repo::upsert_change_cursor(
        db,
        "default",
        corpus_kind,
        source.drive_id.as_deref(),
        &start_token.start_page_token,
    )
    .await
    .context("upsert change cursor")?;

    let watch_id = repo::create_watch_subscription(
        db,
        repo::NewWatchSubscription {
            google_account_id: "default".to_string(),
            cursor_id: cursor.id.clone(),
            telegram_user_id: params.telegram_user_id,
            chat_id: params.chat_id,
            source_root_id: source.id.clone(),
            source_resource_key: params
                .source
                .resource_key
                .clone()
                .or(source.resource_key.clone()),
            source_drive_id: source.drive_id.clone(),
            destination_root_id: dest.id.clone(),
            destination_drive_id: dest.drive_id.clone(),
            content_update_policy: params
                .content_update_policy
                .unwrap_or_else(|| config.watch.default_content_update_policy.clone()),
            deletion_policy: config.watch.default_deletion_policy.clone(),
            move_out_policy: config.watch.default_move_out_policy.clone(),
            exclude_globs: super::glob::serialize_glob_list(&params.exclude_globs),
            baseline_sequence: cursor.last_event_sequence,
            source_name: Some(source.name.clone()),
            destination_name: Some(dest.name.clone()),
        },
    )
    .await
    .context("create watch subscription")?;

    Ok(CreatedWatch {
        watch_id,
        source_name: source.name,
        source_id: source.id,
        destination_name: dest.name,
        destination_id: dest.id,
    })
}

/// Build a `DriveClient` matching the engine's request timeout — shared by the
/// telegram handler and the UI-request consumer.
pub fn drive_client_for(config: &AppConfig) -> DriveClient {
    DriveClient::with_timeout(Duration::from_secs(config.engine.request_timeout_seconds))
}
