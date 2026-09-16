//! Tests for the UI request queue (`ui_requests`, migration 0008) and the
//! shared watch-summary cursor join used by the GUI's `list_watches`.

use gdclone_bot::state::{
    db::Database,
    repo,
    ui_requests::{self, KIND_CLONE, KIND_RETRY, KIND_WATCH},
};

async fn test_db() -> Database {
    let db = Database::in_memory().await.unwrap();
    db.run_migrations().await.unwrap();
    db
}

// ── migration ────────────────────────────────────────────────────────────────

#[tokio::test]
async fn ui_requests_table_exists_after_migrations() {
    let db = test_db().await;
    assert!(
        db.table_exists("ui_requests").await.unwrap(),
        "migration 0008 must create ui_requests"
    );
    assert!(
        !db.table_exists("no_such_table").await.unwrap(),
        "table_exists must not report phantom tables"
    );
}

// ── insert / get round trip ──────────────────────────────────────────────────

#[tokio::test]
async fn insert_and_get_round_trip() {
    let db = test_db().await;
    let id = ui_requests::enqueue(&db, KIND_CLONE, r#"{"source_url":"x"}"#, Some("gui"))
        .await
        .unwrap();

    let request = ui_requests::get(&db, &id)
        .await
        .unwrap()
        .expect("request row");

    assert_eq!(request.kind, "clone");
    assert_eq!(request.status, "pending");
    assert_eq!(request.requested_by.as_deref(), Some("gui"));
    assert_eq!(request.payload_json, r#"{"source_url":"x"}"#);
    assert!(request.decided_at_ms.is_none());
    assert!(request.note.is_none());
}

#[tokio::test]
async fn get_unknown_id_returns_none() {
    let db = test_db().await;
    let found = ui_requests::get(&db, "no-such-id").await.unwrap();
    assert!(found.is_none());
}

// ── consume / decide ─────────────────────────────────────────────────────────

#[tokio::test]
async fn pending_returns_only_pending_oldest_first() {
    let db = test_db().await;
    for kind in [KIND_WATCH, KIND_CLONE, KIND_RETRY] {
        ui_requests::enqueue(&db, kind, "{}", None).await.unwrap();
    }
    // Decide the first one — it must disappear from `pending`.
    let queue = ui_requests::pending(&db).await.unwrap();
    assert_eq!(queue.len(), 3);
    ui_requests::decide(&db, &queue[0].id, true, "ok")
        .await
        .unwrap();

    let queue = ui_requests::pending(&db).await.unwrap();
    assert_eq!(queue.len(), 2);
    assert!(queue.iter().all(|r| r.status == "pending"));
}

#[tokio::test]
async fn decide_is_atomic_and_single_shot() {
    let db = test_db().await;
    let id = ui_requests::enqueue(&db, KIND_CLONE, "{}", None)
        .await
        .unwrap();

    // First decide wins.
    let claimed = ui_requests::decide(&db, &id, true, "accepted")
        .await
        .unwrap();
    assert!(claimed);

    // A second (concurrent) consumer must lose the claim.
    let second = ui_requests::decide(&db, &id, false, "late rejection")
        .await
        .unwrap();
    assert!(!second);

    let request = ui_requests::get(&db, &id).await.unwrap().unwrap();
    assert_eq!(request.status, "accepted");
    assert_eq!(request.note.as_deref(), Some("accepted"));
    assert!(request.decided_at_ms.is_some());
}

#[tokio::test]
async fn reject_records_note() {
    let db = test_db().await;
    let id = ui_requests::enqueue(&db, KIND_WATCH, "{}", None)
        .await
        .unwrap();

    ui_requests::decide(&db, &id, false, "no default destination")
        .await
        .unwrap();
    let request = ui_requests::get(&db, &id).await.unwrap().unwrap();
    assert_eq!(request.status, "rejected");
    assert_eq!(request.note.as_deref(), Some("no default destination"));
}

#[tokio::test]
async fn set_note_updates_note_without_touching_status() {
    let db = test_db().await;
    let id = ui_requests::enqueue(&db, KIND_RETRY, "{}", None)
        .await
        .unwrap();
    ui_requests::decide(&db, &id, true, "accepted; clone queued")
        .await
        .unwrap();

    ui_requests::set_note(&db, &id, "job abc-123 — completed")
        .await
        .unwrap();
    let request = ui_requests::get(&db, &id).await.unwrap().unwrap();
    assert_eq!(request.status, "accepted");
    assert_eq!(request.note.as_deref(), Some("job abc-123 — completed"));
}

// ── watch summary cursor join ────────────────────────────────────────────────

#[tokio::test]
async fn watch_summaries_backlog_uses_cursor_not_baseline() {
    let db = test_db().await;
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

    // Baseline 10 at creation; cursor advances to 25 via a committed page.
    let cursor = repo::upsert_change_cursor(&db, "default", "user", None, "tok")
        .await
        .unwrap();
    let watch_id = repo::create_watch_subscription(
        &db,
        repo::NewWatchSubscription {
            google_account_id: "default".to_string(),
            cursor_id: cursor.id.clone(),
            telegram_user_id: 0,
            chat_id: 0,
            source_root_id: "src".to_string(),
            source_resource_key: None,
            source_drive_id: None,
            destination_root_id: "dst".to_string(),
            destination_drive_id: None,
            content_update_policy: "versioned_copy".to_string(),
            deletion_policy: "preserve_destination".to_string(),
            move_out_policy: "detach".to_string(),
            exclude_globs: r#"["*.tmp"]"#.to_string(),
            baseline_sequence: 10,
        },
    )
    .await
    .unwrap();

    let events: Vec<repo::NewChangeEventRow> = (0..25)
        .map(|i| repo::NewChangeEventRow {
            cursor_id: cursor.id.clone(),
            request_page_token: "tok".to_string(),
            ordinal_in_page: i,
            file_id: format!("f{i}"),
            removed: false,
            file_json: None,
        })
        .collect();
    repo::commit_change_page(&db, &cursor.id, events, None, Some("tok2"), 1000)
        .await
        .unwrap();

    // Catch-up consumed everything (last_consumed = 25): the old
    // baseline-based formula would report backlog 15 forever.
    repo::advance_watch_consumed_sequence(&db, &watch_id, 25)
        .await
        .unwrap();

    let summaries = repo::watch_summaries(&db).await.unwrap();
    assert_eq!(summaries.len(), 1);
    let s = &summaries[0];
    assert_eq!(s.id, watch_id);
    assert_eq!(s.cursor_last_event_sequence, 25);
    assert_eq!(s.last_consumed_sequence, 25);
    assert_eq!(
        s.backlog_count, 0,
        "backlog must come from the cursor, not the baseline"
    );
    assert_eq!(s.exclude_globs, r#"["*.tmp"]"#);
    assert_eq!(s.baseline_sequence, 10);
}

#[tokio::test]
async fn watch_summaries_counts_pending_backlog() {
    let db = test_db().await;
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
    let watch_id = repo::create_watch_subscription(
        &db,
        repo::NewWatchSubscription {
            google_account_id: "default".to_string(),
            cursor_id: cursor.id.clone(),
            telegram_user_id: 0,
            chat_id: 0,
            source_root_id: "src".to_string(),
            source_resource_key: None,
            source_drive_id: None,
            destination_root_id: "dst".to_string(),
            destination_drive_id: None,
            content_update_policy: "versioned_copy".to_string(),
            deletion_policy: "preserve_destination".to_string(),
            move_out_policy: "detach".to_string(),
            exclude_globs: "[]".to_string(),
            baseline_sequence: 0,
        },
    )
    .await
    .unwrap();

    let events: Vec<repo::NewChangeEventRow> = (0..7)
        .map(|i| repo::NewChangeEventRow {
            cursor_id: cursor.id.clone(),
            request_page_token: "tok".to_string(),
            ordinal_in_page: i,
            file_id: format!("f{i}"),
            removed: false,
            file_json: None,
        })
        .collect();
    repo::commit_change_page(&db, &cursor.id, events, None, Some("tok2"), 1000)
        .await
        .unwrap();
    // Consumed 3 of 7.
    repo::advance_watch_consumed_sequence(&db, &watch_id, 3)
        .await
        .unwrap();

    let summaries = repo::watch_summaries(&db).await.unwrap();
    assert_eq!(summaries[0].backlog_count, 4);
}
