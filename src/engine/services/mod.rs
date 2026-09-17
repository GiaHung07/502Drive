//! Application-services layer shared by every frontend (telegram bot, GUI, CLI).
//!
//! Historically the GUI wrote job/watch status with raw SQL behind the
//! engine's back (skipping the `pausing` handshake, resurrecting the dead
//! `queued` status, force-cancelling running jobs) while the telegram handlers
//! called the repo state machines directly. This module is the single place
//! where user-facing state transitions happen:
//!
//! * [`job::JobService`] — pause/resume/cancel/retry over the repo state
//!   machines, job-id resolution from user input, and progress derivation.
//! * [`watch::WatchService`] — watch pause/resume/stop, one policy validator
//!   for all three policy kinds, exclude globs, and root-folder labels.
//! * [`destination::DestinationService`] — set-default destination and Drive
//!   folder browsing (shared by the GUI picker and the telegram browser).
//! * [`account::AccountService`] — Google account disconnect (decrypt, revoke,
//!   status guard).
//!
//! Services are thin: they never spawn teloxide tasks, never import the
//! telegram module, and never issue raw SQL that diverges from
//! [`crate::state::repo`]. Worker spawns (resume/retry) live in
//! [`crate::engine::recovery`]; services may call it because they run inside
//! the engine crate, but out-of-process callers (the GUI) must instead go
//! through the `ui_requests` queue.

pub mod account;
pub mod destination;
pub mod job;
pub mod watch;

pub use account::{AccountService, DisconnectOutcome};
pub use destination::{DestinationEntry, DestinationService};
pub use job::{JobProgress, JobResolveError, JobService};
pub use watch::{WatchLabels, WatchPolicyKind, WatchService};
