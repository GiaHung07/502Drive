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
