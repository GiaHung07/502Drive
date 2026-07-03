use serde::{Deserialize, Serialize};
use thiserror::Error;

use super::links::{DriveItemKindHint, DriveLinkError, parse_drive_reference};

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
