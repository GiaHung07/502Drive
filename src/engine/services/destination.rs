//! Destination services — the shared implementation of "set default
//! destination" and Drive folder browsing.
//!
//! `set_default` is extracted from the telegram `/set_destination` handler so
//! the telegram bot and the GUI (via the wizard / future commands) share one
//! validation path: resolve the reference, require a folder the account can
//! write to, then save the destination profile. `browse` unifies the GUI
//! folder picker's paging with the telegram destination browser's underlying
//! Drive calls.

use std::time::Duration;

use crate::config::AppConfig;
use crate::drive::{
    client::DriveClient,
    links::parse_drive_reference,
    token_manager::TokenManager,
    types::{DriveFile, FOLDER_MIME_TYPE},
};
use crate::state::db::Database;
use crate::state::repo;

/// One entry of a folder listing (shared by the GUI picker and the telegram
/// destination browser).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DestinationEntry {
    pub id: String,
    pub name: String,
    pub is_folder: bool,
    /// Set on shared-drive entries so callers can pass it back as `drive_id`
    /// when listing that folder's children.
    pub drive_id: Option<String>,
}

pub struct DestinationService;

impl DestinationService {
    /// Resolve a user-supplied Drive link/id and validate that it is a writable folder.
    pub async fn inspect_folder(
        config: &AppConfig,
        db: &Database,
        input: &str,
    ) -> anyhow::Result<(DriveFile, Option<String>)> {
        let reference = parse_drive_reference(input)?;
        let token_manager = TokenManager::new(config.clone(), db.clone());
        let access_token = token_manager.access_token("default").await?;
        let drive =
            DriveClient::with_timeout(Duration::from_secs(config.engine.request_timeout_seconds));
        let file = drive
            .get_reference(access_token.as_str(), &reference)
            .await?;
        Self::validate_writable_folder(&file)?;
        let resource_key = reference.resource_key.or(file.resource_key.clone());
        Ok((file, resource_key))
    }

    /// Save a previously validated Drive folder as the default destination profile.
    pub async fn save_as_default(
        db: &Database,
        file: &DriveFile,
        resource_key: Option<String>,
    ) -> anyhow::Result<()> {
        repo::upsert_destination_profile(
            db,
            repo::NewDestinationProfile {
                google_account_id: "default".to_string(),
                label: file.name.clone(),
                destination_parent_id: file.id.clone(),
                destination_drive_id: file.drive_id.clone(),
                destination_resource_key: resource_key.or_else(|| file.resource_key.clone()),
                is_default: true,
            },
        )
        .await?;
        Ok(())
    }

    /// Resolve a user-supplied Drive link/id, validate it is a folder the
    /// connected account can write to, and save it as the default
    /// destination profile (for the hardcoded single account `"default"`).
    ///
    /// Returns the validated Drive file so callers can render their own
    /// confirmation message.
    pub async fn set_default(
        config: &AppConfig,
        db: &Database,
        input: &str,
    ) -> anyhow::Result<DriveFile> {
        let (file, resource_key) = Self::inspect_folder(config, db, input).await?;
        Self::save_as_default(db, &file, resource_key).await?;
        Ok(file)
    }

    /// A destination must be a folder the connected account can add children
    /// to.
    pub fn validate_writable_folder(file: &DriveFile) -> anyhow::Result<()> {
        if file.mime_type != FOLDER_MIME_TYPE {
            anyhow::bail!("destination must be a Google Drive folder");
        }
        if file.capabilities.as_ref().and_then(|c| c.can_add_children) != Some(true) {
            anyhow::bail!("the connected Google account cannot write to this folder");
        }
        Ok(())
    }

    /// List folders for a destination picker.
    ///
    /// * `parent = None` → the account's shared drives followed by My Drive
    ///   root folders (a 403 on shared drives — plain Gmail accounts —
    ///   degrades gracefully to My Drive only);
    /// * `parent = Some(id)` → the folder's child folders, scoped to
    ///   `drive_id` when given.
    ///
    /// Read-only Drive calls; safe to run from any process holding a valid
    /// token.
    pub async fn browse(
        config: &AppConfig,
        db: &Database,
        parent: Option<&str>,
        drive_id: Option<&str>,
    ) -> anyhow::Result<Vec<DestinationEntry>> {
        let token_manager = TokenManager::new(config.clone(), db.clone());
        let token = token_manager.access_token("default").await?;
        let drive = DriveClient::new();

        match parent {
            None => {
                let mut entries = Vec::new();
                match drive
                    .list_shared_drives_page_size(token.as_str(), None, 200)
                    .await
                {
                    Ok(drives) => {
                        for d in drives.drives {
                            entries.push(DestinationEntry {
                                id: d.id.clone(),
                                name: d.name,
                                is_folder: true,
                                drive_id: Some(d.id),
                            });
                        }
                    }
                    Err(err) => {
                        // Shared drives can 403 on plain Gmail accounts —
                        // degrade gracefully and still show My Drive.
                        if err.status().map(|s| s.as_u16()) != Some(403) {
                            return Err(err.into());
                        }
                    }
                }
                let page = drive
                    .list_child_folders_page_size(token.as_str(), "root", None, None, 200, None)
                    .await?;
                entries.extend(page.files.into_iter().map(folder_entry));
                Ok(entries)
            }
            Some(parent) => {
                let page = drive
                    .list_child_folders_page_size(token.as_str(), parent, None, None, 200, drive_id)
                    .await?;
                Ok(page.files.into_iter().map(folder_entry).collect())
            }
        }
    }
}

fn folder_entry(file: DriveFile) -> DestinationEntry {
    let is_folder = file.is_folder();
    DestinationEntry {
        id: file.id,
        name: file.name,
        is_folder,
        drive_id: file.drive_id,
    }
}
