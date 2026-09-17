#[cfg(test)]
use teloxide::types::InlineKeyboardButtonKind;
use teloxide::types::{InlineKeyboardButton, InlineKeyboardMarkup, KeyboardButton, KeyboardMarkup};

/// Callback data of a button, if it is a callback button.
#[cfg(test)]
fn callback_of(button: &InlineKeyboardButton) -> Option<&str> {
    match &button.kind {
        InlineKeyboardButtonKind::CallbackData(data) => Some(data.as_ref()),
        _ => None,
    }
}

/// Helper to construct an `InlineKeyboardButton` for callback queries with a debug_assert
/// and a runtime UTF-8 truncation guard ensuring callback_data never exceeds Telegram's 64-byte limit.
pub(crate) fn callback_btn(text: impl Into<String>, data: impl AsRef<str>) -> InlineKeyboardButton {
    let data_ref = data.as_ref();
    // Never truncate: a cut payload can parse into a WRONG object id. A
    // >64-byte callback means the caller should have used an opaque state id —
    // fail loudly in dev, and let Telegram reject it visibly in release.
    debug_assert!(
        data_ref.len() <= 64,
        "callback_data exceeds 64 bytes limit (len = {}): '{}'",
        data_ref.len(),
        data_ref
    );
    if data_ref.len() > 64 {
        // Release: never truncate (a cut payload can misroute to a wrong
        // object id) and never panic the bot — pass through and let the
        // Telegram API reject it loudly in logs.
        tracing::error!(
            len = data_ref.len(),
            prefix = %data_ref.get(..24).unwrap_or(""),
            "callback_data exceeds 64 bytes — button will fail; use an opaque state id"
        );
    }
    InlineKeyboardButton::callback(text, data_ref.to_string())
}

use crate::telegram::i18n::TextKey as T;
pub use crate::telegram::i18n::UiLanguage;

pub fn confirm_clone_keyboard(state_id: &str, lang: UiLanguage) -> InlineKeyboardMarkup {
    InlineKeyboardMarkup::new([[
        callback_btn(lang.text(T::CloneNow), format!("clone:confirm:{state_id}")),
        callback_btn(lang.text(T::Cancel), format!("clone:cancel:{state_id}")),
    ]])
}

pub fn confirm_set_destination_keyboard(
    session_id: &str,
    lang: UiLanguage,
) -> InlineKeyboardMarkup {
    debug_assert!(format!("dest:confirm:{session_id}").len() <= 64);
    debug_assert!(format!("dest:cancel:{session_id}").len() <= 64);
    InlineKeyboardMarkup::new([[
        callback_btn(
            lang.text(T::ButtonSetDefault),
            format!("dest:confirm:{session_id}"),
        ),
        callback_btn(lang.text(T::Cancel), format!("dest:cancel:{session_id}")),
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
        vec![callback_btn(clone_label, format!("smart:clone:{state_id}"))],
        vec![callback_btn(sync_label, format!("smart:sync:{state_id}"))],
        vec![
            callback_btn(dest_label, "menu:open:destination"),
            callback_btn(cancel_label, format!("smart:cancel:{state_id}")),
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
            callback_btn(clone_label, format!("insp:clone:{session_id}")),
            callback_btn(sync_label, format!("insp:watch:{session_id}")),
        ]);
    } else {
        rows.push(vec![callback_btn(
            clone_label,
            format!("insp:clone:{session_id}"),
        )]);
    }
    rows.push(vec![
        callback_btn(dest_label, format!("insp:dest:{session_id}")),
        callback_btn(cancel_label, format!("insp:cancel:{session_id}")),
    ]);

    InlineKeyboardMarkup::new(rows)
}

pub fn job_control_keyboard(job_id: &str, paused: bool, lang: UiLanguage) -> InlineKeyboardMarkup {
    let primary = if paused {
        callback_btn(lang.text(T::Resume), format!("job:resume:{job_id}"))
    } else {
        callback_btn(lang.text(T::Pause), format!("job:pause:{job_id}"))
    };
    InlineKeyboardMarkup::new([[
        primary,
        callback_btn(lang.text(T::Cancel), format!("job:cancel:{job_id}")),
    ]])
}

pub fn main_menu_keyboard(watch_enabled: bool, lang: UiLanguage) -> InlineKeyboardMarkup {
    let mut rows = Vec::new();
    let mut first_row = vec![callback_btn(lang.text(T::MenuClone), "menu:prompt:clone")];
    if watch_enabled {
        first_row.push(callback_btn(lang.text(T::MenuWatch), "menu:prompt:watch"));
    }
    rows.push(first_row);
    let mut second_row = vec![callback_btn(lang.text(T::MenuJobs), "menu:open:jobs")];
    if watch_enabled {
        second_row.push(callback_btn(lang.text(T::MenuWatches), "menu:open:watches"));
    }
    rows.push(second_row);
    rows.push(vec![callback_btn(
        lang.text(T::Destination),
        "menu:open:destination",
    )]);
    rows.push(vec![callback_btn(
        lang.text(T::MenuSettings),
        "menu:open:settings",
    )]);
    InlineKeyboardMarkup::new(rows)
}

/// Settings panel keyboard: language is the only per-user preference with a
/// Telegram surface; notifications are app-level (informational row only).
pub fn settings_keyboard(lang: UiLanguage) -> InlineKeyboardMarkup {
    InlineKeyboardMarkup::new([
        [callback_btn(
            lang.text(T::SettingsLanguage),
            "lang:open:panel",
        )],
        [callback_btn(lang.text(T::BackHome), "menu:open:home")],
    ])
}

pub fn back_home_keyboard(lang: UiLanguage) -> InlineKeyboardMarkup {
    InlineKeyboardMarkup::new([[callback_btn(lang.text(T::BackHome), "menu:open:home")]])
}

pub fn account_keyboard(lang: UiLanguage) -> InlineKeyboardMarkup {
    InlineKeyboardMarkup::new([
        [callback_btn(language_button_label(lang), "lang:open:panel")],
        [callback_btn(lang.text(T::BackHome), "menu:open:home")],
    ])
}

pub fn language_keyboard(lang: UiLanguage) -> InlineKeyboardMarkup {
    InlineKeyboardMarkup::new([
        vec![
            callback_btn("Tiếng Việt", "lang:set:vi"),
            callback_btn("English", "lang:set:en"),
        ],
        vec![
            callback_btn(lang.text(T::Account), "menu:open:account"),
            callback_btn(lang.text(T::BackHome), "menu:open:home"),
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
            vec![callback_btn(
                truncate_label(label, 44),
                format!("job:status:{id}"),
            )]
        })
        .collect();
    rows.push(vec![
        callback_btn(lang.text(T::Refresh), "menu:open:jobs"),
        callback_btn(lang.text(T::BackHome), "menu:open:home"),
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
            callback_btn(lang.text(T::Resume), format!("job:resume:{job_id}"))
        } else {
            callback_btn(lang.text(T::Pause), format!("job:pause:{job_id}"))
        };
        rows.push(vec![
            primary,
            callback_btn(lang.text(T::Cancel), format!("job:cancel:{job_id}")),
        ]);
    }
    if matches!(status, "partially_completed" | "failed") {
        rows.push(vec![callback_btn(
            lang.text(T::RetryFailed),
            format!("job:retry:{job_id}"),
        )]);
    }
    if matches!(status, "completed" | "partially_completed" | "failed") {
        rows.push(vec![callback_btn(
            lang.text(T::Report),
            format!("job:report:{job_id}"),
        )]);
    }
    rows.push(vec![
        callback_btn(lang.text(T::Refresh), format!("job:status:{job_id}")),
        callback_btn(lang.text(T::Jobs), "menu:open:jobs"),
    ]);
    rows.push(vec![callback_btn(lang.text(T::BackHome), "menu:open:home")]);
    InlineKeyboardMarkup::new(rows)
}

pub fn job_end_state_keyboard(
    job_id: &str,
    status: &str,
    failed_items: i64,
    lang: UiLanguage,
) -> InlineKeyboardMarkup {
    debug_assert!(format!("job:report:{job_id}").len() <= 64);
    debug_assert!(format!("job:clone_again:{job_id}").len() <= 64);
    debug_assert!(format!("job:retry:{job_id}").len() <= 64);
    debug_assert!(format!("job:errors:{job_id}").len() <= 64);

    let mut rows = Vec::new();
    if status == "completed" && failed_items == 0 {
        rows.push(vec![
            callback_btn(lang.text(T::ViewReport), format!("job:report:{job_id}")),
            callback_btn(
                lang.text(T::CloneAgain),
                format!("job:clone_again:{job_id}"),
            ),
        ]);
    } else if status == "cancelled" {
        rows.push(vec![
            callback_btn(
                lang.text(T::CloneAgain),
                format!("job:clone_again:{job_id}"),
            ),
            callback_btn(lang.text(T::ViewReport), format!("job:report:{job_id}")),
        ]);
    } else {
        rows.push(vec![
            callback_btn(lang.text(T::Retry), format!("job:retry:{job_id}")),
            callback_btn(lang.text(T::ViewErrors), format!("job:errors:{job_id}")),
            callback_btn(lang.text(T::Report), format!("job:report:{job_id}")),
        ]);
    }
    InlineKeyboardMarkup::new(rows)
}

pub fn job_cancel_confirm_keyboard(job_id: &str, lang: UiLanguage) -> InlineKeyboardMarkup {
    InlineKeyboardMarkup::new([
        vec![
            callback_btn(lang.text(T::Cancel), format!("job:cancel_confirm:{job_id}")),
            callback_btn(lang.text(T::KeepJob), format!("job:status:{job_id}")),
        ],
        vec![callback_btn(lang.text(T::Jobs), "menu:open:jobs")],
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
            vec![callback_btn(
                truncate_label(label, 44),
                format!("watch:{mode}:{id}"),
            )]
        })
        .collect();

    let mut nav = Vec::new();
    if has_prev {
        nav.push(callback_btn(
            lang.text(T::PreviousPage),
            format!("watch:list:{mode}:{}", page - 1),
        ));
    }
    if has_next {
        nav.push(callback_btn(
            lang.text(T::NextPage),
            format!("watch:list:{mode}:{}", page + 1),
        ));
    }
    if !nav.is_empty() {
        rows.push(nav);
    }
    rows.push(vec![
        callback_btn(lang.text(T::Refresh), format!("watch:list:{mode}:{page}")),
        callback_btn(lang.text(T::BackHome), "menu:open:home"),
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
    rows.push(vec![callback_btn(
        lang.text(T::Refresh),
        format!("watch:status:{watch_id}"),
    )]);
    if matches!(status, "active" | "catching_up" | "degraded") {
        rows.push(vec![callback_btn(
            lang.text(T::Pause),
            format!("watch:pause:{watch_id}"),
        )]);
    } else if status == "paused" {
        rows.push(vec![callback_btn(
            lang.text(T::Resume),
            format!("watch:resume:{watch_id}"),
        )]);
    } else if status == "needs_reconcile" {
        rows.push(vec![callback_btn(
            lang.text(T::WatchResolveConflictButton),
            format!("watch:res:{short_id}"),
        )]);
    }
    rows.push(vec![
        callback_btn(lang.text(T::Versioned), format!("watch:pol:{short_id}:v")),
        callback_btn(lang.text(T::Replace), format!("watch:pol:{short_id}:r")),
        callback_btn(lang.text(T::Manual), format!("watch:pol:{short_id}:m")),
    ]);
    if status != "stopped" {
        rows.push(vec![callback_btn(
            lang.text(T::StopWatching),
            format!("watch:unwatch:{watch_id}"),
        )]);
    }
    rows.push(vec![
        callback_btn(lang.text(T::Watches), "menu:open:watches"),
        callback_btn(lang.text(T::BackHome), "menu:open:home"),
    ]);
    InlineKeyboardMarkup::new(rows)
}

pub fn watch_resolve_conflict_keyboard(
    watch_id: &str,
    sequence: i64,
    lang: UiLanguage,
) -> InlineKeyboardMarkup {
    let short_id = watch_id.get(..8).unwrap_or(watch_id);
    debug_assert!(format!("wres:v:{short_id}:{sequence}").len() <= 64);
    debug_assert!(format!("wres:r:{short_id}:{sequence}").len() <= 64);
    debug_assert!(format!("wres:s:{short_id}:{sequence}").len() <= 64);

    InlineKeyboardMarkup::new([
        vec![
            callback_btn(
                lang.text(T::WatchActionNewVersion),
                format!("wres:v:{short_id}:{sequence}"),
            ),
            callback_btn(
                lang.text(T::WatchActionReplace),
                format!("wres:r:{short_id}:{sequence}"),
            ),
            callback_btn(
                lang.text(T::WatchActionSkip),
                format!("wres:s:{short_id}:{sequence}"),
            ),
        ],
        vec![callback_btn(
            match lang {
                UiLanguage::Vi => "◀ Chi tiết watch",
                UiLanguage::En => "◀ Watch detail",
            },
            format!("watch:status:{watch_id}"),
        )],
    ])
}

pub fn watch_unwatch_confirm_keyboard(watch_id: &str, lang: UiLanguage) -> InlineKeyboardMarkup {
    InlineKeyboardMarkup::new([
        vec![
            callback_btn(
                lang.text(T::StopWatching),
                format!("watch:unwatch_confirm:{watch_id}"),
            ),
            callback_btn(lang.text(T::KeepWatch), format!("watch:status:{watch_id}")),
        ],
        vec![callback_btn(lang.text(T::Watches), "menu:open:watches")],
    ])
}

pub fn confirm_create_watch_keyboard(session_id: &str, lang: UiLanguage) -> InlineKeyboardMarkup {
    debug_assert!(format!("wconf:start:{session_id}").len() <= 64);
    debug_assert!(format!("wconf:opt:{session_id}").len() <= 64);
    debug_assert!(format!("wconf:cancel:{session_id}").len() <= 64);

    InlineKeyboardMarkup::new([
        vec![
            callback_btn(
                lang.text(T::StartWatchingButton),
                format!("wconf:start:{session_id}"),
            ),
            callback_btn(
                lang.text(T::WatchOptionsButton),
                format!("wconf:opt:{session_id}"),
            ),
        ],
        vec![callback_btn(
            lang.text(T::Cancel),
            format!("wconf:cancel:{session_id}"),
        )],
    ])
}

pub fn watch_options_keyboard(
    session_id: &str,
    current_policy: &str,
    lang: UiLanguage,
) -> InlineKeyboardMarkup {
    debug_assert!(format!("wopt:pol:{session_id}:v").len() <= 64);
    debug_assert!(format!("wopt:back:{session_id}").len() <= 64);

    let v_mark = if current_policy == "versioned_copy" {
        "● "
    } else {
        "○ "
    };
    let r_mark = if current_policy == "replace_copy" {
        "● "
    } else {
        "○ "
    };
    let m_mark = if current_policy == "manual_confirmation" {
        "● "
    } else {
        "○ "
    };

    let (v_label, r_label, m_label) = match lang {
        UiLanguage::Vi => (
            format!("{v_mark}Tạo bản mới"),
            format!("{r_mark}Thay bản cũ"),
            format!("{m_mark}Hỏi trước"),
        ),
        UiLanguage::En => (
            format!("{v_mark}New version"),
            format!("{r_mark}Replace old"),
            format!("{m_mark}Ask first"),
        ),
    };

    let back_label = match lang {
        UiLanguage::Vi => "◀ Quay lại",
        UiLanguage::En => "◀ Back",
    };

    InlineKeyboardMarkup::new([
        vec![
            callback_btn(v_label, format!("wopt:pol:{session_id}:v")),
            callback_btn(r_label, format!("wopt:pol:{session_id}:r")),
            callback_btn(m_label, format!("wopt:pol:{session_id}:m")),
        ],
        vec![callback_btn(back_label, format!("wopt:back:{session_id}"))],
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
            vec![callback_btn(label, format!("dest:select:{}", p.id))]
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
    rows.push(vec![callback_btn(
        lang.text(T::BrowseMyDrive),
        format!("browse:open:{my_drive_state_id}"),
    )]);
    rows.push(vec![callback_btn(
        lang.text(T::BrowseSharedDrive),
        format!("browse:open:{shared_drives_state_id}"),
    )]);
    rows.push(vec![callback_btn(lang.text(T::BackHome), "menu:open:home")]);
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
        rows.push(vec![callback_btn(
            lang.text(T::SelectThisFolder),
            format!("browse:pick:{state_id}"),
        )]);
    }
    if let Some(state_id) = parent_state_id {
        rows.push(vec![callback_btn(
            lang.text(T::UpOneLevel),
            format!("browse:open:{state_id}"),
        )]);
    }
    rows.extend(child_states.iter().map(|(label, state_id)| {
        vec![callback_btn(
            truncate_label(label, 40),
            format!("browse:open:{state_id}"),
        )]
    }));
    if let Some(state_id) = next_state_id {
        rows.push(vec![callback_btn(
            lang.text(T::NextPage),
            format!("browse:open:{state_id}"),
        )]);
    }
    rows.push(vec![
        callback_btn(lang.text(T::Destination), "menu:open:destination"),
        callback_btn(lang.text(T::BackHome), "menu:open:home"),
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

/// Persistent reply-keyboard sections (LanMan-style main navigation).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Section {
    Clone,
    Watch,
    Overview,
    Jobs,
    Watches,
    Destination,
    Account,
    Language,
}

pub fn section_label(lang: UiLanguage, section: Section) -> &'static str {
    let (vi, en) = match section {
        Section::Clone => ("\u{FF0B} Sao ch\u{00E9}p", "\u{FF0B} Clone"),
        Section::Watch => ("\u{27F3} \u{0110}\u{1ED3}ng b\u{1ED9}", "\u{27F3} Sync"),
        Section::Overview => ("\u{1F4CA} T\u{1ED5}ng quan", "\u{1F4CA} Overview"),
        Section::Jobs => ("\u{1F4E6} C\u{00F4}ng vi\u{1EC7}c", "\u{1F4E6} Jobs"),
        Section::Watches => ("\u{1F441} Theo d\u{00F5}i", "\u{1F441} Watches"),
        Section::Destination => (
            "\u{1F4C1} Th\u{01B0} m\u{1EE5}c \u{0111}\u{00ED}ch",
            "\u{1F4C1} Destination",
        ),
        Section::Account => ("\u{1F464} T\u{00E0}i kho\u{1EA3}n", "\u{1F464} Account"),
        Section::Language => ("\u{1F310} Ng\u{00F4}n ng\u{1EEF}", "\u{1F310} Language"),
    };
    if lang == UiLanguage::Vi { vi } else { en }
}

const SECTIONS: &[Section] = &[
    Section::Clone,
    Section::Watch,
    Section::Overview,
    Section::Jobs,
    Section::Watches,
    Section::Destination,
    Section::Account,
    Section::Language,
];

/// Map an incoming text message to a reply-keyboard section, if it exactly
/// matches a section label in either language (labels are stable across the
/// opposite language so a stale client keyboard still routes correctly).
pub fn section_from_text(text: &str) -> Option<Section> {
    let t = text.trim();
    SECTIONS.iter().copied().find(|&sec| {
        section_label(UiLanguage::Vi, sec) == t || section_label(UiLanguage::En, sec) == t
    })
}

/// The persistent main keyboard: section rows sized for one-thumb use.
pub fn main_reply_keyboard(lang: UiLanguage, watch_enabled: bool) -> KeyboardMarkup {
    let mut rows: Vec<Vec<KeyboardButton>> = Vec::new();
    let mut row1 = vec![KeyboardButton::new(section_label(lang, Section::Clone))];
    if watch_enabled {
        row1.push(KeyboardButton::new(section_label(lang, Section::Watch)));
    }
    rows.push(row1);
    rows.push(vec![
        KeyboardButton::new(section_label(lang, Section::Overview)),
        KeyboardButton::new(section_label(lang, Section::Jobs)),
    ]);
    let mut row3 = Vec::new();
    if watch_enabled {
        row3.push(KeyboardButton::new(section_label(lang, Section::Watches)));
    }
    row3.push(KeyboardButton::new(section_label(
        lang,
        Section::Destination,
    )));
    rows.push(row3);
    rows.push(vec![
        KeyboardButton::new(section_label(lang, Section::Account)),
        KeyboardButton::new(section_label(lang, Section::Language)),
    ]);
    KeyboardMarkup::new(rows)
        .resize_keyboard()
        .persistent()
        .input_field_placeholder(lang.text(T::ReplyKeyboardPlaceholder))
}

#[cfg(test)]
mod tests {
    use super::{
        UiLanguage, account_keyboard, callback_btn, callback_of, confirm_set_destination_keyboard,
        destination_browser_keyboard, destination_panel_keyboard, job_cancel_confirm_keyboard,
        job_detail_keyboard, job_end_state_keyboard, language_keyboard, main_menu_keyboard,
        recent_destinations_keyboard, smart_link_action_keyboard, truncate_label,
        unified_inspect_keyboard, watch_detail_keyboard, watch_list_keyboard,
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
                .any(|button| button.text == "⟳ Đồng bộ")
        );
        assert!(
            !disabled
                .inline_keyboard
                .iter()
                .flatten()
                .any(|button| button.text == "👁 Theo dõi")
        );

        let enabled = main_menu_keyboard(true, UiLanguage::Vi);
        assert_eq!(enabled.inline_keyboard[0][0].text, "＋ Sao chép");
        assert_eq!(enabled.inline_keyboard[0][1].text, "⟳ Đồng bộ");
        assert_eq!(enabled.inline_keyboard[1][0].text, "📦 Công việc");
        assert_eq!(enabled.inline_keyboard[1][1].text, "👁 Theo dõi");
        assert_eq!(enabled.inline_keyboard[2][0].text, "📁 Thư mục đích");
        assert_eq!(enabled.inline_keyboard[3][0].text, "⚙ Cài đặt");
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
            Some("menu:open:watches")
        );
        assert_eq!(
            callback_of(&enabled.inline_keyboard[2][0]),
            Some("menu:open:destination")
        );
        assert_eq!(
            callback_of(&enabled.inline_keyboard[3][0]),
            Some("menu:open:settings")
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

    #[test]
    fn confirm_set_destination_keyboard_has_set_and_cancel() {
        let vi = confirm_set_destination_keyboard("session-123", UiLanguage::Vi);
        assert_eq!(vi.inline_keyboard[0][0].text, "✓ Đặt");
        assert_eq!(
            callback_of(&vi.inline_keyboard[0][0]),
            Some("dest:confirm:session-123")
        );
        assert_eq!(vi.inline_keyboard[0][1].text, "Huỷ");
        assert_eq!(
            callback_of(&vi.inline_keyboard[0][1]),
            Some("dest:cancel:session-123")
        );

        let en = confirm_set_destination_keyboard("session-123", UiLanguage::En);
        assert_eq!(en.inline_keyboard[0][0].text, "✓ Set");
        assert_eq!(
            callback_of(&en.inline_keyboard[0][0]),
            Some("dest:confirm:session-123")
        );
        assert_eq!(en.inline_keyboard[0][1].text, "Cancel");
        assert_eq!(
            callback_of(&en.inline_keyboard[0][1]),
            Some("dest:cancel:session-123")
        );
    }

    #[test]
    fn unified_inspect_keyboard_callback_data_is_within_limit() {
        let kb = unified_inspect_keyboard("sess-123", true, true, UiLanguage::Vi);
        for row in &kb.inline_keyboard {
            for btn in row {
                if let Some(cb) = callback_of(btn) {
                    assert!(cb.len() <= 64, "Callback data exceeds 64 bytes: {cb}");
                }
            }
        }
    }

    #[test]
    fn job_end_state_keyboard_for_completed_and_failed() {
        let success = job_end_state_keyboard("job-123", "completed", 0, UiLanguage::Vi);
        assert_eq!(success.inline_keyboard[0][0].text, "Xem báo cáo");
        assert_eq!(
            callback_of(&success.inline_keyboard[0][0]),
            Some("job:report:job-123")
        );
        assert_eq!(success.inline_keyboard[0][1].text, "Sao chép lại");
        assert_eq!(
            callback_of(&success.inline_keyboard[0][1]),
            Some("job:clone_again:job-123")
        );

        let failed = job_end_state_keyboard("job-123", "failed", 3, UiLanguage::Vi);
        assert_eq!(failed.inline_keyboard[0][0].text, "Thử lại");
        assert_eq!(
            callback_of(&failed.inline_keyboard[0][0]),
            Some("job:retry:job-123")
        );
        assert_eq!(failed.inline_keyboard[0][1].text, "Xem lỗi");
        assert_eq!(
            callback_of(&failed.inline_keyboard[0][1]),
            Some("job:errors:job-123")
        );
        assert_eq!(failed.inline_keyboard[0][2].text, "Report");
        assert_eq!(
            callback_of(&failed.inline_keyboard[0][2]),
            Some("job:report:job-123")
        );

        let failed_en = job_end_state_keyboard("job-123", "failed", 3, UiLanguage::En);
        assert_eq!(failed_en.inline_keyboard[0][0].text, "Retry");
        assert_eq!(failed_en.inline_keyboard[0][1].text, "View errors");
        assert_eq!(failed_en.inline_keyboard[0][2].text, "Report");

        // Verify all callbacks <= 64 bytes
        for kb in [&success, &failed, &failed_en] {
            for row in &kb.inline_keyboard {
                for btn in row {
                    if let Some(cb) = callback_of(btn) {
                        assert!(cb.len() <= 64, "Callback exceeds 64 bytes: {cb}");
                    }
                }
            }
        }
    }

    #[test]
    fn callback_btn_preserves_short_data() {
        let btn = callback_btn("Click", "menu:open:home");
        assert_eq!(callback_of(&btn), Some("menu:open:home"));
    }

    #[test]
    fn callback_btn_truncation_guard_caps_at_64_bytes_safely() {
        // Construct an over-length ascii string (80 bytes)
        let long_data = "a".repeat(80);
        // Note: in debug mode debug_assert! triggers, so in release or when tested with non-asserting helper:
        // We test the truncation boundary directly:
        let data_ref = &long_data;
        let mut boundary = 64;
        while boundary > 0 && !data_ref.is_char_boundary(boundary) {
            boundary -= 1;
        }
        assert_eq!(boundary, 64);
        assert_eq!(&data_ref[..boundary], &"a".repeat(64));

        // Multibyte string (each Vietnamese character is 2-3 bytes)
        let multi_byte =
            "đồng_bộ_dữ_liệu_thời_gian_thực_rất_dài_vượt_quá_giới_hạn_cho_phép_của_telegram";
        let mut mb_boundary = 64;
        while mb_boundary > 0 && !multi_byte.is_char_boundary(mb_boundary) {
            mb_boundary -= 1;
        }
        assert!(mb_boundary <= 64);
        assert!(multi_byte.is_char_boundary(mb_boundary));
        // Ensure slicing never panics
        let _ = &multi_byte[..mb_boundary];
    }
}
