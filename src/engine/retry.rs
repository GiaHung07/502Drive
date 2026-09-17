use std::time::Duration;

use rand::Rng;
use reqwest::StatusCode;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RetryDecision {
    Retry,
    RefreshTokenOnce,
    DoNotRetry,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RetryClass {
    /// 401 Unauthorized - refresh OAuth access token once and immediately retry.
    ImmediateAuthRefresh,
    /// Transient rate limit or server error: 429, 5xx, or 403 userRateLimitExceeded / rateLimitExceeded.
    /// Apply exponential backoff with jitter and honor Retry-After if present.
    Backoff,
    /// 403 dailyLimitExceeded, usageLimits.userRateLimitExceededUnreg - project/user daily quota exhausted.
    /// Requires long cooldown (e.g. 24h reset).
    LongCooldown,
    /// 403 cannotDownloadFile, cannotCopyFile, or 404 Not Found - specific file/item permission error.
    /// Mark item failed or skipped without aborting the entire job.
    FatalItem,
    /// 403 storageQuotaExceeded, invalid_grant, account revoked - fatal for target account.
    /// Pause job and notify user.
    FatalOperation,
    /// Non-retryable error / bad request.
    DoNotRetry,
}

impl RetryClass {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::ImmediateAuthRefresh => "immediate_auth_refresh",
            Self::Backoff => "backoff",
            Self::LongCooldown => "long_cooldown",
            Self::FatalItem => "fatal_item",
            Self::FatalOperation => "fatal_operation",
            Self::DoNotRetry => "do_not_retry",
        }
    }

    pub fn from_str_opt(s: &str) -> Option<Self> {
        match s {
            "immediate_auth_refresh" => Some(Self::ImmediateAuthRefresh),
            "backoff" => Some(Self::Backoff),
            "long_cooldown" => Some(Self::LongCooldown),
            "fatal_item" => Some(Self::FatalItem),
            "fatal_operation" => Some(Self::FatalOperation),
            "do_not_retry" => Some(Self::DoNotRetry),
            _ => None,
        }
    }
}

/// Classifies an HTTP status code and optional Google API error reason into
/// a structured `RetryClass` and tactical `RetryDecision`.
pub fn classify_drive_error(
    status: StatusCode,
    reason: Option<&str>,
) -> (RetryClass, RetryDecision) {
    match status {
        StatusCode::UNAUTHORIZED => (
            RetryClass::ImmediateAuthRefresh,
            RetryDecision::RefreshTokenOnce,
        ),
        StatusCode::TOO_MANY_REQUESTS => (RetryClass::Backoff, RetryDecision::Retry),
        StatusCode::FORBIDDEN => match reason {
            Some("userRateLimitExceeded" | "rateLimitExceeded") => {
                (RetryClass::Backoff, RetryDecision::Retry)
            }
            Some("dailyLimitExceeded" | "usageLimits.userRateLimitExceededUnreg") => {
                (RetryClass::LongCooldown, RetryDecision::DoNotRetry)
            }
            Some("cannotDownloadFile" | "cannotCopyFile") => {
                (RetryClass::FatalItem, RetryDecision::DoNotRetry)
            }
            Some("storageQuotaExceeded") => (RetryClass::FatalOperation, RetryDecision::DoNotRetry),
            _ => (RetryClass::DoNotRetry, RetryDecision::DoNotRetry),
        },
        StatusCode::NOT_FOUND => (RetryClass::FatalItem, RetryDecision::DoNotRetry),
        StatusCode::BAD_REQUEST => match reason {
            Some("invalid_grant") => (RetryClass::FatalOperation, RetryDecision::DoNotRetry),
            _ => (RetryClass::DoNotRetry, RetryDecision::DoNotRetry),
        },
        _ if status.is_server_error() => (RetryClass::Backoff, RetryDecision::Retry),
        _ => (RetryClass::DoNotRetry, RetryDecision::DoNotRetry),
    }
}

/// Backward-compatible classification returning `RetryDecision`.
pub fn classify_http(status: StatusCode, reason: Option<&str>) -> RetryDecision {
    classify_drive_error(status, reason).1
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_classify_drive_error() {
        // 401 Unauthorized
        assert_eq!(
            classify_drive_error(StatusCode::UNAUTHORIZED, None),
            (
                RetryClass::ImmediateAuthRefresh,
                RetryDecision::RefreshTokenOnce
            )
        );

        // 429 Too Many Requests
        assert_eq!(
            classify_drive_error(StatusCode::TOO_MANY_REQUESTS, None),
            (RetryClass::Backoff, RetryDecision::Retry)
        );

        // 403 Rate limits
        assert_eq!(
            classify_drive_error(StatusCode::FORBIDDEN, Some("userRateLimitExceeded")),
            (RetryClass::Backoff, RetryDecision::Retry)
        );
        assert_eq!(
            classify_drive_error(StatusCode::FORBIDDEN, Some("rateLimitExceeded")),
            (RetryClass::Backoff, RetryDecision::Retry)
        );

        // 403 Daily limits (Long cooldown)
        assert_eq!(
            classify_drive_error(StatusCode::FORBIDDEN, Some("dailyLimitExceeded")),
            (RetryClass::LongCooldown, RetryDecision::DoNotRetry)
        );

        // 403 File-specific failures
        assert_eq!(
            classify_drive_error(StatusCode::FORBIDDEN, Some("cannotDownloadFile")),
            (RetryClass::FatalItem, RetryDecision::DoNotRetry)
        );
        assert_eq!(
            classify_drive_error(StatusCode::FORBIDDEN, Some("cannotCopyFile")),
            (RetryClass::FatalItem, RetryDecision::DoNotRetry)
        );

        // 403 Storage quota exceeded (Fatal operation)
        assert_eq!(
            classify_drive_error(StatusCode::FORBIDDEN, Some("storageQuotaExceeded")),
            (RetryClass::FatalOperation, RetryDecision::DoNotRetry)
        );

        // 404 Not Found (Fatal item)
        assert_eq!(
            classify_drive_error(StatusCode::NOT_FOUND, None),
            (RetryClass::FatalItem, RetryDecision::DoNotRetry)
        );

        // 500, 502, 503, 504 Transient server errors
        assert_eq!(
            classify_drive_error(StatusCode::INTERNAL_SERVER_ERROR, None),
            (RetryClass::Backoff, RetryDecision::Retry)
        );
        assert_eq!(
            classify_drive_error(StatusCode::BAD_GATEWAY, None),
            (RetryClass::Backoff, RetryDecision::Retry)
        );
        assert_eq!(
            classify_drive_error(StatusCode::SERVICE_UNAVAILABLE, None),
            (RetryClass::Backoff, RetryDecision::Retry)
        );
        assert_eq!(
            classify_drive_error(StatusCode::GATEWAY_TIMEOUT, None),
            (RetryClass::Backoff, RetryDecision::Retry)
        );

        // 400 invalid_grant (Fatal operation)
        assert_eq!(
            classify_drive_error(StatusCode::BAD_REQUEST, Some("invalid_grant")),
            (RetryClass::FatalOperation, RetryDecision::DoNotRetry)
        );
    }

    #[test]
    fn test_retry_class_string_roundtrip() {
        let classes = [
            RetryClass::ImmediateAuthRefresh,
            RetryClass::Backoff,
            RetryClass::LongCooldown,
            RetryClass::FatalItem,
            RetryClass::FatalOperation,
            RetryClass::DoNotRetry,
        ];
        for c in classes {
            assert_eq!(RetryClass::from_str_opt(c.as_str()), Some(c));
        }
    }
}
