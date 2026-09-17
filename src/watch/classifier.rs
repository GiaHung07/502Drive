use serde_json::Value;
use tracing::{debug, warn};

use crate::{drive::types::DriveFile, state::repo::WatchSubscription};

/// Outcome of classifying a single change event against one watch subscription.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Classification {
    /// A file appeared in a tracked parent that we have not seen before.
    NewItem,
    /// File content changed (md5 / version differs) — dest copy needs update.
    ContentChanged,
    /// Name changed in place within the tracked subtree.
    Renamed,
    /// File moved to a different tracked parent folder (still inside our tree).
    MovedInside,
    /// File moved out of every tracked parent — dest should be detached.
    MovedOutside,
    /// File previously moved outside came back under a tracked parent.
    MovedBack,
    /// File was trashed, deleted, or access lost — `removed` flag set or
    /// `trashed == Some(true)`.
    TrashedOrRemoved,
    /// The file is not within our tracked subtree and is not relevant.
    Irrelevant,
    /// Change is within scope but the state is ambiguous (e.g. no prior
    /// fingerprint available and `removed` is false).
    Ambiguous,
}

impl Classification {
    pub fn as_str(&self) -> &'static str {
        match self {
            Classification::NewItem => "new_item",
            Classification::ContentChanged => "content_changed",
            Classification::Renamed => "renamed",
            Classification::MovedInside => "moved_inside",
            Classification::MovedOutside => "moved_outside",
            Classification::MovedBack => "moved_back",
            Classification::TrashedOrRemoved => "trashed_or_removed",
            Classification::Irrelevant => "irrelevant",
            Classification::Ambiguous => "ambiguous",
        }
    }

    pub fn from_str(s: &str) -> Option<Self> {
        match s {
            "new_item" => Some(Classification::NewItem),
            "content_changed" => Some(Classification::ContentChanged),
            "renamed" => Some(Classification::Renamed),
            "moved_inside" => Some(Classification::MovedInside),
            "moved_outside" => Some(Classification::MovedOutside),
            "moved_back" => Some(Classification::MovedBack),
            "trashed_or_removed" => Some(Classification::TrashedOrRemoved),
            "irrelevant" => Some(Classification::Irrelevant),
            "ambiguous" => Some(Classification::Ambiguous),
            _ => None,
        }
    }
}

/// Prior knowledge about a source item that we use to detect what changed.
#[derive(Debug, Clone, Default)]
pub struct ItemFingerprint {
    /// Last seen parent IDs.
    pub parents: Vec<String>,
    /// Last seen file name.
    pub name: Option<String>,
    /// Last seen md5 checksum (binary files).
    pub md5: Option<String>,
    /// Last seen Drive version number string.
    pub version: Option<String>,
    /// Whether the item was previously known to be trashed.
    pub trashed: bool,
}

impl ItemFingerprint {
    pub fn from_file(file: &DriveFile) -> Self {
        Self {
            parents: file.parents.clone(),
            name: Some(file.name.clone()),
            md5: file.md5_checksum.clone(),
            version: file.version.clone(),
            trashed: file.trashed.unwrap_or(false),
        }
    }

    /// Parse from the stored `file_json` in change_events (best-effort).
    pub fn from_json(json: &str) -> Option<Self> {
        let v: Value = serde_json::from_str(json).ok()?;
        Some(Self {
            parents: v["parents"]
                .as_array()
                .map(|a| {
                    a.iter()
                        .filter_map(|x| x.as_str().map(ToOwned::to_owned))
                        .collect()
                })
                .unwrap_or_default(),
            name: v["name"].as_str().map(ToOwned::to_owned),
            md5: v["md5Checksum"].as_str().map(ToOwned::to_owned),
            version: v["version"].as_str().map(ToOwned::to_owned),
            trashed: v["trashed"].as_bool().unwrap_or(false),
        })
    }
}

/// Classify a change event given:
/// - `watch` — the subscription being evaluated
/// - `in_tree` — whether the `file_id` has an active source_mapping in this watch
/// - `parents_in_tree` — for each parent of the current file, whether it is in-tree
/// - `prior` — optional fingerprint of what we knew before about this file
/// - `file` — current file metadata (`None` → removed = true)
/// - `removed` — the `removed` flag from the change event
pub fn classify(
    watch: &WatchSubscription,
    file_id: &str,
    in_tree: bool,
    parents_in_tree: &[bool],
    prior: Option<&ItemFingerprint>,
    file: Option<&DriveFile>,
    removed: bool,
) -> Classification {
    // 1. Hard remove signal.
    if removed {
        if in_tree {
            return Classification::TrashedOrRemoved;
        }
        return Classification::Irrelevant;
    }

    let Some(file) = file else {
        // removed=false but no file data — shouldn't happen, treat cautiously.
        warn!(
            watch_id = watch.id,
            file_id, "change event missing file object"
        );
        return Classification::Ambiguous;
    };

    // 2. Trashed.
    if file.trashed == Some(true) {
        if in_tree {
            return Classification::TrashedOrRemoved;
        }
        return Classification::Irrelevant;
    }

    // 3. Watch-level exclude filters: any event whose file name matches one
    // of the subscription's `exclude_globs` is irrelevant, no matter where it
    // sits. (Hard removal/trash signals above still win so an already-mapped
    // item can be cleaned up.)
    let globs = crate::watch::glob::parse_glob_list(&watch.exclude_globs);
    if !globs.is_empty() && crate::watch::glob::matches_any(&globs, &file.name) {
        debug!(
            watch_id = watch.id,
            file_id,
            name = file.name,
            "file matches exclude_globs — irrelevant"
        );
        return Classification::Irrelevant;
    }

    // 4. Root itself changed — always relevant if it's our source root.
    let is_root = file_id == watch.source_root_id;
    let any_parent_in_tree = parents_in_tree.iter().any(|&b| b);

    match prior {
        None => {
            // No prior: first time we see this file.
            if is_root || any_parent_in_tree {
                if in_tree {
                    // Already mapped — treat as ambiguous (safe: dispatcher
                    // will call get_file and compare).
                    Classification::Ambiguous
                } else {
                    Classification::NewItem
                }
            } else {
                Classification::Irrelevant
            }
        }
        Some(p) => {
            // Trashed → untrashed transition (moved back from trash).
            if p.trashed && !file.trashed.unwrap_or(false) {
                if in_tree || any_parent_in_tree {
                    return Classification::MovedBack;
                }
                return Classification::Irrelevant;
            }

            let prior_any_in_tree = !p.parents.is_empty();
            let _ = prior_any_in_tree; // used implicitly via `in_tree`

            // Parent change.
            let name_changed = p.name.as_deref() != Some(file.name.as_str());

            if in_tree && !any_parent_in_tree {
                // Was in tree, now all parents are out of tree.
                return Classification::MovedOutside;
            }
            if !in_tree && any_parent_in_tree {
                // Appeared under a tracked parent — new child.
                return Classification::NewItem;
            }

            // Still inside tree (or irrelevant).
            if !in_tree && !any_parent_in_tree && !is_root {
                return Classification::Irrelevant;
            }

            // Content change detection.
            let md5_changed =
                p.md5.is_some() && file.md5_checksum.is_some() && p.md5 != file.md5_checksum;
            let version_changed =
                p.version.is_some() && file.version.is_some() && p.version != file.version;
            let content_changed = md5_changed || version_changed;

            // Parent move inside tree.
            let parents_changed = p.parents != file.parents;

            match (name_changed, parents_changed, content_changed) {
                (_, true, _) => Classification::MovedInside,
                (true, false, false) => Classification::Renamed,
                (_, _, true) => Classification::ContentChanged,
                (false, false, false) => {
                    debug!(
                        watch_id = watch.id,
                        file_id, "change event: no detectable diff, treating as irrelevant"
                    );
                    Classification::Irrelevant
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{drive::types::DriveFile, state::repo::WatchSubscription};

    fn fake_watch(source_root_id: &str) -> WatchSubscription {
        WatchSubscription {
            id: "w1".into(),
            google_account_id: "default".into(),
            cursor_id: "c1".into(),
            telegram_user_id: 1,
            chat_id: 1,
            source_root_id: source_root_id.into(),
            source_resource_key: None,
            source_drive_id: None,
            destination_root_id: "dst-root".into(),
            destination_drive_id: None,
            status: "active".into(),
            content_update_policy: "versioned_copy".into(),
            deletion_policy: "preserve_destination".into(),
            move_out_policy: "detach".into(),
            exclude_globs: "[]".into(),
            baseline_sequence: 0,
            last_consumed_sequence: 0,
            created_at_ms: 0,
            updated_at_ms: 0,
            source_name: None,
            destination_name: None,
            last_consumed_at_ms: None,
        }
    }

    fn base_file(id: &str, parents: &[&str]) -> DriveFile {
        DriveFile {
            id: id.into(),
            name: "foo.txt".into(),
            mime_type: "text/plain".into(),
            size: None,
            parents: parents.iter().map(|&s| s.into()).collect(),
            drive_id: None,
            resource_key: None,
            shortcut_details: None,
            trashed: Some(false),
            modified_time: None,
            md5_checksum: Some("abc123".into()),
            version: Some("1".into()),
            capabilities: None,
            copy_requires_writer_permission: None,
            app_properties: Default::default(),
        }
    }

    #[test]
    fn removed_in_tree() {
        let w = fake_watch("root");
        let c = classify(&w, "f1", true, &[], None, None, true);
        assert_eq!(c, Classification::TrashedOrRemoved);
    }

    #[test]
    fn removed_not_in_tree() {
        let w = fake_watch("root");
        let c = classify(&w, "f1", false, &[], None, None, true);
        assert_eq!(c, Classification::Irrelevant);
    }

    #[test]
    fn new_item_under_tracked_parent() {
        let w = fake_watch("root");
        let f = base_file("f1", &["parent-in-tree"]);
        let c = classify(&w, "f1", false, &[true], None, Some(&f), false);
        assert_eq!(c, Classification::NewItem);
    }

    #[test]
    fn content_changed_detection() {
        let w = fake_watch("root");
        let prior = ItemFingerprint {
            parents: vec!["p1".into()],
            name: Some("foo.txt".into()),
            md5: Some("old-hash".into()),
            version: Some("1".into()),
            trashed: false,
        };
        let mut f = base_file("f1", &["p1"]);
        f.md5_checksum = Some("new-hash".into());
        f.version = Some("2".into());
        let c = classify(&w, "f1", true, &[true], Some(&prior), Some(&f), false);
        assert_eq!(c, Classification::ContentChanged);
    }

    #[test]
    fn renamed_detection() {
        let w = fake_watch("root");
        let prior = ItemFingerprint {
            parents: vec!["p1".into()],
            name: Some("old-name.txt".into()),
            // Same md5 and version as the updated file — only name changed.
            md5: Some("abc123".into()),
            version: Some("1".into()),
            trashed: false,
        };
        let mut f = base_file("f1", &["p1"]);
        f.name = "new-name.txt".into();
        let c = classify(&w, "f1", true, &[true], Some(&prior), Some(&f), false);
        assert_eq!(c, Classification::Renamed);
    }

    #[test]
    fn moved_outside() {
        let w = fake_watch("root");
        let prior = ItemFingerprint {
            parents: vec!["p-in-tree".into()],
            name: Some("foo.txt".into()),
            md5: None,
            version: None,
            trashed: false,
        };
        let f = base_file("f1", &["p-out-of-tree"]);
        // in_tree=true (we have a mapping), but parents_in_tree all false
        let c = classify(&w, "f1", true, &[false], Some(&prior), Some(&f), false);
        assert_eq!(c, Classification::MovedOutside);
    }

    #[test]
    fn exclude_globs_classify_as_irrelevant() {
        let mut w = fake_watch("root");
        w.exclude_globs = r#"["*.tmp", "~$*"]"#.into();

        // A brand-new excluded file under a tracked parent is ignored.
        let mut f = base_file("f1", &["parent-in-tree"]);
        f.name = "cache.tmp".into();
        let c = classify(&w, "f1", false, &[true], None, Some(&f), false);
        assert_eq!(c, Classification::Irrelevant);

        // A previously mapped file that now matches a new filter is ignored.
        let prior = ItemFingerprint {
            parents: vec!["p1".into()],
            name: Some("report.docx".into()),
            md5: Some("abc123".into()),
            version: Some("1".into()),
            trashed: false,
        };
        let mut f = base_file("f1", &["p1"]);
        f.name = "~$report.docx".into();
        let c = classify(&w, "f1", true, &[true], Some(&prior), Some(&f), false);
        assert_eq!(c, Classification::Irrelevant);

        // Non-matching names still classify normally.
        let mut f = base_file("f2", &["parent-in-tree"]);
        f.name = "keep-me.txt".into();
        let c = classify(&w, "f2", false, &[true], None, Some(&f), false);
        assert_eq!(c, Classification::NewItem);
    }

    #[test]
    fn empty_exclude_globs_never_filter() {
        let w = fake_watch("root");
        let f = base_file("f1", &["parent-in-tree"]);
        let c = classify(&w, "f1", false, &[true], None, Some(&f), false);
        assert_eq!(c, Classification::NewItem);
    }
}
