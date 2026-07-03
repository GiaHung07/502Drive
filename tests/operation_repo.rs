use gdclone_bot::{
    engine::operation::{OperationType, ScopeType, idempotency_key},
    state::{
        db::Database,
        repo::{self, NewDestinationProfile, NewOperationIntent},
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
async fn operation_intent_is_unique_by_idempotency_key() {
    let db = test_db().await;
    let job_id = repo::create_job(&db, 1, 2, "default", "source", "parent")
        .await
        .unwrap();
    let key = idempotency_key(ScopeType::Job, &job_id, "src", "parent", 1);

    for _ in 0..2 {
        repo::plan_operation_intent(
            &db,
            NewOperationIntent {
                idempotency_key: key.clone(),
                job_id: Some(job_id.clone()),
                watch_id: None,
                operation_type: OperationType::CopyFile,
                source_item_id: Some("src".to_string()),
                destination_parent_id: Some("parent".to_string()),
                destination_item_id: None,
                request_json: "{}".to_string(),
            },
        )
        .await
        .unwrap();
    }

    let intent = repo::find_operation_intent_by_key(&db, &key)
        .await
        .unwrap()
        .unwrap();
    assert_eq!(intent.idempotency_key, key);
    assert_eq!(intent.status, "planned");

    repo::mark_operation_applied(&db, &intent.idempotency_key, "dst")
        .await
        .unwrap();
    let intent = repo::find_operation_intent_by_key(&db, &intent.idempotency_key)
        .await
        .unwrap()
        .unwrap();
    assert_eq!(intent.status, "applied");
    assert_eq!(intent.destination_item_id.as_deref(), Some("dst"));
}

#[tokio::test]
async fn operation_intent_tracks_executing_attempts() {
    let db = test_db().await;
    let job_id = repo::create_job(&db, 1, 2, "default", "source", "parent")
        .await
        .unwrap();
    let key = idempotency_key(ScopeType::Job, &job_id, "src", "parent", 1);
    repo::plan_operation_intent(
        &db,
        NewOperationIntent {
            idempotency_key: key.clone(),
            job_id: Some(job_id),
            watch_id: None,
            operation_type: OperationType::CreateFolder,
            source_item_id: Some("src".to_string()),
            destination_parent_id: Some("parent".to_string()),
            destination_item_id: None,
            request_json: "{}".to_string(),
        },
    )
    .await
    .unwrap();

    repo::mark_operation_executing(&db, &key).await.unwrap();
    let intent = repo::find_operation_intent_by_key(&db, &key)
        .await
        .unwrap()
        .unwrap();
    assert_eq!(intent.status, "executing");
    assert_eq!(intent.attempts, 1);
}

#[tokio::test]
async fn default_destination_profile_is_single_per_account() {
    let db = test_db().await;

    for id in ["folder-a", "folder-b"] {
        repo::upsert_destination_profile(
            &db,
            NewDestinationProfile {
                google_account_id: "default".to_string(),
                label: id.to_string(),
                destination_parent_id: id.to_string(),
                destination_drive_id: None,
                destination_resource_key: None,
                is_default: true,
            },
        )
        .await
        .unwrap();
    }

    let profile = repo::default_destination_profile(&db, "default")
        .await
        .unwrap()
        .unwrap();
    assert_eq!(profile.destination_parent_id, "folder-b");

    assert_eq!(
        repo::clear_default_destination(&db, "default")
            .await
            .unwrap(),
        1
    );
    assert!(
        repo::default_destination_profile(&db, "default")
            .await
            .unwrap()
            .is_none()
    );
}

#[tokio::test]
async fn callback_state_is_scoped_and_one_time() {
    let db = test_db().await;
    let state_id = repo::create_callback_state(
        &db,
        repo::NewCallbackState {
            telegram_user_id: 42,
            chat_id: 100,
            action: "clone_confirm".to_string(),
            payload: "https://drive.google.com/file/d/abc1234567890/view".to_string(),
            ttl_ms: 60_000,
        },
    )
    .await
    .unwrap();

    let wrong_user = repo::consume_callback_state(&db, &state_id, 43, 100, "clone_confirm")
        .await
        .unwrap();
    assert!(wrong_user.is_none());

    let payload = repo::consume_callback_state(&db, &state_id, 42, 100, "clone_confirm")
        .await
        .unwrap();
    assert_eq!(
        payload.as_deref(),
        Some("https://drive.google.com/file/d/abc1234567890/view")
    );

    let replay = repo::consume_callback_state(&db, &state_id, 42, 100, "clone_confirm")
        .await
        .unwrap();
    assert!(replay.is_none());
}
