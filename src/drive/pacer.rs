//! Adaptive pacer — cross-worker, token-bucket-based rate limiting for
//! Google Drive API requests.
//!
//! ## Design
//! A single `Pacer` instance is shared via `Arc` across the engine copy
//! workers and the watch poller/dispatcher. It implements a token-bucket with:
//!
//! - A configurable **quota** (tokens/second) that matches Drive's per-user
//!   quotas (default: 1 000 req/s, adjustable via config or env).
//! - A **circuit breaker** that opens after N consecutive 429/500/503 errors,
//!   then re-closes after a back-off period. While open all callers receive
//!   `PacerDecision::Wait(duration)` without consuming tokens.
//!
//! Callers **must** call `acquire()` before each API request and respect the
//! returned decision. The pacer does not sleep internally — it returns the
//! duration to sleep so callers can select! against a stop signal.

use std::{
    sync::{
        Arc, Mutex,
        atomic::{AtomicU32, Ordering},
    },
    time::{Duration, Instant},
};

use tracing::{debug, warn};

/// Caller's required action before issuing a Drive API request.
#[derive(Debug, Clone)]
pub enum PacerDecision {
    /// Proceed immediately.
    Proceed,
    /// Wait at least this long before proceeding (circuit open or bucket empty).
    Wait(Duration),
}

/// Shared, thread-safe adaptive pacer.
#[derive(Clone, Debug)]
pub struct Pacer {
    inner: Arc<PacerInner>,
}

#[derive(Debug)]
struct PacerInner {
    state: Mutex<PacerState>,
    consecutive_errors: AtomicU32,
    /// Maximum consecutive errors before opening the circuit.
    trip_threshold: u32,
    /// How long to keep the circuit open.
    open_duration: Duration,
    /// Tokens added per second.
    tokens_per_second: f64,
    /// Maximum burst size (bucket capacity).
    burst: f64,
}

#[derive(Debug)]
struct PacerState {
    /// Current tokens in the bucket (0..=burst).
    tokens: f64,
    /// Last time tokens were replenished.
    last_refill: Instant,
    /// If `Some`, the circuit stays open until this instant.
    circuit_open_until: Option<Instant>,
}

impl Pacer {
    /// Create a new pacer.
    ///
    /// - `tokens_per_second`: sustained throughput limit (e.g. 1000.0 for Drive).
    /// - `burst`: maximum burst capacity (e.g. 200.0).
    /// - `trip_threshold`: consecutive errors to trip the circuit.
    /// - `open_duration`: how long the circuit stays open after tripping.
    pub fn new(
        tokens_per_second: f64,
        burst: f64,
        trip_threshold: u32,
        open_duration: Duration,
    ) -> Self {
        Pacer {
            inner: Arc::new(PacerInner {
                state: Mutex::new(PacerState {
                    tokens: burst,
                    last_refill: Instant::now(),
                    circuit_open_until: None,
                }),
                consecutive_errors: AtomicU32::new(0),
                trip_threshold,
                open_duration,
                tokens_per_second,
                burst,
            }),
        }
    }

    /// Obtain permission to issue one API request.
    ///
    /// Returns `PacerDecision::Proceed` when the circuit is closed and a token
    /// is available, or `PacerDecision::Wait(duration)` otherwise.
    pub fn acquire(&self) -> PacerDecision {
        let inner = &*self.inner;
        let mut state = inner.state.lock().expect("pacer lock poisoned");
        let now = Instant::now();

        // ── Circuit breaker check ────────────────────────────────────────────
        if let Some(open_until) = state.circuit_open_until {
            if now < open_until {
                let remaining = open_until.duration_since(now);
                debug!(
                    remaining_ms = remaining.as_millis(),
                    "circuit breaker open — waiting"
                );
                return PacerDecision::Wait(remaining);
            }
            // Circuit expired: close it.
            warn!("circuit breaker closing — resuming requests");
            state.circuit_open_until = None;
            inner.consecutive_errors.store(0, Ordering::Relaxed);
        }

        // ── Token bucket refill ──────────────────────────────────────────────
        let elapsed_secs = now.duration_since(state.last_refill).as_secs_f64();
        let new_tokens = elapsed_secs * inner.tokens_per_second;
        state.tokens = (state.tokens + new_tokens).min(inner.burst);
        state.last_refill = now;

        if state.tokens >= 1.0 {
            state.tokens -= 1.0;
            PacerDecision::Proceed
        } else {
            // Compute time until next token is available.
            let deficit = 1.0 - state.tokens;
            let wait_secs = deficit / inner.tokens_per_second;
            PacerDecision::Wait(Duration::from_secs_f64(wait_secs))
        }
    }

    /// Report a successful API response — resets the consecutive error counter.
    pub fn record_success(&self) {
        self.inner.consecutive_errors.store(0, Ordering::Relaxed);
    }

    /// Report a retryable API error (429, 500, 503).
    ///
    /// After `trip_threshold` consecutive errors the circuit opens.
    pub fn record_error(&self) {
        let inner = &*self.inner;
        let prev = inner.consecutive_errors.fetch_add(1, Ordering::Relaxed);
        let new_count = prev + 1;
        if new_count >= inner.trip_threshold {
            let mut state = inner.state.lock().expect("pacer lock poisoned");
            let deadline = Instant::now() + inner.open_duration;
            if state.circuit_open_until.is_none_or(|d| deadline > d) {
                warn!(
                    consecutive_errors = new_count,
                    open_for_ms = inner.open_duration.as_millis(),
                    "circuit breaker tripped"
                );
                state.circuit_open_until = Some(deadline);
            }
        }
    }

    /// Force-open the circuit for a specific duration (e.g. after a 429 with
    /// `Retry-After` header).
    pub fn force_open(&self, duration: Duration) {
        let mut state = self.inner.state.lock().expect("pacer lock poisoned");
        let deadline = Instant::now() + duration;
        if state.circuit_open_until.is_none_or(|d| deadline > d) {
            warn!(
                force_open_ms = duration.as_millis(),
                "circuit breaker force-opened (Retry-After)"
            );
            state.circuit_open_until = Some(deadline);
        }
    }

    /// Current consecutive error count (for diagnostics).
    pub fn consecutive_errors(&self) -> u32 {
        self.inner.consecutive_errors.load(Ordering::Relaxed)
    }
}

impl Default for Pacer {
    fn default() -> Self {
        // Drive API: 1 000 queries/user/second (sustained); 200 burst.
        // Circuit trips after 5 consecutive errors; opens for 60 s.
        Self::new(1_000.0, 200.0, 5, Duration::from_secs(60))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::Duration;

    #[test]
    fn proceeds_within_burst() {
        let p = Pacer::new(100.0, 10.0, 3, Duration::from_secs(60));
        // First 10 requests should all proceed immediately (burst = 10).
        let mut waits = 0u32;
        for _ in 0..10 {
            if matches!(p.acquire(), PacerDecision::Wait(_)) {
                waits += 1;
            }
        }
        assert_eq!(waits, 0, "unexpected waits within burst");
        // 11th should wait.
        assert!(matches!(p.acquire(), PacerDecision::Wait(_)));
    }

    #[test]
    fn circuit_trips_after_threshold() {
        let p = Pacer::new(1000.0, 100.0, 3, Duration::from_secs(60));
        for _ in 0..3 {
            p.record_error();
        }
        // Circuit should now be open.
        assert!(matches!(p.acquire(), PacerDecision::Wait(_)));
    }

    #[test]
    fn success_resets_error_counter() {
        let p = Pacer::new(1000.0, 100.0, 3, Duration::from_secs(60));
        p.record_error();
        p.record_error();
        p.record_success();
        assert_eq!(p.consecutive_errors(), 0);
    }

    #[test]
    fn force_open_respected() {
        let p = Pacer::new(1000.0, 100.0, 3, Duration::from_millis(100));
        p.force_open(Duration::from_secs(3600));
        assert!(matches!(p.acquire(), PacerDecision::Wait(d) if d > Duration::from_secs(3500)));
    }
}
