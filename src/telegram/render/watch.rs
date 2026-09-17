//! Watch list/detail texts, status and policy labels, filter and action texts.

use crate::state::repo;
use crate::telegram::handlers::WatchListMode;
use crate::telegram::i18n::TextKey as T;
use crate::telegram::keyboards;
use crate::telegram::render::{human_bytes, progress_status_field, push_field, short_id};

pub(crate) fn watch_list_mode_title(
    mode: WatchListMode,
    lang: keyboards::UiLanguage,
) -> &'static str {
    match (lang, mode) {
        (keyboards::UiLanguage::Vi, WatchListMode::Status) => "Chọn watch để xem chi tiết",
        (keyboards::UiLanguage::Vi, WatchListMode::Pause) => "Chọn watch để tạm dừng",
        (keyboards::UiLanguage::Vi, WatchListMode::Resume) => "Chọn watch để tiếp tục",
        (keyboards::UiLanguage::Vi, WatchListMode::Unwatch) => "Chọn watch để dừng theo dõi",
        (keyboards::UiLanguage::En, WatchListMode::Status) => "Choose a watch to view details",
        (keyboards::UiLanguage::En, WatchListMode::Pause) => "Choose a watch to pause",
        (keyboards::UiLanguage::En, WatchListMode::Resume) => "Choose a watch to resume",
        (keyboards::UiLanguage::En, WatchListMode::Unwatch) => "Choose a watch to stop",
    }
}

pub(crate) fn render_watch_detail(
    lang: keyboards::UiLanguage,
    watch: &repo::WatchSubscription,
    source: &str,
    destination: &str,
    _cursor_seq: i64,
    pending_events: i64,
    mapped_files: i64,
    now_ms: i64,
) -> String {
    let mut lines = vec![format!("● {}", watch_status_label(lang, &watch.status))];

    lines.push(String::new());
    lines.push(source.to_string());
    lines.push("        ↓".to_string());
    lines.push(destination.to_string());

    lines.push(String::new());
    let sync_time = match watch.last_consumed_at_ms {
        Some(ts) => {
            let elapsed_secs = (now_ms.saturating_sub(ts) / 1000).max(0);
            format_sync_elapsed(lang, elapsed_secs as u64)
        }
        None => match lang {
            keyboards::UiLanguage::Vi => "Chưa có sự kiện nào".to_string(),
            keyboards::UiLanguage::En => "No events yet".to_string(),
        },
    };
    push_field(&mut lines, watch_last_synced_field(lang), &sync_time);
    push_field(
        &mut lines,
        watch_pending_field(lang),
        &pending_events.to_string(),
    );
    let mapped_label = match lang {
        keyboards::UiLanguage::Vi => format!("{mapped_files} tệp"),
        keyboards::UiLanguage::En => format!("{mapped_files} files"),
    };
    push_field(&mut lines, watch_mapped_files_field(lang), &mapped_label);
    push_field(
        &mut lines,
        watch_content_policy_field(lang),
        content_update_policy_label(lang, &watch.content_update_policy),
    );
    lines.join("\n")
}

pub(crate) fn watch_title() -> &'static str {
    "WATCH/SYNC"
}

pub(crate) fn watch_empty_text(lang: keyboards::UiLanguage) -> &'static str {
    match lang {
        keyboards::UiLanguage::Vi => {
            "WATCH/SYNC\n━━━━━━━━━━\nChưa có thư mục nào đang được theo dõi."
        }
        keyboards::UiLanguage::En => "WATCH/SYNC\n━━━━━━━━━━\nNo watched folders yet.",
    }
}

pub(crate) fn watch_page_label(lang: keyboards::UiLanguage) -> &'static str {
    match lang {
        keyboards::UiLanguage::Vi => "Trang",
        keyboards::UiLanguage::En => "Page",
    }
}

pub(crate) fn watch_name_field(lang: keyboards::UiLanguage) -> &'static str {
    match lang {
        keyboards::UiLanguage::Vi => "Tên",
        keyboards::UiLanguage::En => "Name",
    }
}

pub(crate) fn watch_id_field(lang: keyboards::UiLanguage) -> &'static str {
    match lang {
        keyboards::UiLanguage::Vi => "ID watch",
        keyboards::UiLanguage::En => "Watch ID",
    }
}

pub(crate) fn watch_source_field(lang: keyboards::UiLanguage) -> &'static str {
    match lang {
        keyboards::UiLanguage::Vi => "Nguồn (folder cần lưu)",
        keyboards::UiLanguage::En => "Source folder",
    }
}

pub(crate) fn watch_destination_field(lang: keyboards::UiLanguage) -> &'static str {
    match lang {
        keyboards::UiLanguage::Vi => "Đích (folder nhận copy)",
        keyboards::UiLanguage::En => "Destination folder",
    }
}

pub(crate) fn watch_content_policy_field(lang: keyboards::UiLanguage) -> &'static str {
    match lang {
        keyboards::UiLanguage::Vi => "Khi nội dung file đổi",
        keyboards::UiLanguage::En => "When file content changes",
    }
}

pub(crate) fn watch_deletion_policy_field(lang: keyboards::UiLanguage) -> &'static str {
    match lang {
        keyboards::UiLanguage::Vi => "Khi nguồn xoá file",
        keyboards::UiLanguage::En => "When source deletes a file",
    }
}

pub(crate) fn watch_move_policy_field(lang: keyboards::UiLanguage) -> &'static str {
    match lang {
        keyboards::UiLanguage::Vi => "Khi file rời khỏi nguồn",
        keyboards::UiLanguage::En => "When a file leaves source",
    }
}

pub(crate) fn watch_baseline_field(lang: keyboards::UiLanguage) -> &'static str {
    match lang {
        keyboards::UiLanguage::Vi => "Mốc bắt đầu theo dõi",
        keyboards::UiLanguage::En => "Watch baseline",
    }
}

pub(crate) fn watch_consumed_field(lang: keyboards::UiLanguage) -> &'static str {
    match lang {
        keyboards::UiLanguage::Vi => "Đã áp đến sự kiện",
        keyboards::UiLanguage::En => "Applied through event",
    }
}

pub(crate) fn watch_cursor_field(lang: keyboards::UiLanguage) -> &'static str {
    match lang {
        keyboards::UiLanguage::Vi => "Sự kiện mới nhất",
        keyboards::UiLanguage::En => "Latest event",
    }
}

pub(crate) fn watch_pending_field(lang: keyboards::UiLanguage) -> &'static str {
    match lang {
        keyboards::UiLanguage::Vi => "Số thay đổi còn chờ",
        keyboards::UiLanguage::En => "Pending changes",
    }
}

pub(crate) fn format_sync_elapsed(lang: keyboards::UiLanguage, secs: u64) -> String {
    match lang {
        keyboards::UiLanguage::Vi => {
            if secs < 60 {
                format!("Vừa đồng bộ {secs} giây trước")
            } else if secs < 3600 {
                format!("Đồng bộ {} phút trước", secs / 60)
            } else {
                format!("Đồng bộ {} giờ trước", secs / 3600)
            }
        }
        keyboards::UiLanguage::En => {
            if secs < 60 {
                format!("Synced {secs}s ago")
            } else if secs < 3600 {
                format!("Synced {}m ago", secs / 60)
            } else {
                format!("Synced {}h ago", secs / 3600)
            }
        }
    }
}

pub(crate) fn watch_last_synced_field(lang: keyboards::UiLanguage) -> &'static str {
    lang.text(T::WatchLastSynced)
}

pub(crate) fn watch_mapped_files_field(lang: keyboards::UiLanguage) -> &'static str {
    lang.text(T::WatchMappedFiles)
}

pub(crate) fn vi_watch_status(status: &str) -> &str {
    match status {
        "active" => "đang theo dõi",
        "paused" => "tạm dừng",
        "initializing" => "đang khởi tạo",
        "catching_up" => "đang bắt kịp",
        "degraded" => "bị lỗi",
        "needs_reconcile" => "cần đồng bộ lại",
        "stopped" => "đã dừng",
        other => other,
    }
}

pub(crate) fn watch_status_label(lang: keyboards::UiLanguage, status: &str) -> &str {
    if lang == keyboards::UiLanguage::Vi {
        return vi_watch_status(status);
    }
    match status {
        "active" => "watching",
        "paused" => "paused",
        "initializing" => "initializing",
        "catching_up" => "catching up",
        "degraded" => "degraded",
        "needs_reconcile" => "needs reconcile",
        "stopped" => "stopped",
        other => other,
    }
}

pub(crate) fn vi_content_update_policy(policy: &str) -> &str {
    match policy {
        "versioned_copy" => "Tạo phiên bản mới",
        "replace_copy" => "Thay thế bản cũ",
        "manual_confirmation" => "Hỏi trước khi cập nhật",
        other => other,
    }
}

pub(crate) fn content_update_policy_label(lang: keyboards::UiLanguage, policy: &str) -> &str {
    if lang == keyboards::UiLanguage::Vi {
        return vi_content_update_policy(policy);
    }
    match policy {
        "versioned_copy" => "Create new version",
        "replace_copy" => "Replace old copy",
        "manual_confirmation" => "Ask before updating",
        other => other,
    }
}

pub(crate) fn vi_deletion_policy(policy: &str) -> &str {
    match policy {
        "preserve_destination" => "Giữ bản ở đích",
        "manual_confirmation" => "Hỏi trước khi xóa",
        other => other,
    }
}

pub(crate) fn deletion_policy_label(lang: keyboards::UiLanguage, policy: &str) -> &str {
    if lang == keyboards::UiLanguage::Vi {
        return vi_deletion_policy(policy);
    }
    match policy {
        "preserve_destination" => "Keep destination copy",
        "manual_confirmation" => "Ask before deleting",
        other => other,
    }
}

pub(crate) fn vi_move_out_policy(policy: &str) -> &str {
    match policy {
        "detach" => "Ngừng theo dõi file",
        "keep_following" => "Tiếp tục theo dõi",
        other => other,
    }
}

pub(crate) fn move_out_policy_label(lang: keyboards::UiLanguage, policy: &str) -> &str {
    if lang == keyboards::UiLanguage::Vi {
        return vi_move_out_policy(policy);
    }
    match policy {
        "detach" => "Stop tracking file",
        "keep_following" => "Keep following",
        other => other,
    }
}

pub(crate) fn render_watch_create_confirm(
    lang: keyboards::UiLanguage,
    source_name: &str,
    source_id: &str,
    dest_name: &str,
    dest_id: &str,
    content_policy: &str,
    deletion_policy: &str,
) -> String {
    let title = lang.text(T::ConfirmWatchTitle);
    let flow_label = lang.text(T::WatchDirectionOneWay);
    let mut lines = vec![title.to_string(), "━━━━━━━━━━".to_string()];
    push_field(
        &mut lines,
        watch_source_field(lang),
        &format!("{source_name} ({})", short_id(source_id)),
    );
    push_field(
        &mut lines,
        watch_destination_field(lang),
        &format!("{dest_name} ({})", short_id(dest_id)),
    );
    lines.push(flow_label.to_string());
    lines.push(String::new());
    push_field(
        &mut lines,
        watch_content_policy_field(lang),
        content_update_policy_label(lang, content_policy),
    );
    push_field(
        &mut lines,
        watch_deletion_policy_field(lang),
        deletion_policy_label(lang, deletion_policy),
    );
    lines.join("\n")
}

pub(crate) fn render_watch_options(lang: keyboards::UiLanguage) -> String {
    let title = lang.text(T::WatchOptionsTitle);
    let help = lang.text(T::WatchOptionsHelp);
    let (v_desc, r_desc, m_desc) = match lang {
        keyboards::UiLanguage::Vi => (
            "• Tạo bản mới: Giữ cả 2 bản, đánh số phiên bản",
            "• Thay bản cũ: Copy bản mới, đưa bản cũ vào thùng rác",
            "• Hỏi trước: Tạm dừng để bạn quyết định",
        ),
        keyboards::UiLanguage::En => (
            "• New version: Keep both, add version suffix",
            "• Replace old: Copy new, move old to trash",
            "• Ask first: Pause and ask for manual decision",
        ),
    };
    vec![
        title.to_string(),
        "━━━━━━━━━━".to_string(),
        help.to_string(),
        String::new(),
        v_desc.to_string(),
        r_desc.to_string(),
        m_desc.to_string(),
    ]
    .join("\n")
}

pub(crate) fn render_conflict_card(
    lang: keyboards::UiLanguage,
    details: &crate::watch::service::ConflictDetails,
) -> String {
    let title = lang.text(T::WatchConflictTitle);
    let mut lines = vec![
        format!("{title} — {}", details.file_name),
        "━━━━━━━━━━".to_string(),
    ];

    let current_info = match &details.current_dest_modified {
        Some(mod_time) => mod_time.as_str(),
        None => "—",
    };
    push_field(&mut lines, lang.text(T::WatchConflictCurrent), current_info);

    let new_info = match (
        details.new_source_size,
        details.new_source_modified.as_deref(),
    ) {
        (Some(size), Some(m)) => format!("{} · {}", human_bytes(size as i64), m),
        (Some(size), None) => human_bytes(size as i64),
        (None, Some(m)) => m.to_string(),
        _ => "—".to_string(),
    };
    push_field(&mut lines, lang.text(T::WatchConflictNew), &new_info);

    if details.remaining > 1 {
        let suffix = match lang {
            keyboards::UiLanguage::Vi => "tệp",
            keyboards::UiLanguage::En => "files",
        };
        push_field(
            &mut lines,
            lang.text(T::WatchConflictRemaining),
            &format!("{} {suffix}", details.remaining),
        );
    }

    lines.join("\n")
}

pub(crate) fn watch_disabled_text(lang: keyboards::UiLanguage) -> &'static str {
    match lang {
        keyboards::UiLanguage::Vi => {
            "Watch đang tắt. Bật `watch.enabled = true` trong config rồi khởi động lại bot."
        }
        keyboards::UiLanguage::En => {
            "Watch is disabled. Set `watch.enabled = true` in config, then restart the bot."
        }
    }
}

pub(crate) fn watch_usage_text(lang: keyboards::UiLanguage) -> &'static str {
    match lang {
        keyboards::UiLanguage::Vi => {
            "Cú pháp: /sync <nguồn> [đích] (hoặc /watch <nguồn> [đích])\n\
             • Nguồn: link/ID thư mục Drive cần theo dõi và đồng bộ.\n\
             • Đích: (tuỳ chọn nếu đã có đích mặc định) link/ID thư mục nhận bản copy.\n\
             Ví dụ: /sync https://drive.google.com/drive/folders/NGUON https://drive.google.com/drive/folders/DICH"
        }
        keyboards::UiLanguage::En => {
            "Usage: /sync <source> [destination] (or /watch <source> [destination])\n\
             • Source: Drive folder link/ID to watch and sync.\n\
             • Destination: (optional if default set) Drive folder link/ID that receives copies.\n\
             Example: /sync https://drive.google.com/drive/folders/SOURCE https://drive.google.com/drive/folders/DEST"
        }
    }
}

pub(crate) fn watch_source_must_be_folder(lang: keyboards::UiLanguage) -> &'static str {
    match lang {
        keyboards::UiLanguage::Vi => {
            "Nguồn phải là thư mục Google Drive. Hãy gửi link/ID folder cần lưu."
        }
        keyboards::UiLanguage::En => {
            "Source must be a Google Drive folder. Send the folder link/ID to preserve."
        }
    }
}

pub(crate) fn watch_destination_must_be_folder(lang: keyboards::UiLanguage) -> &'static str {
    match lang {
        keyboards::UiLanguage::Vi => {
            "Đích phải là thư mục Google Drive. Hãy gửi link/ID folder nhận bản copy."
        }
        keyboards::UiLanguage::En => {
            "Destination must be a Google Drive folder. Send the folder link/ID that receives copies."
        }
    }
}

pub(crate) fn watch_destination_not_writable(lang: keyboards::UiLanguage) -> &'static str {
    match lang {
        keyboards::UiLanguage::Vi => "Không có quyền ghi vào thư mục đích.",
        keyboards::UiLanguage::En => "No write access to the destination folder.",
    }
}

pub(crate) fn watch_created_text(
    lang: keyboards::UiLanguage,
    watch_id: &str,
    source_name: &str,
    source_id: &str,
    destination_name: &str,
) -> String {
    match lang {
        keyboards::UiLanguage::Vi => format!(
            "Watch đã tạo thành công.\n\
             ID          : {short}\n\
             Nguồn       : {source_name} ({source_id})\n\
             Đích        : {destination_name}\n\
             Khi nguồn xoá file: giữ bản copy ở đích.\n\
             Khi nguồn đổi tên: cố gắng đổi tên bản copy theo.\n\
             Clone ban đầu đang chạy nền. Dùng /watch_status {short} để theo dõi.",
            short = short_id(watch_id),
        ),
        keyboards::UiLanguage::En => format!(
            "Watch created.\n\
             ID          : {short}\n\
             Source      : {source_name} ({source_id})\n\
             Destination : {destination_name}\n\
             When source deletes a file: keep the destination copy.\n\
             When source renames a file: try to rename the copy too.\n\
             Initial clone is running in the background. Use /watch_status {short} to follow it.",
            short = short_id(watch_id),
        ),
    }
}

pub(crate) fn watch_not_found_text(lang: keyboards::UiLanguage) -> &'static str {
    match lang {
        keyboards::UiLanguage::Vi => "Không tìm thấy watch.",
        keyboards::UiLanguage::En => "Watch not found.",
    }
}

pub(crate) fn watch_prefix_ambiguous_text(lang: keyboards::UiLanguage) -> &'static str {
    match lang {
        keyboards::UiLanguage::Vi => "Có nhiều watch trùng prefix. Nhập thêm vài ký tự watch ID.",
        keyboards::UiLanguage::En => {
            "More than one watch matches that prefix. Enter a few more watch ID characters."
        }
    }
}

pub(crate) fn watch_paused_text(lang: keyboards::UiLanguage, watch_id: &str) -> String {
    match lang {
        keyboards::UiLanguage::Vi => format!("Watch {} đã tạm dừng.", short_id(watch_id)),
        keyboards::UiLanguage::En => format!("Watch {} paused.", short_id(watch_id)),
    }
}

pub(crate) fn watch_pause_failed_text(lang: keyboards::UiLanguage) -> &'static str {
    match lang {
        keyboards::UiLanguage::Vi => "Không tìm thấy watch hoặc không thể tạm dừng.",
        keyboards::UiLanguage::En => "Watch not found or cannot be paused.",
    }
}

pub(crate) fn watch_resumed_text(lang: keyboards::UiLanguage, watch_id: &str) -> String {
    match lang {
        keyboards::UiLanguage::Vi => {
            format!("Watch {} đã tiếp tục (đang bắt kịp).", short_id(watch_id))
        }
        keyboards::UiLanguage::En => format!("Watch {} resumed (catching up).", short_id(watch_id)),
    }
}

pub(crate) fn watch_needs_reconcile_text(
    lang: keyboards::UiLanguage,
    watch_id: &str,
    pending_events: i64,
    limit: u64,
) -> String {
    match lang {
        keyboards::UiLanguage::Vi => format!(
            "Watch {} cần đồng bộ lại vì backlog đã tới {} events (limit {}). Tạo lại watch hoặc chạy reconcile trước khi resume.",
            short_id(watch_id),
            pending_events,
            limit
        ),
        keyboards::UiLanguage::En => format!(
            "Watch {} needs reconciliation because backlog reached {} events (limit {}). Recreate the watch or reconcile before resuming.",
            short_id(watch_id),
            pending_events,
            limit
        ),
    }
}

pub(crate) fn watch_resume_failed_text(lang: keyboards::UiLanguage) -> &'static str {
    match lang {
        keyboards::UiLanguage::Vi => "Không tìm thấy watch hoặc watch chưa ở trạng thái tạm dừng.",
        keyboards::UiLanguage::En => "Watch not found or is not paused.",
    }
}

pub(crate) fn watch_stopped_text(lang: keyboards::UiLanguage, watch_id: &str) -> String {
    match lang {
        keyboards::UiLanguage::Vi => format!("Watch {} đã dừng.", short_id(watch_id)),
        keyboards::UiLanguage::En => format!("Watch {} stopped.", short_id(watch_id)),
    }
}

pub(crate) fn watch_stop_failed_text(lang: keyboards::UiLanguage) -> &'static str {
    match lang {
        keyboards::UiLanguage::Vi => "Không tìm thấy watch hoặc đã dừng trước đó.",
        keyboards::UiLanguage::En => "Watch not found or was already stopped.",
    }
}

pub(crate) fn watch_policy_usage_text(lang: keyboards::UiLanguage) -> &'static str {
    match lang {
        keyboards::UiLanguage::Vi => {
            "Cú pháp: /watch_policy <id_watch> <policy>\n\
             Policy hợp lệ:\n\
             • versioned_copy: file nguồn đổi nội dung thì tạo bản copy mới.\n\
             • replace_copy: copy mới rồi đưa bản cũ vào thùng rác.\n\
             • manual_confirmation: dừng lại để xác nhận thủ công."
        }
        keyboards::UiLanguage::En => {
            "Usage: /watch_policy <watch_id> <policy>\n\
             Valid policies:\n\
             • versioned_copy: create a new copy when source file content changes.\n\
             • replace_copy: copy new, then trash the old copy.\n\
             • manual_confirmation: stop for manual confirmation."
        }
    }
}

pub(crate) fn watch_policy_invalid(lang: keyboards::UiLanguage) -> &'static str {
    match lang {
        keyboards::UiLanguage::Vi => "Policy không hợp lệ.",
        keyboards::UiLanguage::En => "Invalid policy.",
    }
}

pub(crate) fn watch_policy_invalid_value(lang: keyboards::UiLanguage, policy: &str) -> String {
    match lang {
        keyboards::UiLanguage::Vi => format!(
            "Policy không hợp lệ '{policy}'. Chọn một trong: versioned_copy | replace_copy | manual_confirmation"
        ),
        keyboards::UiLanguage::En => format!(
            "Invalid policy '{policy}'. Choose one of: versioned_copy | replace_copy | manual_confirmation"
        ),
    }
}

pub(crate) fn watch_policy_changed_text(
    lang: keyboards::UiLanguage,
    watch_id: &str,
    policy: &str,
) -> String {
    match lang {
        keyboards::UiLanguage::Vi => format!(
            "Watch {} đã đổi policy: {}.",
            short_id(watch_id),
            content_update_policy_label(lang, policy)
        ),
        keyboards::UiLanguage::En => format!(
            "Watch {} policy changed: {}.",
            short_id(watch_id),
            content_update_policy_label(lang, policy)
        ),
    }
}

pub(crate) fn render_watch_filter_list(
    lang: keyboards::UiLanguage,
    watch: &repo::WatchSubscription,
) -> String {
    let globs = crate::watch::glob::parse_glob_list(&watch.exclude_globs);
    if globs.is_empty() {
        return match lang {
            keyboards::UiLanguage::Vi => format!(
                "Watch `{short}` chưa có glob loại trừ.\nDùng /watch_filter {short} add <glob> để thêm.",
                short = short_id(&watch.id)
            ),
            keyboards::UiLanguage::En => format!(
                "Watch `{short}` has no exclude globs yet.\nUse /watch_filter {short} add <glob> to add one.",
                short = short_id(&watch.id)
            ),
        };
    }
    let mut text = match lang {
        keyboards::UiLanguage::Vi => format!(
            "GLOB LOẠI TRỪ — watch `{}`\n━━━━━━━━━━\n",
            short_id(&watch.id)
        ),
        keyboards::UiLanguage::En => format!(
            "EXCLUDE GLOBS — watch `{}`\n━━━━━━━━━━\n",
            short_id(&watch.id)
        ),
    };
    for glob in &globs {
        text.push_str(&format!("• {glob}\n"));
    }
    text
}

pub(crate) fn watch_filter_usage_text(lang: keyboards::UiLanguage) -> String {
    match lang {
        keyboards::UiLanguage::Vi => "Cú pháp: /watch_filter <id_watch> <lệnh>\n\
             \n\
             • list — xem các glob hiện có\n\
             • add <glob> — thêm glob loại trừ (vd: *.tmp, ~$*)\n\
             • remove <glob> — xóa một glob\n\
             • clear — xóa tất cả\n\
             \n\
             File trùng glob sẽ bị bỏ qua khi đồng bộ. Dùng /watches để chọn watch."
            .to_string(),
        keyboards::UiLanguage::En => "Syntax: /watch_filter <watch_id> <action>\n\
             \n\
             • list — show current globs\n\
             • add <glob> — add an exclude glob (e.g. *.tmp, ~$*)\n\
             • remove <glob> — remove one glob\n\
             • clear — remove all\n\
             \n\
             Files matching a glob are skipped during sync. Use /watches to pick a watch."
            .to_string(),
    }
}

pub(crate) fn watch_filter_added_text(
    lang: keyboards::UiLanguage,
    glob: &str,
    watch_id: &str,
) -> String {
    match lang {
        keyboards::UiLanguage::Vi => format!(
            "Đã thêm glob `{glob}` vào watch `{short}`.",
            short = short_id(watch_id)
        ),
        keyboards::UiLanguage::En => format!(
            "Added glob `{glob}` to watch `{short}`.",
            short = short_id(watch_id)
        ),
    }
}

pub(crate) fn watch_filter_removed_text(
    lang: keyboards::UiLanguage,
    glob: &str,
    watch_id: &str,
) -> String {
    match lang {
        keyboards::UiLanguage::Vi => format!(
            "Đã xóa glob `{glob}` khỏi watch `{short}`.",
            short = short_id(watch_id)
        ),
        keyboards::UiLanguage::En => format!(
            "Removed glob `{glob}` from watch `{short}`.",
            short = short_id(watch_id)
        ),
    }
}

pub(crate) fn watch_filter_cleared_text(lang: keyboards::UiLanguage, watch_id: &str) -> String {
    match lang {
        keyboards::UiLanguage::Vi => format!(
            "Đã xóa toàn bộ glob của watch `{short}`.",
            short = short_id(watch_id)
        ),
        keyboards::UiLanguage::En => format!(
            "Cleared all globs of watch `{short}`.",
            short = short_id(watch_id)
        ),
    }
}

pub(crate) fn watch_filter_exists_text(
    lang: keyboards::UiLanguage,
    glob: &str,
    watch_id: &str,
) -> String {
    match lang {
        keyboards::UiLanguage::Vi => format!(
            "Glob `{glob}` đã có trong watch `{short}`.",
            short = short_id(watch_id)
        ),
        keyboards::UiLanguage::En => format!(
            "Glob `{glob}` is already on watch `{short}`.",
            short = short_id(watch_id)
        ),
    }
}

pub(crate) fn watch_filter_missing_text(
    lang: keyboards::UiLanguage,
    glob: &str,
    watch_id: &str,
) -> String {
    match lang {
        keyboards::UiLanguage::Vi => format!(
            "Glob `{glob}` không có trong watch `{short}`.",
            short = short_id(watch_id)
        ),
        keyboards::UiLanguage::En => format!(
            "Glob `{glob}` is not on watch `{short}`.",
            short = short_id(watch_id)
        ),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::watch::service::ConflictDetails;

    #[test]
    fn renders_conflict_card_domain_snapshot_vi_and_en() {
        let details = ConflictDetails {
            watch_id: "w-123456789".to_string(),
            sequence: 12,
            file_id: "file_abc123".to_string(),
            file_name: "report_q3.pdf".to_string(),
            current_dest_modified: Some("14:30 15/09".to_string()),
            new_source_size: Some(15_400_000), // ~14.7 MB
            new_source_modified: Some("15:00 15/09".to_string()),
            remaining: 3,
        };

        // Vietnamese domain assertions
        let vi = render_conflict_card(keyboards::UiLanguage::Vi, &details);
        assert!(vi.contains("report_q3.pdf"));
        assert!(vi.contains("14:30 15/09"));
        assert!(vi.contains("15:00 15/09"));
        assert!(vi.contains("3 tệp"));

        // English domain assertions
        let en = render_conflict_card(keyboards::UiLanguage::En, &details);
        assert!(en.contains("report_q3.pdf"));
        assert!(en.contains("14:30 15/09"));
        assert!(en.contains("15:00 15/09"));
        assert!(en.contains("3 files"));
    }

    #[test]
    fn renders_watch_options_domain_snapshot() {
        let vi = render_watch_options(keyboards::UiLanguage::Vi);
        assert!(vi.contains("Tạo bản mới"));
        assert!(vi.contains("Thay bản cũ"));
        assert!(vi.contains("Hỏi trước"));

        let en = render_watch_options(keyboards::UiLanguage::En);
        assert!(en.contains("New version"));
        assert!(en.contains("Replace old"));
        assert!(en.contains("Ask first"));
    }
}
