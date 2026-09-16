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

    /// Slim JSON projection stored in `change_events.file_json`.
    ///
    /// Keeps exactly the fields the watch dispatcher/classifier/clone engine
    /// read (id, name, mimeType, size, parents, driveId, resourceKey,
    /// shortcutDetails, trashed, modifiedTime, md5Checksum, version,
    /// capabilities, copyRequiresWriterPermission, appProperties) and omits
    /// null/empty optionals so backlog rows stay small. The result still
    /// deserializes into `DriveFile` because every omittable field is
    /// `#[serde(default)]`.
    pub fn event_projection_json(&self) -> String {
        use serde_json::{Map, Value};
        let mut m = Map::new();
        m.insert("id".into(), Value::String(self.id.clone()));
        m.insert("name".into(), Value::String(self.name.clone()));
        m.insert("mimeType".into(), Value::String(self.mime_type.clone()));
        if !self.parents.is_empty() {
            m.insert(
                "parents".into(),
                serde_json::to_value(&self.parents).unwrap_or_default(),
            );
        }
        if let Some(v) = &self.size {
            m.insert("size".into(), Value::String(v.clone()));
        }
        if let Some(v) = &self.drive_id {
            m.insert("driveId".into(), Value::String(v.clone()));
        }
        if let Some(v) = &self.resource_key {
            m.insert("resourceKey".into(), Value::String(v.clone()));
        }
        if let Some(v) = &self.shortcut_details {
            m.insert(
                "shortcutDetails".into(),
                serde_json::to_value(v).unwrap_or_default(),
            );
        }
        if let Some(v) = self.trashed {
            m.insert("trashed".into(), Value::Bool(v));
        }
        if let Some(v) = &self.modified_time {
            m.insert("modifiedTime".into(), Value::String(v.clone()));
        }
        if let Some(v) = &self.md5_checksum {
            m.insert("md5Checksum".into(), Value::String(v.clone()));
        }
        if let Some(v) = &self.version {
            m.insert("version".into(), Value::String(v.clone()));
        }
        if let Some(v) = &self.capabilities {
            m.insert(
                "capabilities".into(),
                serde_json::to_value(v).unwrap_or_default(),
            );
        }
        if let Some(v) = self.copy_requires_writer_permission {
            m.insert("copyRequiresWriterPermission".into(), Value::Bool(v));
        }
        if !self.app_properties.is_empty() {
            m.insert(
                "appProperties".into(),
                serde_json::to_value(&self.app_properties).unwrap_or_default(),
            );
        }
        Value::Object(m).to_string()
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

#[cfg(test)]
mod tests {
    use super::*;

    fn full_file() -> DriveFile {
        let mut app_properties = std::collections::HashMap::new();
        app_properties.insert("gdclone_scope".to_string(), "watch".to_string());
        DriveFile {
            id: "f1".into(),
            name: "foo.txt".into(),
            mime_type: "text/plain".into(),
            size: Some("123".into()),
            parents: vec!["p1".into()],
            drive_id: Some("drive-1".into()),
            resource_key: Some("rk".into()),
            shortcut_details: Some(ShortcutDetails {
                target_id: "t1".into(),
                target_mime_type: Some("text/plain".into()),
                target_resource_key: Some("trk".into()),
            }),
            trashed: Some(false),
            modified_time: Some("2026-01-01T00:00:00.000Z".into()),
            md5_checksum: Some("abc".into()),
            version: Some("7".into()),
            capabilities: Some(FileCapabilities {
                can_copy: Some(true),
                can_add_children: Some(false),
                can_list_children: Some(true),
                can_rename: Some(true),
                can_move_item_within_drive: Some(false),
                can_trash: Some(true),
            }),
            copy_requires_writer_permission: Some(false),
            app_properties,
        }
    }

    #[test]
    fn slim_projection_round_trips_into_drive_file() {
        let file = full_file();
        let json = file.event_projection_json();
        // The projection must omit null/empty fields entirely.
        assert!(!json.contains("null"));
        let round_tripped: DriveFile = serde_json::from_str(&json).expect("deserialize projection");
        assert_eq!(
            serde_json::to_value(&round_tripped).unwrap(),
            serde_json::to_value(&file).unwrap()
        );
    }

    #[test]
    fn slim_projection_with_only_required_fields_deserializes() {
        let file = full_file();
        let json = file.event_projection_json();
        let value: serde_json::Value = serde_json::from_str(&json).unwrap();
        // Simulate a maximally slim event row: required fields only.
        let slim = serde_json::json!({
            "id": value["id"],
            "name": value["name"],
            "mimeType": value["mimeType"],
        });
        let parsed: DriveFile = serde_json::from_value(slim).expect("slim json parses");
        assert_eq!(parsed.id, "f1");
        assert_eq!(parsed.name, "foo.txt");
        assert_eq!(parsed.parents, Vec::<String>::new());
        assert_eq!(parsed.trashed, None);
        assert!(parsed.app_properties.is_empty());
    }
}
