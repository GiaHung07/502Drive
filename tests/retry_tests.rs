use std::time::Duration;

use gdclone_bot::engine::retry::{RetryDecision, backoff_delay, classify_http};
use reqwest::StatusCode;

#[test]
fn retries_rate_limit_reasons() {
    assert_eq!(
        classify_http(StatusCode::FORBIDDEN, Some("userRateLimitExceeded")),
        RetryDecision::Retry
    );
    assert_eq!(
        classify_http(StatusCode::FORBIDDEN, Some("insufficientPermissions")),
        RetryDecision::DoNotRetry
    );
}

#[test]
fn refreshes_unauthorized_once() {
    assert_eq!(
        classify_http(StatusCode::UNAUTHORIZED, None),
        RetryDecision::RefreshTokenOnce
    );
}

#[test]
fn caps_backoff() {
    let delay = backoff_delay(20, Duration::from_secs(1), Duration::from_secs(64));
    assert!(delay >= Duration::from_secs(64));
    assert!(delay <= Duration::from_secs(65));
}
