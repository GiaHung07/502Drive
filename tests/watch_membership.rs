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
            source_name: None,
            destination_root_id: "dst-root".into(),
            destination_drive_id: None,
            destination_name: None,
            content_update_policy: "versioned_copy".into(),
            deletion_policy: "preserve_destination".into(),
            move_out_policy: "detach".into(),
            exclude_globs: "[]".to_string(),
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

#[tokio::test]
async fn watch_prefix_lookup_is_user_scoped() {
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
            source_name: None,
            destination_root_id: "dst-root".into(),
            destination_drive_id: None,
            destination_name: None,
            content_update_policy: "versioned_copy".into(),
            deletion_policy: "preserve_destination".into(),
            move_out_policy: "detach".into(),
            exclude_globs: "[]".to_string(),
            baseline_sequence: 0,
        },
    )
    .await
    .unwrap();
    repo::create_watch_subscription(
        &db,
        repo::NewWatchSubscription {
            google_account_id: "default".into(),
            cursor_id,
            telegram_user_id: 99,
            chat_id: 100,
            source_root_id: "other-src".into(),
            source_resource_key: None,
            source_drive_id: None,
            source_name: None,
            destination_root_id: "other-dst".into(),
            destination_drive_id: None,
            destination_name: None,
            content_update_policy: "versioned_copy".into(),
            deletion_policy: "preserve_destination".into(),
            move_out_policy: "detach".into(),
            exclude_globs: "[]".to_string(),
            baseline_sequence: 0,
        },
    )
    .await
    .unwrap();

    let short = &watch_id[..8];
    let matches = repo::watches_for_user_prefix(&db, 42, short, 2)
        .await
        .unwrap();
    assert_eq!(matches.len(), 1);
    assert_eq!(matches[0].id, watch_id);

    let matches = repo::watches_for_user_prefix(&db, 99, short, 2)
        .await
        .unwrap();
    assert!(matches.is_empty());
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
            source_name: None,
            destination_root_id: "dst".into(),
            destination_drive_id: None,
            destination_name: None,
            content_update_policy: "versioned_copy".into(),
            deletion_policy: "preserve_destination".into(),
            move_out_policy: "detach".into(),
            exclude_globs: "[]".to_string(),
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
    let resumed = repo::resume_watch_for_user(&db, 1, &watch_id, 100)
        .await
        .unwrap();
    assert_eq!(resumed, repo::WatchResumeResult::Resumed);
    let watches = repo::list_watches_for_user(&db, 1).await.unwrap();
    assert_eq!(watches[0].status, "catching_up");
}

#[tokio::test]
async fn watch_resume_over_backlog_limit_needs_reconcile() {
    let (db, cursor_id) = test_db_with_cursor().await;

    let watch_id = repo::create_watch_subscription(
        &db,
        repo::NewWatchSubscription {
            google_account_id: "default".into(),
            cursor_id: cursor_id.clone(),
            telegram_user_id: 1,
            chat_id: 1,
            source_root_id: "src".into(),
            source_resource_key: None,
            source_drive_id: None,
            source_name: None,
            destination_root_id: "dst".into(),
            destination_drive_id: None,
            destination_name: None,
            content_update_policy: "versioned_copy".into(),
            deletion_policy: "preserve_destination".into(),
            move_out_policy: "detach".into(),
            exclude_globs: "[]".to_string(),
            baseline_sequence: 0,
        },
    )
    .await
    .unwrap();

    repo::commit_change_page(
        &db,
        &cursor_id,
        vec![
            repo::NewChangeEventRow {
                cursor_id: cursor_id.clone(),
                request_page_token: "tok".into(),
                ordinal_in_page: 0,
                file_id: "a".into(),
                removed: false,
                file_json: None,
            },
            repo::NewChangeEventRow {
                cursor_id: cursor_id.clone(),
                request_page_token: "tok".into(),
                ordinal_in_page: 1,
                file_id: "b".into(),
                removed: false,
                file_json: None,
            },
        ],
        None,
        Some("tok2"),
        0,
    )
    .await
    .unwrap();

    repo::update_watch_status(&db, &watch_id, "active")
        .await
        .unwrap();
    assert!(repo::pause_watch_for_user(&db, 1, &watch_id).await.unwrap());

    let resumed = repo::resume_watch_for_user(&db, 1, &watch_id, 1)
        .await
        .unwrap();
    assert_eq!(
        resumed,
        repo::WatchResumeResult::NeedsReconcile { pending_events: 2 }
    );
    let watches = repo::list_watches_for_user(&db, 1).await.unwrap();
    assert_eq!(watches[0].status, "needs_reconcile");
}

#[tokio::test]
async fn paused_watch_over_backlog_limit_needs_reconcile() {
    let (db, cursor_id) = test_db_with_cursor().await;

    let watch_id = repo::create_watch_subscription(
        &db,
        repo::NewWatchSubscription {
            google_account_id: "default".into(),
            cursor_id: cursor_id.clone(),
            telegram_user_id: 1,
            chat_id: 1,
            source_root_id: "src".into(),
            source_resource_key: None,
            source_drive_id: None,
            source_name: None,
            destination_root_id: "dst".into(),
            destination_drive_id: None,
            destination_name: None,
            content_update_policy: "versioned_copy".into(),
            deletion_policy: "preserve_destination".into(),
            move_out_policy: "detach".into(),
            exclude_globs: "[]".to_string(),
            baseline_sequence: 0,
        },
    )
    .await
    .unwrap();
    repo::update_watch_status(&db, &watch_id, "active")
        .await
        .unwrap();
    assert!(repo::pause_watch_for_user(&db, 1, &watch_id).await.unwrap());

    repo::commit_change_page(
        &db,
        &cursor_id,
        vec![
            repo::NewChangeEventRow {
                cursor_id: cursor_id.clone(),
                request_page_token: "tok".into(),
                ordinal_in_page: 0,
                file_id: "a".into(),
                removed: false,
                file_json: None,
            },
            repo::NewChangeEventRow {
                cursor_id: cursor_id.clone(),
                request_page_token: "tok".into(),
                ordinal_in_page: 1,
                file_id: "b".into(),
                removed: false,
                file_json: None,
            },
        ],
        None,
        Some("tok2"),
        0,
    )
    .await
    .unwrap();

    let changed = repo::mark_paused_watches_over_backlog_limit(&db, &cursor_id, 2)
        .await
        .unwrap();
    assert_eq!(changed, 0);

    let changed = repo::mark_paused_watches_over_backlog_limit(&db, &cursor_id, 1)
        .await
        .unwrap();
    assert_eq!(changed, 1);
    let watches = repo::list_watches_for_user(&db, 1).await.unwrap();
    assert_eq!(watches[0].status, "needs_reconcile");
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
            source_name: None,
            destination_root_id: "d".into(),
            destination_drive_id: None,
            destination_name: None,
            content_update_policy: "versioned_copy".into(),
            deletion_policy: "preserve_destination".into(),
            move_out_policy: "detach".into(),
            exclude_globs: "[]".to_string(),
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
            source_name: None,
            destination_root_id: "d".into(),
            destination_drive_id: None,
            destination_name: None,
            content_update_policy: "versioned_copy".into(),
            deletion_policy: "preserve_destination".into(),
            move_out_policy: "detach".into(),
            exclude_globs: "[]".to_string(),
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

#[tokio::test]
async fn watch_backlog_tracks_cursor_gap() {
    let (db, cursor_id) = test_db_with_cursor().await;

    let watch_id = repo::create_watch_subscription(
        &db,
        repo::NewWatchSubscription {
            google_account_id: "default".into(),
            cursor_id: cursor_id.clone(),
            telegram_user_id: 9,
            chat_id: 9,
            source_root_id: "s".into(),
            source_resource_key: None,
            source_drive_id: None,
            source_name: None,
            destination_root_id: "d".into(),
            destination_drive_id: None,
            destination_name: None,
            content_update_policy: "versioned_copy".into(),
            deletion_policy: "preserve_destination".into(),
            move_out_policy: "detach".into(),
            exclude_globs: "[]".to_string(),
            baseline_sequence: 0,
        },
    )
    .await
    .unwrap();

    repo::commit_change_page(
        &db,
        &cursor_id,
        vec![
            repo::NewChangeEventRow {
                cursor_id: cursor_id.clone(),
                request_page_token: "tok".into(),
                ordinal_in_page: 0,
                file_id: "a".into(),
                removed: false,
                file_json: None,
            },
            repo::NewChangeEventRow {
                cursor_id: cursor_id.clone(),
                request_page_token: "tok".into(),
                ordinal_in_page: 1,
                file_id: "b".into(),
                removed: false,
                file_json: None,
            },
        ],
        None,
        Some("tok2"),
        0,
    )
    .await
    .unwrap();

    let backlog = repo::watch_backlog(&db, &watch_id).await.unwrap().unwrap();
    assert_eq!(backlog.pending_events, 2);

    repo::advance_watch_consumed_sequence(&db, &watch_id, 1)
        .await
        .unwrap();
    let backlog = repo::watch_backlog(&db, &watch_id).await.unwrap().unwrap();
    assert_eq!(backlog.pending_events, 1);
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
            source_name: None,
            destination_root_id: "d".into(),
            destination_drive_id: None,
            destination_name: None,
            content_update_policy: "versioned_copy".into(),
            deletion_policy: "preserve_destination".into(),
            move_out_policy: "detach".into(),
            exclude_globs: "[]".to_string(),
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

// ── exclude_globs round-trip ──────────────────────────────────────────────────

#[tokio::test]
async fn exclude_globs_round_trip() {
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
            source_name: None,
            destination_root_id: "d".into(),
            destination_drive_id: None,
            destination_name: None,
            content_update_policy: "versioned_copy".into(),
            deletion_policy: "preserve_destination".into(),
            move_out_policy: "detach".into(),
            exclude_globs: "[]".to_string(),
            baseline_sequence: 0,
        },
    )
    .await
    .unwrap();

    // Fresh subscriptions default to an empty glob list.
    let watch = repo::watch_for_user(&db, 7, &watch_id)
        .await
        .unwrap()
        .unwrap();
    assert_eq!(watch.exclude_globs, "[]");

    // Set a list and read it back through the full-row watch queries.
    let globs =
        gdclone_bot::watch::glob::serialize_glob_list(&["*.tmp".to_string(), "~$*".to_string()]);
    assert!(
        repo::set_watch_exclude_globs(&db, 7, &watch_id, &globs)
            .await
            .unwrap()
    );

    // Activate the watch so active_watches() also picks it up.
    repo::update_watch_status(&db, &watch_id, "active")
        .await
        .unwrap();

    let views = vec![
        repo::watch_for_user(&db, 7, &watch_id)
            .await
            .unwrap()
            .unwrap(),
        repo::active_watches(&db).await.unwrap()[0].clone(),
        repo::list_watches_for_user(&db, 7).await.unwrap()[0].clone(),
        repo::watch_for_user_unchecked(&db, &watch_id)
            .await
            .unwrap()
            .unwrap(),
    ];
    for fetched in views {
        assert_eq!(fetched.id, watch_id);
        assert_eq!(fetched.exclude_globs, globs);
        let parsed = gdclone_bot::watch::glob::parse_glob_list(&fetched.exclude_globs);
        assert_eq!(parsed, vec!["*.tmp".to_string(), "~$*".to_string()]);
    }

    // Other users cannot mutate the watch's filters.
    assert!(
        !repo::set_watch_exclude_globs(&db, 99, &watch_id, "[]")
            .await
            .unwrap()
    );

    // Clearing works and unknown ids report false.
    assert!(
        repo::set_watch_exclude_globs(&db, 7, &watch_id, "[]")
            .await
            .unwrap()
    );
    assert!(
        !repo::set_watch_exclude_globs(&db, 7, "no-such-watch", "[]")
            .await
            .unwrap()
    );
    let watch = repo::watch_for_user(&db, 7, &watch_id)
        .await
        .unwrap()
        .unwrap();
    assert_eq!(watch.exclude_globs, "[]");
}

#[tokio::test]
async fn test_resolve_pending_and_conflict_details() {
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
            source_name: Some("Source Folder".into()),
            destination_root_id: "dst-root".into(),
            destination_drive_id: None,
            destination_name: Some("Dest Folder".into()),
            content_update_policy: "manual_confirmation".into(),
            deletion_policy: "manual_confirmation".into(),
            move_out_policy: "detach".into(),
            exclude_globs: "[]".to_string(),
            baseline_sequence: 0,
        },
    )
    .await
    .unwrap();

    repo::update_watch_status(&db, &watch_id, "needs_reconcile")
        .await
        .unwrap();

    // Commit a change event for the cursor
    let events = vec![repo::NewChangeEventRow {
        cursor_id: cursor_id.clone(),
        request_page_token: "tok".into(),
        ordinal_in_page: 0,
        file_id: "file-xyz".into(),
        removed: false,
        file_json: Some(
            r#"{"id":"file-xyz","name":"doc.pdf","mimeType":"application/pdf","size":"1024","modifiedTime":"2026-09-17T12:00:00Z"}"#.into(),
        ),
    }];
    let seq = repo::commit_change_page(&db, &cursor_id, events, None, Some("tok-2"), 0)
        .await
        .unwrap();

    // Save mapping for file-xyz
    gdclone_bot::engine::mapping::record_mapping(
        &db,
        gdclone_bot::engine::mapping::MappingRecord {
            scope_type: "watch".to_string(),
            scope_id: watch_id.clone(),
            source_item_id: "file-xyz".to_string(),
            destination_item_id: "dest-file-xyz".to_string(),
            source_parent_id: None,
            destination_parent_id: Some("dst-root".to_string()),
            mime_type: "application/pdf".to_string(),
            source_name: "doc.pdf".to_string(),
            source_version: None,
            source_modified_time: Some("2026-09-15T10:00:00Z".to_string()),
            source_md5_checksum: None,
        },
    )
    .await
    .unwrap();

    // Upsert event application as "pending"
    repo::upsert_event_application(&db, &watch_id, seq, "content_changed", "pending")
        .await
        .unwrap();

    assert_eq!(
        repo::count_pending_confirmation_events(&db, &watch_id)
            .await
            .unwrap(),
        1
    );

    let next_pending = repo::next_pending_confirmation_for_watch(&db, &watch_id)
        .await
        .unwrap()
        .unwrap();
    assert_eq!(next_pending.sequence, seq);
    assert_eq!(next_pending.file_id, "file-xyz");
    assert_eq!(next_pending.classification, "content_changed");

    let details = gdclone_bot::watch::service::get_pending_conflict_details(&db, &watch_id)
        .await
        .unwrap()
        .unwrap();
    assert_eq!(details.watch_id, watch_id);
    assert_eq!(details.file_name, "doc.pdf");
    assert_eq!(
        details.current_dest_modified.as_deref(),
        Some("2026-09-15T10:00:00Z")
    );
    assert_eq!(details.new_source_size, Some(1024));
    assert_eq!(
        details.new_source_modified.as_deref(),
        Some("2026-09-17T12:00:00Z")
    );
    assert_eq!(details.remaining, 1);

    let config =
        gdclone_bot::config::AppConfig::load(std::path::Path::new("config.sample.toml")).unwrap();
    let drive = gdclone_bot::watch::service::drive_client_for(&config);

    // Resolve as Skip
    let outcome = gdclone_bot::watch::service::resolve_pending(
        &config,
        &db,
        &drive,
        &watch_id,
        gdclone_bot::watch::service::PendingDecision::Skip,
    )
    .await
    .unwrap()
    .unwrap();

    assert_eq!(
        outcome.decision,
        gdclone_bot::watch::service::PendingDecision::Skip
    );
    assert_eq!(outcome.remaining_pending, 0);

    // Verify watch status transitioned back to "active" because 0 pending remaining
    let watch = repo::watch_for_user_unchecked(&db, &watch_id)
        .await
        .unwrap()
        .unwrap();
    assert_eq!(watch.status, "active");
    assert_eq!(watch.last_consumed_sequence, seq);
}
