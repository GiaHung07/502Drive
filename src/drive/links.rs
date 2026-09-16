use serde::{Deserialize, Serialize};
use thiserror::Error;
use url::Url;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum DriveItemKindHint {
    File,
    Folder,
    GoogleDocument,
    GoogleSpreadsheet,
    GooglePresentation,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DriveReference {
    pub file_id: String,
    pub resource_key: Option<String>,
    pub hinted_kind: Option<DriveItemKindHint>,
}

impl DriveReference {
    pub fn resource_key_header_value(&self) -> Option<String> {
        self.resource_key
            .as_ref()
            .map(|key| format!("{}/{}", self.file_id, key))
    }
}

#[derive(Debug, Error, PartialEq, Eq)]
pub enum DriveLinkError {
    #[error("empty Drive URL or ID")]
    Empty,
    #[error("not a supported Google Drive or Docs URL and not a valid Drive ID")]
    Invalid,
    #[error("Google URL did not contain a Drive file or folder ID")]
    MissingId,
    #[error("Drive ID length is outside the allowed range")]
    InvalidLength,
    #[error("Drive ID contains unsupported characters")]
    InvalidCharacters,
}

pub fn parse_drive_reference(input: &str) -> Result<DriveReference, DriveLinkError> {
    let input = input.trim();
    if input.is_empty() {
        return Err(DriveLinkError::Empty);
    }
    if is_valid_id(input).is_ok() {
        return Ok(DriveReference {
            file_id: input.to_string(),
            resource_key: None,
            hinted_kind: None,
        });
    }

    let bare_google_url = input.starts_with("drive.google.com/")
        || input.starts_with("docs.google.com/")
        || input.starts_with("drive.usercontent.google.com/")
        || input.starts_with("lh3.googleusercontent.com/");
    let normalized = if bare_google_url {
        format!("https://{input}")
    } else {
        input.to_string()
    };
    let url = Url::parse(&normalized).map_err(|_| DriveLinkError::Invalid)?;
    let host = url.host_str().unwrap_or_default();
    if !is_supported_host(host) {
        return Err(DriveLinkError::Invalid);
    }

    let resource_key = url
        .query_pairs()
        .find_map(|(key, value)| {
            matches!(key.as_ref(), "resourcekey" | "resourceKey").then(|| value.into_owned())
        })
        .filter(|value| !value.is_empty());

    if let Some(id) = url.query_pairs().find_map(|(key, value)| {
        matches!(key.as_ref(), "id" | "fileId" | "folderId").then(|| value.into_owned())
    }) {
        validate_id(&id)?;
        return Ok(DriveReference {
            file_id: id,
            resource_key,
            hinted_kind: None,
        });
    }

    let segments: Vec<&str> = url.path_segments().map(|s| s.collect()).unwrap_or_default();
    for window in segments.windows(2) {
        match window {
            ["folders", id] => {
                validate_id(id)?;
                return Ok(DriveReference {
                    file_id: (*id).to_string(),
                    resource_key,
                    hinted_kind: Some(DriveItemKindHint::Folder),
                });
            }
            ["d", id] => {
                validate_id(id)?;
                return Ok(DriveReference {
                    file_id: (*id).to_string(),
                    resource_key,
                    hinted_kind: docs_kind_hint(host, &segments).or(Some(DriveItemKindHint::File)),
                });
            }
            _ => {}
        }
    }

    Err(DriveLinkError::MissingId)
}

pub fn validate_id(value: &str) -> Result<(), DriveLinkError> {
    is_valid_id(value).map(|_| ())
}

fn is_valid_id(value: &str) -> Result<(), DriveLinkError> {
    let len = value.len();
    if !(10..=256).contains(&len) {
        return Err(DriveLinkError::InvalidLength);
    }
    if !value
        .bytes()
        .all(|b| b.is_ascii_alphanumeric() || b == b'_' || b == b'-')
    {
        return Err(DriveLinkError::InvalidCharacters);
    }
    Ok(())
}

fn is_supported_host(host: &str) -> bool {
    matches!(
        host,
        "drive.google.com"
            | "docs.google.com"
            | "drive.usercontent.google.com"
            | "lh3.googleusercontent.com"
    )
}

fn docs_kind_hint(host: &str, segments: &[&str]) -> Option<DriveItemKindHint> {
    if host != "docs.google.com" {
        return None;
    }
    match segments.first().copied() {
        Some("document") => Some(DriveItemKindHint::GoogleDocument),
        Some("spreadsheets") => Some(DriveItemKindHint::GoogleSpreadsheet),
        Some("presentation") => Some(DriveItemKindHint::GooglePresentation),
        _ => None,
    }
}
