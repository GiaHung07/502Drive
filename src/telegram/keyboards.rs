#[cfg(test)]
use teloxide::types::InlineKeyboardButtonKind;
use teloxide::types::{InlineKeyboardButton, InlineKeyboardMarkup};

/// Callback data of a button, if it is a callback button.
#[cfg(test)]
fn callback_of(button: &InlineKeyboardButton) -> Option<&str> {
    match &button.kind {
        InlineKeyboardButtonKind::CallbackData(data) => Some(data.as_ref()),
        _ => None,
    }
}

use crate::telegram::i18n::TextKey as T;
pub use crate::telegram::i18n::UiLanguage;

pub fn confirm_clone_keyboard(state_id: &str, lang: UiLanguage) -> InlineKeyboardMarkup {
    InlineKeyboardMarkup::new([[
        InlineKeyboardButton::callback(lang.text(T::CloneNow), format!("clone:confirm:{state_id}")),
        InlineKeyboardButton::callback(lang.text(T::Cancel), format!("clone:cancel:{state_id}")),
    ]])
}

pub fn smart_link_action_keyboard(state_id: &str, lang: UiLanguage) -> InlineKeyboardMarkup {
    let (clone_label, sync_label, dest_label, cancel_label) = match lang {
        UiLanguage::Vi => (
            "Sao chép (Clone)",
            "Đồng bộ (Realtime Sync)",
            "Đổi thư mục đích",
            "Huỷ",
        ),
        UiLanguage::En => ("Clone now", "Realtime Sync", "Change destination", "Cancel"),
    };
    InlineKeyboardMarkup::new([
        vec![InlineKeyboardButton::callback(
            clone_label,
            format!("smart:clone:{state_id}"),
        )],
        vec![InlineKeyboardButton::callback(
            sync_label,
            format!("smart:sync:{state_id}"),
        )],
        vec![
            InlineKeyboardButton::callback(dest_label, "menu:open:destination"),
            InlineKeyboardButton::callback(cancel_label, format!("smart:cancel:{state_id}")),
        ],
    ])
}

pub fn unified_inspect_keyboard(
    session_id: &str,
    is_folder: bool,
    watch_enabled: bool,
    lang: UiLanguage,
) -> InlineKeyboardMarkup {
    let (clone_label, sync_label, dest_label, cancel_label) = match lang {
        UiLanguage::Vi => (
            "Sao chép ngay",
            "⟳ Theo dõi realtime",
            "Đổi thư mục đích",
            "Hủy",
        ),
        UiLanguage::En => (
            "Clone now",
            "⟳ Realtime Sync",
            "Change destination",
            "Cancel",
        ),
    };

    let mut rows = Vec::new();
    if is_folder && watch_enabled {
        rows.push(vec![
            InlineKeyboardButton::callback(clone_label, format!("insp:clone:{session_id}")),
            InlineKeyboardButton::callback(sync_label, format!("insp:watch:{session_id}")),
        ]);
    } else {
        rows.push(vec![InlineKeyboardButton::callback(
            clone_label,
            format!("insp:clone:{session_id}"),
        )]);
    }
    rows.push(vec![
        InlineKeyboardButton::callback(dest_label, format!("insp:dest:{session_id}")),
        InlineKeyboardButton::callback(cancel_label, format!("insp:cancel:{session_id}")),
    ]);

    InlineKeyboardMarkup::new(rows)
}

pub fn job_control_keyboard(job_id: &str, paused: bool, lang: UiLanguage) -> InlineKeyboardMarkup {
    let primary = if paused {
        InlineKeyboardButton::callback(lang.text(T::Resume), format!("job:resume:{job_id}"))
    } else {
        InlineKeyboardButton::callback(lang.text(T::Pause), format!("job:pause:{job_id}"))
    };
    InlineKeyboardMarkup::new([[
        primary,
        InlineKeyboardButton::callback(lang.text(T::Cancel), format!("job:cancel:{job_id}")),
    ]])
}

pub fn main_menu_keyboard(watch_enabled: bool, lang: UiLanguage) -> InlineKeyboardMarkup {
    let mut rows = Vec::new();
    let mut first_row = vec![InlineKeyboardButton::callback(
        lang.text(T::MenuClone),
        "menu:prompt:clone",
    )];
    if watch_enabled {
        first_row.push(InlineKeyboardButton::callback(
            lang.text(T::MenuWatch),
            "menu:prompt:watch",
        ));
    }
    rows.push(first_row);
    rows.push(vec![
        InlineKeyboardButton::callback(lang.text(T::MenuJobs), "menu:open:jobs"),
        InlineKeyboardButton::callback(lang.text(T::Destination), "menu:open:destination"),
    ]);
    rows.push(vec![InlineKeyboardButton::callback(
        lang.text(T::MenuSettings),
        "menu:open:account",
    )]);
    InlineKeyboardMarkup::new(rows)
}

pub fn back_home_keyboard(lang: UiLanguage) -> InlineKeyboardMarkup {
    InlineKeyboardMarkup::new([[InlineKeyboardButton::callback(
        lang.text(T::BackHome),
        "menu:open:home",
    )]])
}

pub fn account_keyboard(lang: UiLanguage) -> InlineKeyboardMarkup {
    InlineKeyboardMarkup::new([
        [InlineKeyboardButton::callback(
            language_button_label(lang),
            "lang:open:panel",
        )],
        [InlineKeyboardButton::callback(
            lang.text(T::BackHome),
            "menu:open:home",
        )],
    ])
}

pub fn language_keyboard(lang: UiLanguage) -> InlineKeyboardMarkup {
    InlineKeyboardMarkup::new([
        vec![
            InlineKeyboardButton::callback("Tiếng Việt", "lang:set:vi"),
            InlineKeyboardButton::callback("English", "lang:set:en"),
        ],
        vec![
            InlineKeyboardButton::callback(lang.text(T::Account), "menu:open:account"),
            InlineKeyboardButton::callback(lang.text(T::BackHome), "menu:open:home"),
        ],
    ])
}

fn language_button_label(lang: UiLanguage) -> &'static str {
    match lang {
        UiLanguage::Vi => "Ngôn ngữ",
        UiLanguage::En => "Language",
    }
}

pub fn job_list_keyboard(jobs: &[(String, String)], lang: UiLanguage) -> InlineKeyboardMarkup {
    let mut rows: Vec<Vec<InlineKeyboardButton>> = jobs
        .iter()
        .map(|(id, label)| {
            vec![InlineKeyboardButton::callback(
                truncate_label(label, 44),
                format!("job:status:{id}"),
            )]
        })
        .collect();
    rows.push(vec![
        InlineKeyboardButton::callback(lang.text(T::Refresh), "menu:open:jobs"),
        InlineKeyboardButton::callback(lang.text(T::BackHome), "menu:open:home"),
    ]);
    InlineKeyboardMarkup::new(rows)
}

pub fn job_detail_keyboard(job_id: &str, status: &str, lang: UiLanguage) -> InlineKeyboardMarkup {
    let mut rows = Vec::new();
    if !matches!(
        status,
        "completed" | "partially_completed" | "failed" | "cancelled"
    ) {
        let primary = if status == "paused" {
            InlineKeyboardButton::callback(lang.text(T::Resume), format!("job:resume:{job_id}"))
        } else {
            InlineKeyboardButton::callback(lang.text(T::Pause), format!("job:pause:{job_id}"))
        };
        rows.push(vec![
            primary,
            InlineKeyboardButton::callback(lang.text(T::Cancel), format!("job:cancel:{job_id}")),
        ]);
    }
    if matches!(status, "partially_completed" | "failed") {
        rows.push(vec![InlineKeyboardButton::callback(
            lang.text(T::RetryFailed),
            format!("job:retry:{job_id}"),
        )]);
    }
    if matches!(status, "completed" | "partially_completed" | "failed") {
        rows.push(vec![InlineKeyboardButton::callback(
            lang.text(T::Report),
            format!("job:report:{job_id}"),
        )]);
    }
    rows.push(vec![
        InlineKeyboardButton::callback(lang.text(T::Refresh), format!("job:status:{job_id}")),
        InlineKeyboardButton::callback(lang.text(T::Jobs), "menu:open:jobs"),
    ]);
    rows.push(vec![InlineKeyboardButton::callback(
        lang.text(T::BackHome),
        "menu:open:home",
    )]);
    InlineKeyboardMarkup::new(rows)
}

pub fn job_cancel_confirm_keyboard(job_id: &str, lang: UiLanguage) -> InlineKeyboardMarkup {
    InlineKeyboardMarkup::new([
        vec![
            InlineKeyboardButton::callback(
                lang.text(T::Cancel),
                format!("job:cancel_confirm:{job_id}"),
            ),
            InlineKeyboardButton::callback(lang.text(T::KeepJob), format!("job:status:{job_id}")),
        ],
        vec![InlineKeyboardButton::callback(
            lang.text(T::Jobs),
            "menu:open:jobs",
        )],
    ])
}

pub fn watch_list_keyboard(
    watches: &[(String, String)],
    mode: &str,
    page: usize,
    has_prev: bool,
    has_next: bool,
    lang: UiLanguage,
) -> InlineKeyboardMarkup {
    let mut rows: Vec<Vec<InlineKeyboardButton>> = watches
        .iter()
        .map(|(id, label)| {
            vec![InlineKeyboardButton::callback(
                truncate_label(label, 44),
                format!("watch:{mode}:{id}"),
            )]
        })
        .collect();

    let mut nav = Vec::new();
    if has_prev {
        nav.push(InlineKeyboardButton::callback(
            lang.text(T::PreviousPage),
            format!("watch:list:{mode}:{}", page - 1),
        ));
    }
    if has_next {
        nav.push(InlineKeyboardButton::callback(
            lang.text(T::NextPage),
            format!("watch:list:{mode}:{}", page + 1),
        ));
    }
    if !nav.is_empty() {
        rows.push(nav);
    }
    rows.push(vec![
        InlineKeyboardButton::callback(lang.text(T::Refresh), format!("watch:list:{mode}:{page}")),
        InlineKeyboardButton::callback(lang.text(T::BackHome), "menu:open:home"),
    ]);
    InlineKeyboardMarkup::new(rows)
}

pub fn watch_detail_keyboard(
    watch_id: &str,
    status: &str,
    lang: UiLanguage,
) -> InlineKeyboardMarkup {
    let mut rows = Vec::new();
    let short_id = watch_id.get(..8).unwrap_or(watch_id);
    rows.push(vec![InlineKeyboardButton::callback(
        lang.text(T::Refresh),
        format!("watch:status:{watch_id}"),
    )]);
    if matches!(status, "active" | "catching_up" | "degraded") {
        rows.push(vec![InlineKeyboardButton::callback(
            lang.text(T::Pause),
            format!("watch:pause:{watch_id}"),
        )]);
    } else if status == "paused" {
        rows.push(vec![InlineKeyboardButton::callback(
            lang.text(T::Resume),
            format!("watch:resume:{watch_id}"),
        )]);
    }
    rows.push(vec![
        InlineKeyboardButton::callback(lang.text(T::Versioned), format!("watch:pol:{short_id}:v")),
        InlineKeyboardButton::callback(lang.text(T::Replace), format!("watch:pol:{short_id}:r")),
        InlineKeyboardButton::callback(lang.text(T::Manual), format!("watch:pol:{short_id}:m")),
    ]);
    if status != "stopped" {
        rows.push(vec![InlineKeyboardButton::callback(
            lang.text(T::StopWatching),
            format!("watch:unwatch:{watch_id}"),
        )]);
    }
    rows.push(vec![
        InlineKeyboardButton::callback(lang.text(T::Watches), "menu:open:watches"),
        InlineKeyboardButton::callback(lang.text(T::BackHome), "menu:open:home"),
    ]);
    InlineKeyboardMarkup::new(rows)
}

pub fn watch_unwatch_confirm_keyboard(watch_id: &str, lang: UiLanguage) -> InlineKeyboardMarkup {
    InlineKeyboardMarkup::new([
        vec![
            InlineKeyboardButton::callback(
                lang.text(T::StopWatching),
                format!("watch:unwatch_confirm:{watch_id}"),
            ),
            InlineKeyboardButton::callback(
                lang.text(T::KeepWatch),
                format!("watch:status:{watch_id}"),
            ),
        ],
        vec![InlineKeyboardButton::callback(
            lang.text(T::Watches),
            "menu:open:watches",
        )],
    ])
}

/// One row per destination, label truncated to 32 chars.
/// Callback: `dest:select:<profile_id>`
pub fn recent_destinations_keyboard(
    profiles: &[crate::state::repo::DestinationProfile],
    lang: UiLanguage,
) -> InlineKeyboardMarkup {
    let rows: Vec<Vec<InlineKeyboardButton>> = profiles
        .iter()
        .map(|p| {
            let marker = if p.is_default {
                lang.text(T::BadgeDefault)
            } else {
                ""
            };
            let drive = if p.destination_drive_id.is_some() {
                lang.text(T::BadgeSharedDrive)
            } else {
                "[My]"
            };
            let label = truncate_label(&format!("{marker}{drive} {}", p.label), 32);
            vec![InlineKeyboardButton::callback(
                label,
                format!("dest:select:{}", p.id),
            )]
        })
        .collect();
    InlineKeyboardMarkup::new(rows)
}

pub fn destination_panel_keyboard(
    profiles: &[crate::state::repo::DestinationProfile],
    my_drive_state_id: &str,
    shared_drives_state_id: &str,
    lang: UiLanguage,
) -> InlineKeyboardMarkup {
    let mut rows = recent_destinations_keyboard(profiles, lang).inline_keyboard;
    rows.push(vec![InlineKeyboardButton::callback(
        lang.text(T::BrowseMyDrive),
        format!("browse:open:{my_drive_state_id}"),
    )]);
    rows.push(vec![InlineKeyboardButton::callback(
        lang.text(T::BrowseSharedDrive),
        format!("browse:open:{shared_drives_state_id}"),
    )]);
    rows.push(vec![InlineKeyboardButton::callback(
        lang.text(T::BackHome),
        "menu:open:home",
    )]);
    InlineKeyboardMarkup::new(rows)
}

pub fn destination_browser_keyboard(
    pick_state_id: Option<&str>,
    parent_state_id: Option<&str>,
    next_state_id: Option<&str>,
    child_states: &[(String, String)],
    lang: UiLanguage,
) -> InlineKeyboardMarkup {
    let mut rows = Vec::new();
    if let Some(state_id) = pick_state_id {
        rows.push(vec![InlineKeyboardButton::callback(
            lang.text(T::SelectThisFolder),
            format!("browse:pick:{state_id}"),
        )]);
    }
    if let Some(state_id) = parent_state_id {
        rows.push(vec![InlineKeyboardButton::callback(
            lang.text(T::UpOneLevel),
            format!("browse:open:{state_id}"),
        )]);
    }
    rows.extend(child_states.iter().map(|(label, state_id)| {
        vec![InlineKeyboardButton::callback(
            truncate_label(label, 40),
            format!("browse:open:{state_id}"),
        )]
    }));
    if let Some(state_id) = next_state_id {
        rows.push(vec![InlineKeyboardButton::callback(
            lang.text(T::NextPage),
            format!("browse:open:{state_id}"),
        )]);
    }
    rows.push(vec![
        InlineKeyboardButton::callback(lang.text(T::Destination), "menu:open:destination"),
        InlineKeyboardButton::callback(lang.text(T::BackHome), "menu:open:home"),
    ]);
    InlineKeyboardMarkup::new(rows)
}

fn truncate_label(label: &str, max_chars: usize) -> String {
    if label.chars().count() <= max_chars {
        return label.to_string();
    }
    let keep = max_chars.saturating_sub(3);
    format!("{}...", label.chars().take(keep).collect::<String>())
}

#[cfg(test)]
mod tests {
    use super::{
        UiLanguage, account_keyboard, callback_of, destination_browser_keyboard,
        destination_panel_keyboard, job_cancel_confirm_keyboard, job_detail_keyboard,
        language_keyboard, main_menu_keyboard, recent_destinations_keyboard,
        smart_link_action_keyboard, truncate_label, watch_detail_keyboard, watch_list_keyboard,
        watch_unwatch_confirm_keyboard,
    };

    #[test]
    fn truncates_unicode_on_char_boundary() {
        assert_eq!(
            truncate_label("Thư mục tiếng Việt rất dài", 12),
            "Thư mục t..."
        );
    }

    #[test]
    fn destination_browser_can_render_next_page() {
        let keyboard =
            destination_browser_keyboard(None, None, Some("next-state"), &[], UiLanguage::Vi);
        assert_eq!(keyboard.inline_keyboard[0][0].text, "Trang sau");
    }

    #[test]
    fn destination_panel_can_open_shared_drives() {
        let keyboard = destination_panel_keyboard(&[], "my-state", "shared-state", UiLanguage::Vi);
        assert_eq!(keyboard.inline_keyboard[1][0].text, "Duyệt Shared Drive");
    }

    #[test]
    fn recent_destination_buttons_show_drive_and_default_markers() {
        let profiles = vec![crate::state::repo::DestinationProfile {
            id: "profile-id".to_string(),
            google_account_id: "default".to_string(),
            label: "Folder".to_string(),
            destination_parent_id: "folder-id".to_string(),
            destination_drive_id: Some("shared-drive-id".to_string()),
            destination_resource_key: None,
            is_default: true,
        }];
        let keyboard = recent_destinations_keyboard(&profiles, UiLanguage::Vi);
        assert_eq!(
            keyboard.inline_keyboard[0][0].text,
            "[mặc định] [SD] Folder"
        );
    }

    #[test]
    fn watch_list_can_render_next_page() {
        let keyboard = watch_list_keyboard(
            &[("watch-id".to_string(), "Nguồn -> Đích".to_string())],
            "status",
            0,
            false,
            true,
            UiLanguage::Vi,
        );
        assert_eq!(keyboard.inline_keyboard[0][0].text, "Nguồn -> Đích");
        assert_eq!(keyboard.inline_keyboard[1][0].text, "Trang sau");
    }

    #[test]
    fn watch_detail_uses_resume_for_paused_watch() {
        let keyboard = watch_detail_keyboard("watch-id", "paused", UiLanguage::Vi);
        assert_eq!(keyboard.inline_keyboard[1][0].text, "Tiếp tục");
    }

    #[test]
    fn watch_policy_buttons_follow_language() {
        let vi = watch_detail_keyboard("watch-id", "active", UiLanguage::Vi);
        assert_eq!(vi.inline_keyboard[2][0].text, "Tạo bản mới");
        assert_eq!(vi.inline_keyboard[2][1].text, "Thay bản cũ");
        assert_eq!(vi.inline_keyboard[2][2].text, "Xác nhận tay");

        let en = watch_detail_keyboard("watch-id", "active", UiLanguage::En);
        assert_eq!(en.inline_keyboard[2][0].text, "Versioned");
        assert_eq!(en.inline_keyboard[2][1].text, "Replace");
        assert_eq!(en.inline_keyboard[2][2].text, "Manual");
    }

    #[test]
    fn main_menu_can_hide_watch_actions() {
        let disabled = main_menu_keyboard(false, UiLanguage::Vi);
        assert!(
            !disabled
                .inline_keyboard
                .iter()
                .flatten()
                .any(|button| button.text == "⟳ Theo dõi")
        );

        let enabled = main_menu_keyboard(true, UiLanguage::Vi);
        assert_eq!(enabled.inline_keyboard[0][0].text, "＋ Sao chép");
        assert_eq!(enabled.inline_keyboard[0][1].text, "⟳ Theo dõi");
        assert_eq!(enabled.inline_keyboard[1][0].text, "Công việc");
        assert_eq!(enabled.inline_keyboard[1][1].text, "Thư mục đích");
        assert_eq!(enabled.inline_keyboard[2][0].text, "Cài đặt");
        assert_eq!(
            callback_of(&enabled.inline_keyboard[0][0]),
            Some("menu:prompt:clone")
        );
        assert_eq!(
            callback_of(&enabled.inline_keyboard[0][1]),
            Some("menu:prompt:watch")
        );
        assert_eq!(
            callback_of(&enabled.inline_keyboard[1][0]),
            Some("menu:open:jobs")
        );
        assert_eq!(
            callback_of(&enabled.inline_keyboard[1][1]),
            Some("menu:open:destination")
        );
        assert_eq!(
            callback_of(&enabled.inline_keyboard[2][0]),
            Some("menu:open:account")
        );
    }

    #[test]
    fn account_keyboard_exposes_language_panel() {
        let vi = account_keyboard(UiLanguage::Vi);
        assert_eq!(vi.inline_keyboard[0][0].text, "Ngôn ngữ");

        let en = language_keyboard(UiLanguage::En);
        assert!(
            en.inline_keyboard
                .iter()
                .flatten()
                .any(|button| button.text == "Tiếng Việt")
        );
    }

    #[test]
    fn job_detail_has_back_to_jobs() {
        let keyboard = job_detail_keyboard("job-id", "running", UiLanguage::Vi);
        assert!(
            keyboard
                .inline_keyboard
                .iter()
                .flatten()
                .any(|button| button.text == "Jobs")
        );
    }

    #[test]
    fn failed_job_detail_can_retry_failed_items() {
        let keyboard = job_detail_keyboard("job-id", "failed", UiLanguage::Vi);
        assert!(
            keyboard
                .inline_keyboard
                .iter()
                .flatten()
                .any(|button| button.text == "Retry lỗi")
        );
        assert!(
            keyboard
                .inline_keyboard
                .iter()
                .flatten()
                .any(|button| button.text == "Report")
        );
    }

    #[test]
    fn completed_job_detail_can_send_report() {
        let keyboard = job_detail_keyboard("job-id", "completed", UiLanguage::Vi);
        assert!(
            keyboard
                .inline_keyboard
                .iter()
                .flatten()
                .any(|button| button.text == "Report")
        );
    }

    #[test]
    fn job_cancel_confirmation_can_keep_job() {
        let keyboard = job_cancel_confirm_keyboard("job-id", UiLanguage::Vi);
        assert_eq!(keyboard.inline_keyboard[0][0].text, "Huỷ");
        assert_eq!(keyboard.inline_keyboard[0][1].text, "Giữ job");
        assert_eq!(keyboard.inline_keyboard[1][0].text, "Jobs");
    }

    #[test]
    fn watch_unwatch_confirmation_can_keep_watch() {
        let keyboard = watch_unwatch_confirm_keyboard("watch-id", UiLanguage::Vi);
        assert_eq!(keyboard.inline_keyboard[0][0].text, "Dừng theo dõi");
        assert_eq!(keyboard.inline_keyboard[0][1].text, "Giữ watch");
        assert_eq!(keyboard.inline_keyboard[1][0].text, "Đồng bộ (Sync)");
    }

    #[test]
    fn smart_link_action_keyboard_renders_clone_and_sync() {
        let kb = smart_link_action_keyboard("state-123", UiLanguage::Vi);
        assert_eq!(kb.inline_keyboard[0][0].text, "Sao chép (Clone)");
        assert_eq!(kb.inline_keyboard[1][0].text, "Đồng bộ (Realtime Sync)");
        assert_eq!(kb.inline_keyboard[2][0].text, "Đổi thư mục đích");
        assert_eq!(kb.inline_keyboard[2][1].text, "Huỷ");
    }
}
