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
            let label = if p.label.len() > 32 {
                format!("{}...", &p.label[..29])
            } else {
                p.label.clone()
            };
            let marker = if p.is_default { "[mặc định] " } else { "" };
            vec![InlineKeyboardButton::callback(
                format!("{marker}{label}"),
                format!("dest:select:{}", p.id),
            )]
        })
        .collect();
    InlineKeyboardMarkup::new(rows)
}
