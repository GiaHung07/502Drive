use teloxide::types::{InlineKeyboardButton, InlineKeyboardMarkup};

pub fn confirm_clone_keyboard(state_id: &str) -> InlineKeyboardMarkup {
    InlineKeyboardMarkup::new([[
        InlineKeyboardButton::callback("Clone ngay", format!("clone:confirm:{state_id}")),
        InlineKeyboardButton::callback("Huỷ", format!("clone:cancel:{state_id}")),
    ]])
}

pub fn job_control_keyboard(job_id: &str, paused: bool) -> InlineKeyboardMarkup {
    let primary = if paused {
        InlineKeyboardButton::callback("Tiếp tục", format!("job:resume:{job_id}"))
    } else {
        InlineKeyboardButton::callback("Tạm dừng", format!("job:pause:{job_id}"))
    };
    InlineKeyboardMarkup::new([[
        primary,
        InlineKeyboardButton::callback("Huỷ", format!("job:cancel:{job_id}")),
    ]])
}

/// One row per destination, label truncated to 32 chars.
/// Callback: `dest:select:<profile_id>`
pub fn recent_destinations_keyboard(
    profiles: &[crate::state::repo::DestinationProfile],
) -> InlineKeyboardMarkup {
    let rows: Vec<Vec<InlineKeyboardButton>> = profiles
        .iter()
        .map(|p| {
            let label = truncate_label(&p.label, 32);
            let marker = if p.is_default { "[mặc định] " } else { "" };
            vec![InlineKeyboardButton::callback(
                format!("{marker}{label}"),
                format!("dest:select:{}", p.id),
            )]
        })
        .collect();
    InlineKeyboardMarkup::new(rows)
}

pub fn destination_panel_keyboard(
    profiles: &[crate::state::repo::DestinationProfile],
    browse_state_id: &str,
) -> InlineKeyboardMarkup {
    let mut rows = recent_destinations_keyboard(profiles).inline_keyboard;
    rows.push(vec![InlineKeyboardButton::callback(
        "Duyệt My Drive",
        format!("browse:open:{browse_state_id}"),
    )]);
    InlineKeyboardMarkup::new(rows)
}

pub fn destination_browser_keyboard(
    pick_state_id: Option<&str>,
    parent_state_id: Option<&str>,
    child_states: &[(String, String)],
) -> InlineKeyboardMarkup {
    let mut rows = Vec::new();
    if let Some(state_id) = pick_state_id {
        rows.push(vec![InlineKeyboardButton::callback(
            "Chọn thư mục này",
            format!("browse:pick:{state_id}"),
        )]);
    }
    if let Some(state_id) = parent_state_id {
        rows.push(vec![InlineKeyboardButton::callback(
            "Lên một cấp",
            format!("browse:open:{state_id}"),
        )]);
    }
    rows.extend(child_states.iter().map(|(label, state_id)| {
        vec![InlineKeyboardButton::callback(
            truncate_label(label, 40),
            format!("browse:open:{state_id}"),
        )]
    }));
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
    use super::truncate_label;

    #[test]
    fn truncates_unicode_on_char_boundary() {
        assert_eq!(
            truncate_label("Thư mục tiếng Việt rất dài", 12),
            "Thư mục t..."
        );
    }
}
