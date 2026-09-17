//! Watch control services — pause/resume/stop, the single policy validator
//! for all three policy kinds, exclude globs, and root-folder label lookup.
//!
//! Before this module the GUI and telegram each had their own policy
//! validator (and the GUI wrote status transitions with unguarded raw SQL);
//! both frontends now go through here.

use rusqlite::OptionalExtension;
use serde::Serialize;

use crate::state::db::Database;
use crate::state::repo::{self, WatchResumeResult, WatchSubscription};
use crate::watch::glob;

/// Namespace over the repo watch state machines.
pub struct WatchService;

/// The three watch policy columns, in one enum so every frontend validates
/// against the same value sets (mirrors the `watch_subscriptions` CHECK
/// constraints).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WatchPolicyKind {
    ContentUpdate,
    Deletion,
    MoveOut,
}

impl WatchPolicyKind {
    /// Parse a frontend-facing kind string (`content_update`, `deletion`,
    /// `move_out`).
    pub fn parse(kind: &str) -> Option<Self> {
        match kind {
            "content_update" => Some(Self::ContentUpdate),
            "deletion" => Some(Self::Deletion),
            "move_out" => Some(Self::MoveOut),
            _ => None,
        }
    }

    pub fn as_str(self) -> &'static str {
        match self {
            Self::ContentUpdate => "content_update",
            Self::Deletion => "deletion",
            Self::MoveOut => "move_out",
        }
    }

    /// The `watch_subscriptions` column this policy kind maps to.
    pub fn column(self) -> &'static str {
        match self {
            Self::ContentUpdate => "content_update_policy",
            Self::Deletion => "deletion_policy",
            Self::MoveOut => "move_out_policy",
        }
    }

    /// Allowed values (same sets as the table's CHECK constraints).
    pub fn allowed_values(self) -> &'static [&'static str] {
        match self {
            Self::ContentUpdate => &["versioned_copy", "replace_copy", "manual_confirmation"],
            Self::Deletion => &["preserve_destination", "manual_confirmation"],
            Self::MoveOut => &["detach", "keep_following"],
        }
    }
}

/// Human-facing folder names for a watch's source/destination roots.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize)]
pub struct WatchLabels {
    pub source: Option<String>,
    pub destination: Option<String>,
}

impl WatchService {
    /// Pause a watch: `active|catching_up|degraded → 'paused'` (guarded).
    pub async fn pause(db: &Database, actor_user_id: i64, watch_id: &str) -> anyhow::Result<bool> {
        repo::pause_watch_for_user(db, actor_user_id, watch_id).await
    }

    /// Resume a paused watch. The backlog decides the next status:
    /// within `max_backlog_events_per_watch` → `'catching_up'`, beyond it →
    /// `'needs_reconcile'`. Never writes the (nonexistent) `'active'` target
    /// directly — the dispatcher promotes `catching_up → active` itself.
    pub async fn resume(
        db: &Database,
        actor_user_id: i64,
        watch_id: &str,
        max_backlog_events_per_watch: u64,
    ) -> anyhow::Result<WatchResumeResult> {
        repo::resume_watch_for_user(db, actor_user_id, watch_id, max_backlog_events_per_watch).await
    }

    /// Stop (unwatch) a watch: any status `!= 'stopped' → 'stopped'`.
    /// The dispatcher ignores stopped watches.
    pub async fn stop(db: &Database, actor_user_id: i64, watch_id: &str) -> anyhow::Result<bool> {
        repo::stop_watch_for_user(db, actor_user_id, watch_id).await
    }

    /// The one policy validator shared by every frontend.
    pub fn validate_policy(kind: WatchPolicyKind, value: &str) -> Result<(), String> {
        if kind.allowed_values().contains(&value) {
            Ok(())
        } else {
            Err(format!(
                "invalid {} policy '{}'; expected one of: {}",
                kind.as_str(),
                value,
                kind.allowed_values().join(", ")
            ))
        }
    }

    /// Validate and set one watch policy. Returns `false` when the watch does
    /// not exist for this user.
    pub async fn set_policy(
        db: &Database,
        actor_user_id: i64,
        watch_id: &str,
        kind: WatchPolicyKind,
        value: &str,
    ) -> anyhow::Result<bool> {
        Self::validate_policy(kind, value).map_err(anyhow::Error::msg)?;
        match kind {
            WatchPolicyKind::ContentUpdate => {
                repo::set_watch_content_update_policy(db, actor_user_id, watch_id, value).await
            }
            WatchPolicyKind::Deletion => {
                repo::set_watch_deletion_policy(db, actor_user_id, watch_id, value).await
            }
            WatchPolicyKind::MoveOut => {
                repo::set_watch_move_out_policy(db, actor_user_id, watch_id, value).await
            }
        }
    }

    /// Replace a watch's exclude-glob list. Every glob is normalized through
    /// the engine's glob rules and the list is stored as a JSON string array.
    pub async fn set_exclude_globs(
        db: &Database,
        actor_user_id: i64,
        watch_id: &str,
        globs: &[String],
    ) -> anyhow::Result<bool> {
        let mut normalized = Vec::with_capacity(globs.len());
        for glob in globs {
            normalized.push(glob::normalize_glob(glob).map_err(|e| anyhow::anyhow!("{e}"))?);
        }
        let json = serde_json::to_string(&normalized)?;
        repo::set_watch_exclude_globs(db, actor_user_id, watch_id, &json).await
    }

    /// Resolve display names for a watch's source/destination roots.
    ///
    /// * source: the root folder's `source_mappings` row (falling back to any
    ///   direct child — the root itself is not always mapped);
    /// * destination: the matching `destination_profiles` label.
    ///
    /// Both are best-effort: `None` means "no stored name, show the id".
    pub async fn labels(db: &Database, watch: &WatchSubscription) -> anyhow::Result<WatchLabels> {
        let source = mapped_source_name(db, &watch.id, &watch.source_root_id).await?;
        let destination =
            destination_profile_label(db, &watch.google_account_id, &watch.destination_root_id)
                .await?;
        Ok(WatchLabels {
            source,
            destination,
        })
    }
}

async fn mapped_source_name(
    db: &Database,
    watch_id: &str,
    source_root_id: &str,
) -> anyhow::Result<Option<String>> {
    let watch_id = watch_id.to_string();
    let root_id = source_root_id.to_string();
    Ok(db
        .conn()
        .call(move |conn| {
            let name: Option<String> = conn
                .query_row(
                    "SELECT source_name FROM source_mappings
                     WHERE scope_type = 'watch' AND scope_id = ?1 AND source_item_id = ?2
                     LIMIT 1",
                    rusqlite::params![watch_id, root_id],
                    |row| row.get(0),
                )
                .optional()?;
            if name.is_some() {
                return Ok::<Option<String>, rusqlite::Error>(name);
            }
            conn.query_row(
                "SELECT source_name FROM source_mappings
                 WHERE scope_type = 'watch' AND scope_id = ?1 AND source_parent_id = ?2
                 LIMIT 1",
                rusqlite::params![watch_id, root_id],
                |row| row.get(0),
            )
            .optional()
        })
        .await?)
}

async fn destination_profile_label(
    db: &Database,
    google_account_id: &str,
    destination_root_id: &str,
) -> anyhow::Result<Option<String>> {
    let google_account_id = google_account_id.to_string();
    let destination_root_id = destination_root_id.to_string();
    Ok(db
        .conn()
        .call(move |conn| {
            conn.query_row(
                "SELECT label FROM destination_profiles
                 WHERE google_account_id = ?1 AND destination_parent_id = ?2
                 LIMIT 1",
                rusqlite::params![google_account_id, destination_root_id],
                |row| row.get(0),
            )
            .optional()
        })
        .await?)
}
