//! `/preview` realtime overview dashboard.

use crate::state::{db::Database, repo};
use crate::telegram::i18n::TextKey as T;
use crate::telegram::keyboards;
use crate::telegram::render::{
    account_status_label, job_status_label, push_field, short_id, short_job_id,
};

pub(crate) async fn render_preview_dashboard(
    db: &Database,
    telegram_user_id: i64,
    lang: keyboards::UiLanguage,
) -> anyhow::Result<String> {
    let account = repo::account_status(db)
        .await?
        .unwrap_or_else(|| "chưa kết nối".to_string());
    let destination = repo::default_destination_profile(db, "default").await?;
    let jobs = repo::list_active_jobs_for_user(db, telegram_user_id, 5).await?;
    let counts = repo::job_status_counts(db).await?;

    let mut lines = vec![
        lang.text(T::PreviewTitle).to_string(),
        "━━━━━━━━━━━━".to_string(),
    ];
    push_field(&mut lines, "Google", account_status_label(lang, &account));
    match &destination {
        Some(p) => push_field(
            &mut lines,
            lang.text(T::Destination),
            &preview_destination_label(p),
        ),
        None => lines.push(format!(
            "• {}: {}",
            lang.text(T::Destination),
            lang.text(T::PreviewDestinationMissing)
        )),
    }

    if !counts.is_empty() {
        let summary = counts
            .iter()
            .map(|c| format!("{}: {}", job_status_label(lang, &c.status), c.count))
            .collect::<Vec<_>>()
            .join(" | ");
        push_field(&mut lines, lang.text(T::PreviewTotalJobs), &summary);
    }

    if jobs.is_empty() {
        lines.push(format!("• {}", lang.text(T::PreviewNoRunning)));
    } else {
        lines.push(String::new());
        lines.push(lang.text(T::PreviewRunningHeader).to_string());
        for job in &jobs {
            lines.push(format!(
                "• {} | {} | {} | {} | {}",
                short_job_id(&job.id),
                job_status_label(lang, &job.status),
                preview_scanned_word(lang),
                preview_done_word(lang),
                preview_failed_word(lang),
            ));
        }
    }

    lines.push(lang.text(T::PreviewFooter).to_string());
    Ok(lines.join("\n"))
}

fn preview_scanned_word(lang: keyboards::UiLanguage) -> &'static str {
    match lang {
        keyboards::UiLanguage::Vi => "quét",
        keyboards::UiLanguage::En => "scanned",
    }
}

fn preview_done_word(lang: keyboards::UiLanguage) -> &'static str {
    match lang {
        keyboards::UiLanguage::Vi => "xong",
        keyboards::UiLanguage::En => "done",
    }
}

fn preview_failed_word(lang: keyboards::UiLanguage) -> &'static str {
    match lang {
        keyboards::UiLanguage::Vi => "lỗi",
        keyboards::UiLanguage::En => "failed",
    }
}

pub(crate) fn preview_destination_label(p: &repo::DestinationProfile) -> String {
    let drive = if p.destination_drive_id.is_some() {
        "SD"
    } else {
        "My"
    };
    format!(
        "{} [{}:{}]",
        p.label,
        drive,
        short_id(&p.destination_parent_id)
    )
}

pub(crate) fn preview_load_error(lang: keyboards::UiLanguage, err: &anyhow::Error) -> String {
    match lang {
        keyboards::UiLanguage::Vi => format!("Lỗi tải bảng tổng quan: {err}"),
        keyboards::UiLanguage::En => format!("Could not load overview: {err}"),
    }
}

pub(crate) fn preview_update_error(lang: keyboards::UiLanguage, err: &anyhow::Error) -> String {
    match lang {
        keyboards::UiLanguage::Vi => format!("Lỗi cập nhật: {err}"),
        keyboards::UiLanguage::En => format!("Could not update overview: {err}"),
    }
}
