//! Job detail, progress rendering, job action messages and job status labels.

use crate::engine::services::JobService;
use crate::state::db::now_ms;
use crate::state::repo;
use crate::telegram::i18n::TextKey as T;
use crate::telegram::keyboards;
use crate::telegram::progress::{estimate_eta_secs, format_duration_secs, render_progress};
use crate::telegram::render::{push_field, short_id, short_job_id};

pub(crate) fn render_job_progress(
    job: &repo::JobDetail,
    elapsed_secs: u64,
    lang: keyboards::UiLanguage,
) -> String {
    let progress = JobService::progress(job, now_ms());
    let elapsed = if elapsed_secs > 0 {
        elapsed_secs
    } else {
        progress.elapsed_seconds
    };

    let total = if matches!(
        job.status.as_str(),
        "running" | "completed" | "partially_completed" | "failed"
    ) && job.total_discovered > 0
    {
        Some(job.total_discovered as u64)
    } else {
        None
    };

    let done = job.completed_items.max(0) as u64;

    let rate_str = if elapsed > 0 {
        progress_rate_value(lang, done as f64 / elapsed as f64)
    } else {
        progress
            .items_per_second
            .map(|r| progress_rate_value(lang, r))
            .unwrap_or_else(|| "--".to_string())
    };

    let eta_str = total
        .and_then(|t| {
            if elapsed > 0 {
                estimate_eta_secs(done, t, elapsed)
            } else {
                progress.eta_seconds
            }
        })
        .map(format_duration_secs)
        .unwrap_or_else(|| "--".to_string());

    [
        progress_title(lang).to_string(),
        "━━━━━━━━━━━━━━".to_string(),
        format!(
            "• {}: {}",
            progress_status_field(lang),
            job_status_label(lang, &job.status)
        ),
        format!("• Job: {}", short_job_id(&job.id)),
        format!(
            "• {}: {}",
            progress_elapsed_field(lang),
            format_duration_secs(elapsed)
        ),
        format!("• {}: {rate_str}", progress_rate_field(lang)),
        format!("• {}: {eta_str}", progress_eta_field(lang)),
        format!(
            "• {}: {}",
            progress_scanned_field(lang),
            job.total_discovered
        ),
        format!("• {}: {}", progress_skipped_field(lang), job.skipped_items),
        String::new(),
        render_progress(lang, total, done, job.failed_items as u64),
    ]
    .join("\n")
}

/// Clean up error strings for display in Telegram without stack traces or raw panic lines.
pub(crate) fn sanitize_error_summary(raw: &str) -> String {
    let mut cleaned = raw.trim();
    // Strip common prefixes
    if let Some(rest) = cleaned.strip_prefix("Error:") {
        cleaned = rest.trim();
    } else if let Some(rest) = cleaned.strip_prefix("error:") {
        cleaned = rest.trim();
    }
    // Take only the first non-empty line
    let first_line = cleaned
        .lines()
        .map(str::trim)
        .find(|l| !l.is_empty())
        .unwrap_or(cleaned);
    // Ignore lines that look like stack backtrace indicators
    if first_line.starts_with("stack backtrace:") || first_line.starts_with("at ") {
        return String::new();
    }
    // Truncate to reasonable length for Telegram UI
    if first_line.chars().count() > 160 {
        let keep: String = first_line.chars().take(157).collect();
        format!("{keep}...")
    } else {
        first_line.to_string()
    }
}

pub(crate) fn render_job_end_state(
    job: &repo::JobDetail,
    elapsed_secs: u64,
    lang: keyboards::UiLanguage,
) -> String {
    let mut lines = Vec::new();
    let title = if job.status == "completed" && job.failed_items == 0 {
        lang.text(T::CloneCompletedTitle)
    } else if job.status == "cancelled" {
        lang.text(T::CloneCancelledTitle)
    } else {
        lang.text(T::CloneIncompleteTitle)
    };
    lines.push(title.to_string());
    lines.push("━━━━━━━━━━━━━━".to_string());
    push_field(&mut lines, "Job", short_job_id(&job.id));
    push_field(
        &mut lines,
        progress_status_field(lang),
        job_status_label(lang, &job.status),
    );
    push_field(
        &mut lines,
        job_detail_completed_field(lang),
        &job.completed_items.to_string(),
    );
    if job.failed_items > 0 {
        push_field(
            &mut lines,
            job_detail_failed_field(lang),
            &job.failed_items.to_string(),
        );
    }
    if job.skipped_items > 0 {
        push_field(
            &mut lines,
            progress_skipped_field(lang),
            &job.skipped_items.to_string(),
        );
    }
    push_field(
        &mut lines,
        progress_elapsed_field(lang),
        &format_duration_secs(elapsed_secs),
    );
    if let Some(err) = &job.error_summary {
        let clean_err = sanitize_error_summary(err);
        if !clean_err.is_empty() {
            push_field(&mut lines, job_detail_error_field(lang), &clean_err);
        }
    }
    lines.join("\n")
}

pub(crate) fn progress_title(lang: keyboards::UiLanguage) -> &'static str {
    match lang {
        keyboards::UiLanguage::Vi => "TIẾN TRÌNH CLONE",
        keyboards::UiLanguage::En => "CLONE PROGRESS",
    }
}

pub(crate) fn progress_status_field(lang: keyboards::UiLanguage) -> &'static str {
    match lang {
        keyboards::UiLanguage::Vi => "Trạng thái",
        keyboards::UiLanguage::En => "Status",
    }
}

#[allow(dead_code)]
pub(crate) fn progress_queued_label(lang: keyboards::UiLanguage) -> &'static str {
    match lang {
        keyboards::UiLanguage::Vi => "đang xếp hàng",
        keyboards::UiLanguage::En => "queued",
    }
}

pub(crate) fn progress_preparing_label(lang: keyboards::UiLanguage) -> &'static str {
    match lang {
        keyboards::UiLanguage::Vi => "đang chuẩn bị",
        keyboards::UiLanguage::En => "preparing",
    }
}

pub(crate) fn progress_elapsed_field(lang: keyboards::UiLanguage) -> &'static str {
    match lang {
        keyboards::UiLanguage::Vi => "Thời gian",
        keyboards::UiLanguage::En => "Elapsed",
    }
}

pub(crate) fn progress_rate_field(lang: keyboards::UiLanguage) -> &'static str {
    match lang {
        keyboards::UiLanguage::Vi => "Tốc độ",
        keyboards::UiLanguage::En => "Rate",
    }
}

pub(crate) fn progress_eta_field(lang: keyboards::UiLanguage) -> &'static str {
    match lang {
        keyboards::UiLanguage::Vi => "Dự kiến",
        keyboards::UiLanguage::En => "ETA",
    }
}

pub(crate) fn progress_scanned_field(lang: keyboards::UiLanguage) -> &'static str {
    match lang {
        keyboards::UiLanguage::Vi => "Đã quét",
        keyboards::UiLanguage::En => "Scanned",
    }
}

pub(crate) fn progress_skipped_field(lang: keyboards::UiLanguage) -> &'static str {
    match lang {
        keyboards::UiLanguage::Vi => "Bỏ qua",
        keyboards::UiLanguage::En => "Skipped",
    }
}

pub(crate) fn progress_rate_value(lang: keyboards::UiLanguage, rate: f64) -> String {
    match lang {
        keyboards::UiLanguage::Vi => format!("{rate:.1} item/giây"),
        keyboards::UiLanguage::En => format!("{rate:.1} item/s"),
    }
}

pub(crate) fn is_terminal_status(status: &str) -> bool {
    matches!(
        status,
        "completed" | "partially_completed" | "failed" | "cancelled"
    )
}

pub(crate) fn render_job_detail(job: &repo::JobDetail, lang: keyboards::UiLanguage) -> String {
    let mut lines = vec![job_detail_title(lang).to_string(), "━━━━━━━━━━".to_string()];
    push_field(&mut lines, "Job", &job.id);
    push_field(&mut lines, job_detail_kind_field(lang), &job.kind);
    push_field(
        &mut lines,
        progress_status_field(lang),
        job_status_label(lang, &job.status),
    );
    push_field(
        &mut lines,
        job_detail_source_field(lang),
        &job.source_root_id,
    );
    push_field(
        &mut lines,
        job_detail_destination_field(lang),
        &job.destination_parent_id,
    );
    lines.push(String::new());
    push_field(
        &mut lines,
        progress_scanned_field(lang),
        &job.total_discovered.to_string(),
    );
    push_field(
        &mut lines,
        job_detail_completed_field(lang),
        &job.completed_items.to_string(),
    );
    push_field(
        &mut lines,
        job_detail_failed_field(lang),
        &job.failed_items.to_string(),
    );
    push_field(
        &mut lines,
        progress_skipped_field(lang),
        &job.skipped_items.to_string(),
    );
    if let Some(error) = &job.error_summary {
        lines.push(String::new());
        push_field(&mut lines, job_detail_error_field(lang), error);
    }
    lines.join("\n")
}

pub(crate) fn job_detail_title(lang: keyboards::UiLanguage) -> &'static str {
    match lang {
        keyboards::UiLanguage::Vi => "CHI TIẾT JOB",
        keyboards::UiLanguage::En => "JOB DETAIL",
    }
}

pub(crate) fn job_detail_kind_field(lang: keyboards::UiLanguage) -> &'static str {
    match lang {
        keyboards::UiLanguage::Vi => "Loại",
        keyboards::UiLanguage::En => "Type",
    }
}

pub(crate) fn job_detail_source_field(lang: keyboards::UiLanguage) -> &'static str {
    match lang {
        keyboards::UiLanguage::Vi => "Nguồn",
        keyboards::UiLanguage::En => "Source",
    }
}

pub(crate) fn job_detail_destination_field(lang: keyboards::UiLanguage) -> &'static str {
    match lang {
        keyboards::UiLanguage::Vi => "Đích",
        keyboards::UiLanguage::En => "Destination",
    }
}

pub(crate) fn job_detail_completed_field(lang: keyboards::UiLanguage) -> &'static str {
    match lang {
        keyboards::UiLanguage::Vi => "Hoàn tất",
        keyboards::UiLanguage::En => "Completed",
    }
}

pub(crate) fn job_detail_failed_field(lang: keyboards::UiLanguage) -> &'static str {
    match lang {
        keyboards::UiLanguage::Vi => "Lỗi",
        keyboards::UiLanguage::En => "Failed",
    }
}

pub(crate) fn job_detail_error_field(lang: keyboards::UiLanguage) -> &'static str {
    match lang {
        keyboards::UiLanguage::Vi => "Lỗi gần nhất",
        keyboards::UiLanguage::En => "Latest error",
    }
}

pub(crate) fn job_no_active_text(lang: keyboards::UiLanguage) -> &'static str {
    match lang {
        keyboards::UiLanguage::Vi => {
            "Không có job đang chạy. Dùng /last_report để lấy report gần nhất."
        }
        keyboards::UiLanguage::En => "No active job. Use /last_report to get the latest report.",
    }
}

pub(crate) fn job_active_missing_text(lang: keyboards::UiLanguage) -> &'static str {
    match lang {
        keyboards::UiLanguage::Vi => "Job đang chạy không còn tồn tại.",
        keyboards::UiLanguage::En => "The active job no longer exists.",
    }
}

pub(crate) fn job_not_found_text(lang: keyboards::UiLanguage) -> &'static str {
    match lang {
        keyboards::UiLanguage::Vi => "Không tìm thấy job thuộc tài khoản của bạn.",
        keyboards::UiLanguage::En => "No job found for your account.",
    }
}

pub(crate) fn job_prefix_ambiguous_text(lang: keyboards::UiLanguage) -> &'static str {
    match lang {
        keyboards::UiLanguage::Vi => "Có nhiều job trùng prefix. Nhập thêm vài ký tự job ID.",
        keyboards::UiLanguage::En => {
            "More than one job matches that prefix. Enter a few more job ID characters."
        }
    }
}

pub(crate) fn job_pause_requested_text(lang: keyboards::UiLanguage, job_id: &str) -> String {
    match lang {
        keyboards::UiLanguage::Vi => format!("Đã yêu cầu tạm dừng job {}.", short_job_id(job_id)),
        keyboards::UiLanguage::En => format!("Pause requested for job {}.", short_job_id(job_id)),
    }
}

pub(crate) fn job_pause_failed_text(lang: keyboards::UiLanguage) -> &'static str {
    match lang {
        keyboards::UiLanguage::Vi => {
            "Không tìm thấy job hoặc trạng thái hiện tại không cho tạm dừng."
        }
        keyboards::UiLanguage::En => "Job not found or cannot be paused from its current state.",
    }
}

pub(crate) fn job_resume_requested_text(lang: keyboards::UiLanguage, job_id: &str) -> String {
    match lang {
        keyboards::UiLanguage::Vi => format!("Đã yêu cầu tiếp tục job {}.", short_job_id(job_id)),
        keyboards::UiLanguage::En => format!("Resume requested for job {}.", short_job_id(job_id)),
    }
}

pub(crate) fn job_resume_failed_text(lang: keyboards::UiLanguage) -> &'static str {
    match lang {
        keyboards::UiLanguage::Vi => "Không tìm thấy job hoặc job chưa ở trạng thái tạm dừng.",
        keyboards::UiLanguage::En => "Job not found or is not paused.",
    }
}

pub(crate) fn job_cancel_requested_text(lang: keyboards::UiLanguage, job_id: &str) -> String {
    match lang {
        keyboards::UiLanguage::Vi => format!("Đã yêu cầu huỷ job {}.", short_job_id(job_id)),
        keyboards::UiLanguage::En => format!("Cancel requested for job {}.", short_job_id(job_id)),
    }
}

pub(crate) fn job_cancel_failed_text(lang: keyboards::UiLanguage) -> &'static str {
    match lang {
        keyboards::UiLanguage::Vi => "Không tìm thấy job hoặc trạng thái hiện tại không cho huỷ.",
        keyboards::UiLanguage::En => "Job not found or cannot be cancelled from its current state.",
    }
}

pub(crate) fn job_retry_requested_text(
    lang: keyboards::UiLanguage,
    job_id: &str,
    summary: &repo::RetryFailedSummary,
) -> String {
    match lang {
        keyboards::UiLanguage::Vi => format!(
            "Đã xếp hàng làm lại job {}.\nThư mục: {}  Item: {}  Thao tác: {}",
            short_job_id(job_id),
            summary.traversal_folders_requeued,
            summary.job_items_requeued,
            summary.operation_intents_replanned,
        ),
        keyboards::UiLanguage::En => format!(
            "Retry queued for job {}.\nFolders: {}  Items: {}  Operations: {}",
            short_job_id(job_id),
            summary.traversal_folders_requeued,
            summary.job_items_requeued,
            summary.operation_intents_replanned,
        ),
    }
}

pub(crate) fn job_retry_failed_text(lang: keyboards::UiLanguage) -> &'static str {
    match lang {
        keyboards::UiLanguage::Vi => {
            "Không tìm thấy job, job không thể retry, hoặc không có phần lỗi để làm lại."
        }
        keyboards::UiLanguage::En => {
            "Job not found, cannot be retried, or has no failed items to retry."
        }
    }
}

pub(crate) fn render_report_summary(
    lang: keyboards::UiLanguage,
    title: T,
    job_id: &str,
    status: &str,
    total_discovered: i64,
    completed_items: i64,
    failed_items: i64,
    skipped_items: i64,
) -> String {
    let next = if failed_items > 0 {
        lang.text(T::ReportHintRetry)
    } else if skipped_items > 0 {
        lang.text(T::ReportHintSkipped)
    } else {
        lang.text(T::ReportHintClean)
    };
    format!(
        "{}\n\
         ━━━━━━━━━━━━\n\
         • Job: {}\n\
         • {}: {}\n\
         • {}: {total_discovered}\n\
         • {}: {completed_items}\n\
         • {}: {failed_items}\n\
         • {}: {skipped_items}\n\
         \n\
        {next}",
        lang.text(title),
        short_id(job_id),
        lang.text(T::ReportStatus),
        job_status_label(lang, status),
        lang.text(T::ReportScanned),
        lang.text(T::ReportCompleted),
        lang.text(T::ReportFailed),
        lang.text(T::ReportSkipped),
    )
}

pub(crate) fn report_none_text(lang: keyboards::UiLanguage) -> &'static str {
    match lang {
        keyboards::UiLanguage::Vi => "Chưa có job hoàn tất/lỗi nào để gửi report.",
        keyboards::UiLanguage::En => "No completed or failed job has a report yet.",
    }
}

pub(crate) fn report_job_not_found(lang: keyboards::UiLanguage) -> &'static str {
    match lang {
        keyboards::UiLanguage::Vi => "Không tìm thấy job này.",
        keyboards::UiLanguage::En => "Job not found.",
    }
}

pub(crate) fn vi_job_status(status: &str) -> &str {
    match status {
        "queued" => "đang chờ",
        "discovering" => "đang quét",
        "running" => "đang chạy",
        "pausing" => "đang tạm dừng",
        "paused" => "tạm dừng",
        "cancelling" => "đang huỷ",
        "cancelled" => "đã huỷ",
        "recovering" => "đang phục hồi",
        "completed" => "hoàn tất",
        "partially_completed" => "hoàn tất một phần",
        "failed" => "thất bại",
        other => other,
    }
}

pub(crate) fn job_status_label(lang: keyboards::UiLanguage, status: &str) -> &str {
    if lang == keyboards::UiLanguage::Vi {
        return vi_job_status(status);
    }
    match status {
        "queued" => "queued",
        "discovering" => "discovering",
        "running" => "running",
        "pausing" => "pausing",
        "paused" => "paused",
        "cancelling" => "cancelling",
        "cancelled" => "cancelled",
        "recovering" => "recovering",
        "completed" => "completed",
        "partially_completed" => "partially completed",
        "failed" => "failed",
        other => other,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sanitize_error_summary_strips_prefixes_and_backtraces() {
        assert_eq!(
            sanitize_error_summary("Error: Rate limit exceeded (429)"),
            "Rate limit exceeded (429)"
        );
        assert_eq!(
            sanitize_error_summary("error: Network timeout\nstack backtrace:\n   0: ..."),
            "Network timeout"
        );
        assert_eq!(sanitize_error_summary("stack backtrace:\n 0: foo"), "");
    }

    #[test]
    fn render_job_end_state_formats_success_and_failure() {
        let success = repo::JobDetail {
            id: "job-123456789".to_string(),
            kind: "one_shot".to_string(),
            status: "completed".to_string(),
            source_root_id: "src-id".to_string(),
            destination_parent_id: "dst-id".to_string(),
            source_name: None,
            destination_name: None,
            total_discovered: 42,
            completed_items: 42,
            failed_items: 0,
            skipped_items: 0,
            error_summary: None,
            created_at_ms: 1000,
            updated_at_ms: 15000,
        };
        let text_vi = render_job_end_state(&success, 14, keyboards::UiLanguage::Vi);
        assert!(text_vi.contains("✓ Đã sao chép"));
        assert!(text_vi.contains("Job: job-1234"));
        assert!(text_vi.contains("Hoàn tất: 42"));
        assert!(text_vi.contains("Thời gian: 00:14"));

        let failed = repo::JobDetail {
            id: "job-123456789".to_string(),
            kind: "one_shot".to_string(),
            status: "failed".to_string(),
            source_root_id: "src-id".to_string(),
            destination_parent_id: "dst-id".to_string(),
            source_name: None,
            destination_name: None,
            total_discovered: 42,
            completed_items: 40,
            failed_items: 2,
            skipped_items: 0,
            error_summary: Some("Error: Permission denied on 2 items".to_string()),
            created_at_ms: 1000,
            updated_at_ms: 15000,
        };
        let text_en = render_job_end_state(&failed, 14, keyboards::UiLanguage::En);
        assert!(text_en.contains("⚠ Clone incomplete"));
        assert!(text_en.contains("Completed: 40"));
        assert!(text_en.contains("Failed: 2"));
        assert!(text_en.contains("Latest error: Permission denied on 2 items"));
    }
}
