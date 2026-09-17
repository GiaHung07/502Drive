//! Tests for the shared application-services layer
//! (`gdclone_bot::engine::services`) and the `resume` ui_request kind
//! (migration 0009).

use gdclone_bot::engine::services::job::JobService;
use gdclone_bot::engine::services::watch::{WatchPolicyKind, WatchService};
use gdclone_bot::engine::ui_requests::ResumeRequestPayload;
use gdclone_bot::state::{
    db::Database,
    repo::{self, JobStatusValue},
    ui_requests::{self, KIND_RESUME},
};

async fn test_db() -> Database {
    let db = Database::in_memory().await.unwrap();
    db.run_migrations().await.unwrap();
    // jobs.google_account_id / watch_subscriptions.google_account_id reference
    // google_accounts(id) — seed the hardcoded "default" account.
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

async fn test_watch(db: &Database, telegram_user_id: i64) -> String {
    let cursor = repo::upsert_change_cursor(db, "default", "user", None, "token")
        .await
        .unwrap();
    repo::create_watch_subscription(
        db,
        repo::NewWatchSubscription {
            google_account_id: "default".to_string(),
            cursor_id: cursor.id,
            telegram_user_id,
            chat_id: 1,
            source_root_id: "src-root".to_string(),
            source_resource_key: None,
            source_drive_id: None,
            destination_root_id: "dst-root".to_string(),
            destination_drive_id: None,
            content_update_policy: "versioned_copy".to_string(),
            deletion_policy: "preserve_destination".to_string(),
            move_out_policy: "detach".to_string(),
            exclude_globs: "[]".to_string(),
            baseline_sequence: 0,
        },
    )
    .await
    .unwrap()
}

// ── JobService::resolve_user_input ───────────────────────────────────────────

#[tokio::test]
async fn resolve_user_input_empty_uses_latest_active_job() {
    let db = test_db().await;
    let job_id = repo::create_job(&db, 1, 42, "default", "src", "dst")
        .await
        .unwrap();

    let job = JobService::resolve_user_input(&db, 42, "").await.unwrap();
    assert_eq!(job.id, job_id);

    // Same via whitespace-only input.
    let job = JobService::resolve_user_input(&db, 42, "   ")
        .await
        .unwrap();
    assert_eq!(job.id, job_id);

    // Once the job is terminal, empty input no longer resolves.
    repo::update_job_status(&db, &job_id, JobStatusValue::Completed, None)
        .await
        .unwrap();
    let err = JobService::resolve_user_input(&db, 42, "")
        .await
        .unwrap_err();
    assert_eq!(
        err,
        gdclone_bot::engine::services::JobResolveError::NoActiveJob
    );
}

#[tokio::test]
async fn resolve_user_input_prefix_ambiguity_and_not_found() {
    let db = test_db().await;

    // UUID ids: 17 jobs guarantee two share their first hex character
    // (pigeonhole over 16 hex digits).
    let mut ids = Vec::new();
    for _ in 0..17 {
        ids.push(
            repo::create_job(&db, 1, 42, "default", "src", "dst")
                .await
                .unwrap(),
        );
    }
    let mut ambiguous_prefix = None;
    for c in "0123456789abcdef".chars() {
        let matching: Vec<&String> = ids.iter().filter(|id| id.starts_with(c)).collect();
        if matching.len() >= 2 {
            ambiguous_prefix = Some(c.to_string());
            break;
        }
    }
    let prefix = ambiguous_prefix.expect("17 uuids must contain a shared first hex char");

    let err = JobService::resolve_user_input(&db, 42, &prefix)
        .await
        .unwrap_err();
    assert_eq!(
        err,
        gdclone_bot::engine::services::JobResolveError::Ambiguous
    );

    // Full id still resolves.
    let job = JobService::resolve_user_input(&db, 42, &ids[0])
        .await
        .unwrap();
    assert_eq!(job.id, ids[0]);

    // Non-hex garbage matches nothing.
    let err = JobService::resolve_user_input(&db, 42, "zzzz-not-a-job")
        .await
        .unwrap_err();
    assert_eq!(
        err,
        gdclone_bot::engine::services::JobResolveError::NotFound
    );
}

#[tokio::test]
async fn resolve_user_input_is_user_scoped() {
    let db = test_db().await;
    let job_id = repo::create_job(&db, 1, 42, "default", "src", "dst")
        .await
        .unwrap();

    let err = JobService::resolve_user_input(&db, 7, &job_id)
        .await
        .unwrap_err();
    assert_eq!(
        err,
        gdclone_bot::engine::services::JobResolveError::NotFound
    );
}

// ── JobService pause / resume / cancel state machine ─────────────────────────

#[tokio::test]
async fn pause_then_resume_follows_the_repo_state_machine() {
    let db = test_db().await;
    let job_id = repo::create_job(&db, 1, 42, "default", "src", "dst")
        .await
        .unwrap();

    // Pause only applies to discovering|running|recovering — a queued job is
    // not pausable (repo semantics).
    assert!(!JobService::pause(&db, 42, &job_id).await.unwrap());
    repo::update_job_status(&db, &job_id, JobStatusValue::Running, None)
        .await
        .unwrap();

    // Pause: running → pausing (cooperative handshake, never straight to
    // 'paused').
    assert!(JobService::pause(&db, 42, &job_id).await.unwrap());
    assert_eq!(
        repo::job_status(&db, &job_id).await.unwrap().as_deref(),
        Some("pausing")
    );

    // Resume only works from 'paused'.
    assert!(!JobService::resume(&db, 42, &job_id).await.unwrap());

    repo::update_job_status(&db, &job_id, JobStatusValue::Paused, None)
        .await
        .unwrap();
    assert!(JobService::resume(&db, 42, &job_id).await.unwrap());
    assert_eq!(
        repo::job_status(&db, &job_id).await.unwrap().as_deref(),
        Some("recovering")
    );
}

#[tokio::test]
async fn cancel_running_job_uses_cooperative_cancelling() {
    let db = test_db().await;

    // Running job → 'cancelling' (cooperative stop), never 'cancelled'.
    let running = repo::create_job(&db, 1, 42, "default", "src", "dst")
        .await
        .unwrap();
    repo::update_job_status(&db, &running, JobStatusValue::Running, None)
        .await
        .unwrap();
    assert!(JobService::cancel(&db, 42, &running).await.unwrap());
    assert_eq!(
        repo::job_status(&db, &running).await.unwrap().as_deref(),
        Some("cancelling")
    );

    // Queued job → 'cancelled' directly (nothing is executing).
    let queued = repo::create_job(&db, 1, 42, "default", "src", "dst")
        .await
        .unwrap();
    assert!(JobService::cancel(&db, 42, &queued).await.unwrap());
    assert_eq!(
        repo::job_status(&db, &queued).await.unwrap().as_deref(),
        Some("cancelled")
    );

    // Terminal jobs are never touched.
    let done = repo::create_job(&db, 1, 42, "default", "src", "dst")
        .await
        .unwrap();
    repo::update_job_status(&db, &done, JobStatusValue::Completed, None)
        .await
        .unwrap();
    assert!(!JobService::cancel(&db, 42, &done).await.unwrap());
    assert_eq!(
        repo::job_status(&db, &done).await.unwrap().as_deref(),
        Some("completed")
    );
}

// ── JobService::progress ─────────────────────────────────────────────────────

#[test]
fn progress_derives_rate_eta_and_percentage() {
    let now = 1_700_000_000_000_i64;
    let base = repo::JobDetail {
        id: "job-1".to_string(),
        kind: "one_shot".to_string(),
        status: "running".to_string(),
        source_root_id: "src".to_string(),
        destination_parent_id: "dst".to_string(),
        total_discovered: 40,
        completed_items: 10,
        failed_items: 0,
        skipped_items: 0,
        error_summary: None,
        created_at_ms: now - 10_000,
        updated_at_ms: now,
    };

    let progress = JobService::progress(&base, now);
    assert_eq!(progress.elapsed_seconds, 10);
    assert!((progress.items_per_second.unwrap() - 1.0).abs() < 1e-9);
    assert_eq!(progress.eta_seconds, Some(30));
    assert_eq!(progress.progress_pct, Some(25.0));

    // While discovering, the total is unknown → no ETA / percentage.
    let mut discovering = base.clone();
    discovering.status = "discovering".to_string();
    let progress = JobService::progress(&discovering, now);
    assert!(progress.eta_seconds.is_none());
    assert!(progress.progress_pct.is_none());
    assert_eq!(progress.items_per_second, Some(1.0));
}

// ── WatchService::set_policy ─────────────────────────────────────────────────

#[tokio::test]
async fn set_policy_accepts_valid_values_for_all_kinds() {
    let db = test_db().await;
    let watch_id = test_watch(&db, 42).await;

    assert!(
        WatchService::set_policy(
            &db,
            42,
            &watch_id,
            WatchPolicyKind::ContentUpdate,
            "replace_copy"
        )
        .await
        .unwrap()
    );
    assert!(
        WatchService::set_policy(
            &db,
            42,
            &watch_id,
            WatchPolicyKind::Deletion,
            "manual_confirmation"
        )
        .await
        .unwrap()
    );
    assert!(
        WatchService::set_policy(
            &db,
            42,
            &watch_id,
            WatchPolicyKind::MoveOut,
            "keep_following"
        )
        .await
        .unwrap()
    );

    let sub = repo::watch_for_user(&db, 42, &watch_id)
        .await
        .unwrap()
        .unwrap();
    assert_eq!(sub.content_update_policy, "replace_copy");
    assert_eq!(sub.deletion_policy, "manual_confirmation");
    assert_eq!(sub.move_out_policy, "keep_following");
}

#[tokio::test]
async fn set_policy_rejects_invalid_values_for_all_kinds() {
    let db = test_db().await;
    let watch_id = test_watch(&db, 42).await;

    let invalid = [
        (WatchPolicyKind::ContentUpdate, "active"),
        (WatchPolicyKind::ContentUpdate, "preserve_destination"),
        (WatchPolicyKind::Deletion, "detach"),
        (WatchPolicyKind::Deletion, "versioned_copy"),
        (WatchPolicyKind::MoveOut, "manual_confirmation"),
        (WatchPolicyKind::MoveOut, "replace_copy"),
    ];
    for (kind, value) in invalid {
        let err = WatchService::set_policy(&db, 42, &watch_id, kind, value)
            .await
            .unwrap_err();
        assert!(
            err.to_string()
                .contains(&format!("invalid {}", kind.as_str())),
            "unexpected error for {kind:?} = {value}: {err}"
        );
    }

    // Unknown kind strings do not parse at all.
    assert_eq!(WatchPolicyKind::parse("bogus"), None);
    assert_eq!(
        WatchPolicyKind::parse("content_update"),
        Some(WatchPolicyKind::ContentUpdate)
    );

    // Values are unchanged after all the rejections.
    let sub = repo::watch_for_user(&db, 42, &watch_id)
        .await
        .unwrap()
        .unwrap();
    assert_eq!(sub.content_update_policy, "versioned_copy");
    assert_eq!(sub.deletion_policy, "preserve_destination");
    assert_eq!(sub.move_out_policy, "detach");
}

#[tokio::test]
async fn pause_resume_stop_guard_watch_transitions() {
    let db = test_db().await;
    let watch_id = test_watch(&db, 42).await;

    // Newly created watches are 'initializing' — not pausable.
    assert!(!WatchService::pause(&db, 42, &watch_id).await.unwrap());

    repo::update_watch_status(&db, &watch_id, "active")
        .await
        .unwrap();
    assert!(WatchService::pause(&db, 42, &watch_id).await.unwrap());
    assert_eq!(
        repo::watch_for_user(&db, 42, &watch_id)
            .await
            .unwrap()
            .unwrap()
            .status,
        "paused"
    );

    // Resume from paused with an empty backlog → catching_up.
    let resumed = WatchService::resume(&db, 42, &watch_id, 100).await.unwrap();
    assert_eq!(resumed, repo::WatchResumeResult::Resumed);

    // Stop works from any status; repeat stops report false.
    assert!(WatchService::stop(&db, 42, &watch_id).await.unwrap());
    assert!(!WatchService::stop(&db, 42, &watch_id).await.unwrap());
}

#[tokio::test]
async fn set_exclude_globs_normalizes_and_persists() {
    let db = test_db().await;
    let watch_id = test_watch(&db, 42).await;

    WatchService::set_exclude_globs(
        &db,
        42,
        &watch_id,
        &["*.tmp".to_string(), "~$*".to_string()],
    )
    .await
    .unwrap();

    let sub = repo::watch_for_user(&db, 42, &watch_id)
        .await
        .unwrap()
        .unwrap();
    let globs: Vec<String> = serde_json::from_str(&sub.exclude_globs).unwrap();
    assert_eq!(globs, vec!["*.tmp".to_string(), "~$*".to_string()]);

    // An invalid glob is rejected before any write.
    let err = WatchService::set_exclude_globs(&db, 42, &watch_id, &["has space".to_string()])
        .await
        .unwrap_err();
    assert!(!err.to_string().is_empty());
}

// ── ui_requests 'resume' round trip ──────────────────────────────────────────

#[tokio::test]
async fn ui_requests_resume_kind_round_trip() {
    let db = test_db().await;
    // GUI-created jobs are owned by telegram_user_id 0.
    let job_id = repo::create_job(&db, 0, 0, "default", "src", "dst")
        .await
        .unwrap();
    repo::update_job_status(&db, &job_id, JobStatusValue::Paused, None)
        .await
        .unwrap();

    // Migration 0009 widened the kind CHECK to allow 'resume'.
    let payload = serde_json::to_string(&ResumeRequestPayload {
        job_id: job_id.clone(),
    })
    .unwrap();
    let request_id = ui_requests::enqueue(&db, KIND_RESUME, &payload, Some("gui"))
        .await
        .unwrap();

    let request = ui_requests::get(&db, &request_id)
        .await
        .unwrap()
        .expect("request row");
    assert_eq!(request.kind, "resume");
    assert_eq!(request.status, "pending");

    // The daemon consumer claims the row atomically and performs the
    // transition; simulate the accepted half of the round trip here.
    assert!(ui_requests::pending(&db).await.unwrap().len() == 1);
    assert!(
        ui_requests::decide(&db, &request_id, true, "resume accepted")
            .await
            .unwrap()
    );
    let request = ui_requests::get(&db, &request_id).await.unwrap().unwrap();
    assert_eq!(request.status, "accepted");
    assert_eq!(request.note.as_deref(), Some("resume accepted"));

    // The transition itself is exactly JobService::resume, run in-daemon.
    assert!(JobService::resume(&db, 0, &job_id).await.unwrap());
    assert_eq!(
        repo::job_status(&db, &job_id).await.unwrap().as_deref(),
        Some("recovering")
    );
}
