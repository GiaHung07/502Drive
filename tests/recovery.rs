use gdclone_bot::{
    engine::{
        operation::{OperationType, ScopeType, idempotency_key},
        recovery,
    },
    state::{
        db::Database,
        repo::{self, JobStatusValue, NewItem, NewOperationIntent},
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
async fn startup_recovery_requeues_interrupted_work() {
    let (db, job_id) = test_db().await;

    repo::update_job_status(&db, &job_id, JobStatusValue::Running, None)
        .await
        .unwrap();

    repo::record_traversal_folder(&db, &job_id, "src-folder", None, "dst-folder", None)
        .await
        .unwrap();
    let claimed = repo::next_pending_traversal_folder(&db, &job_id)
        .await
        .unwrap();
    assert!(claimed.is_some());

    let item_id = repo::insert_item(
        &db,
        NewItem {
            job_id: job_id.clone(),
            source_item_id: "src-file".to_string(),
            destination_parent_id: "dst-folder".to_string(),
            mime_type: "application/octet-stream".to_string(),
            item_kind: "binary".to_string(),
            source_name: "file.bin".to_string(),
            size_bytes: Some(42),
        },
    )
    .await
    .unwrap();
    assert!(!item_id.is_empty());
    repo::mark_item_copying(&db, &job_id, "src-file")
        .await
        .unwrap();

    let key = idempotency_key(ScopeType::Job, &job_id, "src-file", "dst-folder", 1);
    repo::plan_operation_intent(
        &db,
        NewOperationIntent {
            idempotency_key: key.clone(),
            job_id: Some(job_id.clone()),
            watch_id: None,
            operation_type: OperationType::CopyFile,
            source_item_id: Some("src-file".to_string()),
            destination_parent_id: Some("dst-folder".to_string()),
            destination_item_id: None,
            request_json: "{}".to_string(),
        },
    )
    .await
    .unwrap();
    repo::mark_operation_executing(&db, &key).await.unwrap();

    let summary = recovery::recover_on_startup(&db).await.unwrap();
    assert_eq!(summary.jobs_marked_recovering, 1);
    assert_eq!(summary.traversal_folders_requeued, 1);
    assert_eq!(summary.operation_intents_replanned, 1);
    assert_eq!(summary.job_items_requeued, 1);

    assert_eq!(
        repo::job_status(&db, &job_id).await.unwrap().as_deref(),
        Some("recovering")
    );
    let counts = repo::job_status_counts(&db).await.unwrap();
    assert_eq!(counts.len(), 1);
    assert_eq!(counts[0].status, "recovering");
    assert_eq!(counts[0].count, 1);
    assert_eq!(
        repo::item_status(&db, &job_id, "src-file")
            .await
            .unwrap()
            .as_deref(),
        Some("ready")
    );

    let intent = repo::find_operation_intent_by_key(&db, &key)
        .await
        .unwrap()
        .unwrap();
    assert_eq!(intent.status, "planned");
    assert_eq!(intent.attempts, 1);

    let reclaimed = repo::next_pending_traversal_folder(&db, &job_id)
        .await
        .unwrap()
        .unwrap();
    assert_eq!(reclaimed.source_folder_id, "src-folder");

    let second = recovery::recover_on_startup(&db).await.unwrap();
    assert_eq!(second.jobs_marked_recovering, 0);
    assert_eq!(second.traversal_folders_requeued, 1);
    assert_eq!(second.operation_intents_replanned, 0);
    assert_eq!(second.job_items_requeued, 0);
}

#[tokio::test]
async fn resumable_jobs_preserve_source_metadata() {
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

    let job_id = repo::create_job_with_metadata(
        &db,
        repo::NewJob {
            chat_id: 100,
            telegram_user_id: 200,
            google_account_id: "default".to_string(),
            source_root_id: "source-root".to_string(),
            source_resource_key: Some("resource-key".to_string()),
            source_drive_id: Some("source-drive".to_string()),
            destination_parent_id: "dest-parent".to_string(),
            destination_drive_id: Some("dest-drive".to_string()),
            progress_message_id: None,
        },
    )
    .await
    .unwrap();

    let jobs = repo::resumable_one_shot_jobs(&db).await.unwrap();
    assert_eq!(jobs.len(), 1);
    assert_eq!(jobs[0].id, job_id);
    assert_eq!(jobs[0].chat_id, 100);
    assert_eq!(jobs[0].telegram_user_id, 200);
    assert_eq!(jobs[0].source_root_id, "source-root");
    assert_eq!(jobs[0].source_resource_key.as_deref(), Some("resource-key"));
    assert_eq!(jobs[0].destination_parent_id, "dest-parent");

    repo::update_job_status(&db, &job_id, JobStatusValue::Completed, None)
        .await
        .unwrap();
    assert!(repo::resumable_one_shot_jobs(&db).await.unwrap().is_empty());
}

#[tokio::test]
async fn job_visibility_queries_are_scoped_to_user() {
    let (db, active_job_id) = test_db().await;
    repo::update_job_counts(&db, &active_job_id, 3, 2, 1, 0)
        .await
        .unwrap();
    repo::update_job_status(&db, &active_job_id, JobStatusValue::Running, None)
        .await
        .unwrap();

    let completed_job_id = repo::create_job(&db, 1, 2, "default", "done-source", "dest-parent")
        .await
        .unwrap();
    repo::update_job_status(&db, &completed_job_id, JobStatusValue::Completed, None)
        .await
        .unwrap();
    let other_active_job_id = repo::create_job(&db, 1, 3, "default", "other-source", "dest-parent")
        .await
        .unwrap();
    repo::update_job_status(&db, &other_active_job_id, JobStatusValue::Queued, None)
        .await
        .unwrap();

    let active = repo::list_active_jobs_for_user(&db, 2, 10).await.unwrap();
    assert_eq!(active.len(), 1);
    assert_eq!(repo::active_job_count_for_user(&db, 2).await.unwrap(), 1);
    assert_eq!(repo::active_job_count(&db).await.unwrap(), 2);
    assert_eq!(active[0].id, active_job_id);
    assert_eq!(active[0].status, "running");
    assert_eq!(active[0].total_discovered, 3);
    assert_eq!(active[0].completed_items, 2);
    assert_eq!(active[0].failed_items, 1);

    let detail = repo::job_detail_for_user(&db, 2, &completed_job_id)
        .await
        .unwrap()
        .unwrap();
    assert_eq!(detail.status, "completed");
    assert_eq!(detail.source_root_id, "done-source");

    assert!(
        repo::job_detail_for_user(&db, 999, &active_job_id)
            .await
            .unwrap()
            .is_none()
    );
}

#[tokio::test]
async fn job_control_transitions_are_guarded_by_owner_and_state() {
    let (db, job_id) = test_db().await;

    assert!(!repo::pause_job_for_user(&db, 999, &job_id).await.unwrap());
    assert!(!repo::pause_job_for_user(&db, 2, &job_id).await.unwrap());

    repo::update_job_status(&db, &job_id, JobStatusValue::Running, None)
        .await
        .unwrap();
    assert!(repo::pause_job_for_user(&db, 2, &job_id).await.unwrap());
    assert_eq!(
        repo::job_status(&db, &job_id).await.unwrap().as_deref(),
        Some("pausing")
    );

    repo::update_job_status(&db, &job_id, JobStatusValue::Paused, None)
        .await
        .unwrap();
    assert!(repo::resume_job_for_user(&db, 2, &job_id).await.unwrap());
    assert_eq!(
        repo::job_status(&db, &job_id).await.unwrap().as_deref(),
        Some("recovering")
    );

    assert!(repo::cancel_job_for_user(&db, 2, &job_id).await.unwrap());
    assert_eq!(
        repo::job_status(&db, &job_id).await.unwrap().as_deref(),
        Some("cancelling")
    );

    repo::update_job_status(&db, &job_id, JobStatusValue::Paused, None)
        .await
        .unwrap();
    assert!(repo::cancel_job_for_user(&db, 2, &job_id).await.unwrap());
    assert_eq!(
        repo::job_status(&db, &job_id).await.unwrap().as_deref(),
        Some("cancelled")
    );
}

#[tokio::test]
async fn retry_failed_job_requeues_failed_work() {
    let (db, job_id) = test_db().await;
    repo::record_traversal_folder(&db, &job_id, "src-folder", None, "dst-folder", None)
        .await
        .unwrap();
    repo::fail_traversal_folder(&db, &job_id, "src-folder", "rate limit")
        .await
        .unwrap();
    repo::insert_item(
        &db,
        NewItem {
            job_id: job_id.clone(),
            source_item_id: "src-file".to_string(),
            destination_parent_id: "dst-folder".to_string(),
            mime_type: "application/octet-stream".to_string(),
            item_kind: "binary".to_string(),
            source_name: "file.bin".to_string(),
            size_bytes: Some(42),
        },
    )
    .await
    .unwrap();
    repo::mark_item_failed(&db, &job_id, "src-file", "rateLimitExceeded", "try later")
        .await
        .unwrap();

    let key = idempotency_key(ScopeType::Job, &job_id, "src-file", "dst-folder", 1);
    repo::plan_operation_intent(
        &db,
        NewOperationIntent {
            idempotency_key: key.clone(),
            job_id: Some(job_id.clone()),
            watch_id: None,
            operation_type: OperationType::CopyFile,
            source_item_id: Some("src-file".to_string()),
            destination_parent_id: Some("dst-folder".to_string()),
            destination_item_id: None,
            request_json: "{}".to_string(),
        },
    )
    .await
    .unwrap();
    repo::mark_operation_executing(&db, &key).await.unwrap();
    repo::update_job_status(
        &db,
        &job_id,
        JobStatusValue::PartiallyCompleted,
        Some("failed"),
    )
    .await
    .unwrap();
    repo::update_job_counts(&db, &job_id, 2, 1, 2, 0)
        .await
        .unwrap();

    assert!(
        repo::retry_failed_job_for_user(&db, 999, &job_id)
            .await
            .unwrap()
            .is_none()
    );

    let summary = repo::retry_failed_job_for_user(&db, 2, &job_id)
        .await
        .unwrap()
        .unwrap();
    assert_eq!(summary.traversal_folders_requeued, 1);
    assert_eq!(summary.job_items_requeued, 1);

    assert_eq!(
        repo::job_status(&db, &job_id).await.unwrap().as_deref(),
        Some("recovering")
    );
    assert_eq!(
        repo::job_counts(&db, &job_id).await.unwrap().failed_items,
        0
    );
    assert_eq!(
        repo::item_status(&db, &job_id, "src-file")
            .await
            .unwrap()
            .as_deref(),
        Some("ready")
    );
    assert!(
        repo::next_pending_traversal_folder(&db, &job_id)
            .await
            .unwrap()
            .is_some()
    );
}

#[tokio::test]
async fn owner_can_grant_and_revoke_operator() {
    let db = Database::in_memory().await.unwrap();
    db.run_migrations().await.unwrap();
    db.ensure_owner(10).await.unwrap();

    assert!(repo::is_owner(&db, 10).await.unwrap());
    assert!(repo::is_authorized(&db, 10).await.unwrap());
    assert!(!repo::is_authorized(&db, 20).await.unwrap());

    repo::grant_operator(&db, 20).await.unwrap();
    let user = repo::authorized_user(&db, 20).await.unwrap().unwrap();
    assert_eq!(user.role, "operator");
    assert!(user.enabled);
    assert!(repo::is_authorized(&db, 20).await.unwrap());
    assert!(!repo::is_owner(&db, 20).await.unwrap());

    assert!(repo::revoke_operator(&db, 20).await.unwrap());
    assert!(!repo::is_authorized(&db, 20).await.unwrap());
    assert!(!repo::revoke_operator(&db, 10).await.unwrap());
    assert!(repo::is_authorized(&db, 10).await.unwrap());
}
