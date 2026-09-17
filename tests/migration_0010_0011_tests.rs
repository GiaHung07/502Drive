use gdclone_bot::state::{
    db::Database,
    repo::{self, NewJob, NewWatchSubscription, TelegramSession},
};

async fn test_db() -> Database {
    let db = Database::in_memory().await.unwrap();
    db.run_migrations().await.unwrap();
    repo::upsert_google_account(
        &db,
        "Test",
        Some("test@example.com"),
        vec![0; 8],
        "file",
        "{}",
    )
    .await
    .unwrap();
    db
}

#[tokio::test]
async fn migrations_0010_and_0011_columns_exist_and_round_trip() {
    let db = test_db().await;
    let user_id = 777;
    let chat_id = 111;
    db.ensure_owner(user_id).await.unwrap();

    // 1. Verify 0010 telegram_sessions table and TTL index
    let now = gdclone_bot::state::db::now_ms();
    repo::upsert_telegram_session(
        &db,
        TelegramSession {
            id: "test-sess-0010".to_string(),
            user_id,
            chat_id,
            flow: "inspect".to_string(),
            step: "inspected".to_string(),
            payload_json: r#"{"source":"ok"}"#.to_string(),
            created_at_ms: now,
            expires_at_ms: now + 60_000,
        },
    )
    .await
    .unwrap();

    let sess = repo::get_telegram_session(&db, user_id, chat_id)
        .await
        .unwrap()
        .expect("session must exist");
    assert_eq!(sess.id, "test-sess-0010");
    assert_eq!(sess.flow, "inspect");

    // 2. Verify 0011 jobs columns (source_name, destination_name)
    let job_id = repo::create_job_with_metadata(
        &db,
        NewJob {
            chat_id,
            telegram_user_id: user_id,
            google_account_id: "default".to_string(),
            source_root_id: "src_root".to_string(),
            source_resource_key: None,
            source_drive_id: None,
            destination_parent_id: "dest_parent".to_string(),
            destination_drive_id: None,
            progress_message_id: None,
            duplicate_policy: "keep_both".to_string(),
            source_name: Some("My Folder Name".to_string()),
            destination_name: Some("Backup Target Name".to_string()),
        },
    )
    .await
    .unwrap();

    let job_detail = repo::job_detail_for_user(&db, user_id, &job_id)
        .await
        .unwrap()
        .expect("job must exist");
    assert_eq!(job_detail.source_name.as_deref(), Some("My Folder Name"));
    assert_eq!(
        job_detail.destination_name.as_deref(),
        Some("Backup Target Name")
    );

    // 3. Verify 0011 watch_subscriptions columns (source_name, destination_name, last_consumed_at_ms)
    let cursor = repo::upsert_change_cursor(&db, "default", "user", None, "token")
        .await
        .unwrap();

    let watch_id = repo::create_watch_subscription(
        &db,
        NewWatchSubscription {
            google_account_id: "default".to_string(),
            cursor_id: cursor.id,
            telegram_user_id: user_id,
            chat_id,
            source_root_id: "folder_1".to_string(),
            source_resource_key: None,
            source_drive_id: None,
            destination_root_id: "folder_2".to_string(),
            destination_drive_id: None,
            content_update_policy: "versioned_copy".to_string(),
            deletion_policy: "preserve_destination".to_string(),
            move_out_policy: "detach".to_string(),
            exclude_globs: "[]".to_string(),
            baseline_sequence: 0,
            source_name: Some("Watch Source".to_string()),
            destination_name: Some("Watch Dest".to_string()),
        },
    )
    .await
    .unwrap();

    // Advance sequence: this automatically stamps last_consumed_at_ms
    repo::advance_watch_consumed_sequence(&db, &watch_id, 42)
        .await
        .unwrap();

    let watch = repo::watch_for_user_unchecked(&db, &watch_id)
        .await
        .unwrap()
        .expect("watch must exist");
    assert_eq!(watch.source_name.as_deref(), Some("Watch Source"));
    assert_eq!(watch.destination_name.as_deref(), Some("Watch Dest"));
    assert!(watch.last_consumed_at_ms.is_some());
    assert!(watch.last_consumed_at_ms.unwrap() >= now);
}
