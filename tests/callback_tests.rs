use gdclone_bot::{
    state::{
        db::Database,
        repo::{self, NewCallbackState, NewJob},
    },
    telegram::parse_callback_action,
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

#[test]
fn parse_callback_action_valid_and_malformed() {
    // Valid standard 3-part callbacks
    assert_eq!(
        parse_callback_action("menu:open:home"),
        Some(("menu", "open", "home"))
    );
    assert_eq!(
        parse_callback_action("job:pause:job_12345"),
        Some(("job", "pause", "job_12345"))
    );
    assert_eq!(
        parse_callback_action("wres:v:short_id:42"),
        Some(("wres", "v", "short_id:42"))
    );

    // Multiple colons preserved in 3rd element
    assert_eq!(
        parse_callback_action("wopt:pol:session_123:v"),
        Some(("wopt", "pol", "session_123:v"))
    );

    // Malformed inputs
    assert_eq!(parse_callback_action(""), None);
    assert_eq!(parse_callback_action("one"), None);
    assert_eq!(parse_callback_action("one:two"), None);

    // Empty components
    assert_eq!(parse_callback_action("::"), Some(("", "", "")));
}

#[tokio::test]
async fn authorization_guard_rejects_unauthorized_users() {
    let db = test_db().await;

    let authorized_owner_id = 123456789;
    let unauthorized_user_id = 987654321;

    db.ensure_owner(authorized_owner_id).await.unwrap();

    assert!(repo::is_authorized(&db, authorized_owner_id).await.unwrap());
    assert!(
        !repo::is_authorized(&db, unauthorized_user_id)
            .await
            .unwrap()
    );
}

#[tokio::test]
async fn stale_or_nonexistent_job_returns_not_found() {
    let db = test_db().await;
    let user_id = 100;
    db.ensure_owner(user_id).await.unwrap();

    // Querying non-existent job
    let job = repo::job_detail_for_user(&db, user_id, "nonexistent-job-id")
        .await
        .unwrap();
    assert!(job.is_none());

    // Create job then verify retrieval
    let job_id = repo::create_job_with_metadata(
        &db,
        NewJob {
            chat_id: 200,
            telegram_user_id: user_id,
            google_account_id: "default".to_string(),
            source_root_id: "src_123".to_string(),
            source_resource_key: None,
            source_drive_id: None,
            destination_parent_id: "dest_456".to_string(),
            destination_drive_id: None,
            progress_message_id: None,
            duplicate_policy: "keep_both".to_string(),
            source_name: Some("Test Folder".to_string()),
            destination_name: Some("Backup".to_string()),
        },
    )
    .await
    .unwrap();

    let job_active = repo::job_detail_for_user(&db, user_id, &job_id)
        .await
        .unwrap();
    assert!(job_active.is_some());

    // Query by unauthorized user returns None
    let foreign_query = repo::job_detail_for_user(&db, 99999, &job_id)
        .await
        .unwrap();
    assert!(foreign_query.is_none());
}

#[tokio::test]
async fn callback_state_consumed_only_once() {
    let db = test_db().await;
    let user_id = 101;
    let chat_id = 202;

    let state_id = repo::create_callback_state(
        &db,
        NewCallbackState {
            telegram_user_id: user_id,
            chat_id,
            action: "smart_link".to_string(),
            payload: "source_ref_data".to_string(),
            ttl_ms: 600_000,
        },
    )
    .await
    .unwrap();

    // First consumption succeeds
    let first = repo::consume_callback_state(&db, &state_id, user_id, chat_id, "smart_link")
        .await
        .unwrap();
    assert_eq!(first.as_deref(), Some("source_ref_data"));

    // Second consumption (stale click) returns None
    let second = repo::consume_callback_state(&db, &state_id, user_id, chat_id, "smart_link")
        .await
        .unwrap();
    assert!(second.is_none());
}
