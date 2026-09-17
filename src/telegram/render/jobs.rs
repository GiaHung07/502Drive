//! Jobs list panel texts.

use teloxide::types::InlineKeyboardMarkup;

use crate::state::{db::Database, repo};
use crate::telegram::keyboards;
use crate::telegram::render::{job_status_label, push_field, short_job_id};

pub(crate) async fn render_jobs_panel(
    db: &Database,
    telegram_user_id: i64,
    lang: keyboards::UiLanguage,
) -> anyhow::Result<(String, InlineKeyboardMarkup)> {
    let jobs = repo::list_active_jobs_for_user(db, telegram_user_id, 10).await?;
    if jobs.is_empty() {
        return Ok((
            jobs_empty_text(lang).to_string(),
            keyboards::job_list_keyboard(&[], lang),
        ));
    }
    let mut lines = vec![jobs_title(lang).to_string(), "━━━━━━━━━━".to_string()];
    let mut buttons = Vec::with_capacity(jobs.len());
    for job in &jobs {
        lines.push(String::new());
        push_field(&mut lines, "Job", short_job_id(&job.id));
        push_field(
            &mut lines,
            jobs_status_field(lang),
            job_status_label(lang, &job.status),
        );
        push_field(
            &mut lines,
            jobs_progress_field(lang),
            &jobs_progress_text(
                lang,
                job.completed_items,
                job.total_discovered,
                job.failed_items,
            ),
        );
        buttons.push((
            job.id.clone(),
            format!(
                "{} · {} · {}/{}",
                short_job_id(&job.id),
                job_status_label(lang, &job.status),
                job.completed_items,
                job.total_discovered
            ),
        ));
    }
    Ok((
        lines.join("\n"),
        keyboards::job_list_keyboard(&buttons, lang),
    ))
}

pub(crate) fn jobs_title(lang: keyboards::UiLanguage) -> &'static str {
    match lang {
        keyboards::UiLanguage::Vi => "JOB ĐANG CHẠY",
        keyboards::UiLanguage::En => "ACTIVE JOBS",
    }
}

pub(crate) fn jobs_empty_text(lang: keyboards::UiLanguage) -> &'static str {
    match lang {
        keyboards::UiLanguage::Vi => "JOB ĐANG CHẠY\n━━━━━━━━━━\nKhông có job đang chạy.",
        keyboards::UiLanguage::En => "ACTIVE JOBS\n━━━━━━━━━━\nNo active jobs.",
    }
}

pub(crate) fn jobs_status_field(lang: keyboards::UiLanguage) -> &'static str {
    match lang {
        keyboards::UiLanguage::Vi => "Trạng thái",
        keyboards::UiLanguage::En => "Status",
    }
}

pub(crate) fn jobs_progress_field(lang: keyboards::UiLanguage) -> &'static str {
    match lang {
        keyboards::UiLanguage::Vi => "Tiến độ",
        keyboards::UiLanguage::En => "Progress",
    }
}

pub(crate) fn jobs_progress_text(
    lang: keyboards::UiLanguage,
    completed: i64,
    discovered: i64,
    failed: i64,
) -> String {
    match lang {
        keyboards::UiLanguage::Vi => {
            format!("{completed} xong / {discovered} quét / {failed} lỗi")
        }
        keyboards::UiLanguage::En => {
            format!("{completed} done / {discovered} scanned / {failed} failed")
        }
    }
}
