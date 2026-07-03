use gdclone_bot::state::{db::Database, repo};

async fn test_db() -> Database {
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
    db
}

// ── cursor upsert (idempotent) ────────────────────────────────────────────────

#[tokio::test]
async fn upsert_cursor_is_idempotent() {
    let db = test_db().await;

    let c1 = repo::upsert_change_cursor(&db, "default", "user", None, "token-A")
        .await
        .unwrap();
    let c2 = repo::upsert_change_cursor(&db, "default", "user", None, "token-B")
        .await
        .unwrap();

    // Same row — c2 must carry c1's id and token (ON CONFLICT DO NOTHING).
    assert_eq!(c1.id, c2.id);
    assert_eq!(c2.current_page_token, "token-A");
}

// ── cursor per corpus key ─────────────────────────────────────────────────────

#[tokio::test]
async fn shared_drive_cursor_is_separate() {
    let db = test_db().await;

    let user = repo::upsert_change_cursor(&db, "default", "user", None, "user-tok")
        .await
        .unwrap();
    let shared =
        repo::upsert_change_cursor(&db, "default", "shared_drive", Some("drv123"), "drv-tok")
            .await
            .unwrap();

    assert_ne!(user.id, shared.id);
    assert_eq!(user.corpus_kind, "user");
    assert_eq!(shared.corpus_kind, "shared_drive");
    assert_eq!(shared.drive_id.as_deref(), Some("drv123"));
}

// ── commit_change_page advances token ────────────────────────────────────────

#[tokio::test]
async fn commit_page_advances_token() {
    let db = test_db().await;

    let cursor = repo::upsert_change_cursor(&db, "default", "user", None, "start-tok")
        .await
        .unwrap();

    let events = vec![
        repo::NewChangeEventRow {
            cursor_id: cursor.id.clone(),
            request_page_token: "start-tok".into(),
            ordinal_in_page: 0,
            file_id: "file-001".into(),
            removed: false,
            file_json: None,
        },
        repo::NewChangeEventRow {
            cursor_id: cursor.id.clone(),
            request_page_token: "start-tok".into(),
            ordinal_in_page: 1,
            file_id: "file-002".into(),
            removed: true,
            file_json: None,
        },
    ];

    repo::commit_change_page(&db, &cursor.id, events, None, Some("next-start-tok"), 0)
        .await
        .unwrap();

    // The cursor token should have advanced.
    let cursors = repo::all_change_cursors(&db).await.unwrap();
    let updated = cursors.iter().find(|c| c.id == cursor.id).unwrap();
    assert_eq!(updated.current_page_token, "next-start-tok");
    assert_eq!(updated.consecutive_error_count, 0);
}

// ── duplicate event rows are ignored (crash-safe) ────────────────────────────

#[tokio::test]
async fn commit_page_dedupe_on_replay() {
    let db = test_db().await;

    let cursor = repo::upsert_change_cursor(&db, "default", "user", None, "tok")
        .await
        .unwrap();

    let event = repo::NewChangeEventRow {
        cursor_id: cursor.id.clone(),
        request_page_token: "tok".into(),
        ordinal_in_page: 0,
        file_id: "file-x".into(),
        removed: false,
        file_json: None,
    };

    // First commit — should succeed.
    repo::commit_change_page(&db, &cursor.id, vec![event.clone()], None, Some("tok-2"), 0)
        .await
        .unwrap();

    // Second commit with same page — should succeed via OR IGNORE.
    repo::commit_change_page(&db, &cursor.id, vec![event], None, Some("tok-2"), 0)
        .await
        .unwrap();
}

// ── error counter increments on record_cursor_error ───────────────────────────

#[tokio::test]
async fn record_cursor_error_increments() {
    let db = test_db().await;

    let cursor = repo::upsert_change_cursor(&db, "default", "user", None, "tok")
        .await
        .unwrap();
    assert_eq!(cursor.consecutive_error_count, 0);

    repo::record_cursor_error(&db, &cursor.id, 9999)
        .await
        .unwrap();
    repo::record_cursor_error(&db, &cursor.id, 9999)
        .await
        .unwrap();

    let cursors = repo::all_change_cursors(&db).await.unwrap();
    let updated = cursors.iter().find(|c| c.id == cursor.id).unwrap();
    assert_eq!(updated.consecutive_error_count, 2);
}
