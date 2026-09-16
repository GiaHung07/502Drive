use serde::{Deserialize, Serialize};

pub const FOLDER_MIME_TYPE: &str = "application/vnd.google-apps.folder";
pub const SHORTCUT_MIME_TYPE: &str = "application/vnd.google-apps.shortcut";

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DriveFile {
    pub id: String,
    pub name: String,
    pub mime_type: String,
    #[serde(default)]
    pub size: Option<String>,
    #[serde(default)]
    pub parents: Vec<String>,
    #[serde(default)]
    pub drive_id: Option<String>,
    #[serde(default)]
    pub resource_key: Option<String>,
    #[serde(default)]
    pub shortcut_details: Option<ShortcutDetails>,
    #[serde(default)]
    pub trashed: Option<bool>,
    #[serde(default)]
    pub modified_time: Option<String>,
    #[serde(default)]
    pub md5_checksum: Option<String>,
    #[serde(default)]
    pub version: Option<String>,
    #[serde(default)]
    pub capabilities: Option<FileCapabilities>,
    #[serde(default)]
    pub copy_requires_writer_permission: Option<bool>,
    #[serde(default)]
    pub app_properties: std::collections::HashMap<String, String>,
}

impl DriveFile {
    pub fn is_folder(&self) -> bool {
        self.mime_type == FOLDER_MIME_TYPE
    }

    pub fn is_shortcut(&self) -> bool {
        self.mime_type == SHORTCUT_MIME_TYPE
    }
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ShortcutDetails {
    pub target_id: String,
    #[serde(default)]
    pub target_mime_type: Option<String>,
    #[serde(default)]
    pub target_resource_key: Option<String>,
}

#[derive(Debug, Clone, Default, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct FileCapabilities {
    #[serde(default)]
    pub can_copy: Option<bool>,
    #[serde(default)]
    pub can_add_children: Option<bool>,
    #[serde(default)]
    pub can_list_children: Option<bool>,
    #[serde(default)]
    pub can_rename: Option<bool>,
    #[serde(default)]
    pub can_move_item_within_drive: Option<bool>,
    #[serde(default)]
    pub can_trash: Option<bool>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct FileList {
    #[serde(default)]
    pub next_page_token: Option<String>,
    #[serde(default)]
    pub files: Vec<DriveFile>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SharedDriveList {
    #[serde(default)]
    pub next_page_token: Option<String>,
    #[serde(default)]
    pub drives: Vec<SharedDrive>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SharedDrive {
    pub id: String,
    pub name: String,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DriveAbout {
    #[serde(default)]
    pub user: Option<DriveUser>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DriveUser {
    #[serde(default)]
    pub display_name: Option<String>,
    #[serde(default)]
    pub email_address: Option<String>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct GoogleErrorEnvelope {
    pub error: GoogleError,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct GoogleError {
    pub code: u16,
    pub message: String,
    #[serde(default)]
    pub errors: Vec<GoogleErrorDetail>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct GoogleErrorDetail {
    #[serde(default)]
    pub reason: Option<String>,
}
