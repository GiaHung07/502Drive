use crate::state::{db::Database, repo};

/// Prune raw change_events per retention policy.
pub async fn prune_old_events(db: &Database, retention_days: u64) -> anyhow::Result<usize> {
    repo::prune_consumed_events(db, retention_days).await
}
