use gdclone_bot::{
    drive::types::FOLDER_MIME_TYPE,
    engine::{
        mapping::{MappingRecord, record_mapping},
        operation::ScopeType,
    },
    report,
    state::{
        db::Database,
        repo::{self, NewItem},
    },
};

async fn test_db() -> (Database, String) {
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
    let job_id = repo::create_job(&db, 1, 2, "default", "source-root", "dest-parent")
        .await
        .unwrap();
    (db, job_id)
}

#[tokio::test]
async fn job_report_contains_items_and_folder_mappings() {
    let (db, job_id) = test_db().await;
    repo::insert_item(
        &db,
        NewItem {
            job_id: job_id.clone(),
            source_item_id: "file-ok".to_string(),
            destination_parent_id: "dest-parent".to_string(),
            mime_type: "application/octet-stream".to_string(),
            item_kind: "binary".to_string(),
            source_name: "ok.bin".to_string(),
            size_bytes: Some(1),
        },
    )
    .await
    .unwrap();
    repo::mark_item_done(&db, &job_id, "file-ok", "dest-ok")
        .await
        .unwrap();

    repo::insert_item(
        &db,
        NewItem {
            job_id: job_id.clone(),
            source_item_id: "file-failed".to_string(),
            destination_parent_id: "dest-parent".to_string(),
            mime_type: "application/octet-stream".to_string(),
            item_kind: "binary".to_string(),
            source_name: "bad.bin".to_string(),
            size_bytes: Some(2),
        },
    )
    .await
    .unwrap();
    repo::mark_item_failed(
        &db,
        &job_id,
        "file-failed",
        "rateLimitExceeded",
        "try later",
    )
    .await
    .unwrap();

    record_mapping(
        &db,
        MappingRecord {
            scope_type: ScopeType::Job.as_str().to_string(),
            scope_id: job_id.clone(),
            source_item_id: "folder".to_string(),
            destination_item_id: "dest-folder".to_string(),
            source_parent_id: None,
            destination_parent_id: Some("dest-parent".to_string()),
            mime_type: FOLDER_MIME_TYPE.to_string(),
            source_name: "folder".to_string(),
        },
    )
    .await
    .unwrap();

    let items = report::collect_job_items(&db, &job_id).await.unwrap();
    assert_eq!(items.len(), 3);
    assert!(items.iter().any(|item| {
        item.source_item_id == "file-ok"
            && item.dest_item_id.as_deref() == Some("dest-ok")
            && item.status == "done"
    }));
    assert!(items.iter().any(|item| {
        item.source_item_id == "file-failed"
            && item.error_category.as_deref() == Some("rate_limit")
            && item.last_error.as_deref() == Some("rateLimitExceeded: try later")
    }));
    assert!(items.iter().any(|item| {
        item.source_item_id == "folder"
            && item.dest_item_id.as_deref() == Some("dest-folder")
            && item.status == "active"
    }));

    let report_dir =
        std::env::temp_dir().join(format!("gdclone-report-test-{}", uuid::Uuid::new_v4()));
    let paths = report::write_job_reports(&db, &report_dir, &job_id)
        .await
        .unwrap();
    assert!(paths.json.exists());
    assert!(paths.csv.exists());
    assert!(
        std::fs::read_to_string(paths.json)
            .unwrap()
            .contains("file-ok")
    );
    assert!(
        std::fs::read_to_string(paths.csv)
            .unwrap()
            .contains("bad.bin")
    );
}

#[test]
fn cleanup_old_reports_retention_zero_deletes_nothing() {
    let dir = std::env::temp_dir().join(format!("gdclone-cleanup-test-{}", uuid::Uuid::new_v4()));
    std::fs::create_dir_all(&dir).unwrap();
    let json = dir.join("report.json");
    let csv = dir.join("report.csv");
    std::fs::write(&json, "{}").unwrap();
    std::fs::write(&csv, "a,b\n").unwrap();

    assert_eq!(report::cleanup_old_reports(&dir, 0).unwrap(), 0);
    assert!(json.exists());
    assert!(csv.exists());

    let _ = std::fs::remove_dir_all(dir);
}
