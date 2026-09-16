//! Minimal glob matching for watch exclude filters.
//!
//! Supported syntax (case-sensitive):
//! - `*`  — matches any sequence of characters except the path separator `/`
//! - `**` — matches any sequence of characters including `/`
//! - `?`  — matches exactly one character (any character except `/`)
//!
//! Everything else matches literally. No dependency is added on purpose: the
//! matcher is intentionally tiny because watch filters only ever run against
//! file names collected from the Drive change feed.

/// Maximum number of globs accepted per watch subscription.
pub const MAX_GLOBS_PER_WATCH: usize = 50;

/// Maximum length (in characters) accepted for a single glob pattern.
pub const MAX_GLOB_LEN: usize = 256;

/// Match `text` against a single glob `pattern`.
pub fn glob_match(pattern: &str, text: &str) -> bool {
    glob_match_inner(pattern.as_bytes(), text.as_bytes())
}

/// Match `text` against any pattern in `patterns` (an empty list never matches).
pub fn matches_any(patterns: &[String], text: &str) -> bool {
    patterns.iter().any(|pattern| glob_match(pattern, text))
}

/// Parse a JSON string array of globs, tolerating malformed input by returning
/// an empty list. Non-string entries are dropped.
pub fn parse_glob_list(json: &str) -> Vec<String> {
    match serde_json::from_str::<serde_json::Value>(json) {
        Ok(serde_json::Value::Array(items)) => items
            .into_iter()
            .filter_map(|item| match item {
                serde_json::Value::String(pattern) => Some(pattern),
                _ => None,
            })
            .collect(),
        _ => Vec::new(),
    }
}

/// Serialize a glob list back to the canonical JSON storage format.
pub fn serialize_glob_list(globs: &[String]) -> String {
    serde_json::to_string(globs).unwrap_or_else(|_| "[]".to_string())
}

/// Normalize a user-supplied glob: trim whitespace and reject empty,
/// oversized, or whitespace-embedded patterns.
pub fn normalize_glob(input: &str) -> Result<String, String> {
    let glob = input.trim();
    if glob.is_empty() {
        return Err("glob rỗng".to_string());
    }
    if glob.chars().count() > MAX_GLOB_LEN {
        return Err(format!("glob quá dài (tối đa {} ký tự)", MAX_GLOB_LEN));
    }
    if glob.chars().any(char::is_whitespace) {
        return Err("glob không được chứa khoảng trắng".to_string());
    }
    Ok(glob.to_string())
}

fn glob_match_inner(pattern: &[u8], text: &[u8]) -> bool {
    match (pattern.first(), text.first()) {
        (None, None) => true,
        (Some(b'*'), _) => {
            if pattern.get(1) == Some(&b'*') {
                // '**/' matches zero or more directories (including none), so
                // "a/**/d" matches both "a/d" and "a/b/c/d".
                if pattern.get(2) == Some(&b'/') {
                    return glob_match_inner(&pattern[3..], text)
                        || (!text.is_empty() && glob_match_inner(pattern, &text[1..]));
                }
                // Bare '**' matches everything, including nothing and
                // separators.
                glob_match_inner(&pattern[2..], text)
                    || (!text.is_empty() && glob_match_inner(pattern, &text[1..]))
            } else {
                // '*' matches zero or more non-separator characters.
                glob_match_inner(&pattern[1..], text)
                    || (!text.is_empty()
                        && text[0] != b'/'
                        && glob_match_inner(pattern, &text[1..]))
            }
        }
        (Some(b'?'), Some(&c)) if c != b'/' => glob_match_inner(&pattern[1..], &text[1..]),
        (Some(&p), Some(&c)) if p == c => glob_match_inner(&pattern[1..], &text[1..]),
        _ => false,
    }
}

#[cfg(test)]
mod tests {
    use super::{glob_match, matches_any, normalize_glob, parse_glob_list, serialize_glob_list};

    #[test]
    fn star_matches_within_a_single_path_segment() {
        assert!(glob_match("*.tmp", "notes.tmp"));
        assert!(glob_match("*.tmp", "archive.old.tmp"));
        assert!(!glob_match("*.tmp", "tmp/notes.tmp"));
        assert!(glob_match("data/*", "data/notes.txt"));
        assert!(!glob_match("data/*", "data/sub/notes.txt"));
        assert!(glob_match("data/**", "data/sub/notes.txt"));
        assert!(!glob_match("data/**", "database/x"));
    }

    #[test]
    fn question_mark_matches_exactly_one_char() {
        assert!(glob_match("file?.txt", "file1.txt"));
        assert!(glob_match("file?.txt", "file?.txt"));
        assert!(!glob_match("file?.txt", "file10.txt"));
        assert!(!glob_match("file?.txt", "file1.bak"));
    }

    #[test]
    fn double_star_matches_anything() {
        assert!(glob_match("**", "a/b/c.txt"));
        assert!(glob_match("**", ""));
        assert!(glob_match("a/**/d", "a/b/c/d"));
        assert!(glob_match("a/**/d", "a/d"));
        assert!(!glob_match("a/**/d", "a/b/c/e"));
    }

    #[test]
    fn literal_patterns_match_exactly() {
        assert!(glob_match("exact.tmp", "exact.tmp"));
        assert!(!glob_match("exact.tmp", "Exact.tmp"));
        assert!(!glob_match("exact.tmp", "exact.tmp2"));
        assert!(!glob_match("", "x"));
        assert!(glob_match("", ""));
    }

    #[test]
    fn matches_any_scans_every_pattern() {
        let globs = vec!["~$*".to_string(), "*.tmp".to_string()];
        assert!(matches_any(&globs, "~$report.docx"));
        assert!(matches_any(&globs, "cache.tmp"));
        assert!(!matches_any(&globs, "report.docx"));
        assert!(!matches_any(&[], "anything.tmp"));
    }

    #[test]
    fn glob_list_round_trips_and_tolerates_garbage() {
        let globs = vec!["*.tmp".to_string(), "~$*".to_string()];
        let json = serialize_glob_list(&globs);
        assert_eq!(parse_glob_list(&json), globs);
        assert!(parse_glob_list("not json").is_empty());
        assert!(parse_glob_list(r#"["ok", 1, null]"#).len() == 1);
    }

    #[test]
    fn normalize_glob_trims_and_rejects_bad_input() {
        assert_eq!(normalize_glob("  *.tmp  ").unwrap(), "*.tmp");
        assert!(normalize_glob("   ").is_err());
        assert!(normalize_glob("a b").is_err());
        assert!(normalize_glob(&"x".repeat(300)).is_err());
    }
}
