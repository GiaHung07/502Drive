use serde::{Deserialize, Serialize};
use thiserror::Error;

pub use super::links::{DriveItemKindHint, DriveLinkError, DriveReference, parse_drive_reference};

pub const MIN_BARE_DRIVE_ID_LEN: usize = 20;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum DriveResourceKind {
    File,
    Folder,
    Unknown,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DriveResource {
    pub id: String,
    pub kind: DriveResourceKind,
}

#[derive(Debug, Error, PartialEq, Eq)]
pub enum ParseDriveIdError {
    #[error("empty Drive URL or ID")]
    Empty,
    #[error("not a Google Drive URL and not a valid Drive ID")]
    Invalid,
    #[error("Google Drive URL did not contain a file or folder ID")]
    MissingId,
}

pub fn parse(input: &str) -> Result<DriveResource, ParseDriveIdError> {
    let parsed = parse_drive_reference(input).map_err(|err| match err {
        DriveLinkError::Empty => ParseDriveIdError::Empty,
        DriveLinkError::MissingId => ParseDriveIdError::MissingId,
        DriveLinkError::Invalid
        | DriveLinkError::InvalidLength
        | DriveLinkError::InvalidCharacters => ParseDriveIdError::Invalid,
    })?;
    let kind = match parsed.hinted_kind {
        Some(DriveItemKindHint::Folder) => DriveResourceKind::Folder,
        Some(_) => DriveResourceKind::File,
        None => DriveResourceKind::Unknown,
    };
    Ok(DriveResource {
        id: parsed.file_id,
        kind,
    })
}

/// Auto-detect a Google Drive reference from user message text.
///
/// Handles:
/// - Full/bare URLs: `drive.google.com/drive/folders/{id}`, `/file/d/{id}`, `docs.google.com/...`
/// - Query parameters: `?id=...`, `?folders=...`, `?fileId=...`, `?folderId=...`
/// - URLs embedded within message sentences (e.g. "xem link này https://drive.google.com/...")
/// - Bare Drive ID: only if the trimmed message is a single token of length >= 20 chars
///   consisting solely of valid ID characters `[a-zA-Z0-9_-]`.
pub fn detect_drive_reference(input: &str) -> Option<DriveReference> {
    let trimmed = input.trim();
    if trimmed.is_empty() {
        return None;
    }

    // 1. If trimmed contains URL patterns (drive.google.com or docs.google.com):
    if trimmed.contains("drive.google.com")
        || trimmed.contains("docs.google.com")
        || trimmed.contains("drive.usercontent.google.com")
        || trimmed.contains("lh3.googleusercontent.com")
    {
        // Try parsing the trimmed string directly
        if let Ok(reference) = parse_drive_reference(trimmed) {
            return Some(reference);
        }
        // If message has surrounding text, look for word with google.com
        for word in trimmed.split_whitespace() {
            let cleaned = word.trim_matches(|c: char| {
                matches!(
                    c,
                    '<' | '>' | '(' | ')' | '[' | ']' | '{' | '}' | '"' | '\'' | ',' | ';'
                )
            });
            if let Ok(reference) = parse_drive_reference(cleaned) {
                return Some(reference);
            }
        }
    }

    // 2. Direct parse attempt (e.g. bare ID or relative folder link)
    if let Ok(reference) = parse_drive_reference(trimmed) {
        // If it was a bare ID, enforce MIN_BARE_DRIVE_ID_LEN to avoid false positives
        // on ordinary English words or short commands (e.g. "disconnect", "cancel").
        let is_bare = trimmed == reference.file_id;
        if is_bare && reference.file_id.len() < MIN_BARE_DRIVE_ID_LEN {
            return None;
        }
        return Some(reference);
    }

    None
}
