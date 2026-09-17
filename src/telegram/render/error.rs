//! User-facing error / access texts.

use crate::telegram::keyboards;

pub(crate) fn access_denied_message(lang: keyboards::UiLanguage) -> &'static str {
    match lang {
        keyboards::UiLanguage::Vi => {
            "Không có quyền truy cập. Liên hệ chủ sở hữu để được cấp phép."
        }
        keyboards::UiLanguage::En => "Access denied. Contact the owner to be allowed.",
    }
}

pub(crate) fn access_denied_short(lang: keyboards::UiLanguage) -> &'static str {
    match lang {
        keyboards::UiLanguage::Vi => "Không có quyền truy cập.",
        keyboards::UiLanguage::En => "Access denied.",
    }
}

pub(crate) fn load_home_error(lang: keyboards::UiLanguage, err: &anyhow::Error) -> String {
    match lang {
        keyboards::UiLanguage::Vi => format!("Lỗi tải trang chính: {err}"),
        keyboards::UiLanguage::En => format!("Could not load home: {err}"),
    }
}

pub(crate) fn load_jobs_error(lang: keyboards::UiLanguage, err: &anyhow::Error) -> String {
    match lang {
        keyboards::UiLanguage::Vi => format!("Lỗi tải job: {err}"),
        keyboards::UiLanguage::En => format!("Could not load jobs: {err}"),
    }
}

pub(crate) fn load_watch_error(lang: keyboards::UiLanguage, err: &anyhow::Error) -> String {
    match lang {
        keyboards::UiLanguage::Vi => format!("Lỗi tải watch: {err}"),
        keyboards::UiLanguage::En => format!("Could not load watches: {err}"),
    }
}

pub(crate) fn load_watch_list_error(lang: keyboards::UiLanguage, err: &anyhow::Error) -> String {
    match lang {
        keyboards::UiLanguage::Vi => format!("Lỗi tải danh sách watch: {err}"),
        keyboards::UiLanguage::En => format!("Could not load watch list: {err}"),
    }
}

pub(crate) fn browse_folder_error(lang: keyboards::UiLanguage, err: &anyhow::Error) -> String {
    match lang {
        keyboards::UiLanguage::Vi => {
            format!("Lỗi duyệt thư mục:\n{}", format_error_for_user(err, lang))
        }
        keyboards::UiLanguage::En => {
            format!(
                "Could not browse folder:\n{}",
                format_error_for_user(err, lang)
            )
        }
    }
}

pub(crate) fn pick_destination_error(lang: keyboards::UiLanguage, err: &anyhow::Error) -> String {
    match lang {
        keyboards::UiLanguage::Vi => {
            format!(
                "Lỗi đặt thư mục đích:\n{}",
                format_error_for_user(err, lang)
            )
        }
        keyboards::UiLanguage::En => {
            format!(
                "Could not set destination folder:\n{}",
                format_error_for_user(err, lang)
            )
        }
    }
}

pub(crate) fn clone_run_error(lang: keyboards::UiLanguage, err: &anyhow::Error) -> String {
    match lang {
        keyboards::UiLanguage::Vi => format!("Lỗi clone:\n{}", format_error_for_user(err, lang)),
        keyboards::UiLanguage::En => format!("Clone failed:\n{}", format_error_for_user(err, lang)),
    }
}

pub(crate) fn clone_inspect_error(lang: keyboards::UiLanguage, err: &anyhow::Error) -> String {
    match lang {
        keyboards::UiLanguage::Vi => {
            format!("Lỗi kiểm tra nguồn:\n{}", format_error_for_user(err, lang))
        }
        keyboards::UiLanguage::En => {
            format!(
                "Could not inspect source:\n{}",
                format_error_for_user(err, lang)
            )
        }
    }
}

pub(crate) fn invalid_drive_link_text(
    lang: keyboards::UiLanguage,
    err: impl std::fmt::Display,
) -> String {
    match lang {
        keyboards::UiLanguage::Vi => {
            format!(
                "Link Drive không hợp lệ: {err}\nGửi link file/folder Google Drive hoặc folder ID."
            )
        }
        keyboards::UiLanguage::En => {
            format!("Invalid Drive link: {err}\nSend a Google Drive file/folder link or folder ID.")
        }
    }
}

pub(crate) fn unknown_action(lang: keyboards::UiLanguage) -> &'static str {
    match lang {
        keyboards::UiLanguage::Vi => "Hành động không xác định.",
        keyboards::UiLanguage::En => "Unknown action.",
    }
}

pub(crate) fn report_latest_read_error(
    lang: keyboards::UiLanguage,
    err: impl std::fmt::Display,
) -> String {
    match lang {
        keyboards::UiLanguage::Vi => format!("Lỗi đọc job gần nhất: {err}"),
        keyboards::UiLanguage::En => format!("Could not read the latest job: {err}"),
    }
}

pub(crate) fn report_job_read_error(
    lang: keyboards::UiLanguage,
    err: impl std::fmt::Display,
) -> String {
    match lang {
        keyboards::UiLanguage::Vi => format!("Lỗi đọc job: {err}"),
        keyboards::UiLanguage::En => format!("Could not read job: {err}"),
    }
}

pub(crate) fn report_write_error(
    lang: keyboards::UiLanguage,
    err: impl std::fmt::Display,
) -> String {
    match lang {
        keyboards::UiLanguage::Vi => format!("Lỗi tạo report: {err}"),
        keyboards::UiLanguage::En => format!("Could not create report: {err}"),
    }
}

pub(crate) fn format_error_for_user(err: &anyhow::Error, lang: keyboards::UiLanguage) -> String {
    crate::watch::errors::format_anyhow_error(err, lang)
}
