use tokio::task::JoinHandle;
use tracing::{error, info, warn};

use crate::{
    config::AppConfig,
    drive::client::DriveClient,
    engine::copy::{CloneService, JobControlStop},
    state::{
        db::Database,
        repo::{self, JobStatusValue, StartupRecoverySummary},
    },
    watch::run_initial_clone,
};

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct ResumeSummary {
    pub attempted_jobs: usize,
    pub completed_jobs: usize,
    pub failed_jobs: usize,
}

pub async fn recover_on_startup(db: &Database) -> anyhow::Result<StartupRecoverySummary> {
    let summary = repo::recover_interrupted_state(db).await?;
    if summary != StartupRecoverySummary::default() {
        info!(
            jobs_marked_recovering = summary.jobs_marked_recovering,
            traversal_folders_requeued = summary.traversal_folders_requeued,
            operation_intents_replanned = summary.operation_intents_replanned,
            job_items_requeued = summary.job_items_requeued,
            "startup recovery pass completed"
        );
    }
    Ok(summary)
}

pub fn spawn_startup_resume_worker(
    config: AppConfig,
    db: Database,
    drive: DriveClient,
) -> JoinHandle<()> {
    tokio::spawn(async move {
        match resume_startup_jobs(config, db, drive).await {
            Ok(summary) if summary != ResumeSummary::default() => {
                info!(
                    attempted_jobs = summary.attempted_jobs,
                    completed_jobs = summary.completed_jobs,
                    failed_jobs = summary.failed_jobs,
                    "startup resume worker completed"
                );
            }
            Ok(_) => {}
            Err(err) => error!(error = %err, "startup resume worker failed"),
        }
    })
}

pub async fn resume_startup_jobs(
    config: AppConfig,
    db: Database,
    drive: DriveClient,
) -> anyhow::Result<ResumeSummary> {
    let jobs = repo::resumable_one_shot_jobs(&db).await?;
    if jobs.is_empty() {
        return Ok(ResumeSummary::default());
    }

    let service = CloneService::new(config, db.clone(), drive);
    let mut summary = ResumeSummary {
        attempted_jobs: jobs.len(),
        ..ResumeSummary::default()
    };

    for job in jobs {
        match service.resume_one_shot_job(&job).await {
            Ok(outcome) => {
                summary.completed_jobs += 1;
                info!(job_id = outcome.job_id, "one-shot job resumed");
            }
            Err(err) => {
                if err.downcast_ref::<JobControlStop>().is_some() {
                    continue;
                }
                summary.failed_jobs += 1;
                let error_summary = err.to_string();
                repo::update_job_status(&db, &job.id, JobStatusValue::Failed, Some(&error_summary))
                    .await?;
                error!(
                    job_id = job.id,
                    error = %error_summary,
                    "one-shot job resume failed"
                );
            }
        }
    }

    Ok(summary)
}

/// Re-spawn `run_initial_clone` for any watch subscription still in the
/// `initializing` state. This handles the case where the bot crashed during
/// the first initial-clone run.
///
/// Called from `main.rs` after `recover_on_startup`.
pub fn recover_initializing_watches(
    config: AppConfig,
    db: Database,
    drive: DriveClient,
) -> JoinHandle<()> {
    tokio::spawn(async move {
        let watches = match repo::initializing_watches(&db).await {
            Ok(w) => w,
            Err(err) => {
                warn!(error = %err, "failed to load initializing watches for recovery");
                return;
            }
        };

        if watches.is_empty() {
            return;
        }

        info!(
            count = watches.len(),
            "recovering stuck initializing watches"
        );

        for watch in watches {
            let config2 = config.clone();
            let db2 = db.clone();
            let drive2 = drive.clone();
            let watch_id = watch.id.clone();
            tokio::spawn(async move {
                // Silent notify — no active Telegram session at startup.
                let noop = |_msg: String| {};
                if let Err(err) =
                    run_initial_clone(config2, db2, drive2, watch_id.clone(), noop).await
                {
                    warn!(
                        watch_id,
                        error = %err,
                        "startup recovery of initializing watch failed"
                    );
                } else {
                    info!(watch_id, "startup recovery of initializing watch complete");
                }
            });
        }
    })
}
