//! Pure text/rendering helpers for Telegram messages, extracted from
//! `handlers.rs`. Functions here do not talk to the Bot; only data
//! rendering (some panels read state via `repo`).

pub(crate) mod clone;
pub(crate) mod destination;
pub(crate) mod error;
pub(crate) mod home;
pub(crate) mod inspect;
pub(crate) mod job;
pub(crate) mod jobs;
pub(crate) mod prompts;
pub(crate) mod watch;

pub(crate) use clone::*;
pub(crate) use destination::*;
pub(crate) use error::*;
pub(crate) use home::*;
pub(crate) use inspect::*;
pub(crate) use job::*;
pub(crate) use jobs::*;
pub(crate) use prompts::*;
pub(crate) use watch::*;

/// Append a `• label: value` bullet line.
pub(crate) fn push_field(lines: &mut Vec<String>, label: &str, value: &str) {
    lines.push(format!("• {label}: {value}"));
}

/// First 8 chars of an id (or the whole id when shorter).
pub(crate) fn short_id(id: &str) -> &str {
    id.get(..8).unwrap_or(id)
}

pub(crate) fn short_job_id(job_id: &str) -> &str {
    short_id(job_id)
}
