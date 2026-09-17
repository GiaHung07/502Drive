//! Destination panel/browser texts.

use crate::drive::types::DriveFile;
use crate::state::repo;
use crate::telegram::keyboards;
use crate::telegram::render::{push_field, short_id};

pub(crate) fn destination_title(lang: keyboards::UiLanguage) -> &'static str {
    match lang {
        keyboards::UiLanguage::Vi => "THƯ MỤC ĐÍCH",
        keyboards::UiLanguage::En => "DESTINATIONS",
    }
}

pub(crate) fn destination_empty_text(lang: keyboards::UiLanguage) -> &'static str {
    match lang {
        keyboards::UiLanguage::Vi => {
            "Chưa có thư mục đích nào được lưu.\nDùng /set_destination <folder_url>."
        }
        keyboards::UiLanguage::En => {
            "No saved destination folders yet.\nUse /set_destination <folder_url>."
        }
    }
}

pub(crate) fn destination_default_marker(lang: keyboards::UiLanguage) -> &'static str {
    match lang {
        keyboards::UiLanguage::Vi => "mặc định",
        keyboards::UiLanguage::En => "default",
    }
}

pub(crate) fn destination_my_drive_label(lang: keyboards::UiLanguage) -> &'static str {
    match lang {
        keyboards::UiLanguage::Vi => "My Drive / được chia sẻ",
        keyboards::UiLanguage::En => "My Drive / shared",
    }
}

pub(crate) fn destination_name_field(lang: keyboards::UiLanguage) -> &'static str {
    match lang {
        keyboards::UiLanguage::Vi => "Tên",
        keyboards::UiLanguage::En => "Name",
    }
}

pub(crate) fn destination_location_field(lang: keyboards::UiLanguage) -> &'static str {
    match lang {
        keyboards::UiLanguage::Vi => "Vị trí",
        keyboards::UiLanguage::En => "Location",
    }
}

pub(crate) fn destination_short_id_field(lang: keyboards::UiLanguage) -> &'static str {
    match lang {
        keyboards::UiLanguage::Vi => "ID ngắn",
        keyboards::UiLanguage::En => "Short ID",
    }
}

pub(crate) fn destination_hint(lang: keyboards::UiLanguage) -> &'static str {
    match lang {
        keyboards::UiLanguage::Vi => {
            "Nhấn vào tên để đặt làm mặc định. Thêm mới: /set_destination <url>"
        }
        keyboards::UiLanguage::En => {
            "Tap a name to make it default. Add one with /set_destination <url>"
        }
    }
}

pub(crate) fn destination_browser_title(lang: keyboards::UiLanguage) -> &'static str {
    match lang {
        keyboards::UiLanguage::Vi => "CHỌN THƯ MỤC ĐÍCH",
        keyboards::UiLanguage::En => "CHOOSE DESTINATION FOLDER",
    }
}

pub(crate) fn destination_folder_id_field(lang: keyboards::UiLanguage) -> &'static str {
    match lang {
        keyboards::UiLanguage::Vi => "Folder ID",
        keyboards::UiLanguage::En => "Folder ID",
    }
}

pub(crate) fn destination_writable_field(lang: keyboards::UiLanguage) -> &'static str {
    match lang {
        keyboards::UiLanguage::Vi => "Có thể ghi",
        keyboards::UiLanguage::En => "Writable",
    }
}

pub(crate) fn capability_text_lang(
    value: Option<bool>,
    lang: keyboards::UiLanguage,
) -> &'static str {
    match (lang, value) {
        (keyboards::UiLanguage::Vi, Some(true)) => "Có",
        (keyboards::UiLanguage::Vi, Some(false)) => "Không",
        (keyboards::UiLanguage::Vi, None) => "Không rõ",
        (keyboards::UiLanguage::En, Some(true)) => "Yes",
        (keyboards::UiLanguage::En, Some(false)) => "No",
        (keyboards::UiLanguage::En, None) => "Unknown",
    }
}

pub(crate) fn destination_no_child_folders(lang: keyboards::UiLanguage) -> &'static str {
    match lang {
        keyboards::UiLanguage::Vi => "Không có thư mục con trong trang này.",
        keyboards::UiLanguage::En => "No child folders on this page.",
    }
}

pub(crate) fn destination_child_count(lang: keyboards::UiLanguage, count: usize) -> String {
    match lang {
        keyboards::UiLanguage::Vi => format!("Thư mục con: {count}"),
        keyboards::UiLanguage::En => format!("Child folders: {count}"),
    }
}

pub(crate) fn destination_folder_page_hint(lang: keyboards::UiLanguage) -> &'static str {
    match lang {
        keyboards::UiLanguage::Vi => "Chỉ hiện 20 thư mục mỗi trang.",
        keyboards::UiLanguage::En => "Showing 20 folders per page.",
    }
}

pub(crate) fn destination_shared_drives_title(lang: keyboards::UiLanguage) -> &'static str {
    match lang {
        keyboards::UiLanguage::Vi => "SHARED DRIVES",
        keyboards::UiLanguage::En => "SHARED DRIVES",
    }
}

pub(crate) fn destination_no_shared_drives(lang: keyboards::UiLanguage) -> &'static str {
    match lang {
        keyboards::UiLanguage::Vi => "Không tìm thấy Shared Drive nào cho tài khoản này.",
        keyboards::UiLanguage::En => "No Shared Drives found for this account.",
    }
}

pub(crate) fn destination_shared_drive_count(lang: keyboards::UiLanguage, count: usize) -> String {
    match lang {
        keyboards::UiLanguage::Vi => format!("Shared Drive: {count}"),
        keyboards::UiLanguage::En => format!("Shared Drives: {count}"),
    }
}

pub(crate) fn destination_drive_page_hint(lang: keyboards::UiLanguage) -> &'static str {
    match lang {
        keyboards::UiLanguage::Vi => "Chỉ hiện 20 drive mỗi trang.",
        keyboards::UiLanguage::En => "Showing 20 drives per page.",
    }
}

pub(crate) fn destination_not_found(lang: keyboards::UiLanguage) -> &'static str {
    match lang {
        keyboards::UiLanguage::Vi => "Không tìm thấy thư mục đích.",
        keyboards::UiLanguage::En => "Destination folder not found.",
    }
}

pub(crate) fn destination_switch_error(
    lang: keyboards::UiLanguage,
    err: impl std::fmt::Display,
) -> String {
    match lang {
        keyboards::UiLanguage::Vi => format!("Lỗi đổi đích: {err}"),
        keyboards::UiLanguage::En => format!("Could not change destination: {err}"),
    }
}

pub(crate) fn render_destination_list(
    profiles: &[repo::DestinationProfile],
    lang: keyboards::UiLanguage,
) -> String {
    if profiles.is_empty() {
        return destination_empty_text(lang).to_string();
    }
    let mut lines = vec![
        destination_title(lang).to_string(),
        "━━━━━━━━━━━━".to_string(),
    ];
    for p in profiles {
        lines.push(String::new());
        let title = if p.is_default {
            format!("{} [{}]", p.label, destination_default_marker(lang))
        } else {
            p.label.clone()
        };
        let drive = if p.destination_drive_id.is_some() {
            "Shared Drive"
        } else {
            destination_my_drive_label(lang)
        };
        push_field(&mut lines, destination_name_field(lang), &title);
        push_field(&mut lines, destination_location_field(lang), drive);
        push_field(
            &mut lines,
            destination_short_id_field(lang),
            short_id(&p.destination_parent_id),
        );
    }
    lines.push(String::new());
    lines.push(destination_hint(lang).to_string());
    lines.join("\n")
}

pub(crate) fn render_destination_saved(
    file: &DriveFile,
    input_resource_key: Option<String>,
) -> String {
    let mut lines = vec![
        "ĐÃ ĐẶT THƯ MỤC ĐÍCH".to_string(),
        "━━━━━━━━━━━━━━━".to_string(),
    ];
    push_field(&mut lines, "Tên", &file.name);
    push_field(&mut lines, "Folder ID", &file.id);
    push_field(
        &mut lines,
        "Resource key",
        capability_text(Some(
            input_resource_key.is_some() || file.resource_key.is_some(),
        )),
    );
    if let Some(drive_id) = &file.drive_id {
        push_field(&mut lines, "Drive", &format!("Shared Drive ({drive_id})"));
    } else {
        push_field(&mut lines, "Drive", "My Drive / được chia sẻ");
    }
    lines.join("\n")
}

fn capability_text(value: Option<bool>) -> &'static str {
    match value {
        Some(true) => "Có",
        Some(false) => "Không",
        None => "Không rõ",
    }
}
