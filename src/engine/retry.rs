use std::time::Duration;

use rand::Rng;
use reqwest::StatusCode;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RetryDecision {
    Retry,
    RefreshTokenOnce,
    DoNotRetry,
}

pub fn classify_http(status: StatusCode, reason: Option<&str>) -> RetryDecision {
    match status {
        StatusCode::UNAUTHORIZED => RetryDecision::RefreshTokenOnce,
        StatusCode::TOO_MANY_REQUESTS
        | StatusCode::INTERNAL_SERVER_ERROR
        | StatusCode::BAD_GATEWAY
        | StatusCode::SERVICE_UNAVAILABLE
        | StatusCode::GATEWAY_TIMEOUT => RetryDecision::Retry,
        StatusCode::FORBIDDEN => match reason {
            Some("userRateLimitExceeded" | "rateLimitExceeded") => RetryDecision::Retry,
            _ => RetryDecision::DoNotRetry,
        },
        StatusCode::BAD_REQUEST | StatusCode::NOT_FOUND => RetryDecision::DoNotRetry,
        _ if status.is_server_error() => RetryDecision::Retry,
        _ => RetryDecision::DoNotRetry,
    }
}

pub fn backoff_delay(attempt: u32, base: Duration, max: Duration) -> Duration {
    let shift = attempt.min(20);
    let multiplier = 1_u32 << shift;
    let exponential = base.saturating_mul(multiplier).min(max);
    let jitter_ms = if base.is_zero() {
        0
    } else {
        rand::rng().random_range(0..=base.as_millis() as u64)
    };
    exponential + Duration::from_millis(jitter_ms)
}
