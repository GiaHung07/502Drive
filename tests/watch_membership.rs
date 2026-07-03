use gdclone_bot::state::{db::Database, repo};

async fn test_db_with_cursor() -> (Database, String) {
    let db = Database::in_memory().await.unwrap();
    db.run_migrations().await.unwrap();
    repo::upsert_google_account(
        &db,
        "default",
        Some("test@example.com"),
        vec![1u8, 2, 3],
        "test",
        r#"["https://www.googleapis.com/auth/drive"]"#,
    )
    .await
    .unwrap();
    let cursor = repo::upsert_change_cursor(&db, "default", "user", None, "tok")
        .await
        .unwrap();
    (db, cursor.id)
}

// ── create + list ─────────────────────────────────────────────────────────────

#[tokio::test]
async fn create_and_list_watch() {
    let (db, cursor_id) = test_db_with_cursor().await;

    let watch_id = repo::create_watch_subscription(
        &db,
        repo::NewWatchSubscription {
            google_account_id: "default".into(),
            cursor_id: cursor_id.clone(),
            telegram_user_id: 42,
            chat_id: 100,
            source_root_id: "src-root".into(),
            source_resource_key: None,
            source_drive_id: None,
            destination_root_id: "dst-root".into(),
            destination_drive_id: None,
            content_update_policy: "versioned_copy".into(),
            deletion_policy: "preserve_destination".into(),
            move_out_policy: "detach".into(),
            baseline_sequence: 0,
        },
    )
    .await
    .unwrap();

    let watches = repo::list_watches_for_user(&db, 42).await.unwrap();
    assert_eq!(watches.len(), 1);
    assert_eq!(watches[0].id, watch_id);
    assert_eq!(watches[0].status, "initializing");
}

// ── pause / resume state machine ──────────────────────────────────────────────

#[tokio::test]
async fn watch_pause_and_resume() {
    let (db, cursor_id) = test_db_with_cursor().await;

    let watch_id = repo::create_watch_subscription(
        &db,
        repo::NewWatchSubscription {
            google_account_id: "default".into(),
            cursor_id,
            telegram_user_id: 1,
            chat_id: 1,
            source_root_id: "src".into(),
            source_resource_key: None,
            source_drive_id: None,
            destination_root_id: "dst".into(),
            destination_drive_id: None,
            content_update_policy: "versioned_copy".into(),
            deletion_policy: "preserve_destination".into(),
            move_out_policy: "detach".into(),
            baseline_sequence: 0,
        },
    )
    .await
    .unwrap();

    // Can't pause from 'initializing'.
    let paused = repo::pause_watch_for_user(&db, 1, &watch_id).await.unwrap();
    assert!(!paused, "cannot pause from initializing");

    // Transition to active, then pause.
    repo::update_watch_status(&db, &watch_id, "active")
        .await
        .unwrap();
    let paused = repo::pause_watch_for_user(&db, 1, &watch_id).await.unwrap();
    assert!(paused, "should pause from active");

    let watches = repo::list_watches_for_user(&db, 1).await.unwrap();
    assert_eq!(watches[0].status, "paused");

    // Resume from paused → catching_up.
    let resumed = repo::resume_watch_for_user(&db, 1, &watch_id)
        .await
        .unwrap();
    assert!(resumed);
    let watches = repo::list_watches_for_user(&db, 1).await.unwrap();
    assert_eq!(watches[0].status, "catching_up");
}

// ── stop ─────────────────────────────────────────────────────────────────────

#[tokio::test]
async fn watch_stop() {
    let (db, cursor_id) = test_db_with_cursor().await;

    let watch_id = repo::create_watch_subscription(
        &db,
        repo::NewWatchSubscription {
            google_account_id: "default".into(),
            cursor_id,
            telegram_user_id: 7,
            chat_id: 7,
            source_root_id: "s".into(),
            source_resource_key: None,
            source_drive_id: None,
            destination_root_id: "d".into(),
            destination_drive_id: None,
            content_update_policy: "versioned_copy".into(),
            deletion_policy: "preserve_destination".into(),
            move_out_policy: "detach".into(),
            baseline_sequence: 0,
        },
    )
    .await
    .unwrap();

    let stopped = repo::stop_watch_for_user(&db, 7, &watch_id).await.unwrap();
    assert!(stopped);

    let watches = repo::list_watches_for_user(&db, 7).await.unwrap();
    assert_eq!(watches[0].status, "stopped");

    // Stop again — idempotent, returns false (already stopped).
    let stopped2 = repo::stop_watch_for_user(&db, 7, &watch_id).await.unwrap();
    assert!(!stopped2);
}

// ── advance consumed sequence ─────────────────────────────────────────────────

#[tokio::test]
async fn advance_consumed_sequence() {
    let (db, cursor_id) = test_db_with_cursor().await;

    let watch_id = repo::create_watch_subscription(
        &db,
        repo::NewWatchSubscription {
            google_account_id: "default".into(),
            cursor_id,
            telegram_user_id: 9,
            chat_id: 9,
            source_root_id: "s".into(),
            source_resource_key: None,
            source_drive_id: None,
            destination_root_id: "d".into(),
            destination_drive_id: None,
            content_update_policy: "versioned_copy".into(),
            deletion_policy: "preserve_destination".into(),
            move_out_policy: "detach".into(),
            baseline_sequence: 5,
        },
    )
    .await
    .unwrap();

    repo::advance_watch_consumed_sequence(&db, &watch_id, 42)
        .await
        .unwrap();

    let watches = repo::list_watches_for_user(&db, 9).await.unwrap();
    assert_eq!(watches[0].last_consumed_sequence, 42);
}

// ── per-user isolation ────────────────────────────────────────────────────────

#[tokio::test]
async fn list_watches_user_isolation() {
    let (db, cursor_id) = test_db_with_cursor().await;

    // User 10 creates a watch.
    repo::create_watch_subscription(
        &db,
        repo::NewWatchSubscription {
            google_account_id: "default".into(),
            cursor_id,
            telegram_user_id: 10,
            chat_id: 10,
            source_root_id: "s".into(),
            source_resource_key: None,
            source_drive_id: None,
            destination_root_id: "d".into(),
            destination_drive_id: None,
            content_update_policy: "versioned_copy".into(),
            deletion_policy: "preserve_destination".into(),
            move_out_policy: "detach".into(),
            baseline_sequence: 0,
        },
    )
    .await
    .unwrap();

    // User 99 should see nothing.
    let watches = repo::list_watches_for_user(&db, 99).await.unwrap();
    assert!(watches.is_empty());

    // User 10 sees their own.
    let watches = repo::list_watches_for_user(&db, 10).await.unwrap();
    assert_eq!(watches.len(), 1);
}
