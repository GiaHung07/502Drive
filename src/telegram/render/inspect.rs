//! Unified inspect card rendering for Google Drive links and folders.

use crate::engine::copy::CloneSourceInspect;
use crate::telegram::i18n::UiLanguage;
use crate::telegram::render::clone::human_bytes;

pub(crate) fn render_unified_inspect_card(
    inspect: &CloneSourceInspect,
    destination_label: Option<&str>,
    lang: UiLanguage,
) -> String {
    let mut lines = Vec::new();

    // Line 1: Icon + Name
    let icon = if inspect.is_folder { "📁" } else { "📄" };
    lines.push(format!("{icon} {}", inspect.file.name));

    // Line 2: Type · Items · Size
    let size_str = human_bytes(inspect.total_bytes as i64);
    let summary = match lang {
        UiLanguage::Vi => {
            if inspect.is_folder {
                let suffix = if inspect.plan_truncated { "+" } else { "" };
                format!(
                    "Google Drive folder · {}{} tệp · {size_str}",
                    inspect.total_files, suffix
                )
            } else {
                format!("Google Drive file · {size_str}")
            }
        }
        UiLanguage::En => {
            if inspect.is_folder {
                let suffix = if inspect.plan_truncated { "+" } else { "" };
                format!(
                    "Google Drive folder · {}{} files · {size_str}",
                    inspect.total_files, suffix
                )
            } else {
                format!("Google Drive file · {size_str}")
            }
        }
    };
    lines.push(summary);

    // Line 3: Source Location
    let source_loc = if let Some(drive_id) = &inspect.file.drive_id {
        format!("Shared Drive ({drive_id}) / {}", inspect.file.name)
    } else {
        format!("My Drive / {}", inspect.file.name)
    };
    let source_label = match lang {
        UiLanguage::Vi => "Nguồn",
        UiLanguage::En => "Source",
    };
    lines.push(format!("{source_label}: {source_loc}"));

    // Line 4: Destination Location
    let dest_label = match lang {
        UiLanguage::Vi => "Đích",
        UiLanguage::En => "Dest",
    };
    let dest_text = match destination_label {
        Some(lbl) => match lang {
            UiLanguage::Vi => format!("{lbl} (mặc định)"),
            UiLanguage::En => format!("{lbl} (default)"),
        },
        None => match lang {
            UiLanguage::Vi => "Chưa đặt (dùng nút bên dưới để cấu hình)".to_string(),
            UiLanguage::En => "Not set (use button below to configure)".to_string(),
        },
    };
    lines.push(format!("{dest_label}:  {dest_text}"));

    // Warnings if any
    if !inspect.can_copy_or_list {
        let warn = match lang {
            UiLanguage::Vi => {
                "⚠ Tài khoản Google kết nối không có quyền truy cập hoặc sao chép đối tượng này."
            }
            UiLanguage::En => {
                "⚠ The connected Google account does not have permission to access or copy this item."
            }
        };
        lines.push(String::new());
        lines.push(warn.to_string());
    } else if inspect.copy_requires_writer {
        let warn = match lang {
            UiLanguage::Vi => "⚠ Tệp này yêu cầu quyền Writer của tài khoản để sao chép.",
            UiLanguage::En => "⚠ This file requires Writer permission to be copied.",
        };
        lines.push(String::new());
        lines.push(warn.to_string());
    }

    lines.join("\n")
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::drive::types::DriveFile;

    fn sample_inspect(is_folder: bool, truncated: bool) -> CloneSourceInspect {
        CloneSourceInspect {
            file: DriveFile {
                id: "file-123".to_string(),
                name: "Project Alpha".to_string(),
                mime_type: if is_folder {
                    "application/vnd.google-apps.folder".to_string()
                } else {
                    "application/pdf".to_string()
                },
                size: Some((1024 * 1024).to_string()),
                parents: vec![],
                drive_id: None,
                resource_key: None,
                shortcut_details: None,
                trashed: Some(false),
                modified_time: None,
                md5_checksum: None,
                version: None,
                capabilities: None,
                copy_requires_writer_permission: None,
                app_properties: std::collections::HashMap::new(),
            },
            is_folder,
            total_files: 42,
            total_folders: 5,
            total_bytes: 42 * 1024 * 1024,
            plan_truncated: truncated,
            can_copy_or_list: true,
            copy_requires_writer: false,
            resource_key: None,
            account_email: Some("test@example.com".to_string()),
            default_destination: None,
        }
    }

    #[test]
    fn renders_folder_inspect_card_vi_and_en() {
        let inspect = sample_inspect(true, false);
        let vi = render_unified_inspect_card(&inspect, Some("Backup 502"), UiLanguage::Vi);
        assert!(vi.contains("📁 Project Alpha"));
        assert!(vi.contains("42 tệp"));
        assert!(vi.contains("Nguồn: My Drive / Project Alpha"));
        assert!(vi.contains("Đích:  Backup 502 (mặc định)"));

        let en = render_unified_inspect_card(&inspect, Some("Backup 502"), UiLanguage::En);
        assert!(en.contains("📁 Project Alpha"));
        assert!(en.contains("42 files"));
        assert!(en.contains("Source: My Drive / Project Alpha"));
        assert!(en.contains("Dest:  Backup 502 (default)"));
    }

    #[test]
    fn renders_file_inspect_card() {
        let inspect = sample_inspect(false, false);
        let vi = render_unified_inspect_card(&inspect, None, UiLanguage::Vi);
        assert!(vi.contains("📄 Project Alpha"));
        assert!(vi.contains("Google Drive file"));
        assert!(vi.contains("Chưa đặt"));
    }

    #[test]
    fn renders_warnings_when_no_permission() {
        let mut inspect = sample_inspect(true, false);
        inspect.can_copy_or_list = false;
        let vi = render_unified_inspect_card(&inspect, None, UiLanguage::Vi);
        assert!(vi.contains("không có quyền truy cập"));
    }
}
