//! Shared Drive error formatting for user-facing messages.
//!
//! This used to live in the Telegram handler layer; the watch dispatcher also
//! needs to surface Drive API failures to users, so the helper moved here and
//! both layers now use the same implementation.

use crate::{drive::client::DriveApiError, telegram::i18n::UiLanguage};

pub fn format_drive_error(err: &DriveApiError, lang: UiLanguage) -> String {
    match err {
        DriveApiError::Api {
            status,
            reason,
            message,
            ..
        } => {
            let friendly = drive_error_friendly(lang, status.as_u16(), reason.as_deref());
            format!(
                "{friendly}\n• HTTP: {}\n• Reason: {}\n• {}: {}",
                status.as_u16(),
                reason
                    .as_deref()
                    .unwrap_or(drive_error_unknown_reason(lang)),
                drive_error_detail_field(lang),
                message
            )
        }
        DriveApiError::Transport(err) => {
            format!(
                "{}\n• {}: {err}",
                drive_error_transport(lang),
                drive_error_detail_field(lang)
            )
        }
    }
}

/// Format an arbitrary error, routing Drive API failures through
/// [`format_drive_error`] and falling back to the error's own text.
pub fn format_anyhow_error(err: &anyhow::Error, lang: UiLanguage) -> String {
    if let Some(drive) = err.downcast_ref::<DriveApiError>() {
        return format_drive_error(drive, lang);
    }
    err.to_string()
}

fn drive_error_friendly(lang: UiLanguage, status: u16, reason: Option<&str>) -> &'static str {
    match (lang, status, reason) {
        (UiLanguage::Vi, 401, _) => {
            "Phiên Google hết hạn. Chạy `502drive auth login` trên máy bot."
        }
        (UiLanguage::En, 401, _) => {
            "Google session expired. Run `502drive auth login` on the bot machine."
        }
        (UiLanguage::Vi, 403, Some("insufficientPermissions")) => {
            "Tài khoản Google hiện tại không đủ quyền với file/folder này."
        }
        (UiLanguage::En, 403, Some("insufficientPermissions")) => {
            "The current Google account does not have access to this file/folder."
        }
        (UiLanguage::Vi, 403, Some("copyRequiresWriterPermission")) => {
            "Nguồn yêu cầu quyền ghi mới được copy."
        }
        (UiLanguage::En, 403, Some("copyRequiresWriterPermission")) => {
            "This source requires writer permission before it can be copied."
        }
        (UiLanguage::Vi, 403, Some("storageQuotaExceeded" | "teamDriveFileLimitExceeded")) => {
            "Google Drive báo hết quota hoặc chạm giới hạn lưu trữ."
        }
        (UiLanguage::En, 403, Some("storageQuotaExceeded" | "teamDriveFileLimitExceeded")) => {
            "Google Drive quota or storage limit was reached."
        }
        (UiLanguage::Vi, 403, Some("userRateLimitExceeded" | "rateLimitExceeded"))
        | (UiLanguage::Vi, 429, _) => {
            "Google đang giới hạn tốc độ. Bot sẽ retry nếu lỗi xảy ra trong job."
        }
        (UiLanguage::En, 403, Some("userRateLimitExceeded" | "rateLimitExceeded"))
        | (UiLanguage::En, 429, _) => {
            "Google is rate limiting requests. The bot will retry inside jobs."
        }
        (UiLanguage::Vi, 404, _) => {
            "Không tìm thấy file/folder, không có quyền truy cập, hoặc link thiếu resource key."
        }
        (UiLanguage::En, 404, _) => {
            "File/folder not found, access is missing, or the link needs a resource key."
        }
        (UiLanguage::Vi, 400, _) => "Request Drive không hợp lệ. Kiểm tra lại link nguồn/đích.",
        (UiLanguage::En, 400, _) => "Invalid Drive request. Check the source/destination link.",
        (UiLanguage::Vi, 500..=599, _) => "Google Drive đang lỗi tạm thời. Thử lại sau ít phút.",
        (UiLanguage::En, 500..=599, _) => {
            "Google Drive has a temporary error. Try again in a few minutes."
        }
        (UiLanguage::Vi, _, _) => "Google Drive trả về lỗi.",
        (UiLanguage::En, _, _) => "Google Drive returned an error.",
    }
}

fn drive_error_unknown_reason(lang: UiLanguage) -> &'static str {
    match lang {
        UiLanguage::Vi => "không rõ",
        UiLanguage::En => "unknown",
    }
}

fn drive_error_detail_field(lang: UiLanguage) -> &'static str {
    match lang {
        UiLanguage::Vi => "Chi tiết",
        UiLanguage::En => "Details",
    }
}

fn drive_error_transport(lang: UiLanguage) -> &'static str {
    match lang {
        UiLanguage::Vi => "Không gọi được Google Drive API.",
        UiLanguage::En => "Could not call the Google Drive API.",
    }
}
