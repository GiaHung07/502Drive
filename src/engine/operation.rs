use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ScopeType {
    Job,
    Watch,
}

impl ScopeType {
    pub fn as_str(self) -> &'static str {
        match self {
            ScopeType::Job => "job",
            ScopeType::Watch => "watch",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum OperationType {
    CreateFolder,
    CopyFile,
    CreateShortcut,
    Rename,
    Move,
    Trash,
    RestoreMapping,
}

impl OperationType {
    pub fn as_str(self) -> &'static str {
        match self {
            OperationType::CreateFolder => "create_folder",
            OperationType::CopyFile => "copy_file",
            OperationType::CreateShortcut => "create_shortcut",
            OperationType::Rename => "rename",
            OperationType::Move => "move",
            OperationType::Trash => "trash",
            OperationType::RestoreMapping => "restore_mapping",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum OperationStatus {
    Planned,
    Executing,
    Applied,
    CleanupPending,
    Failed,
}

impl OperationStatus {
    pub fn as_str(self) -> &'static str {
        match self {
            OperationStatus::Planned => "planned",
            OperationStatus::Executing => "executing",
            OperationStatus::Applied => "applied",
            OperationStatus::CleanupPending => "cleanup_pending",
            OperationStatus::Failed => "failed",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AppProperties {
    pub gdclone_source_id: String,
    pub gdclone_scope_id: String,
    pub gdclone_copy_key: String,
    pub gdclone_generation: String,
}

impl AppProperties {
    pub fn new(
        scope_type: ScopeType,
        scope_id: &str,
        source_item_id: &str,
        destination_parent_id: &str,
        generation: u32,
    ) -> Self {
        let copy_key = idempotency_key(
            scope_type,
            scope_id,
            source_item_id,
            destination_parent_id,
            generation,
        );
        Self {
            gdclone_source_id: source_item_id.to_string(),
            gdclone_scope_id: scope_id.to_string(),
            gdclone_copy_key: copy_key,
            gdclone_generation: generation.to_string(),
        }
    }
}

pub fn idempotency_key(
    scope_type: ScopeType,
    scope_id: &str,
    source_item_id: &str,
    destination_parent_id: &str,
    generation: u32,
) -> String {
    let mut hasher = Sha256::new();
    hasher.update(scope_type.as_str().as_bytes());
    hasher.update(b"\0");
    hasher.update(scope_id.as_bytes());
    hasher.update(b"\0");
    hasher.update(source_item_id.as_bytes());
    hasher.update(b"\0");
    hasher.update(destination_parent_id.as_bytes());
    hasher.update(b"\0");
    hasher.update(generation.to_string().as_bytes());
    format!("gdclone_{:x}", hasher.finalize())
}
