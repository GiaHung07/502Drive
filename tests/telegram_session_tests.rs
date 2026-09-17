use gdclone_bot::{
    state::{
        db::Database,
        repo::{self, TelegramSession},
    },
    telegram::session::{self, SessionFlow, SessionStep},
};

async fn test_db() -> Database {
    let db = Database::in_memory().await.unwrap();
    db.run_migrations().await.unwrap();
    db
}

#[tokio::test]
async fn telegram_sessions_table_and_crud_round_trip() {
    let db = test_db().await;

    // Initially no session
    let sess = repo::get_telegram_session(&db, 100, 200).await.unwrap();
    assert!(sess.is_none());

    // Upsert a session
    let now = gdclone_bot::state::db::now_ms();
    let s = TelegramSession {
        id: "sess-1".to_string(),
        user_id: 100,
        chat_id: 200,
        flow: "clone".to_string(),
        step: "wait_source".to_string(),
        payload_json: r#"{"source_input":"test"}"#.to_string(),
        created_at_ms: now,
        expires_at_ms: now + 600_000,
    };
    repo::upsert_telegram_session(&db, s.clone()).await.unwrap();

    let fetched = repo::get_telegram_session(&db, 100, 200)
        .await
        .unwrap()
        .expect("session must exist");
    assert_eq!(fetched.id, "sess-1");
    assert_eq!(fetched.flow, "clone");
    assert_eq!(fetched.step, "wait_source");

    // Replace session for the same (user_id, chat_id)
    let s2 = TelegramSession {
        id: "sess-2".to_string(),
        user_id: 100,
        chat_id: 200,
        flow: "watch".to_string(),
        step: "wait_source".to_string(),
        payload_json: "{}".to_string(),
        created_at_ms: now + 1000,
        expires_at_ms: now + 601_000,
    };
    repo::upsert_telegram_session(&db, s2).await.unwrap();

    let fetched2 = repo::get_telegram_session(&db, 100, 200)
        .await
        .unwrap()
        .expect("session must exist");
    assert_eq!(fetched2.id, "sess-2");
    assert_eq!(fetched2.flow, "watch");

    // Delete session
    repo::delete_telegram_session(&db, 100, 200).await.unwrap();
    let fetched3 = repo::get_telegram_session(&db, 100, 200).await.unwrap();
    assert!(fetched3.is_none());
}

#[tokio::test]
async fn expired_telegram_sessions_are_cleaned_up() {
    let db = test_db().await;
    let now = gdclone_bot::state::db::now_ms();

    // Insert expired session (in the past)
    repo::upsert_telegram_session(
        &db,
        TelegramSession {
            id: "expired-sess".to_string(),
            user_id: 101,
            chat_id: 201,
            flow: "clone".to_string(),
            step: "wait_source".to_string(),
            payload_json: "{}".to_string(),
            created_at_ms: now - 1000,
            expires_at_ms: now - 500,
        },
    )
    .await
    .unwrap();

    // Insert active session (in the future)
    repo::upsert_telegram_session(
        &db,
        TelegramSession {
            id: "active-sess".to_string(),
            user_id: 102,
            chat_id: 202,
            flow: "clone".to_string(),
            step: "wait_source".to_string(),
            payload_json: "{}".to_string(),
            created_at_ms: now,
            expires_at_ms: now + 600_000,
        },
    )
    .await
    .unwrap();

    let deleted = repo::delete_expired_telegram_sessions(&db).await.unwrap();
    assert_eq!(deleted, 1);

    assert!(
        repo::get_telegram_session(&db, 101, 201)
            .await
            .unwrap()
            .is_none()
    );
    assert!(
        repo::get_telegram_session(&db, 102, 202)
            .await
            .unwrap()
            .is_some()
    );
}

#[tokio::test]
async fn session_fsm_helper_start_and_clear() {
    let db = test_db().await;

    let sess = session::start_session(
        &db,
        555,
        666,
        SessionFlow::Inspect,
        SessionStep::Inspected,
        r#"{"info":"ok"}"#.to_string(),
    )
    .await
    .unwrap();

    assert_eq!(sess.flow, "inspect");
    assert_eq!(sess.step, "inspected");

    let loaded = session::get_session(&db, 555, 666)
        .await
        .unwrap()
        .expect("must be found");
    assert_eq!(loaded.id, sess.id);

    session::clear_session(&db, 555, 666).await.unwrap();
    assert!(session::get_session(&db, 555, 666).await.unwrap().is_none());
}

#[tokio::test]
async fn session_fsm_flow_transitions_and_interleaving() {
    let db = test_db().await;
    let user_id = 999;
    let chat_id = 888;

    // 1. Initial: Start WAIT_SOURCE
    let s1 = session::start_session(
        &db,
        user_id,
        chat_id,
        SessionFlow::Clone,
        SessionStep::WaitSource,
        "{}".to_string(),
    )
    .await
    .unwrap();
    assert_eq!(s1.step, "wait_source");

    // 2. User pastes URL -> advances to INSPECTED
    let s2 = session::start_session(
        &db,
        user_id,
        chat_id,
        SessionFlow::Inspect,
        SessionStep::Inspected,
        r#"{"source_input":"https://drive.google.com/folders/123"}"#.to_string(),
    )
    .await
    .unwrap();
    assert_eq!(s2.step, "inspected");
    assert_eq!(s2.flow, "inspect");

    // 3. User clicks "Change destination" -> advances to WAIT_DESTINATION with return_to_inspect
    let s3 = session::start_session(
        &db,
        user_id,
        chat_id,
        SessionFlow::SetDestination,
        SessionStep::WaitDestination,
        r#"{"return_to_inspect":true}"#.to_string(),
    )
    .await
    .unwrap();
    assert_eq!(s3.step, "wait_destination");
    assert_eq!(s3.flow, "set_destination");

    // 4. Interleaved command: user sends /cancel or another command -> clears session
    session::clear_session(&db, user_id, chat_id).await.unwrap();
    assert!(
        session::get_session(&db, user_id, chat_id)
            .await
            .unwrap()
            .is_none()
    );
}

#[test]
fn session_enums_string_round_trip() {
    for flow in [
        SessionFlow::Clone,
        SessionFlow::CloneHere,
        SessionFlow::Watch,
        SessionFlow::SetDestination,
        SessionFlow::Inspect,
        SessionFlow::Prompt("custom".to_string()),
    ] {
        let s = flow.as_str();
        let parsed = SessionFlow::from_str(s);
        assert_eq!(flow, parsed);
    }

    for step in [
        SessionStep::WaitSource,
        SessionStep::WaitDestination,
        SessionStep::WaitConfirm,
        SessionStep::Inspected,
        SessionStep::WaitInput,
    ] {
        let s = step.as_str();
        let parsed = SessionStep::from_str(s);
        assert_eq!(step, parsed);
    }
}
