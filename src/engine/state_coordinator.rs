use anyhow::{Result, bail};

use crate::{
    engine::retry::{RetryClass, backoff_delay},
    state::{
        db::{Database, now_ms},
        repo::{self, JobStatusValue},
    },
};

/// Coordinates state mutations for jobs and items, ensuring valid transitions,
/// preventing SQLite write congestion, and tracking persistent retry states.
#[derive(Clone)]
pub struct StateCoordinator {
    db: Database,
}

impl StateCoordinator {
    pub fn new(db: Database) -> Self {
        Self { db }
    }

    pub fn db(&self) -> &Database {
        &self.db
    }

    /// Validates and applies a state transition for a job.
    /// Rejects illegal transitions (e.g. from terminal states like Completed, Failed, Cancelled).
    pub async fn transition_job_status(
        &self,
        job_id: &str,
        target_status: JobStatusValue,
        error_summary: Option<&str>,
    ) -> Result<bool> {
        let current = match repo::job_status(&self.db, job_id).await? {
            Some(status) => status,
            None => bail!("job {} not found", job_id),
        };

        if !Self::is_valid_job_transition(&current, target_status.as_str()) {
            tracing::warn!(
                job_id,
                current_status = %current,
                target_status = target_status.as_str(),
                "rejected invalid job state transition"
            );
            return Ok(false);
        }

        repo::update_job_status(&self.db, job_id, target_status, error_summary).await?;
        Ok(true)
    }

    /// Pure transition validation logic.
    pub fn is_valid_job_transition(current: &str, target: &str) -> bool {
        if current == target {
            return true;
        }
        match current {
            // Terminal states cannot transition to anything
            "completed" | "failed" | "cancelled" => false,
            // Paused can transition to running, cancelling, or failed
            "paused" => matches!(target, "running" | "cancelling" | "cancelled" | "failed"),
            // Pausing can transition to paused, cancelling, or failed
            "pausing" => matches!(target, "paused" | "cancelling" | "cancelled" | "failed"),
            // Cancelling can only transition to cancelled or failed
            "cancelling" => matches!(target, "cancelled" | "failed"),
            // Queued can start discovering, running, or be cancelled/failed
            "queued" => matches!(
                target,
                "discovering" | "running" | "cancelling" | "cancelled" | "failed"
            ),
            // Discovering can transition to running, pausing, cancelling, completed, partially_completed, failed
            "discovering" => matches!(
                target,
                "running"
                    | "pausing"
                    | "paused"
                    | "cancelling"
                    | "cancelled"
                    | "completed"
                    | "partially_completed"
                    | "failed"
            ),
            // Running can transition to pausing, paused, cancelling, cancelled, completed, partially_completed, failed
            "running" => matches!(
                target,
                "pausing"
                    | "paused"
                    | "cancelling"
                    | "cancelled"
                    | "completed"
                    | "partially_completed"
                    | "failed"
            ),
            // Recovering can transition to running, cancelled, or failed
            "recovering" => matches!(target, "running" | "cancelling" | "cancelled" | "failed"),
            _ => true,
        }
    }

    /// Records item failure with classification, reason, and computed next attempt time.
    pub async fn record_item_retry_failure(
        &self,
        job_id: &str,
        source_item_id: &str,
        attempt: u32,
        error_code: &str,
        error_message: &str,
        error_reason: Option<&str>,
        retry_class: RetryClass,
        base_delay_ms: u64,
        max_delay_ms: u64,
    ) -> Result<()> {
        let next_attempt_at_ms = match retry_class {
            RetryClass::Backoff => {
                let delay = backoff_delay(
                    attempt,
                    std::time::Duration::from_millis(base_delay_ms),
                    std::time::Duration::from_millis(max_delay_ms),
                );
                Some(now_ms() + delay.as_millis() as i64)
            }
            RetryClass::LongCooldown => {
                // 24 hours cooldown for daily rate limit exceeded
                Some(now_ms() + 24 * 3600 * 1000)
            }
            RetryClass::ImmediateAuthRefresh => Some(now_ms()),
            RetryClass::FatalItem | RetryClass::FatalOperation | RetryClass::DoNotRetry => None,
        };

        repo::mark_item_failed_with_retry_info(
            &self.db,
            job_id,
            source_item_id,
            error_code,
            error_message,
            error_reason,
            Some(retry_class.as_str()),
            next_attempt_at_ms,
        )
        .await?;

        Ok(())
    }

    /// Deduplicates incoming Telegram updates atomically using SQLite
    pub async fn deduplicate_telegram_update(&self, update_id: i64) -> Result<bool> {
        repo::record_telegram_update(&self.db, update_id, None).await
    }

    /// Marks incoming Telegram update as fully processed
    pub async fn mark_telegram_update_processed(&self, update_id: i64) -> Result<()> {
        repo::mark_telegram_update_processed(&self.db, update_id).await
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_valid_job_transitions() {
        // Legal transitions
        assert!(StateCoordinator::is_valid_job_transition(
            "queued", "running"
        ));
        assert!(StateCoordinator::is_valid_job_transition(
            "queued",
            "discovering"
        ));
        assert!(StateCoordinator::is_valid_job_transition(
            "running", "paused"
        ));
        assert!(StateCoordinator::is_valid_job_transition(
            "running",
            "completed"
        ));
        assert!(StateCoordinator::is_valid_job_transition(
            "running", "failed"
        ));
        assert!(StateCoordinator::is_valid_job_transition(
            "paused", "running"
        ));
        assert!(StateCoordinator::is_valid_job_transition(
            "running",
            "cancelling"
        ));
        assert!(StateCoordinator::is_valid_job_transition(
            "cancelling",
            "cancelled"
        ));

        // Illegal transitions from terminal states
        assert!(!StateCoordinator::is_valid_job_transition(
            "completed",
            "running"
        ));
        assert!(!StateCoordinator::is_valid_job_transition(
            "failed", "running"
        ));
        assert!(!StateCoordinator::is_valid_job_transition(
            "cancelled",
            "running"
        ));
        assert!(!StateCoordinator::is_valid_job_transition(
            "completed",
            "discovering"
        ));
    }
}
