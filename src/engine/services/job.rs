//! Job control services — the single entry point for user-facing job state
//! transitions, job-id resolution from user input, and progress derivation.
//!
//! All mutating calls scope by the acting telegram user id
//! (`actor_user_id`) exactly like [`crate::state::repo`]. GUI callers resolve
//! the actor before calling (owner row / `owner_telegram_id`, or user `0` for
//! GUI-created jobs); telegram callers pass `message.from.id`.

use crate::config::AppConfig;
use crate::drive::client::DriveClient;
use crate::engine::recovery;
use crate::state::db::Database;
use crate::state::repo::{self, JobDetail, RetryFailedSummary};
use tokio::task::JoinHandle;

/// Namespace over the repo job state machines.
pub struct JobService;

impl JobService {
    /// Request a pause: `discovering|running|recovering → 'pausing'`.
    ///
    /// This is the cooperative handshake the engine's `check_job_control`
    /// expects — callers must never write `'paused'` directly.
    pub async fn pause(db: &Database, actor_user_id: i64, job_id: &str) -> anyhow::Result<bool> {
        repo::pause_job_for_user(db, actor_user_id, job_id).await
    }

    /// Resume a paused job: `paused → 'recovering'`.
    ///
    /// This is **only the repo transition**. The recovery worker that actually
    /// drives the clone engine must run in the daemon process, so:
    ///
    /// * in-daemon callers (telegram handlers, the `ui_requests` consumer)
    ///   follow this with [`JobService::spawn_resume_worker`] (or use the
    ///   `resume` ui_request processor, which does both);
    /// * out-of-process callers (the GUI) must enqueue a `resume` ui_request
    ///   instead — writing `'recovering'` from the GUI process would not start
    ///   any work.
    pub async fn resume(db: &Database, actor_user_id: i64, job_id: &str) -> anyhow::Result<bool> {
        repo::resume_job_for_user(db, actor_user_id, job_id).await
    }

    /// Spawn the daemon-side resume worker (`engine::recovery`). Only valid
    /// inside the daemon process; the GUI must go through the ui_requests
    /// queue instead.
    pub fn spawn_resume_worker(
        config: AppConfig,
        db: Database,
        drive: DriveClient,
    ) -> JoinHandle<()> {
        recovery::spawn_startup_resume_worker(config, db, drive)
    }

    /// Cancel a job with the repo's two-step guarded semantics:
    ///
    /// * `queued|paused → 'cancelled'` (nothing is executing);
    /// * `discovering|running|pausing|recovering → 'cancelling'`
    ///   (cooperative stop — the engine acknowledges and finishes the write).
    ///
    /// Terminal statuses are never touched.
    pub async fn cancel(db: &Database, actor_user_id: i64, job_id: &str) -> anyhow::Result<bool> {
        repo::cancel_job_for_user(db, actor_user_id, job_id).await
    }

    /// Requeue failed items of a `paused|completed|partially_completed|failed`
    /// job (`→ 'recovering'`). Returns `None` when the job is not retryable or
    /// has no failed items. Like [`JobService::resume`], the caller decides
    /// whether to spawn the resume worker (in-daemon) or enqueue a request
    /// (GUI).
    pub async fn retry(
        db: &Database,
        actor_user_id: i64,
        job_id: &str,
    ) -> anyhow::Result<Option<RetryFailedSummary>> {
        repo::retry_failed_job_for_user(db, actor_user_id, job_id).await
    }

    /// Resolve a user-supplied job identifier:
    ///
    /// * empty input → the user's most recent active job;
    /// * full id → direct lookup (user-scoped);
    /// * otherwise → 8-char-style prefix match, ambiguous when more than one
    ///   job matches.
    ///
    /// Purely a lookup — never mutates state.
    pub async fn resolve_user_input(
        db: &Database,
        actor_user_id: i64,
        input: &str,
    ) -> Result<JobDetail, JobResolveError> {
        let job_id = input.trim();
        if job_id.is_empty() {
            let latest = repo::list_active_jobs_for_user(db, actor_user_id, 1)
                .await
                .map_err(JobResolveError::Db)?
                .into_iter()
                .next();
            let Some(job) = latest else {
                return Err(JobResolveError::NoActiveJob);
            };
            return repo::job_detail_for_user(db, actor_user_id, &job.id)
                .await
                .map_err(JobResolveError::Db)?
                .ok_or(JobResolveError::ActiveJobMissing);
        }

        if let Some(job) = repo::job_detail_for_user(db, actor_user_id, job_id)
            .await
            .map_err(JobResolveError::Db)?
        {
            return Ok(job);
        }

        let matches = repo::job_details_for_user_prefix(db, actor_user_id, job_id, 2)
            .await
            .map_err(JobResolveError::Db)?;
        match matches.as_slice() {
            [job] => Ok(job.clone()),
            [] => Err(JobResolveError::NotFound),
            _ => Err(JobResolveError::Ambiguous),
        }
    }

    /// Derive progress metrics from [`JobDetail`] counts and timestamps.
    ///
    /// Mirrors the telegram progress renderer's math (rate = done / elapsed,
    /// ETA = remaining / rate) without depending on the telegram module.
    /// Elapsed time is measured from `created_at_ms` to `now_ms`.
    pub fn progress(job: &JobDetail, now_ms: i64) -> JobProgress {
        let elapsed_ms = (now_ms - job.created_at_ms).max(0);
        let elapsed_seconds = (elapsed_ms / 1000) as u64;
        let done = job.completed_items.max(0) as u64;

        // The total is only meaningful once discovery has produced rows; for
        // early statuses the total stays unknown (matches the telegram view).
        let total = if matches!(
            job.status.as_str(),
            "running" | "completed" | "partially_completed" | "failed"
        ) && job.total_discovered > 0
        {
            Some(job.total_discovered as u64)
        } else {
            None
        };

        let items_per_second = (elapsed_seconds > 0).then(|| done as f64 / elapsed_seconds as f64);
        let eta_seconds = total.and_then(|t| estimate_eta_secs(done, t, elapsed_seconds));
        let progress_pct = total
            .filter(|t| *t > 0)
            .map(|t| (done as f64 / t as f64 * 100.0).clamp(0.0, 100.0));

        JobProgress {
            elapsed_seconds,
            items_per_second,
            eta_seconds,
            progress_pct,
        }
    }
}

/// Why [`JobService::resolve_user_input`] could not resolve a job.
#[derive(Debug)]
pub enum JobResolveError {
    /// Database error while resolving.
    Db(anyhow::Error),
    /// Empty input and the user has no active job.
    NoActiveJob,
    /// Empty input resolved to the latest active job but its row vanished.
    ActiveJobMissing,
    /// No job with that id or prefix belongs to the user.
    NotFound,
    /// More than one job matches the prefix.
    Ambiguous,
}

impl std::fmt::Display for JobResolveError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            JobResolveError::Db(err) => write!(f, "{err:#}"),
            JobResolveError::NoActiveJob => write!(f, "no active job"),
            JobResolveError::ActiveJobMissing => write!(f, "the active job no longer exists"),
            JobResolveError::NotFound => write!(f, "no job found for your account"),
            JobResolveError::Ambiguous => {
                write!(f, "more than one job matches that prefix")
            }
        }
    }
}

impl std::error::Error for JobResolveError {}

/// Variant-level equality: the `Db` payload is intentionally not compared.
impl PartialEq for JobResolveError {
    fn eq(&self, other: &Self) -> bool {
        match (self, other) {
            (Self::Db(_), Self::Db(_)) => true,
            (Self::NoActiveJob, Self::NoActiveJob) => true,
            (Self::ActiveJobMissing, Self::ActiveJobMissing) => true,
            (Self::NotFound, Self::NotFound) => true,
            (Self::Ambiguous, Self::Ambiguous) => true,
            _ => false,
        }
    }
}

impl Eq for JobResolveError {}

/// Derived progress metrics for a job (see [`JobService::progress`]).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct JobProgress {
    pub elapsed_seconds: u64,
    /// Completed items per second, `None` before the first second elapses.
    pub items_per_second: Option<f64>,
    /// Seconds to finish the remaining items at the current rate, `None`
    /// while the total is unknown or the rate is ~0.
    pub eta_seconds: Option<u64>,
    /// 0.0–100.0, `None` while the total is unknown.
    pub progress_pct: Option<f64>,
}

/// Same math as the telegram progress renderer (`estimate_eta_secs`).
fn estimate_eta_secs(completed: u64, total: u64, elapsed_secs: u64) -> Option<u64> {
    if completed == 0 || elapsed_secs == 0 || completed >= total {
        return None;
    }
    let rate = completed as f64 / elapsed_secs as f64;
    if rate < 0.001 {
        return None;
    }
    let remaining = total.saturating_sub(completed) as f64;
    Some((remaining / rate).ceil() as u64)
}
