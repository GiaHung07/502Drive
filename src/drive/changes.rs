use serde::{Deserialize, Serialize};

use super::types::DriveFile;

/// Response from `changes.getStartPageToken`.
#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct StartPageTokenResponse {
    pub start_page_token: String,
}

/// One entry in a `changes.list` response.
#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Change {
    pub file_id: String,
    /// `true` when the item was removed from the corpus (deleted, trashed, or
    /// access lost) — the `file` field is absent.
    #[serde(default)]
    pub removed: bool,
    #[serde(default)]
    pub file: Option<DriveFile>,
    #[serde(default)]
    pub drive_id: Option<String>,
}

/// Response from `changes.list`.
#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ChangeList {
    /// Token for the next page; absent when this is the last page.
    #[serde(default)]
    pub next_page_token: Option<String>,
    /// Token that marks the current head of the change log; only present on
    /// the last page (no `nextPageToken`).
    #[serde(default)]
    pub new_start_page_token: Option<String>,
    #[serde(default)]
    pub changes: Vec<Change>,
}
