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
