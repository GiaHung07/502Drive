//! Home dashboard, account panel and language panel texts.

use crate::state::{db::Database, repo};
use crate::telegram::keyboards;
use crate::telegram::render::push_field;

pub(crate) async fn render_home_dashboard(
    config: &crate::config::AppConfig,
    db: &Database,
    telegram_user_id: i64,
    lang: keyboards::UiLanguage,
) -> anyhow::Result<String> {
    let account = repo::account_status(db)
        .await?
        .unwrap_or_else(|| "chưa kết nối".to_string());
    let destination = repo::default_destination_profile(db, "default").await?;
    let jobs = repo::list_active_jobs_for_user(db, telegram_user_id, 5).await?;
    let watches = if config.watch.enabled {
        repo::list_watches_for_user(db, telegram_user_id).await?
    } else {
        Vec::new()
    };
    let watch_active = watches
        .iter()
        .filter(|w| matches!(w.status.as_str(), "active" | "catching_up" | "degraded"))
        .count();
    let watch_paused = watches.iter().filter(|w| w.status == "paused").count();

    let mut lines = vec![
        home_title(lang).to_string(),
        "━━━━━━━━━━━━━━━━━━━━".to_string(),
    ];
    push_field(&mut lines, "Google", account_status_label(lang, &account));
    match destination {
        Some(dest) => push_field(&mut lines, home_destination_label(lang), &dest.label),
        None => push_field(
            &mut lines,
            home_destination_label(lang),
            home_destination_missing(lang),
        ),
    }
    push_field(&mut lines, home_jobs_label(lang), &jobs.len().to_string());
    if config.watch.enabled {
        let watch_summary = match lang {
            keyboards::UiLanguage::Vi => format!(
                "{} tổng · {} hoạt động · {} tạm dừng",
                watches.len(),
                watch_active,
                watch_paused
            ),
            keyboards::UiLanguage::En => format!(
                "{} total · {} active · {} paused",
                watches.len(),
                watch_active,
                watch_paused
            ),
        };
        push_field(&mut lines, "Watch", &watch_summary);
    } else {
        push_field(&mut lines, "Watch", home_watch_disabled(lang));
    }
    lines.push(String::new());
    lines.push(home_hint(lang).to_string());
    Ok(lines.join("\n"))
}

pub(crate) fn home_title(lang: keyboards::UiLanguage) -> &'static str {
    match lang {
        keyboards::UiLanguage::Vi => "502DRIVE CONTROL CENTER",
        keyboards::UiLanguage::En => "502DRIVE CONTROL CENTER",
    }
}

pub(crate) fn home_destination_label(lang: keyboards::UiLanguage) -> &'static str {
    match lang {
        keyboards::UiLanguage::Vi => "Đích mặc định",
        keyboards::UiLanguage::En => "Default destination",
    }
}

pub(crate) fn home_destination_missing(lang: keyboards::UiLanguage) -> &'static str {
    match lang {
        keyboards::UiLanguage::Vi => "Chưa đặt",
        keyboards::UiLanguage::En => "Not set",
    }
}

pub(crate) fn home_jobs_label(lang: keyboards::UiLanguage) -> &'static str {
    match lang {
        keyboards::UiLanguage::Vi => "Job đang chạy",
        keyboards::UiLanguage::En => "Active jobs",
    }
}

pub(crate) fn home_watch_disabled(lang: keyboards::UiLanguage) -> &'static str {
    match lang {
        keyboards::UiLanguage::Vi => "Đang tắt trong config",
        keyboards::UiLanguage::En => "Disabled in config",
    }
}

pub(crate) fn home_hint(lang: keyboards::UiLanguage) -> &'static str {
    match lang {
        keyboards::UiLanguage::Vi => "Chọn một mục bên dưới để xem tiếp.",
        keyboards::UiLanguage::En => "Choose an item below to continue.",
    }
}

pub(crate) fn account_title(lang: keyboards::UiLanguage) -> &'static str {
    match lang {
        keyboards::UiLanguage::Vi => "TÀI KHOẢN GOOGLE",
        keyboards::UiLanguage::En => "GOOGLE ACCOUNT",
    }
}

pub(crate) fn account_status_field(lang: keyboards::UiLanguage) -> &'static str {
    match lang {
        keyboards::UiLanguage::Vi => "Trạng thái",
        keyboards::UiLanguage::En => "Status",
    }
}

pub(crate) fn account_name_field(lang: keyboards::UiLanguage) -> &'static str {
    match lang {
        keyboards::UiLanguage::Vi => "Tên",
        keyboards::UiLanguage::En => "Name",
    }
}

pub(crate) fn account_login_hint(lang: keyboards::UiLanguage) -> &'static str {
    match lang {
        keyboards::UiLanguage::Vi => "Chạy trên máy đang chạy bot:",
        keyboards::UiLanguage::En => "Run this on the machine running the bot:",
    }
}

pub(crate) fn language_panel_text(lang: keyboards::UiLanguage) -> String {
    match lang {
        keyboards::UiLanguage::Vi => {
            "NGÔN NGỮ\n━━━━━━━━━━\nHiện tại: Tiếng Việt\n\nBấm English để đổi ngay cho tài khoản Telegram này.".to_string()
        }
        keyboards::UiLanguage::En => {
            "LANGUAGE\n━━━━━━━━━━\nCurrent: English\n\nTap Tiếng Việt to switch this Telegram account immediately.".to_string()
        }
    }
}

pub(crate) fn language_from_callback(code: &str) -> Option<keyboards::UiLanguage> {
    match code {
        "vi" => Some(keyboards::UiLanguage::Vi),
        "en" => Some(keyboards::UiLanguage::En),
        _ => None,
    }
}

pub(crate) fn language_changed_text(lang: keyboards::UiLanguage) -> String {
    match lang {
        keyboards::UiLanguage::Vi => {
            "Đã đổi sang Tiếng Việt.\n\nCác panel và nút mới sẽ dùng Tiếng Việt ngay.".to_string()
        }
        keyboards::UiLanguage::En => {
            "Switched to English.\n\nNew panels and buttons will use English immediately."
                .to_string()
        }
    }
}

pub(crate) fn language_invalid_text(lang: keyboards::UiLanguage) -> String {
    match lang {
        keyboards::UiLanguage::Vi => {
            "Ngôn ngữ không hợp lệ. Chọn Tiếng Việt hoặc English.".to_string()
        }
        keyboards::UiLanguage::En => "Invalid language. Choose Tiếng Việt or English.".to_string(),
    }
}

pub(crate) fn language_save_error(lang: keyboards::UiLanguage, err: &anyhow::Error) -> String {
    match lang {
        keyboards::UiLanguage::Vi => format!("Lỗi lưu ngôn ngữ: {err}"),
        keyboards::UiLanguage::En => format!("Could not save language: {err}"),
    }
}

pub(crate) fn account_destination_title(lang: keyboards::UiLanguage) -> &'static str {
    match lang {
        keyboards::UiLanguage::Vi => "THƯ MỤC ĐÍCH",
        keyboards::UiLanguage::En => "DESTINATION FOLDER",
    }
}

pub(crate) fn account_parent_id_field(lang: keyboards::UiLanguage) -> &'static str {
    match lang {
        keyboards::UiLanguage::Vi => "Parent ID",
        keyboards::UiLanguage::En => "Parent ID",
    }
}

pub(crate) fn account_destination_missing(lang: keyboards::UiLanguage) -> &'static str {
    match lang {
        keyboards::UiLanguage::Vi => "Chưa đặt. Dùng /set_destination <folder_url>.",
        keyboards::UiLanguage::En => "Not set. Use /set_destination <folder_url>.",
    }
}

pub(crate) fn vi_account_status(status: &str) -> &str {
    match status {
        "connected" => "Đã kết nối",
        "reconnect_required" => "Cần đăng nhập lại",
        "revoked" => "Đã thu hồi",
        "disabled" => "Đã tắt",
        other => other,
    }
}

pub(crate) fn account_status_label(lang: keyboards::UiLanguage, status: &str) -> &str {
    if lang == keyboards::UiLanguage::Vi {
        return vi_account_status(status);
    }
    match status {
        "connected" => "Connected",
        "reconnect_required" => "Reconnect required",
        "revoked" => "Revoked",
        "disabled" => "Disabled",
        "chưa kết nối" => "Not connected",
        other => other,
    }
}
