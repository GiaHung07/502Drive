use std::{
    collections::VecDeque,
    time::{Duration, Instant},
};

use teloxide::{
    prelude::*,
    types::{InputFile, MessageId},
    utils::command::BotCommands,
};
use tokio::sync::oneshot;
use tracing::warn;

use crate::{
    config::AppConfig,
    drive::{
        auth as oauth,
        client::{DriveApiError, DriveClient},
        links::parse_drive_reference,
        token_manager::TokenManager,
        types::FOLDER_MIME_TYPE,
    },
    engine::{
        copy::{CloneOutcome, CloneRequest, CloneService},
        recovery,
    },
    report,
    secrets::FileSecretStore,
    state::{db::Database, repo},
    telegram::{
        commands::Command,
        keyboards,
        progress::{estimate_eta_secs, format_duration_secs, render_progress},
    },
    watch::run_initial_clone,
};

const CALLBACK_STATE_TTL_MS: i64 = 15 * 60 * 1000;
const CLONE_PLAN_ITEM_LIMIT: usize = 2_000;

pub async fn handle_message(
    bot: Bot,
    msg: Message,
    config: AppConfig,
    db: Database,
) -> ResponseResult<()> {
    let user_id = match msg.from.as_ref() {
        Some(user) => user.id.0 as i64,
        None => return Ok(()),
    };

    if !repo::is_authorized(&db, user_id).await.unwrap_or(false) {
        bot.send_message(
            msg.chat.id,
            "Không có quyền truy cập. Liên hệ chủ sở hữu để được cấp phép.",
        )
        .await?;
        return Ok(());
    }

    let Some(text) = msg.text() else {
        return Ok(());
    };

    if let Ok(command) = Command::parse(text, "gdclone_bot") {
        return handle_command(bot, msg, config, db, user_id, command).await;
    }

    match parse_drive_reference(text) {
        Ok(_) => {
            handle_clone_request(bot, msg.chat.id, config, db, user_id, text.to_string()).await?;
        }
        Err(err) => {
            bot.send_message(msg.chat.id, format!("Link Drive không hợp lệ: {err}"))
                .await?;
        }
    }
    Ok(())
}

pub async fn handle_callback_query(
    bot: Bot,
    query: CallbackQuery,
    config: AppConfig,
    db: Database,
) -> ResponseResult<()> {
    bot.answer_callback_query(query.id.clone()).await?;

    let user_id = query.from.id.0 as i64;
    let chat_id = query.message.as_ref().map(|m| m.chat().id);
    if !repo::is_authorized(&db, user_id).await.unwrap_or(false) {
        if let Some(chat_id) = chat_id {
            bot.send_message(chat_id, "Không có quyền truy cập.")
                .await?;
        }
        return Ok(());
    }

    let Some(data) = query.data.as_deref() else {
        return Ok(());
    };
    let Some(chat_id) = chat_id else {
        return Ok(());
    };

    let text = match parse_callback_action(data) {
        Some(("clone", "confirm", source)) => {
            let Some(source) =
                repo::consume_callback_state(&db, source, user_id, chat_id.0, "clone_confirm")
                    .await
                    .unwrap_or(None)
            else {
                let msg_text = "Yêu cầu clone đã hết hạn hoặc không thuộc về bạn.";
                if let Some(message) = query.message.as_ref() {
                    bot.edit_message_text(chat_id, message.id(), msg_text)
                        .await?;
                } else {
                    bot.send_message(chat_id, msg_text).await?;
                }
                return Ok(());
            };
            if let Some(message) = query.message.as_ref() {
                let _ = bot
                    .edit_message_text(chat_id, message.id(), "Đang bắt đầu clone...")
                    .await;
            }
            spawn_clone_now(bot.clone(), chat_id, config, db, user_id, source).await?;
            return Ok(());
        }
        Some(("clone", "cancel", state_id)) => {
            let _ =
                repo::consume_callback_state(&db, state_id, user_id, chat_id.0, "clone_confirm")
                    .await;
            if let Some(message) = query.message.as_ref() {
                bot.edit_message_text(chat_id, message.id(), "Đã huỷ yêu cầu clone.")
                    .await?;
                return Ok(());
            }
            "Đã huỷ yêu cầu clone.".to_string()
        }
        Some(("job", "pause", job_id)) => pause_job(&db, user_id, job_id)
            .await
            .unwrap_or_else(|err| err.to_string()),
        Some(("job", "resume", job_id)) => resume_job(&config, &db, user_id, job_id)
            .await
            .unwrap_or_else(|err| err.to_string()),
        Some(("job", "cancel", job_id)) => cancel_job(&db, user_id, job_id)
            .await
            .unwrap_or_else(|err| err.to_string()),
        Some(("dest", "select", profile_id)) => {
            match repo::set_default_destination_by_id(&db, "default", profile_id).await {
                Ok(true) => {
                    // Re-render the destination panel in place.
                    let profiles = repo::list_recent_destinations(&db, "default", 5)
                        .await
                        .unwrap_or_default();
                    let text = render_destination_list(&profiles);
                    if let Some(message) = query.message.as_ref() {
                        let _ = bot
                            .edit_message_text(chat_id, message.id(), &text)
                            .reply_markup(keyboards::recent_destinations_keyboard(&profiles))
                            .await;
                        return Ok(());
                    }
                    text
                }
                Ok(false) => "Không tìm thấy thư mục đích.".to_string(),
                Err(err) => format!("Lỗi đổi đích: {err}"),
            }
        }
        Some(("browse", _, _)) => "Tính năng chọn thư mục chưa được hỗ trợ.".to_string(),
        _ => "Hành động không xác định.".to_string(),
    };

    bot.send_message(chat_id, text).await?;
    Ok(())
}

fn parse_callback_action(data: &str) -> Option<(&str, &str, &str)> {
    let mut parts = data.splitn(3, ':');
    Some((parts.next()?, parts.next()?, parts.next()?))
}

// ── Command dispatch ─────────────────────────────────────────────────────────

async fn handle_command(
    bot: Bot,
    msg: Message,
    config: AppConfig,
    db: Database,
    user_id: i64,
    command: Command,
) -> ResponseResult<()> {
    match command {
        Command::Start => {
            bot.send_message(
                msg.chat.id,
                "DRIVE502 ĐANG CHẠY\n\
                 ━━━━━━━━━━━━━━\n\
                 1. /account để kiểm tra Google\n\
                 2. /destination để kiểm tra thư mục đích\n\
                 3. Dán link Drive hoặc dùng /clone <url>",
            )
            .await?;
        }
        Command::Help => {
            bot.send_message(
                msg.chat.id,
                "LỆNH CHÍNH\n\
                 ━━━━━━━━━\n\
                 /clone <url>           Kiểm tra nguồn, xem kế hoạch, rồi clone\n\
                 /clone_here <url>      Clone ngay vào thư mục đích mặc định\n\
                 /destination           Xem/đổi thư mục đích đã lưu\n\
                 /set_destination <url> Đặt thư mục đích mặc định\n\
                 /clear_destination     Xoá thư mục đích mặc định\n\
                 /jobs                  Job đang chạy hoặc tạm dừng\n\
                 /status <job_id>       Chi tiết một job\n\
                 /pause <job_id>        Tạm dừng\n\
                 /resume <job_id>       Tiếp tục\n\
                 /cancel <job_id>       Huỷ\n\
                 /retry <job_id>        Làm lại phần lỗi\n\
                 /last_report           Gửi lại report job gần nhất\n\
                 /preview               Bảng tổng quan realtime\n\
                 /account               Tài khoản Google\n\
                 \n\
                 Quản trị: /whoami /grant /revoke /disconnect\n\
                 Watch đang để sau, không hiện trong menu chính.",
            )
            .await?;
        }
        Command::Preview => {
            spawn_preview_dashboard(bot, msg.chat.id, db, user_id).await?;
        }
        Command::Connect => {
            bot.send_message(
                msg.chat.id,
                "Google Auth chạy local-first.\n\
                 \n\
                 Chạy lệnh sau trên máy đang chạy bot:\n\
                 \n\
                   gdclone-bot auth login\n\
                 \n\
                 Sau đó dùng /account để kiểm tra kết nối.",
            )
            .await?;
        }
        Command::Account => {
            let text = account_summary(&config, &db)
                .await
                .unwrap_or_else(|err| format!("Lỗi đọc trạng thái tài khoản: {err}"));
            bot.send_message(msg.chat.id, text).await?;
        }
        Command::Disconnect => {
            let text = disconnect_google(&config, &db, user_id).await;
            bot.send_message(msg.chat.id, text.unwrap_or_else(|err| err.to_string()))
                .await?;
        }
        Command::Clone(input) => {
            handle_clone_request(bot, msg.chat.id, config, db, user_id, input).await?;
        }
        Command::CloneHere(input) => {
            spawn_clone_now(bot, msg.chat.id, config, db, user_id, input).await?;
        }
        Command::Whoami => {
            let role = repo::authorized_user(&db, user_id)
                .await
                .ok()
                .flatten()
                .map(|u| u.role)
                .unwrap_or_else(|| "không rõ".to_string());
            bot.send_message(
                msg.chat.id,
                format!(
                    "NGƯỜI DÙNG\n\
                     ━━━━━━━━━\n\
                     Telegram ID : {user_id}\n\
                     Quyền       : {role}"
                ),
            )
            .await?;
        }
        Command::Destination => {
            spawn_destination_panel(bot, msg.chat.id, db).await?;
        }
        Command::ClearDestination => {
            let text = match repo::clear_default_destination(&db, "default").await {
                Ok(0) => "Chưa có thư mục đích mặc định để xoá.".to_string(),
                Ok(_) => "Đã xoá thư mục đích mặc định.".to_string(),
                Err(err) => format!("Lỗi xoá thư mục đích: {err}"),
            };
            bot.send_message(msg.chat.id, text).await?;
        }
        Command::SetDestination(input) => {
            let text = set_destination(&config, &db, &input).await;
            bot.send_message(
                msg.chat.id,
                text.unwrap_or_else(|err| format_error_for_user(&err)),
            )
            .await?;
        }
        Command::Jobs => {
            let text = list_jobs(&db, user_id).await;
            bot.send_message(msg.chat.id, text.unwrap_or_else(|err| err.to_string()))
                .await?;
        }
        Command::Status(job_id) => {
            let text = show_job_status(&db, user_id, &job_id).await;
            bot.send_message(msg.chat.id, text.unwrap_or_else(|err| err.to_string()))
                .await?;
        }
        Command::Pause(job_id) => {
            let text = pause_job(&db, user_id, &job_id).await;
            bot.send_message(msg.chat.id, text.unwrap_or_else(|err| err.to_string()))
                .await?;
        }
        Command::Resume(job_id) => {
            let text = resume_job(&config, &db, user_id, &job_id).await;
            bot.send_message(msg.chat.id, text.unwrap_or_else(|err| err.to_string()))
                .await?;
        }
        Command::Cancel(job_id) => {
            let text = cancel_job(&db, user_id, &job_id).await;
            bot.send_message(msg.chat.id, text.unwrap_or_else(|err| err.to_string()))
                .await?;
        }
        Command::Retry(job_id) => {
            let text = retry_job(&config, &db, user_id, &job_id).await;
            bot.send_message(msg.chat.id, text.unwrap_or_else(|err| err.to_string()))
                .await?;
        }
        Command::LastReport => {
            send_last_report(&bot, msg.chat.id, &config, &db, user_id).await?;
        }
        Command::Grant(input) => {
            let text = grant_user(&db, user_id, &input).await;
            bot.send_message(msg.chat.id, text.unwrap_or_else(|err| err.to_string()))
                .await?;
        }
        Command::Revoke(input) => {
            let text = revoke_user(&db, user_id, &input).await;
            bot.send_message(msg.chat.id, text.unwrap_or_else(|err| err.to_string()))
                .await?;
        }
        // ── Watch ────────────────────────────────────────────────────────────
        Command::Watch(input) => {
            let text = start_watch(&bot, &config, &db, msg.chat.id.0, user_id, &input).await;
            bot.send_message(msg.chat.id, text.unwrap_or_else(|err| err.to_string()))
                .await?;
        }
        Command::Watches => {
            let text = list_watches(&db, user_id).await;
            bot.send_message(msg.chat.id, text.unwrap_or_else(|err| err.to_string()))
                .await?;
        }
        Command::WatchStatus(watch_id) => {
            let text = watch_status_detail(&db, user_id, &watch_id).await;
            bot.send_message(msg.chat.id, text.unwrap_or_else(|err| err.to_string()))
                .await?;
        }
        Command::WatchPause(watch_id) => {
            let text = match repo::pause_watch_for_user(&db, user_id, &watch_id).await {
                Ok(true) => format!("Watch {watch_id} đã tạm dừng."),
                Ok(false) => "Không tìm thấy watch hoặc không thể tạm dừng.".to_string(),
                Err(err) => format!("Lỗi: {err}"),
            };
            bot.send_message(msg.chat.id, text).await?;
        }
        Command::WatchResume(watch_id) => {
            let text = match repo::resume_watch_for_user(&db, user_id, &watch_id).await {
                Ok(true) => format!("Watch {watch_id} đã tiếp tục (đang bắt kịp)."),
                Ok(false) => {
                    "Không tìm thấy watch hoặc watch chưa ở trạng thái tạm dừng.".to_string()
                }
                Err(err) => format!("Lỗi: {err}"),
            };
            bot.send_message(msg.chat.id, text).await?;
        }
        Command::WatchPolicy(input) => {
            let text = set_watch_policy(&db, user_id, &input).await;
            bot.send_message(msg.chat.id, text.unwrap_or_else(|err| err.to_string()))
                .await?;
        }
        Command::Unwatch(watch_id) => {
            let text = match repo::stop_watch_for_user(&db, user_id, &watch_id).await {
                Ok(true) => format!("Watch {watch_id} đã dừng."),
                Ok(false) => "Không tìm thấy watch hoặc đã dừng trước đó.".to_string(),
                Err(err) => format!("Lỗi: {err}"),
            };
            bot.send_message(msg.chat.id, text).await?;
        }
    }
    Ok(())
}

// ── Job listing ──────────────────────────────────────────────────────────────

async fn list_jobs(db: &Database, telegram_user_id: i64) -> anyhow::Result<String> {
    let jobs = repo::list_active_jobs_for_user(db, telegram_user_id, 10).await?;
    if jobs.is_empty() {
        return Ok("JOB ĐANG CHẠY\n━━━━━━━━━━\nKhông có job đang chạy.".to_string());
    }
    let mut lines = vec!["JOB ĐANG CHẠY".to_string(), "━━━━━━━━━━".to_string()];
    for job in &jobs {
        lines.push(String::new());
        push_field(&mut lines, "Job", short_job_id(&job.id));
        push_field(&mut lines, "Trạng thái", vi_job_status(&job.status));
        push_field(&mut lines, "Đã quét", &job.total_discovered.to_string());
        push_field(&mut lines, "Hoàn tất", &job.completed_items.to_string());
        push_field(&mut lines, "Lỗi", &job.failed_items.to_string());
        push_field(&mut lines, "Bỏ qua", &job.skipped_items.to_string());
    }
    Ok(lines.join("\n"))
}

async fn account_summary(config: &AppConfig, db: &Database) -> anyhow::Result<String> {
    let status = repo::account_status(db)
        .await?
        .unwrap_or_else(|| "chưa kết nối".to_string());
    let destination = repo::default_destination_profile(db, "default").await?;

    let mut lines = vec!["TÀI KHOẢN GOOGLE".to_string(), "━━━━━━━━━━━━━━".to_string()];
    push_field(&mut lines, "Trạng thái", vi_account_status(&status));

    if status == "connected" {
        let token_manager = TokenManager::new(config.clone(), db.clone());
        let access_token = token_manager.access_token("default").await?;
        let drive =
            DriveClient::with_timeout(Duration::from_secs(config.engine.request_timeout_seconds));
        if let Ok(about) = drive.about_get(access_token.as_str()).await
            && let Some(user) = about.user
        {
            if let Some(email) = user.email_address {
                push_field(&mut lines, "Email", &email);
            }
            if let Some(name) = user.display_name {
                push_field(&mut lines, "Tên", &name);
            }
        }
    } else {
        lines.push(String::new());
        lines.push("Chạy trên máy đang chạy bot:".to_string());
        lines.push("  gdclone-bot auth login".to_string());
    }

    lines.push(String::new());
    lines.push("THƯ MỤC ĐÍCH".to_string());
    lines.push("━━━━━━━━━━━━".to_string());
    match destination {
        Some(dest) => {
            push_field(&mut lines, "Tên", &dest.label);
            push_field(&mut lines, "Parent ID", &dest.destination_parent_id);
            if let Some(drive_id) = dest.destination_drive_id {
                push_field(&mut lines, "Drive", &format!("Shared Drive ({drive_id})"));
            }
        }
        None => {
            lines.push("Chưa đặt. Dùng /set_destination <folder_url>.".to_string());
        }
    }

    Ok(lines.join("\n"))
}

// ── Preview dashboard ────────────────────────────────────────────────────────

async fn spawn_preview_dashboard(
    bot: Bot,
    chat_id: ChatId,
    db: Database,
    telegram_user_id: i64,
) -> ResponseResult<()> {
    let text = render_preview_dashboard(&db, telegram_user_id)
        .await
        .unwrap_or_else(|err| format!("Lỗi tải bảng tổng quan: {err}"));
    let message = bot.send_message(chat_id, text).await?;
    tokio::spawn(async move {
        let mut last_text = String::new();
        for _ in 0..15 {
            tokio::time::sleep(Duration::from_secs(2)).await;
            let text = match render_preview_dashboard(&db, telegram_user_id).await {
                Ok(t) => t,
                Err(err) => format!("Lỗi cập nhật: {err}"),
            };
            if text == last_text {
                continue;
            }
            last_text = text.clone();
            if let Err(err) = bot.edit_message_text(chat_id, message.id, text).await {
                warn!(error = %err, "edit preview dashboard failed");
                break;
            }
        }
    });
    Ok(())
}

async fn render_preview_dashboard(db: &Database, telegram_user_id: i64) -> anyhow::Result<String> {
    let account = repo::account_status(db)
        .await?
        .unwrap_or_else(|| "chưa kết nối".to_string());
    let destination = repo::default_destination_profile(db, "default").await?;
    let jobs = repo::list_active_jobs_for_user(db, telegram_user_id, 5).await?;
    let counts = repo::job_status_counts(db).await?;

    let mut lines = vec!["BẢNG TỔNG QUAN".to_string(), "━━━━━━━━━━━━".to_string()];
    push_field(&mut lines, "Google", vi_account_status(&account));
    match &destination {
        Some(p) => push_field(
            &mut lines,
            "Thư mục đích",
            &format!("{} ({})", p.label, p.destination_parent_id),
        ),
        None => lines.push("• Thư mục đích: Chưa đặt - dùng /set_destination".to_string()),
    }

    if !counts.is_empty() {
        let summary = counts
            .iter()
            .map(|c| format!("{}: {}", vi_job_status(&c.status), c.count))
            .collect::<Vec<_>>()
            .join(" | ");
        push_field(&mut lines, "Tổng job", &summary);
    }

    if jobs.is_empty() {
        lines.push("• Đang chạy: Không có".to_string());
    } else {
        lines.push(String::new());
        lines.push("ĐANG CHẠY".to_string());
        for job in &jobs {
            lines.push(format!(
                "• {} | {} | quét {} | xong {} | lỗi {}",
                short_job_id(&job.id),
                vi_job_status(&job.status),
                job.total_discovered,
                job.completed_items,
                job.failed_items,
            ));
        }
    }

    lines.push("(Tự cập nhật 30 giây - /preview để mở lại)".to_string());
    Ok(lines.join("\n"))
}

// ── Clone flow ───────────────────────────────────────────────────────────────

async fn send_clone_outcome(
    bot: &Bot,
    chat_id: ChatId,
    outcome: CloneOutcome,
    progress_message_id: Option<MessageId>,
) -> ResponseResult<()> {
    send_or_edit_clone_message(bot, chat_id, progress_message_id, outcome.message).await?;
    if let Some(paths) = outcome.report_paths {
        bot.send_document(chat_id, InputFile::file(paths.json))
            .await?;
        bot.send_document(chat_id, InputFile::file(paths.csv))
            .await?;
    }
    Ok(())
}

async fn send_last_report(
    bot: &Bot,
    chat_id: ChatId,
    config: &AppConfig,
    db: &Database,
    telegram_user_id: i64,
) -> ResponseResult<()> {
    let job = match repo::latest_reportable_job_for_user(db, telegram_user_id).await {
        Ok(Some(job)) => job,
        Ok(None) => {
            bot.send_message(chat_id, "Chưa có job hoàn tất/lỗi nào để gửi report.")
                .await?;
            return Ok(());
        }
        Err(err) => {
            bot.send_message(chat_id, format!("Lỗi đọc job gần nhất: {err}"))
                .await?;
            return Ok(());
        }
    };

    let paths = match report::write_job_reports(db, &config.storage.report_dir, &job.id).await {
        Ok(paths) => paths,
        Err(err) => {
            bot.send_message(chat_id, format!("Lỗi tạo report: {err}"))
                .await?;
            return Ok(());
        }
    };

    bot.send_message(
        chat_id,
        format!(
            "REPORT GẦN NHẤT\n━━━━━━━━━━━━\n• Job: {}\n• Trạng thái: {}\n• Đã quét: {}\n• Hoàn tất: {}\n• Lỗi: {}",
            short_job_id(&job.id),
            vi_job_status(&job.status),
            job.total_discovered,
            job.completed_items,
            job.failed_items,
        ),
    )
    .await?;
    bot.send_document(chat_id, InputFile::file(paths.json))
        .await?;
    bot.send_document(chat_id, InputFile::file(paths.csv))
        .await?;
    Ok(())
}

async fn send_or_edit_clone_message(
    bot: &Bot,
    chat_id: ChatId,
    message_id: Option<MessageId>,
    text: String,
) -> ResponseResult<()> {
    if let Some(message_id) = message_id
        && bot
            .edit_message_text(chat_id, message_id, text.clone())
            .await
            .is_ok()
    {
        return Ok(());
    }
    bot.send_message(chat_id, text).await?;
    Ok(())
}

async fn spawn_clone_now(
    bot: Bot,
    chat_id: ChatId,
    config: AppConfig,
    db: Database,
    telegram_user_id: i64,
    input: String,
) -> ResponseResult<()> {
    let source = match parse_drive_reference(&input) {
        Ok(source) => source,
        Err(err) => {
            bot.send_message(chat_id, format!("Link Drive không hợp lệ: {err}"))
                .await?;
            return Ok(());
        }
    };

    let progress_message = bot
        .send_message(
            chat_id,
            format!("Trạng thái: đang xếp hàng\n{}", render_progress(None, 0, 0)),
        )
        .await?;
    let progress_message_id = progress_message.id;
    let (progress_done_tx, progress_done_rx) = oneshot::channel();
    spawn_progress_updater(
        bot.clone(),
        chat_id,
        progress_message_id,
        db.clone(),
        telegram_user_id,
        Duration::from_millis(config.telegram.progress_edit_min_interval_ms),
        progress_done_rx,
    );

    bot.send_message(
        chat_id,
        "Đã nhận job clone. Theo dõi bằng /jobs hoặc /status <job_id>.",
    )
    .await?;

    tokio::spawn(async move {
        let outcome = start_clone_reference(
            &config,
            &db,
            chat_id.0,
            telegram_user_id,
            source,
            Some(progress_message_id.0),
        )
        .await;
        match outcome {
            Ok(outcome) => {
                let _ = progress_done_tx.send(());
                if let Err(err) =
                    send_clone_outcome(&bot, chat_id, outcome, Some(progress_message_id)).await
                {
                    warn!(error = %err, "send clone outcome failed");
                }
            }
            Err(err) => {
                let _ = progress_done_tx.send(());
                if let Err(send_err) = send_or_edit_clone_message(
                    &bot,
                    chat_id,
                    Some(progress_message_id),
                    format!("Lỗi clone:\n{}", format_error_for_user(&err)),
                )
                .await
                {
                    warn!(error = %send_err, "send clone error failed");
                }
            }
        }
    });
    Ok(())
}

async fn handle_clone_request(
    bot: Bot,
    chat_id: ChatId,
    config: AppConfig,
    db: Database,
    telegram_user_id: i64,
    input: String,
) -> ResponseResult<()> {
    if config.destination.auto_confirm_clone {
        return spawn_clone_now(bot, chat_id, config, db, telegram_user_id, input).await;
    }

    let loading = bot
        .send_message(
            chat_id,
            "Đang kiểm tra nguồn Drive...\n░░░░░░░░░░░░░░░░\nVui lòng chờ.",
        )
        .await?;

    match inspect_clone_source(&config, &db, &input).await {
        Ok(text) => {
            if repo::default_destination_profile(&db, "default")
                .await
                .ok()
                .flatten()
                .is_some()
            {
                let _ = repo::delete_expired_callback_states(&db).await;
                match repo::create_callback_state(
                    &db,
                    repo::NewCallbackState {
                        telegram_user_id,
                        chat_id: chat_id.0,
                        action: "clone_confirm".to_string(),
                        payload: input,
                        ttl_ms: CALLBACK_STATE_TTL_MS,
                    },
                )
                .await
                {
                    Ok(state_id) => {
                        bot.edit_message_text(chat_id, loading.id, text)
                            .reply_markup(keyboards::confirm_clone_keyboard(&state_id))
                            .await?;
                    }
                    Err(err) => {
                        bot.edit_message_text(chat_id, loading.id, err.to_string())
                            .await?;
                    }
                }
            } else {
                bot.edit_message_text(chat_id, loading.id, text).await?;
            }
        }
        Err(err) => {
            bot.edit_message_text(
                chat_id,
                loading.id,
                format!("Lỗi kiểm tra nguồn:\n{}", format_error_for_user(&err)),
            )
            .await?;
        }
    }
    Ok(())
}

// ── Progress updater ─────────────────────────────────────────────────────────

fn spawn_progress_updater(
    bot: Bot,
    chat_id: ChatId,
    message_id: MessageId,
    db: Database,
    telegram_user_id: i64,
    interval: Duration,
    mut done_rx: oneshot::Receiver<()>,
) {
    let base_interval = if interval.is_zero() {
        Duration::from_secs(2)
    } else {
        interval
    };
    tokio::spawn(async move {
        let started_at = Instant::now();
        let mut last_text = String::new();
        // Current wait before the next edit attempt.  Grows on 429, resets on
        // success.  Never exceeds 60 s so the loop stays responsive.
        let mut current_interval = base_interval;

        loop {
            if !last_text.is_empty() {
                tokio::select! {
                    _ = &mut done_rx => break,
                    _ = tokio::time::sleep(current_interval) => {}
                }
            }

            let elapsed_secs = started_at.elapsed().as_secs();
            let detail =
                match repo::job_detail_for_progress_message(&db, telegram_user_id, message_id.0)
                    .await
                {
                    Ok(Some(detail)) => detail,
                    Ok(None) => {
                        let text = format!(
                            "Trạng thái: đang chuẩn bị\nThời gian: {}\n{}",
                            format_duration_secs(elapsed_secs),
                            render_progress(None, 0, 0),
                        );
                        if text != last_text {
                            last_text = text.clone();
                            if let Err(err) = bot.edit_message_text(chat_id, message_id, text).await
                            {
                                current_interval =
                                    handle_edit_error(err, base_interval, current_interval);
                            }
                        }
                        continue;
                    }
                    Err(err) => {
                        warn!(error = %err, "read job progress failed");
                        continue;
                    }
                };

            let text = render_job_progress(&detail, elapsed_secs);
            if text == last_text {
                continue;
            }
            last_text = text.clone();

            let paused = detail.status == "paused";
            let edit_result = if is_terminal_status(&detail.status) {
                bot.edit_message_text(chat_id, message_id, text).await
            } else {
                bot.edit_message_text(chat_id, message_id, text)
                    .reply_markup(keyboards::job_control_keyboard(&detail.id, paused))
                    .await
            };

            match edit_result {
                Ok(_) => {
                    current_interval = base_interval;
                }
                Err(err) => {
                    current_interval = handle_edit_error(err, base_interval, current_interval);
                }
            }

            if is_terminal_status(&detail.status) {
                break;
            }
        }
    });
}

/// Map a Telegram edit error to the next polling interval.
///
/// - 429 RetryAfter(n): wait n seconds then resume.
/// - MessageNotModified: harmless, keep current cadence.
/// - Other: exponential back-off, capped at 60 s.
fn handle_edit_error(
    err: teloxide::RequestError,
    base_interval: Duration,
    current_interval: Duration,
) -> Duration {
    use teloxide::ApiError;
    use teloxide::RequestError;

    match &err {
        // Telegram 429 — exact retry_after + base_interval buffer.
        RequestError::RetryAfter(secs) => {
            let wait = secs.duration().as_secs().max(1);
            warn!(
                retry_after_secs = wait,
                "Telegram 429 — backing off progress edit"
            );
            Duration::from_secs(wait) + base_interval
        }
        // Identical text — Telegram refuses to edit; keep same cadence.
        RequestError::Api(ApiError::MessageNotModified) => current_interval,
        // Message already gone (deleted by user, bot kicked, etc.) — stop loop.
        RequestError::Api(ApiError::MessageToEditNotFound) => {
            warn!("progress message deleted — stopping updater");
            Duration::from_secs(3600) // very long = effectively stop
        }
        _ => {
            warn!(error = %err, "edit progress message failed");
            (current_interval * 2).min(Duration::from_secs(60))
        }
    }
}

fn render_job_progress(job: &repo::JobDetail, elapsed_secs: u64) -> String {
    let total = if matches!(
        job.status.as_str(),
        "running" | "completed" | "partially_completed" | "failed"
    ) && job.total_discovered > 0
    {
        Some(job.total_discovered as u64)
    } else {
        None
    };

    let done = job.completed_items as u64;

    let rate_str = if elapsed_secs > 0 {
        format!("{:.1} item/s", done as f64 / elapsed_secs as f64)
    } else {
        "--".to_string()
    };

    let eta_str = total
        .and_then(|t| estimate_eta_secs(done, t, elapsed_secs))
        .map(format_duration_secs)
        .unwrap_or_else(|| "--".to_string());

    [
        "TIẾN TRÌNH CLONE".to_string(),
        "━━━━━━━━━━━━━━".to_string(),
        format!("• Trạng thái: {}", vi_job_status(&job.status)),
        format!("• Job: {}", short_job_id(&job.id)),
        format!("• Thời gian: {}", format_duration_secs(elapsed_secs)),
        format!("• Tốc độ: {rate_str}"),
        format!("• Dự kiến: {eta_str}"),
        format!("• Đã quét: {}", job.total_discovered),
        format!("• Bỏ qua: {}", job.skipped_items),
        String::new(),
        render_progress(total, done, job.failed_items as u64),
    ]
    .join("\n")
}

fn is_terminal_status(status: &str) -> bool {
    matches!(
        status,
        "completed" | "partially_completed" | "failed" | "cancelled"
    )
}

// ── Job detail / control ─────────────────────────────────────────────────────

async fn show_job_status(
    db: &Database,
    telegram_user_id: i64,
    job_id: &str,
) -> anyhow::Result<String> {
    let Some(job) = repo::job_detail_for_user(db, telegram_user_id, job_id).await? else {
        return Ok("Không tìm thấy job thuộc tài khoản của bạn.".to_string());
    };

    let mut lines = vec!["CHI TIẾT JOB".to_string(), "━━━━━━━━━━".to_string()];
    push_field(&mut lines, "Job", &job.id);
    push_field(&mut lines, "Loại", &job.kind);
    push_field(&mut lines, "Trạng thái", vi_job_status(&job.status));
    push_field(&mut lines, "Nguồn", &job.source_root_id);
    push_field(&mut lines, "Đích", &job.destination_parent_id);
    lines.push(String::new());
    push_field(&mut lines, "Đã quét", &job.total_discovered.to_string());
    push_field(&mut lines, "Hoàn tất", &job.completed_items.to_string());
    push_field(&mut lines, "Lỗi", &job.failed_items.to_string());
    push_field(&mut lines, "Bỏ qua", &job.skipped_items.to_string());
    if let Some(error) = job.error_summary {
        lines.push(String::new());
        push_field(&mut lines, "Lỗi gần nhất", &error);
    }
    Ok(lines.join("\n"))
}

async fn pause_job(db: &Database, telegram_user_id: i64, job_id: &str) -> anyhow::Result<String> {
    if repo::pause_job_for_user(db, telegram_user_id, job_id).await? {
        Ok(format!("Đã yêu cầu tạm dừng job {job_id}."))
    } else {
        Ok("Không tìm thấy job hoặc trạng thái hiện tại không cho tạm dừng.".to_string())
    }
}

async fn resume_job(
    config: &AppConfig,
    db: &Database,
    telegram_user_id: i64,
    job_id: &str,
) -> anyhow::Result<String> {
    if repo::resume_job_for_user(db, telegram_user_id, job_id).await? {
        let _resume_worker = recovery::spawn_startup_resume_worker(config.clone(), db.clone());
        Ok(format!("Đã yêu cầu tiếp tục job {job_id}."))
    } else {
        Ok("Không tìm thấy job hoặc job chưa ở trạng thái tạm dừng.".to_string())
    }
}

async fn cancel_job(db: &Database, telegram_user_id: i64, job_id: &str) -> anyhow::Result<String> {
    if repo::cancel_job_for_user(db, telegram_user_id, job_id).await? {
        Ok(format!("Đã yêu cầu huỷ job {job_id}."))
    } else {
        Ok("Không tìm thấy job hoặc trạng thái hiện tại không cho huỷ.".to_string())
    }
}

async fn retry_job(
    config: &AppConfig,
    db: &Database,
    telegram_user_id: i64,
    job_id: &str,
) -> anyhow::Result<String> {
    if let Some(summary) = repo::retry_failed_job_for_user(db, telegram_user_id, job_id).await? {
        let _resume_worker = recovery::spawn_startup_resume_worker(config.clone(), db.clone());
        Ok(format!(
            "Đã xếp hàng làm lại job {job_id}.\n\
             Thư mục: {}  Item: {}  Thao tác: {}",
            summary.traversal_folders_requeued,
            summary.job_items_requeued,
            summary.operation_intents_replanned,
        ))
    } else {
        Ok(
            "Không tìm thấy job, job không thể retry, hoặc không có phần lỗi để làm lại."
                .to_string(),
        )
    }
}

// ── User management ──────────────────────────────────────────────────────────

async fn grant_user(db: &Database, actor_user_id: i64, input: &str) -> anyhow::Result<String> {
    ensure_owner(db, actor_user_id).await?;
    let target = parse_telegram_user_id(input)?;
    repo::grant_operator(db, target).await?;
    Ok(format!("Đã cấp quyền operator cho {target}."))
}

async fn revoke_user(db: &Database, actor_user_id: i64, input: &str) -> anyhow::Result<String> {
    ensure_owner(db, actor_user_id).await?;
    let target = parse_telegram_user_id(input)?;
    if target == actor_user_id {
        anyhow::bail!("Chủ sở hữu không thể tự thu hồi quyền của mình");
    }
    if repo::revoke_operator(db, target).await? {
        Ok(format!("Đã thu hồi quyền operator của {target}."))
    } else {
        Ok("Không tìm thấy người dùng hoặc đã bị vô hiệu hoá.".to_string())
    }
}

async fn disconnect_google(
    config: &AppConfig,
    db: &Database,
    actor_user_id: i64,
) -> anyhow::Result<String> {
    ensure_owner(db, actor_user_id).await?;
    let Some(account) = repo::google_account_secret(db, "default").await? else {
        return Ok("Chưa có tài khoản Google nào được kết nối.".to_string());
    };
    if account.status != "connected" && account.status != "reconnect_required" {
        return Ok(format!(
            "Tài khoản Google đã ở trạng thái: {}",
            account.status
        ));
    }
    let secret_store = FileSecretStore::new(&config.storage.config_dir);
    let master_key = secret_store.load_or_create_key().await?;
    let refresh_token = oauth::decrypt_token(&account.refresh_token_ciphertext, &master_key)?;
    oauth::revoke_refresh_token(
        &refresh_token,
        Duration::from_secs(config.engine.request_timeout_seconds),
    )
    .await?;
    repo::mark_account_revoked(db, "default").await?;
    Ok("Đã thu hồi refresh token Google và đánh dấu tài khoản là revoked.".to_string())
}

async fn ensure_owner(db: &Database, actor_user_id: i64) -> anyhow::Result<()> {
    if repo::is_owner(db, actor_user_id).await? {
        Ok(())
    } else {
        anyhow::bail!("Chỉ chủ sở hữu mới có thể quản lý người dùng")
    }
}

fn parse_telegram_user_id(input: &str) -> anyhow::Result<i64> {
    let value = input.trim().parse::<i64>()?;
    if value <= 0 {
        anyhow::bail!("Telegram user id phải là số dương");
    }
    Ok(value)
}

// ── Destination ──────────────────────────────────────────────────────────────

/// Send a destination panel message with the current default and recent
/// destinations as inline quick-switch buttons.
async fn spawn_destination_panel(bot: Bot, chat_id: ChatId, db: Database) -> ResponseResult<()> {
    let profiles = repo::list_recent_destinations(&db, "default", 5)
        .await
        .unwrap_or_default();

    let text = render_destination_list(&profiles);

    if profiles.is_empty() {
        bot.send_message(
            chat_id,
            "Chưa đặt thư mục đích mặc định.\n\
             Dùng /set_destination <folder_url> để cấu hình.",
        )
        .await?;
    } else {
        bot.send_message(chat_id, text)
            .reply_markup(keyboards::recent_destinations_keyboard(&profiles))
            .await?;
    }
    Ok(())
}

fn render_destination_list(profiles: &[repo::DestinationProfile]) -> String {
    if profiles.is_empty() {
        return "Chưa có thư mục đích nào được lưu.\nDùng /set_destination <folder_url>."
            .to_string();
    }
    let mut lines = vec!["THƯ MỤC ĐÍCH".to_string(), "━━━━━━━━━━━━".to_string()];
    for p in profiles {
        lines.push(String::new());
        let title = if p.is_default {
            format!("{} [mặc định]", p.label)
        } else {
            p.label.clone()
        };
        push_field(&mut lines, "Tên", &title);
        push_field(&mut lines, "Parent ID", &p.destination_parent_id);
        if let Some(drive_id) = &p.destination_drive_id {
            push_field(&mut lines, "Drive", &format!("Shared Drive ({drive_id})"));
        } else {
            push_field(&mut lines, "Drive", "My Drive / được chia sẻ");
        }
    }
    lines.push(String::new());
    lines.push("Nhấn vào tên để đặt làm mặc định. Thêm mới: /set_destination <url>".to_string());
    lines.join("\n")
}

/// Legacy helper kept for internal callers that only need the default.
#[allow(dead_code)]
async fn destination_summary(db: &Database) -> anyhow::Result<String> {
    let profiles = repo::list_recent_destinations(db, "default", 1).await?;
    match profiles.first() {
        Some(p) => {
            let mut lines = vec![
                "Thư mục đích mặc định:".to_string(),
                format!("  Ten : {}", p.label),
                format!("  ID  : {}", p.destination_parent_id),
            ];
            if let Some(drive_id) = &p.destination_drive_id {
                lines.push(format!("  Shared Drive: {drive_id}"));
            }
            lines.push(String::new());
            lines.push("Thay đổi: /set_destination <url>   Xoá: /clear_destination".to_string());
            Ok(lines.join("\n"))
        }
        None => Ok("Chưa đặt thư mục đích mặc định.\n\
             Dùng /set_destination <folder_url> để cấu hình."
            .to_string()),
    }
}

async fn set_destination(config: &AppConfig, db: &Database, input: &str) -> anyhow::Result<String> {
    let reference = parse_drive_reference(input)?;
    let token_manager = TokenManager::new(config.clone(), db.clone());
    let access_token = token_manager.access_token("default").await?;
    let drive =
        DriveClient::with_timeout(Duration::from_secs(config.engine.request_timeout_seconds));
    let file = drive
        .get_reference(access_token.as_str(), &reference)
        .await?;

    if file.mime_type != FOLDER_MIME_TYPE {
        anyhow::bail!("Thư mục đích phải là Google Drive folder");
    }
    if file.capabilities.as_ref().and_then(|c| c.can_add_children) != Some(true) {
        anyhow::bail!("Tài khoản Google hiện tại không có quyền ghi vào thư mục đích này");
    }

    repo::upsert_destination_profile(
        db,
        repo::NewDestinationProfile {
            google_account_id: "default".to_string(),
            label: file.name.clone(),
            destination_parent_id: file.id.clone(),
            destination_drive_id: file.drive_id.clone(),
            destination_resource_key: reference.resource_key.clone().or(file.resource_key.clone()),
            is_default: true,
        },
    )
    .await?;

    let mut lines = vec![
        "ĐÃ ĐẶT THƯ MỤC ĐÍCH".to_string(),
        "━━━━━━━━━━━━━━━".to_string(),
    ];
    push_field(&mut lines, "Tên", &file.name);
    push_field(&mut lines, "Folder ID", &file.id);
    if let Some(drive_id) = file.drive_id {
        push_field(&mut lines, "Drive", &format!("Shared Drive ({drive_id})"));
    } else {
        push_field(&mut lines, "Drive", "My Drive / được chia sẻ");
    }
    Ok(lines.join("\n"))
}

// ── Clone source inspect ─────────────────────────────────────────────────────

async fn inspect_clone_source(
    config: &AppConfig,
    db: &Database,
    input: &str,
) -> anyhow::Result<String> {
    let reference = parse_drive_reference(input)?;
    let token_manager = TokenManager::new(config.clone(), db.clone());
    let access_token = token_manager.access_token("default").await?;
    let drive =
        DriveClient::with_timeout(Duration::from_secs(config.engine.request_timeout_seconds));
    let source = drive
        .get_reference(access_token.as_str(), &reference)
        .await?;
    let account_email = drive
        .about_get(access_token.as_str())
        .await
        .ok()
        .and_then(|about| about.user)
        .and_then(|user| user.email_address);

    let item_type = if source.is_folder() {
        "Thư mục"
    } else {
        "File"
    };

    let mut lines = vec!["THÔNG TIN CLONE".to_string(), "━━━━━━━━━━━━━━".to_string()];
    push_field(
        &mut lines,
        "Google",
        account_email.as_deref().unwrap_or("không đọc được email"),
    );
    push_field(&mut lines, "Tên nguồn", &source.name);
    push_field(&mut lines, "ID nguồn", &source.id);
    push_field(&mut lines, "Loại", item_type);
    push_field(&mut lines, "MIME", &source.mime_type);

    if let Some(size) = &source.size {
        let bytes: i64 = size.parse().unwrap_or(0);
        push_field(&mut lines, "Kích thước", &human_bytes(bytes));
    }

    if let Some(drive_id) = &source.drive_id {
        push_field(&mut lines, "Vị trí", &format!("Shared Drive ({drive_id})"));
    } else {
        push_field(&mut lines, "Vị trí", "My Drive / được chia sẻ");
    }
    if let Some(modified_time) = &source.modified_time {
        push_field(&mut lines, "Sửa đổi", modified_time);
    }
    if let Some(version) = &source.version {
        push_field(&mut lines, "Version", version);
    }
    if let Some(md5) = &source.md5_checksum {
        push_field(&mut lines, "MD5", md5);
    }

    // Capability warnings
    let caps = source.capabilities.as_ref();
    let can_read = if source.is_folder() {
        caps.and_then(|c| c.can_list_children)
    } else {
        caps.and_then(|c| c.can_copy)
    };
    push_field(&mut lines, "Có thể đọc/copy", capability_text(can_read));
    if source.is_folder() {
        if caps.and_then(|c| c.can_list_children) == Some(false) {
            lines.push(
                "CẢNH BÁO: Không thể liệt kê nội dung - thư mục có thể bị hạn chế quyền."
                    .to_string(),
            );
        }
    } else if caps.and_then(|c| c.can_copy) == Some(false) {
        lines.push("CẢNH BÁO: Không thể sao chép - file bị hạn chế hoặc chống copy.".to_string());
    }

    if source.copy_requires_writer_permission == Some(true) {
        lines.push(
            "CẢNH BÁO: copyRequiresWriterPermission - chỉ người có quyền ghi mới copy được."
                .to_string(),
        );
    }

    if reference.resource_key.is_some() {
        lines.push("Lưu ý: Link dùng resource key (link hạn chế truy cập).".to_string());
    }

    if let Ok(plan) = build_clone_plan(
        &drive,
        access_token.as_str(),
        &source,
        reference.resource_key.as_deref(),
    )
    .await
    {
        lines.push(String::new());
        lines.push("KẾ HOẠCH".to_string());
        lines.push("━━━━━━━━".to_string());
        lines.extend(plan.render_lines());
    }

    if let Some(default_dest) = repo::default_destination_profile(db, "default").await? {
        let destination_preview = if source.is_folder() {
            format!("{}/{}", default_dest.label, source.name)
        } else {
            default_dest.label.clone()
        };
        lines.push(String::new());
        lines.push("ĐÍCH ĐẾN".to_string());
        lines.push("━━━━━━".to_string());
        push_field(&mut lines, "Tên", &destination_preview);
        push_field(&mut lines, "Parent ID", &default_dest.destination_parent_id);
        if let Some(drive_id) = &default_dest.destination_drive_id {
            push_field(&mut lines, "Drive", &format!("Shared Drive ({drive_id})"));
        }
        lines.push(String::new());
        lines.push("Chọn nút bên dưới để bắt đầu hoặc huỷ.".to_string());
    } else {
        lines.push(String::new());
        lines.push("Chưa có thư mục đích. Dùng /set_destination <folder_url>.".to_string());
    }

    Ok(lines.join("\n"))
}

fn push_field(lines: &mut Vec<String>, label: &str, value: &str) {
    lines.push(format!("• {label}: {value}"));
}

fn capability_text(value: Option<bool>) -> &'static str {
    match value {
        Some(true) => "Có",
        Some(false) => "Không",
        None => "Không rõ",
    }
}

fn format_error_for_user(err: &anyhow::Error) -> String {
    if let Some(drive) = err.downcast_ref::<DriveApiError>() {
        return format_drive_error(drive);
    }
    err.to_string()
}

fn format_drive_error(err: &DriveApiError) -> String {
    match err {
        DriveApiError::Api {
            status,
            reason,
            message,
        } => {
            let friendly = match (status.as_u16(), reason.as_deref()) {
                (401, _) => "Phiên Google hết hạn. Chạy `gdclone-bot auth login` trên máy bot.",
                (403, Some("insufficientPermissions")) => {
                    "Tài khoản Google hiện tại không đủ quyền với file/folder này."
                }
                (403, Some("copyRequiresWriterPermission")) => {
                    "Nguồn yêu cầu quyền ghi mới được copy."
                }
                (403, Some("storageQuotaExceeded" | "teamDriveFileLimitExceeded")) => {
                    "Google Drive báo hết quota hoặc chạm giới hạn lưu trữ."
                }
                (403, Some("userRateLimitExceeded" | "rateLimitExceeded")) | (429, _) => {
                    "Google đang giới hạn tốc độ. Bot sẽ retry nếu lỗi xảy ra trong job."
                }
                (404, _) => {
                    "Không tìm thấy file/folder, không có quyền truy cập, hoặc link thiếu resource key."
                }
                (400, _) => "Request Drive không hợp lệ. Kiểm tra lại link nguồn/đích.",
                (500..=599, _) => "Google Drive đang lỗi tạm thời. Thử lại sau ít phút.",
                _ => "Google Drive trả về lỗi.",
            };
            format!(
                "{friendly}\n• HTTP: {}\n• Reason: {}\n• Chi tiết: {}",
                status.as_u16(),
                reason.as_deref().unwrap_or("không rõ"),
                message
            )
        }
        DriveApiError::Transport(err) => {
            format!("Không gọi được Google Drive API.\n• Chi tiết: {err}")
        }
    }
}

#[derive(Debug, Default)]
struct ClonePlan {
    folders: u64,
    files: u64,
    shortcuts: u64,
    google_native: u64,
    known_bytes: u64,
    warning_count: u64,
    scanned_items: usize,
    truncated: bool,
}

impl ClonePlan {
    fn add(&mut self, file: &crate::drive::types::DriveFile) {
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

    fn render_lines(&self) -> Vec<String> {
        let mut lines = vec![
            format!("Đã quét      : {} item", self.scanned_items),
            format!("Thư mục      : {}", self.folders),
            format!("File         : {}", self.files),
            format!("Google-native: {}", self.google_native),
            format!("Shortcut     : {}", self.shortcuts),
            format!("Dung lượng rõ: {}", human_bytes(self.known_bytes as i64)),
        ];
        if self.warning_count > 0 {
            lines.push(format!(
                "Cảnh báo     : {} item cần chú ý",
                self.warning_count
            ));
        }
        if self.truncated {
            lines.push(format!(
                "Lưu ý        : chỉ quét trước {} item đầu",
                self.scanned_items
            ));
        }
        lines
    }
}

async fn build_clone_plan(
    drive: &DriveClient,
    access_token: &str,
    source: &crate::drive::types::DriveFile,
    source_resource_key: Option<&str>,
) -> anyhow::Result<ClonePlan> {
    let mut plan = ClonePlan::default();
    plan.add(source);
    if !source.is_folder() {
        return Ok(plan);
    }

    let mut queue = VecDeque::from([(source.id.clone(), source_resource_key.map(str::to_string))]);
    while let Some((folder_id, folder_resource_key)) = queue.pop_front() {
        let mut page_token = None;
        loop {
            let page = drive
                .list_children(
                    access_token,
                    &folder_id,
                    folder_resource_key.as_deref(),
                    page_token.as_deref(),
                )
                .await?;
            for child in page.files {
                if plan.scanned_items >= CLONE_PLAN_ITEM_LIMIT {
                    plan.truncated = true;
                    return Ok(plan);
                }
                if child.is_folder() {
                    queue.push_back((child.id.clone(), child.resource_key.clone()));
                }
                plan.add(&child);
            }
            let Some(next) = page.next_page_token else {
                break;
            };
            page_token = Some(next);
        }
    }
    Ok(plan)
}

/// Human-readable byte count.
fn human_bytes(bytes: i64) -> String {
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::drive::types::DriveFile;
    use reqwest::StatusCode;

    fn file(id: &str, mime_type: &str, size: Option<&str>) -> DriveFile {
        DriveFile {
            id: id.to_string(),
            name: id.to_string(),
            mime_type: mime_type.to_string(),
            size: size.map(str::to_string),
            parents: vec![],
            drive_id: None,
            resource_key: None,
            shortcut_details: None,
            trashed: None,
            modified_time: None,
            md5_checksum: None,
            version: None,
            capabilities: None,
            copy_requires_writer_permission: None,
            app_properties: Default::default(),
        }
    }

    #[test]
    fn clone_plan_counts_drive_kinds_and_bytes() {
        let mut plan = ClonePlan::default();
        plan.add(&file("folder", FOLDER_MIME_TYPE, None));
        plan.add(&file("bin", "application/octet-stream", Some("2048")));
        plan.add(&file("doc", "application/vnd.google-apps.document", None));
        plan.add(&file(
            "shortcut",
            crate::drive::types::SHORTCUT_MIME_TYPE,
            None,
        ));

        assert_eq!(plan.folders, 1);
        assert_eq!(plan.files, 3);
        assert_eq!(plan.google_native, 1);
        assert_eq!(plan.shortcuts, 1);
        assert_eq!(plan.known_bytes, 2048);
    }

    #[test]
    fn drive_errors_are_translated_for_telegram() {
        let permission = DriveApiError::Api {
            status: StatusCode::FORBIDDEN,
            reason: Some("insufficientPermissions".to_string()),
            message: "The user does not have sufficient permissions".to_string(),
        };
        let text = format_drive_error(&permission);
        assert!(text.contains("không đủ quyền"));
        assert!(text.contains("HTTP: 403"));

        let not_found = DriveApiError::Api {
            status: StatusCode::NOT_FOUND,
            reason: Some("notFound".to_string()),
            message: "File not found".to_string(),
        };
        let text = format_drive_error(&not_found);
        assert!(text.contains("resource key"));
    }
}

// ── Clone engine call ────────────────────────────────────────────────────────

async fn start_clone_reference(
    config: &AppConfig,
    db: &Database,
    chat_id: i64,
    telegram_user_id: i64,
    source: crate::drive::links::DriveReference,
    progress_message_id: Option<i32>,
) -> anyhow::Result<CloneOutcome> {
    // Per-user concurrency guard.
    let active_count = repo::active_job_count_for_user(db, telegram_user_id).await?;
    if active_count >= config.engine.max_active_jobs_per_user as i64 {
        anyhow::bail!(
            "Bạn đang có {active_count} job hoạt động. \
             Chờ hoàn thành hoặc huỷ bằng /cancel <job_id>."
        );
    }
    let service = CloneService::new(config.clone(), db.clone());
    service
        .start_one_shot(CloneRequest {
            chat_id,
            telegram_user_id,
            source,
            progress_message_id,
        })
        .await
}

// ── Watch handlers ───────────────────────────────────────────────────────────

async fn start_watch(
    bot: &Bot,
    config: &AppConfig,
    db: &Database,
    chat_id: i64,
    telegram_user_id: i64,
    input: &str,
) -> anyhow::Result<String> {
    let parts: Vec<&str> = input.splitn(2, char::is_whitespace).collect();
    if parts.len() < 2 {
        anyhow::bail!("Cu phap: /watch <source_url_or_id> <dest_url_or_id>");
    }
    let source_ref = parse_drive_reference(parts[0])?;
    let dest_ref = parse_drive_reference(parts[1])?;

    let token_manager = TokenManager::new(config.clone(), db.clone());
    let access_token = token_manager.access_token("default").await?;
    let drive =
        DriveClient::with_timeout(Duration::from_secs(config.engine.request_timeout_seconds));

    let source = drive
        .get_reference(access_token.as_str(), &source_ref)
        .await?;
    if !source.is_folder() {
        anyhow::bail!("Nguồn phai la Google Drive folder.");
    }
    let dest = drive
        .get_reference(access_token.as_str(), &dest_ref)
        .await?;
    if !dest.is_folder() {
        anyhow::bail!("Đích phải là Google Drive folder.");
    }
    if dest.capabilities.as_ref().and_then(|c| c.can_add_children) != Some(true) {
        anyhow::bail!("Không có quyền ghi vào thư mục đích.");
    }

    let corpus_kind = if source.drive_id.is_some() {
        "shared_drive"
    } else {
        "user"
    };
    let start_token = drive
        .get_start_page_token(access_token.as_str(), source.drive_id.as_deref())
        .await?;
    let cursor = repo::upsert_change_cursor(
        db,
        "default",
        corpus_kind,
        source.drive_id.as_deref(),
        &start_token.start_page_token,
    )
    .await?;

    let watch_id = repo::create_watch_subscription(
        db,
        repo::NewWatchSubscription {
            google_account_id: "default".to_string(),
            cursor_id: cursor.id.clone(),
            telegram_user_id,
            chat_id,
            source_root_id: source.id.clone(),
            source_resource_key: source_ref
                .resource_key
                .clone()
                .or(source.resource_key.clone()),
            source_drive_id: source.drive_id.clone(),
            destination_root_id: dest.id.clone(),
            destination_drive_id: dest.drive_id.clone(),
            content_update_policy: config.watch.default_content_update_policy.clone(),
            deletion_policy: config.watch.default_deletion_policy.clone(),
            move_out_policy: config.watch.default_move_out_policy.clone(),
            baseline_sequence: cursor.last_event_sequence,
        },
    )
    .await?;

    {
        let config2 = config.clone();
        let db2 = db.clone();
        let bot2 = bot.clone();
        let chat_id2 = chat_id;
        let watch_id2 = watch_id.clone();
        tokio::spawn(async move {
            let notify = move |msg_text: String| {
                let bot = bot2.clone();
                let cid = chat_id2;
                tokio::spawn(async move {
                    let _ = bot
                        .send_message(teloxide::types::ChatId(cid), msg_text)
                        .await;
                });
            };
            if let Err(err) = run_initial_clone(config2, db2, watch_id2.clone(), notify).await {
                tracing::error!(watch_id = watch_id2, error = %err, "watch initializer failed");
            }
        });
    }

    Ok(format!(
        "Watch đã tạo thành công.\n\
         ID          : {short}\n\
         Nguồn       : {src_name} ({src_id})\n\
         Đích        : {dst_name}\n\
         Clone ban đầu đang chạy nền - dùng /watch_status {short} để theo dõi.",
        short = &watch_id[..8.min(watch_id.len())],
        src_name = source.name,
        src_id = source.id,
        dst_name = dest.name,
    ))
}

async fn list_watches(db: &Database, telegram_user_id: i64) -> anyhow::Result<String> {
    let watches = repo::list_watches_for_user(db, telegram_user_id).await?;
    if watches.is_empty() {
        return Ok("Chưa có watch subscription nào.".to_string());
    }
    let mut lines = vec!["Danh sach Watch:".to_string()];
    for w in &watches {
        lines.push(format!(
            "  {} [{}]  {} -> {}",
            &w.id[..8.min(w.id.len())],
            vi_watch_status(&w.status),
            w.source_root_id,
            w.destination_root_id,
        ));
    }
    Ok(lines.join("\n"))
}

async fn watch_status_detail(
    db: &Database,
    telegram_user_id: i64,
    watch_id: &str,
) -> anyhow::Result<String> {
    let Some(w) = repo::watch_for_user(db, telegram_user_id, watch_id).await? else {
        return Ok("Không tìm thấy watch.".to_string());
    };
    Ok(format!(
        "Watch       : {id}\n\
         Trạng thái  : {status}\n\
         Nguồn       : {src}\n\
         Đích        : {dst}\n\
         Cap nhat ND : {cup}\n\
         Khi xoa     : {del}\n\
         Khi move ra : {mop}\n\
         Baseline seq: {bseq}\n\
         Consumed seq: {cseq}",
        id = w.id,
        status = vi_watch_status(&w.status),
        src = w.source_root_id,
        dst = w.destination_root_id,
        cup = w.content_update_policy,
        del = w.deletion_policy,
        mop = w.move_out_policy,
        bseq = w.baseline_sequence,
        cseq = w.last_consumed_sequence,
    ))
}

async fn set_watch_policy(
    db: &Database,
    telegram_user_id: i64,
    input: &str,
) -> anyhow::Result<String> {
    let parts: Vec<&str> = input.splitn(2, char::is_whitespace).collect();
    if parts.len() < 2 {
        anyhow::bail!(
            "Cu phap: /watch_policy <watch_id> <versioned_copy|replace_copy|manual_confirmation>"
        );
    }
    let watch_id = parts[0];
    let policy = parts[1].trim();
    if !matches!(
        policy,
        "versioned_copy" | "replace_copy" | "manual_confirmation"
    ) {
        anyhow::bail!(
            "Policy không hợp lệ '{}'. Chọn một trong: versioned_copy | replace_copy | manual_confirmation",
            policy
        );
    }
    if repo::set_watch_content_update_policy(db, telegram_user_id, watch_id, policy).await? {
        Ok(format!(
            "Watch {watch_id} content_update_policy đã đổi thành {policy}."
        ))
    } else {
        Ok("Không tìm thấy watch.".to_string())
    }
}

// ── Helpers ──────────────────────────────────────────────────────────────────

fn short_job_id(job_id: &str) -> &str {
    job_id.get(..8).unwrap_or(job_id)
}

fn vi_account_status(status: &str) -> &str {
    match status {
        "connected" => "Đã kết nối",
        "reconnect_required" => "Cần đăng nhập lại",
        "revoked" => "Đã thu hồi",
        "disabled" => "Đã tắt",
        other => other,
    }
}

fn vi_job_status(status: &str) -> &str {
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

fn vi_watch_status(status: &str) -> &str {
    match status {
        "active" => "hoạt động",
        "paused" => "tạm dừng",
        "initializing" => "đang khởi tạo",
        "catching_up" => "đang bắt kịp",
        "degraded" => "bi lỗi",
        "needs_reconcile" => "cần đồng bộ lại",
        "stopped" => "đã dừng",
        other => other,
    }
}
