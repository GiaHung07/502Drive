use gdclone_bot::{
    engine::mapping::{MappingRecord, lookup_mapping, record_mapping},
    state::{
        db::Database,
        repo::{self, NewItem},
    },
};

async fn test_db() -> Database {
    let db = Database::in_memory().await.unwrap();
    db.run_migrations().await.unwrap();
    repo::upsert_google_account(
        &db,
        "default",
        Some("user@example.com"),
        vec![1, 2, 3],
        "test",
        r#"["https://www.googleapis.com/auth/drive"]"#,
    )
    .await
    .unwrap();
    db
}

#[tokio::test]
async fn stores_and_reads_source_mapping() {
    let db = test_db().await;

    let job_id = repo::create_job(&db, 1, 2, "default", "source", "dest")
        .await
        .unwrap();
    record_mapping(
        &db,
        MappingRecord {
            scope_type: "job".to_string(),
            scope_id: job_id.clone(),
            source_item_id: "src-folder".to_string(),
            destination_item_id: "dst-folder".to_string(),
            source_parent_id: None,
            destination_parent_id: Some("dest".to_string()),
            mime_type: "application/vnd.google-apps.folder".to_string(),
            source_name: "Folder".to_string(),
            source_version: None,
            source_modified_time: None,
            source_md5_checksum: None,
        },
    )
    .await
    .unwrap();

    assert_eq!(
        lookup_mapping(&db, "job", &job_id, "src-folder")
            .await
            .unwrap(),
        Some("dst-folder".to_string())
    );
}

#[tokio::test]
async fn marks_item_done_and_mapping_supports_resume_idempotency() {
    let db = test_db().await;

    let job_id = repo::create_job(&db, 1, 2, "default", "source", "dest")
        .await
        .unwrap();
    repo::insert_item(
        &db,
        NewItem {
            job_id: job_id.clone(),
            source_item_id: "src-file".to_string(),
            destination_parent_id: "dest".to_string(),
            mime_type: "application/octet-stream".to_string(),
            item_kind: "binary".to_string(),
            source_name: "name".to_string(),
            size_bytes: Some(10),
        },
    )
    .await
    .unwrap();
    repo::mark_item_done(&db, &job_id, "src-file", "dst-file")
        .await
        .unwrap();
    record_mapping(
        &db,
        MappingRecord {
            scope_type: "job".to_string(),
            scope_id: job_id.clone(),
            source_item_id: "src-file".to_string(),
            destination_item_id: "dst-file".to_string(),
            source_parent_id: None,
            destination_parent_id: Some("dest".to_string()),
            mime_type: "application/octet-stream".to_string(),
            source_name: "name".to_string(),
            source_version: None,
            source_modified_time: None,
            source_md5_checksum: None,
        },
    )
    .await
    .unwrap();

    assert_eq!(
        repo::existing_dest_for_source(&db, "job", &job_id, "src-file")
            .await
            .unwrap(),
        Some("dst-file".to_string())
    );
}

#[tokio::test]
async fn watch_active_destination_ignores_detached_mapping() {
    let db = test_db().await;
    record_mapping(
        &db,
        MappingRecord {
            scope_type: "watch".to_string(),
            scope_id: "watch-1".to_string(),
            source_item_id: "src-folder".to_string(),
            destination_item_id: "dst-folder".to_string(),
            source_parent_id: None,
            destination_parent_id: Some("dst-root".to_string()),
            mime_type: "application/vnd.google-apps.folder".to_string(),
            source_name: "Folder".to_string(),
            source_version: Some("7".to_string()),
            source_modified_time: Some("2026-07-04T00:00:00Z".to_string()),
            source_md5_checksum: Some("abc".to_string()),
        },
    )
    .await
    .unwrap();
    repo::mark_watch_mapping_detached(&db, "watch-1", "src-folder")
        .await
        .unwrap();

    assert_eq!(
        repo::watch_mapping_destination(&db, "watch-1", "src-folder")
            .await
            .unwrap(),
        Some("dst-folder".to_string())
    );
    assert_eq!(
        repo::active_watch_mapping_destination(&db, "watch-1", "src-folder")
            .await
            .unwrap(),
        None
    );
    let fingerprint = repo::watch_mapping_fingerprint(&db, "watch-1", "src-folder")
        .await
        .unwrap()
        .unwrap();
    assert_eq!(fingerprint.name.as_deref(), Some("Folder"));
    assert_eq!(fingerprint.version.as_deref(), Some("7"));
    assert_eq!(fingerprint.md5_checksum.as_deref(), Some("abc"));
}

#[tokio::test]
async fn watch_folder_scan_lists_only_active_folders() {
    let db = test_db().await;
    for (source, dest, mime, state) in [
        (
            "src-folder",
            "dst-folder",
            "application/vnd.google-apps.folder",
            "active",
        ),
        ("src-file", "dst-file", "application/pdf", "active"),
        (
            "old-folder",
            "old-dst",
            "application/vnd.google-apps.folder",
            "detached",
        ),
    ] {
        record_mapping(
            &db,
            MappingRecord {
                scope_type: "watch".to_string(),
                scope_id: "watch-1".to_string(),
                source_item_id: source.to_string(),
                destination_item_id: dest.to_string(),
                source_parent_id: None,
                destination_parent_id: None,
                mime_type: mime.to_string(),
                source_name: source.to_string(),
                source_version: None,
                source_modified_time: None,
                source_md5_checksum: None,
            },
        )
        .await
        .unwrap();
        if state != "active" {
            repo::mark_watch_mapping_detached(&db, "watch-1", source)
                .await
                .unwrap();
        }
    }

    let folders = repo::active_watch_folder_mappings(&db, "watch-1")
        .await
        .unwrap();
    assert_eq!(folders.len(), 1);
    assert_eq!(folders[0].source_item_id, "src-folder");
    assert_eq!(folders[0].destination_item_id, "dst-folder");
}
