//! Clone preview, plan, outcome texts and the `ClonePlan` renderer.

use crate::drive::types::DriveFile;
use crate::state::repo;
use crate::telegram::keyboards;
use crate::telegram::render::{job_status_label, push_field, short_job_id};

pub(crate) fn clone_loading_text(lang: keyboards::UiLanguage) -> &'static str {
    match lang {
        keyboards::UiLanguage::Vi => {
            "Đang kiểm tra nguồn Drive...\n░░░░░░░░░░░░░░░░\nVui lòng chờ."
        }
        keyboards::UiLanguage::En => "Checking Drive source...\n░░░░░░░░░░░░░░░░\nPlease wait.",
    }
}

pub(crate) fn clone_info_title(lang: keyboards::UiLanguage) -> &'static str {
    match lang {
        keyboards::UiLanguage::Vi => "THÔNG TIN CLONE",
        keyboards::UiLanguage::En => "CLONE PREVIEW",
    }
}

pub(crate) fn clone_missing_email(lang: keyboards::UiLanguage) -> &'static str {
    match lang {
        keyboards::UiLanguage::Vi => "không đọc được email",
        keyboards::UiLanguage::En => "email unavailable",
    }
}

pub(crate) fn clone_source_name_field(lang: keyboards::UiLanguage) -> &'static str {
    match lang {
        keyboards::UiLanguage::Vi => "Tên nguồn",
        keyboards::UiLanguage::En => "Source name",
    }
}

pub(crate) fn clone_source_id_field(lang: keyboards::UiLanguage) -> &'static str {
    match lang {
        keyboards::UiLanguage::Vi => "ID nguồn",
        keyboards::UiLanguage::En => "Source ID",
    }
}

pub(crate) fn clone_type_field(lang: keyboards::UiLanguage) -> &'static str {
    match lang {
        keyboards::UiLanguage::Vi => "Loại",
        keyboards::UiLanguage::En => "Type",
    }
}

pub(crate) fn clone_item_type(lang: keyboards::UiLanguage, is_folder: bool) -> &'static str {
    match (lang, is_folder) {
        (keyboards::UiLanguage::Vi, true) => "Thư mục",
        (keyboards::UiLanguage::Vi, false) => "File",
        (keyboards::UiLanguage::En, true) => "Folder",
        (keyboards::UiLanguage::En, false) => "File",
    }
}

pub(crate) fn clone_size_field(lang: keyboards::UiLanguage) -> &'static str {
    match lang {
        keyboards::UiLanguage::Vi => "Kích thước",
        keyboards::UiLanguage::En => "Size",
    }
}

pub(crate) fn clone_location_field(lang: keyboards::UiLanguage) -> &'static str {
    match lang {
        keyboards::UiLanguage::Vi => "Vị trí",
        keyboards::UiLanguage::En => "Location",
    }
}

pub(crate) fn clone_modified_field(lang: keyboards::UiLanguage) -> &'static str {
    match lang {
        keyboards::UiLanguage::Vi => "Sửa đổi",
        keyboards::UiLanguage::En => "Modified",
    }
}

pub(crate) fn clone_readable_field(lang: keyboards::UiLanguage) -> &'static str {
    match lang {
        keyboards::UiLanguage::Vi => "Có thể đọc/copy",
        keyboards::UiLanguage::En => "Readable/copyable",
    }
}

pub(crate) fn clone_warn_cannot_list(lang: keyboards::UiLanguage) -> &'static str {
    match lang {
        keyboards::UiLanguage::Vi => {
            "CẢNH BÁO: Không thể liệt kê nội dung - thư mục có thể bị hạn chế quyền."
        }
        keyboards::UiLanguage::En => {
            "WARNING: Cannot list contents - this folder may be permission-restricted."
        }
    }
}

pub(crate) fn clone_warn_cannot_copy(lang: keyboards::UiLanguage) -> &'static str {
    match lang {
        keyboards::UiLanguage::Vi => {
            "CẢNH BÁO: Không thể sao chép - file bị hạn chế hoặc chống copy."
        }
        keyboards::UiLanguage::En => "WARNING: Cannot copy - this file may be restricted.",
    }
}

pub(crate) fn clone_warn_writer_required(lang: keyboards::UiLanguage) -> &'static str {
    match lang {
        keyboards::UiLanguage::Vi => {
            "CẢNH BÁO: copyRequiresWriterPermission - chỉ người có quyền ghi mới copy được."
        }
        keyboards::UiLanguage::En => {
            "WARNING: copyRequiresWriterPermission - only writers can copy this source."
        }
    }
}

pub(crate) fn clone_resource_key_note(lang: keyboards::UiLanguage) -> &'static str {
    match lang {
        keyboards::UiLanguage::Vi => "Lưu ý: Link dùng resource key (link hạn chế truy cập).",
        keyboards::UiLanguage::En => "Note: This link uses a resource key.",
    }
}

pub(crate) fn clone_plan_title(lang: keyboards::UiLanguage) -> &'static str {
    match lang {
        keyboards::UiLanguage::Vi => "KẾ HOẠCH",
        keyboards::UiLanguage::En => "PLAN",
    }
}

pub(crate) fn clone_plan_unavailable(lang: keyboards::UiLanguage) -> &'static str {
    match lang {
        keyboards::UiLanguage::Vi => "Không quét trước được. Bot vẫn có thể thử khi bạn bấm clone.",
        keyboards::UiLanguage::En => {
            "Could not pre-scan. The bot can still try when you start cloning."
        }
    }
}

pub(crate) fn clone_reason_field(lang: keyboards::UiLanguage) -> &'static str {
    match lang {
        keyboards::UiLanguage::Vi => "Lý do",
        keyboards::UiLanguage::En => "Reason",
    }
}

pub(crate) fn clone_destination_title(lang: keyboards::UiLanguage) -> &'static str {
    match lang {
        keyboards::UiLanguage::Vi => "ĐÍCH ĐẾN",
        keyboards::UiLanguage::En => "DESTINATION",
    }
}

pub(crate) fn clone_confirm_hint(lang: keyboards::UiLanguage) -> &'static str {
    match lang {
        keyboards::UiLanguage::Vi => "Chọn nút bên dưới để bắt đầu hoặc huỷ.",
        keyboards::UiLanguage::En => "Use the buttons below to start or cancel.",
    }
}

pub(crate) fn clone_missing_destination(lang: keyboards::UiLanguage) -> &'static str {
    match lang {
        keyboards::UiLanguage::Vi => "Chưa có thư mục đích. Dùng /set_destination <folder_url>.",
        keyboards::UiLanguage::En => {
            "No destination folder yet. Use /set_destination <folder_url>."
        }
    }
}

pub(crate) fn clone_plan_scanned_line(lang: keyboards::UiLanguage, count: usize) -> String {
    match lang {
        keyboards::UiLanguage::Vi => format!("Đã quét      : {count} item"),
        keyboards::UiLanguage::En => format!("Scanned      : {count} items"),
    }
}

pub(crate) fn clone_plan_folder_line(lang: keyboards::UiLanguage, count: u64) -> String {
    match lang {
        keyboards::UiLanguage::Vi => format!("Thư mục      : {count}"),
        keyboards::UiLanguage::En => format!("Folders      : {count}"),
    }
}

pub(crate) fn clone_plan_file_line(lang: keyboards::UiLanguage, count: u64) -> String {
    match lang {
        keyboards::UiLanguage::Vi => format!("File         : {count}"),
        keyboards::UiLanguage::En => format!("Files        : {count}"),
    }
}

pub(crate) fn clone_plan_google_native_line(_lang: keyboards::UiLanguage, count: u64) -> String {
    format!("Google-native: {count}")
}

pub(crate) fn clone_plan_shortcut_line(_lang: keyboards::UiLanguage, count: u64) -> String {
    format!("Shortcut     : {count}")
}

pub(crate) fn clone_plan_known_size_line(lang: keyboards::UiLanguage, bytes: u64) -> String {
    match lang {
        keyboards::UiLanguage::Vi => format!("Dung lượng rõ: {}", human_bytes(bytes as i64)),
        keyboards::UiLanguage::En => format!("Known size   : {}", human_bytes(bytes as i64)),
    }
}

pub(crate) fn clone_plan_warning_line(lang: keyboards::UiLanguage, count: u64) -> String {
    match lang {
        keyboards::UiLanguage::Vi => format!("Cảnh báo     : {count} item cần chú ý"),
        keyboards::UiLanguage::En => format!("Warnings     : {count} items need attention"),
    }
}

pub(crate) fn clone_plan_truncated_line(lang: keyboards::UiLanguage, count: usize) -> String {
    match lang {
        keyboards::UiLanguage::Vi => format!("Lưu ý        : chỉ quét trước {count} item đầu"),
        keyboards::UiLanguage::En => format!("Note         : scanned only the first {count} items"),
    }
}

pub(crate) fn clone_plan_scanned_field(lang: keyboards::UiLanguage) -> &'static str {
    match lang {
        keyboards::UiLanguage::Vi => "Đã quét",
        keyboards::UiLanguage::En => "Scanned",
    }
}

pub(crate) fn clone_outcome_title(lang: keyboards::UiLanguage, status: &str) -> &'static str {
    match (lang, status) {
        (keyboards::UiLanguage::Vi, "completed") => "CLONE HOÀN TẤT",
        (keyboards::UiLanguage::En, "completed") => "CLONE COMPLETED",
        (keyboards::UiLanguage::Vi, "partially_completed") => "CLONE HOÀN TẤT MỘT PHẦN",
        (keyboards::UiLanguage::En, "partially_completed") => "CLONE PARTIALLY COMPLETED",
        (keyboards::UiLanguage::Vi, "failed") => "CLONE LỖI",
        (keyboards::UiLanguage::En, "failed") => "CLONE FAILED",
        (keyboards::UiLanguage::Vi, "cancelled") => "CLONE ĐÃ HUỶ",
        (keyboards::UiLanguage::En, "cancelled") => "CLONE CANCELLED",
        (keyboards::UiLanguage::Vi, _) => "KẾT QUẢ CLONE",
        (keyboards::UiLanguage::En, _) => "CLONE RESULT",
    }
}

pub(crate) fn clone_outcome_status_field(lang: keyboards::UiLanguage) -> &'static str {
    match lang {
        keyboards::UiLanguage::Vi => "Trạng thái",
        keyboards::UiLanguage::En => "Status",
    }
}

pub(crate) fn clone_outcome_completed_field(lang: keyboards::UiLanguage) -> &'static str {
    match lang {
        keyboards::UiLanguage::Vi => "Hoàn tất",
        keyboards::UiLanguage::En => "Completed",
    }
}

pub(crate) fn clone_outcome_failed_field(lang: keyboards::UiLanguage) -> &'static str {
    match lang {
        keyboards::UiLanguage::Vi => "Lỗi",
        keyboards::UiLanguage::En => "Failed",
    }
}

pub(crate) fn clone_outcome_skipped_field(lang: keyboards::UiLanguage) -> &'static str {
    match lang {
        keyboards::UiLanguage::Vi => "Bỏ qua",
        keyboards::UiLanguage::En => "Skipped",
    }
}

pub(crate) fn clone_outcome_error_field(lang: keyboards::UiLanguage) -> &'static str {
    match lang {
        keyboards::UiLanguage::Vi => "Lỗi chính",
        keyboards::UiLanguage::En => "Main error",
    }
}

pub(crate) fn clone_outcome_hint(
    lang: keyboards::UiLanguage,
    failed: i64,
    skipped: i64,
) -> &'static str {
    match (lang, failed > 0, skipped > 0) {
        (keyboards::UiLanguage::Vi, true, _) => "Mở job detail để bấm Retry lỗi hoặc lấy report.",
        (keyboards::UiLanguage::En, true, _) => {
            "Open the job detail to retry failed items or get the report."
        }
        (keyboards::UiLanguage::Vi, false, true) => "Mở report để xem các mục đã bỏ qua.",
        (keyboards::UiLanguage::En, false, true) => "Open the report to inspect skipped items.",
        (keyboards::UiLanguage::Vi, false, false) => {
            "Job đã sạch lỗi. Report JSON/CSV đã được gửi nếu có."
        }
        (keyboards::UiLanguage::En, false, false) => {
            "This job has no failed items. JSON/CSV reports were sent if available."
        }
    }
}

pub(crate) fn render_clone_outcome_from_job(
    lang: keyboards::UiLanguage,
    job: &repo::JobDetail,
) -> String {
    let mut lines = vec![
        clone_outcome_title(lang, &job.status).to_string(),
        "━━━━━━━━━━━━".to_string(),
    ];
    push_field(&mut lines, "Job", short_job_id(&job.id));
    push_field(
        &mut lines,
        clone_outcome_status_field(lang),
        job_status_label(lang, &job.status),
    );
    push_field(
        &mut lines,
        clone_plan_scanned_field(lang),
        &job.total_discovered.to_string(),
    );
    push_field(
        &mut lines,
        clone_outcome_completed_field(lang),
        &job.completed_items.to_string(),
    );
    push_field(
        &mut lines,
        clone_outcome_failed_field(lang),
        &job.failed_items.to_string(),
    );
    push_field(
        &mut lines,
        clone_outcome_skipped_field(lang),
        &job.skipped_items.to_string(),
    );
    if let Some(error) = &job.error_summary {
        push_field(&mut lines, clone_outcome_error_field(lang), error);
    }
    lines.push(String::new());
    lines.push(clone_outcome_hint(lang, job.failed_items, job.skipped_items).to_string());
    lines.join("\n")
}

pub(crate) fn clone_request_expired(lang: keyboards::UiLanguage) -> &'static str {
    match lang {
        keyboards::UiLanguage::Vi => "Yêu cầu clone đã hết hạn hoặc không thuộc về bạn.",
        keyboards::UiLanguage::En => "Clone request expired or does not belong to you.",
    }
}

#[allow(dead_code)]
pub(crate) fn clone_starting(lang: keyboards::UiLanguage) -> &'static str {
    match lang {
        keyboards::UiLanguage::Vi => "Đang bắt đầu clone...",
        keyboards::UiLanguage::En => "Starting clone...",
    }
}

pub(crate) fn clone_request_cancelled(lang: keyboards::UiLanguage) -> &'static str {
    match lang {
        keyboards::UiLanguage::Vi => "Đã huỷ yêu cầu clone.",
        keyboards::UiLanguage::En => "Clone request cancelled.",
    }
}

#[allow(dead_code)]
pub(crate) fn clone_job_accepted_text(lang: keyboards::UiLanguage) -> &'static str {
    match lang {
        keyboards::UiLanguage::Vi => "Đã nhận job clone. Theo dõi bằng /jobs hoặc mở lại /menu.",
        keyboards::UiLanguage::En => "Clone job accepted. Follow it from /jobs or reopen /menu.",
    }
}

pub(crate) fn clone_active_limit_text(lang: keyboards::UiLanguage, active_count: i64) -> String {
    match lang {
        keyboards::UiLanguage::Vi => format!(
            "Bạn đang có {active_count} job hoạt động. Chờ hoàn thành hoặc bấm huỷ trong /jobs."
        ),
        keyboards::UiLanguage::En => format!(
            "You already have {active_count} active jobs. Wait for one to finish or cancel one from /jobs."
        ),
    }
}

/// Human-readable byte count.
pub(crate) fn human_bytes(bytes: i64) -> String {
    let bytes = bytes.max(0) as u64;
    const KB: u64 = 1024;
    const MB: u64 = KB * 1024;
    const GB: u64 = MB * 1024;
    if bytes >= GB {
        format!("{:.1} GB", bytes as f64 / GB as f64)
    } else if bytes >= MB {
        format!("{:.1} MB", bytes as f64 / MB as f64)
    } else if bytes >= KB {
        format!("{:.1} KB", bytes as f64 / KB as f64)
    } else {
        format!("{bytes} B")
    }
}

#[derive(Debug, Default)]
pub(crate) struct ClonePlan {
    pub(crate) folders: u64,
    pub(crate) files: u64,
    pub(crate) shortcuts: u64,
    pub(crate) google_native: u64,
    pub(crate) known_bytes: u64,
    pub(crate) warning_count: u64,
    pub(crate) scanned_items: usize,
    pub(crate) truncated: bool,
}

impl ClonePlan {
    pub(crate) fn add(&mut self, file: &DriveFile) {
        self.scanned_items += 1;
        let caps = file.capabilities.as_ref();
        if file.is_folder() {
            self.folders += 1;
            if caps.and_then(|c| c.can_list_children) == Some(false) {
                self.warning_count += 1;
            }
        } else {
            self.files += 1;
            if file.is_shortcut() {
                self.shortcuts += 1;
            } else if file.mime_type.starts_with("application/vnd.google-apps.") {
                self.google_native += 1;
            }
            if caps.and_then(|c| c.can_copy) == Some(false) {
                self.warning_count += 1;
            }
        }
        if file.copy_requires_writer_permission == Some(true) {
            self.warning_count += 1;
        }
        if let Some(size) = &file.size
            && let Ok(bytes) = size.parse::<u64>()
        {
            self.known_bytes = self.known_bytes.saturating_add(bytes);
        }
    }

    pub(crate) fn render_lines(&self, lang: keyboards::UiLanguage) -> Vec<String> {
        let mut lines = vec![
            clone_plan_scanned_line(lang, self.scanned_items),
            clone_plan_folder_line(lang, self.folders),
            clone_plan_file_line(lang, self.files),
            clone_plan_google_native_line(lang, self.google_native),
            clone_plan_shortcut_line(lang, self.shortcuts),
            clone_plan_known_size_line(lang, self.known_bytes),
        ];
        if self.warning_count > 0 {
            lines.push(clone_plan_warning_line(lang, self.warning_count));
        }
        if self.truncated {
            lines.push(clone_plan_truncated_line(lang, self.scanned_items));
        }
        lines
    }
}
