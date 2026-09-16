use std::time::Duration;

use anyhow::Context;
use reqwest::{Client as HttpClient, StatusCode};
use serde_json::{Value, json};

use super::{
    changes::{ChangeList, StartPageTokenResponse},
    links::DriveReference,
    types::{
        DriveAbout, DriveFile, FOLDER_MIME_TYPE, FileList, GoogleErrorEnvelope, SHORTCUT_MIME_TYPE,
        SharedDriveList,
    },
};
use crate::engine::operation::AppProperties;

const DRIVE_FILES_URL: &str = "https://www.googleapis.com/drive/v3/files";
const DRIVE_ABOUT_URL: &str = "https://www.googleapis.com/drive/v3/about";
const DRIVE_CHANGES_URL: &str = "https://www.googleapis.com/drive/v3/changes";
const DRIVE_DRIVES_URL: &str = "https://www.googleapis.com/drive/v3/drives";
pub const FILE_FIELDS: &str = concat!(
    "id,name,mimeType,size,parents,driveId,resourceKey,",
    "shortcutDetails(targetId,targetMimeType,targetResourceKey),",
    "trashed,modifiedTime,md5Checksum,version,",
    "capabilities(canCopy,canAddChildren,canListChildren,canRename,canMoveItemWithinDrive,canTrash),",
    "copyRequiresWriterPermission,appProperties"
);

#[derive(Clone)]
pub struct DriveClient {
    http: HttpClient,
}

impl DriveClient {
    pub fn new() -> Self {
        Self::with_timeout(Duration::from_secs(60))
    }

    pub fn with_timeout(timeout: Duration) -> Self {
        Self {
            http: HttpClient::builder()
                .timeout(timeout)
                .build()
                .expect("build Drive HTTP client"),
        }
    }

    pub async fn about_get(&self, access_token: &str) -> Result<DriveAbout, DriveApiError> {
        decode_response(
            self.http
                .get(DRIVE_ABOUT_URL)
                .bearer_auth(access_token)
                .query(&[("fields", "user(displayName,emailAddress)")])
                .send()
                .await
                .context("send about.get")?,
        )
        .await
    }

    pub async fn get_file(
        &self,
        access_token: &str,
        file_id: &str,
    ) -> Result<DriveFile, DriveApiError> {
        let response = self
            .http
            .get(format!("{DRIVE_FILES_URL}/{file_id}"))
            .bearer_auth(access_token)
            .query(&[("fields", FILE_FIELDS), ("supportsAllDrives", "true")])
            .send()
            .await
            .context("send files.get")?;

        decode_response(response).await
    }

    pub async fn get_reference(
        &self,
        access_token: &str,
        reference: &DriveReference,
    ) -> Result<DriveFile, DriveApiError> {
        let mut request = self
            .http
            .get(format!("{DRIVE_FILES_URL}/{}", reference.file_id))
            .bearer_auth(access_token)
            .query(&[("fields", FILE_FIELDS), ("supportsAllDrives", "true")]);
        if let Some(header) = reference.resource_key_header_value() {
            request = request.header("X-Goog-Drive-Resource-Keys", header);
        }
        decode_response(request.send().await.context("send files.get")?).await
    }

    pub async fn list_children(
        &self,
        access_token: &str,
        parent_id: &str,
        parent_resource_key: Option<&str>,
        page_token: Option<&str>,
    ) -> Result<FileList, DriveApiError> {
        self.list_children_page_size(
            access_token,
            parent_id,
            parent_resource_key,
            page_token,
            1000,
        )
        .await
    }

    pub async fn list_children_page_size(
        &self,
        access_token: &str,
        parent_id: &str,
        parent_resource_key: Option<&str>,
        page_token: Option<&str>,
        page_size: usize,
    ) -> Result<FileList, DriveApiError> {
        self.list_children_query(
            access_token,
            parent_id,
            parent_resource_key,
            page_token,
            page_size,
            None,
            None,
        )
        .await
    }

    pub async fn list_child_folders_page_size(
        &self,
        access_token: &str,
        parent_id: &str,
        parent_resource_key: Option<&str>,
        page_token: Option<&str>,
        page_size: usize,
        drive_id: Option<&str>,
    ) -> Result<FileList, DriveApiError> {
        self.list_children_query(
            access_token,
            parent_id,
            parent_resource_key,
            page_token,
            page_size,
            Some(FOLDER_MIME_TYPE),
            drive_id,
        )
        .await
    }

    #[allow(clippy::too_many_arguments)]
    async fn list_children_query(
        &self,
        access_token: &str,
        parent_id: &str,
        parent_resource_key: Option<&str>,
        page_token: Option<&str>,
        page_size: usize,
        mime_type: Option<&str>,
        drive_id: Option<&str>,
    ) -> Result<FileList, DriveApiError> {
        let mut query = format!("'{parent_id}' in parents and trashed=false");
        if let Some(mime_type) = mime_type {
            query.push_str(&format!(" and mimeType='{mime_type}'"));
        }
        let page_size = page_size.clamp(1, 1000).to_string();
        let mut request = self
            .http
            .get(DRIVE_FILES_URL)
            .bearer_auth(access_token)
            .query(&[
                ("q", query.as_str()),
                ("pageSize", page_size.as_str()),
                ("fields", &format!("nextPageToken,files({FILE_FIELDS})")),
                ("supportsAllDrives", "true"),
                ("includeItemsFromAllDrives", "true"),
            ]);
        if let Some(drive_id) = drive_id {
            request = request.query(&[("corpora", "drive"), ("driveId", drive_id)]);
        }
        if let Some(resource_key) = parent_resource_key {
            request = request.header(
                "X-Goog-Drive-Resource-Keys",
                format!("{parent_id}/{resource_key}"),
            );
        }
        if let Some(token) = page_token {
            request = request.query(&[("pageToken", token)]);
        }

        decode_response(request.send().await.context("send files.list")?).await
    }

    pub async fn list_shared_drives_page_size(
        &self,
        access_token: &str,
        page_token: Option<&str>,
        page_size: usize,
    ) -> Result<SharedDriveList, DriveApiError> {
        let page_size = page_size.clamp(1, 100).to_string();
        let mut request = self
            .http
            .get(DRIVE_DRIVES_URL)
            .bearer_auth(access_token)
            .query(&[
                ("pageSize", page_size.as_str()),
                ("fields", "nextPageToken,drives(id,name)"),
            ]);
        if let Some(token) = page_token {
            request = request.query(&[("pageToken", token)]);
        }
        decode_response(request.send().await.context("send drives.list")?).await
    }

    pub async fn find_by_copy_key(
        &self,
        access_token: &str,
        destination_parent_id: &str,
        copy_key: &str,
    ) -> Result<Vec<DriveFile>, DriveApiError> {
        let query = app_property_copy_key_query(destination_parent_id, copy_key);
        let response = self
            .http
            .get(DRIVE_FILES_URL)
            .bearer_auth(access_token)
            .query(&[
                ("q", query.as_str()),
                ("pageSize", "10"),
                ("fields", &format!("files({FILE_FIELDS})")),
                ("supportsAllDrives", "true"),
                ("includeItemsFromAllDrives", "true"),
            ])
            .send()
            .await
            .context("send files.list appProperties lookup")?;
        Ok(decode_response::<FileList>(response).await?.files)
    }

    pub async fn create_folder(
        &self,
        access_token: &str,
        name: &str,
        parent_id: &str,
        app_properties: Option<&AppProperties>,
    ) -> Result<DriveFile, DriveApiError> {
        let mut body = json!({
            "name": name,
            "mimeType": FOLDER_MIME_TYPE,
            "parents": [parent_id],
        });
        insert_app_properties(&mut body, app_properties);

        decode_response(
            self.http
                .post(DRIVE_FILES_URL)
                .bearer_auth(access_token)
                .query(&[("fields", FILE_FIELDS), ("supportsAllDrives", "true")])
                .json(&body)
                .send()
                .await
                .context("send files.create")?,
        )
        .await
    }

    pub async fn create_shortcut(
        &self,
        access_token: &str,
        name: &str,
        parent_id: &str,
        target_id: &str,
        target_resource_key: Option<&str>,
        app_properties: Option<&AppProperties>,
    ) -> Result<DriveFile, DriveApiError> {
        let mut body = json!({
            "name": name,
            "mimeType": SHORTCUT_MIME_TYPE,
            "parents": [parent_id],
            "shortcutDetails": {
                "targetId": target_id,
            },
        });
        if let Some(resource_key) = target_resource_key {
            body["shortcutDetails"]["targetResourceKey"] = json!(resource_key);
        }
        insert_app_properties(&mut body, app_properties);

        decode_response(
            self.http
                .post(DRIVE_FILES_URL)
                .bearer_auth(access_token)
                .query(&[("fields", FILE_FIELDS), ("supportsAllDrives", "true")])
                .json(&body)
                .send()
                .await
                .context("send files.create shortcut")?,
        )
        .await
    }

    pub async fn copy_file(
        &self,
        access_token: &str,
        source: &DriveReference,
        name: &str,
        parent_id: &str,
        app_properties: Option<&AppProperties>,
    ) -> Result<DriveFile, DriveApiError> {
        let mut body = json!({
            "name": name,
            "parents": [parent_id],
        });
        insert_app_properties(&mut body, app_properties);

        let mut request = self
            .http
            .post(format!("{DRIVE_FILES_URL}/{}/copy", source.file_id))
            .bearer_auth(access_token)
            .query(&[("fields", FILE_FIELDS), ("supportsAllDrives", "true")])
            .json(&body);
        if let Some(header) = source.resource_key_header_value() {
            request = request.header("X-Goog-Drive-Resource-Keys", header);
        }

        decode_response(request.send().await.context("send files.copy")?).await
    }

    /// `changes.getStartPageToken` — returns the token representing the head
    /// of the change log for a corpus.
    ///
    /// `drive_id=None` → user corpus; `Some(id)` → Shared Drive corpus.
    pub async fn get_start_page_token(
        &self,
        access_token: &str,
        drive_id: Option<&str>,
    ) -> Result<StartPageTokenResponse, DriveApiError> {
        let mut req = self
            .http
            .get(format!("{DRIVE_CHANGES_URL}/startPageToken"))
            .bearer_auth(access_token)
            .query(&[
                ("supportsAllDrives", "true"),
                ("includeItemsFromAllDrives", "true"),
            ]);
        if let Some(id) = drive_id {
            req = req.query(&[("driveId", id)]);
        }
        decode_response(req.send().await.context("send changes.getStartPageToken")?).await
    }

    /// `changes.list` — fetch one page of changes starting from `page_token`.
    ///
    /// Callers must commit all returned events atomically with the cursor
    /// advance before calling again (spec §12.3).
    pub async fn list_changes(
        &self,
        access_token: &str,
        page_token: &str,
        drive_id: Option<&str>,
    ) -> Result<ChangeList, DriveApiError> {
        let mut req = self
            .http
            .get(DRIVE_CHANGES_URL)
            .bearer_auth(access_token)
            .query(&[
                ("pageToken", page_token),
                ("fields", concat!(
                    "nextPageToken,newStartPageToken,",
                    "changes(fileId,removed,driveId,",
                    "file(id,name,mimeType,size,parents,driveId,resourceKey,",
                    "shortcutDetails(targetId,targetMimeType,targetResourceKey),",
                    "trashed,modifiedTime,md5Checksum,version,",
                    "capabilities(canCopy,canAddChildren,canListChildren,canRename,",
                    "canMoveItemWithinDrive,canTrash),copyRequiresWriterPermission,appProperties))"
                )),
                ("supportsAllDrives", "true"),
                ("includeItemsFromAllDrives", "true"),
                ("pageSize", "1000"),
            ]);
        if let Some(id) = drive_id {
            req = req.query(&[("driveId", id)]);
        }
        decode_response(req.send().await.context("send changes.list")?).await
    }

    /// `files.update` — rename a file in place.
    pub async fn rename_file(
        &self,
        access_token: &str,
        file_id: &str,
        new_name: &str,
        _timeout: Duration,
    ) -> Result<DriveFile, DriveApiError> {
        let body = serde_json::json!({ "name": new_name });
        decode_response(
            self.http
                .patch(format!("{DRIVE_FILES_URL}/{file_id}"))
                .bearer_auth(access_token)
                .query(&[("fields", FILE_FIELDS), ("supportsAllDrives", "true")])
                .json(&body)
                .send()
                .await
                .context("send files.update (rename)")?,
        )
        .await
    }

    /// `files.update` — move a file to a new parent.
    ///
    /// Drive requires the *current* parent(s) in `removeParents`. We pass the
    /// destination parent in `addParents`. We re-fetch the file first to obtain
    /// the current parents.
    pub async fn move_file(
        &self,
        access_token: &str,
        file_id: &str,
        new_parent_id: &str,
        _timeout: Duration,
    ) -> Result<DriveFile, DriveApiError> {
        // Fetch current parents.
        let current = self.get_file(access_token, file_id).await?;
        let remove_parents = current.parents.join(",");

        decode_response(
            self.http
                .patch(format!("{DRIVE_FILES_URL}/{file_id}"))
                .bearer_auth(access_token)
                .query(&[
                    ("addParents", new_parent_id),
                    ("removeParents", &remove_parents),
                    ("fields", FILE_FIELDS),
                    ("supportsAllDrives", "true"),
                ])
                .json(&serde_json::json!({}))
                .send()
                .await
                .context("send files.update (move)")?,
        )
        .await
    }

    /// `files.update` — move an item to trash.
    pub async fn trash_file(
        &self,
        access_token: &str,
        file_id: &str,
    ) -> Result<DriveFile, DriveApiError> {
        decode_response(
            self.http
                .patch(format!("{DRIVE_FILES_URL}/{file_id}"))
                .bearer_auth(access_token)
                .query(&[("fields", FILE_FIELDS), ("supportsAllDrives", "true")])
                .json(&serde_json::json!({ "trashed": true }))
                .send()
                .await
                .context("send files.update (trash)")?,
        )
        .await
    }
}

pub fn app_property_copy_key_query(destination_parent_id: &str, copy_key: &str) -> String {
    format!(
        "'{}' in parents and trashed=false and appProperties has {{ key='gdclone_copy_key' and value='{}' }}",
        escape_drive_query_value(destination_parent_id),
        escape_drive_query_value(copy_key)
    )
}

fn escape_drive_query_value(value: &str) -> String {
    value.replace('\\', "\\\\").replace('\'', "\\'")
}

impl Default for DriveClient {
    fn default() -> Self {
        Self::new()
    }
}

#[derive(Debug, thiserror::Error)]
pub enum DriveApiError {
    #[error("Drive API HTTP {status}: {message}")]
    Api {
        status: StatusCode,
        message: String,
        reason: Option<String>,
    },
    #[error(transparent)]
    Transport(#[from] anyhow::Error),
}

impl DriveApiError {
    pub fn status(&self) -> Option<StatusCode> {
        match self {
            DriveApiError::Api { status, .. } => Some(*status),
            DriveApiError::Transport(_) => None,
        }
    }

    pub fn reason(&self) -> Option<&str> {
        match self {
            DriveApiError::Api { reason, .. } => reason.as_deref(),
            DriveApiError::Transport(_) => None,
        }
    }
}

async fn decode_response<T: serde::de::DeserializeOwned>(
    response: reqwest::Response,
) -> Result<T, DriveApiError> {
    let status = response.status();
    if status.is_success() {
        return response
            .json::<T>()
            .await
            .map_err(|err| DriveApiError::Transport(err.into()));
    }

    let text = response.text().await.unwrap_or_default();
    let parsed = serde_json::from_str::<GoogleErrorEnvelope>(&text).ok();
    let message = parsed
        .as_ref()
        .map(|err| err.error.message.clone())
        .unwrap_or(text);
    let reason = parsed.and_then(|err| {
        err.error
            .errors
            .into_iter()
            .find_map(|detail| detail.reason)
    });

    Err(DriveApiError::Api {
        status,
        message,
        reason,
    })
}

fn insert_app_properties(body: &mut Value, app_properties: Option<&AppProperties>) {
    let Some(props) = app_properties else {
        return;
    };
    body["appProperties"] = json!({
        "gdclone_source_id": props.gdclone_source_id,
        "gdclone_scope_id": props.gdclone_scope_id,
        "gdclone_copy_key": props.gdclone_copy_key,
        "gdclone_generation": props.gdclone_generation,
    });
}
