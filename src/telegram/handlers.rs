use std::time::{Duration, Instant};

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
        auth as oauth, client::DriveClient, links::parse_drive_reference,
        token_manager::TokenManager, types::FOLDER_MIME_TYPE,
    },
    engine::{
        copy::{CloneOutcome, CloneRequest, CloneService},
        recovery,
    },
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
            "Khong co quyen truy cap. Lien he chu so huu de duoc cap phep.",
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
            bot.send_message(msg.chat.id, format!("Link Drive khong hop le: {err}"))
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
            bot.send_message(chat_id, "Khong co quyen truy cap.")
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
                let msg_text = "Yeu cau clone da het han hoac khong thuoc ve ban.";
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
                    .edit_message_text(chat_id, message.id(), "Dang bat dau clone...")
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
                bot.edit_message_text(chat_id, message.id(), "Da huy yeu cau clone.")
                    .await?;
                return Ok(());
            }
            "Da huy yeu cau clone.".to_string()
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
                Ok(false) => "Khong tim thay thu muc dich.".to_string(),
                Err(err) => format!("Loi doi dich: {err}"),
            }
        }
        Some(("browse", _, _)) => "Tinh nang chon thu muc chua duoc ho tro.".to_string(),
        _ => "Hanh dong khong xac dinh.".to_string(),
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
                "gdclone-bot dang chay.\n\
                 Dung /account de kiem tra ket noi Google, sau do dan link Drive vao de bat dau.",
            )
            .await?;
        }
        Command::Help => {
            bot.send_message(
                msg.chat.id,
                "DANH SACH LENH\n\
                 \n\
                 Clone:\n\
                 /clone <url>          - Xem thong tin va xac nhan\n\
                 /clone_here <url>     - Clone ngay khong can xac nhan\n\
                 /jobs                 - Danh sach job dang chay\n\
                 /status <job_id>      - Chi tiet job\n\
                 /pause <job_id>       - Tam dung job\n\
                 /resume <job_id>      - Tiep tuc job\n\
                 /cancel <job_id>      - Huy job\n\
                 /retry <job_id>       - Lam lai cac item loi\n\
                 \n\
                 Thu muc dich:\n\
                 /destination          - Xem thu muc dich hien tai\n\
                 /set_destination <url>- Dat thu muc dich mac dinh\n\
                 /clear_destination    - Xoa thu muc dich mac dinh\n\
                 /preview              - Bang tong quan\n\
                 \n\
                 Watch/Dong bo:\n\
                 /watch <src> <dst>    - Tao watch subscription\n\
                 /watches              - Danh sach watches\n\
                 /watch_status <id>    - Trang thai watch\n\
                 /watch_pause <id>     - Tam dung watch\n\
                 /watch_resume <id>    - Tiep tuc watch\n\
                 /watch_policy <id> <policy>\n\
                 /unwatch <id>         - Dung watch\n\
                 \n\
                 Tai khoan:\n\
                 /account              - Trang thai Google\n\
                 /whoami               - ID va quyen cua ban\n\
                 /grant <user_id>      - Cap quyen nguoi dung\n\
                 /revoke <user_id>     - Thu hoi quyen",
            )
            .await?;
        }
        Command::Preview => {
            spawn_preview_dashboard(bot, msg.chat.id, db, user_id).await?;
        }
        Command::Connect => {
            bot.send_message(
                msg.chat.id,
                "Google Auth chay local-first.\n\
                 \n\
                 Chay lenh sau tren may dang chay bot:\n\
                 \n\
                   gdclone-bot auth login\n\
                 \n\
                 Sau do dung /account de kiem tra ket noi.",
            )
            .await?;
        }
        Command::Account => {
            let text = match repo::account_status(&db).await {
                Ok(Some(status)) => format!("Tai khoan Google: {status}"),
                Ok(None) => "Chua ket noi Google.\n\
                     Chay 'gdclone-bot auth login' tren may chay bot."
                    .to_string(),
                Err(err) => format!("Loi doc trang thai tai khoan: {err}"),
            };
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
                .unwrap_or_else(|| "khong ro".to_string());
            bot.send_message(
                msg.chat.id,
                format!("Telegram ID: {user_id}\nQuyen: {role}"),
            )
            .await?;
        }
        Command::Destination => {
            spawn_destination_panel(bot, msg.chat.id, db).await?;
        }
        Command::ClearDestination => {
            let text = match repo::clear_default_destination(&db, "default").await {
                Ok(0) => "Chua co thu muc dich mac dinh de xoa.".to_string(),
                Ok(_) => "Da xoa thu muc dich mac dinh.".to_string(),
                Err(err) => format!("Loi xoa thu muc dich: {err}"),
            };
            bot.send_message(msg.chat.id, text).await?;
        }
        Command::SetDestination(input) => {
            let text = set_destination(&config, &db, &input).await;
            bot.send_message(msg.chat.id, text.unwrap_or_else(|err| err.to_string()))
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
                Ok(true) => format!("Watch {watch_id} da tam dung."),
                Ok(false) => "Khong tim thay watch hoac khong the tam dung.".to_string(),
                Err(err) => format!("Loi: {err}"),
            };
            bot.send_message(msg.chat.id, text).await?;
        }
        Command::WatchResume(watch_id) => {
            let text = match repo::resume_watch_for_user(&db, user_id, &watch_id).await {
                Ok(true) => format!("Watch {watch_id} da tiep tuc (dang bat kip)."),
                Ok(false) => {
                    "Khong tim thay watch hoac watch chua o trang thai tam dung.".to_string()
                }
                Err(err) => format!("Loi: {err}"),
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
                Ok(true) => format!("Watch {watch_id} da dung."),
                Ok(false) => "Khong tim thay watch hoac da dung truoc do.".to_string(),
                Err(err) => format!("Loi: {err}"),
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
        return Ok("Khong co job dang chay.".to_string());
    }
    let mut lines = vec!["Job dang hoat dong:".to_string()];
    for job in &jobs {
        lines.push(format!(
            "  {} [{}]  qua_quet:{} hoan_tat:{} loi:{} bo_qua:{}",
            short_job_id(&job.id),
            vi_job_status(&job.status),
            job.total_discovered,
            job.completed_items,
            job.failed_items,
            job.skipped_items,
        ));
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
        .unwrap_or_else(|err| format!("Loi tai bang tong quan: {err}"));
    let message = bot.send_message(chat_id, text).await?;
    tokio::spawn(async move {
        let mut last_text = String::new();
        for _ in 0..15 {
            tokio::time::sleep(Duration::from_secs(2)).await;
            let text = match render_preview_dashboard(&db, telegram_user_id).await {
                Ok(t) => t,
                Err(err) => format!("Loi cap nhat: {err}"),
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
        .unwrap_or_else(|| "chua ket noi".to_string());
    let destination = repo::default_destination_profile(db, "default").await?;
    let jobs = repo::list_active_jobs_for_user(db, telegram_user_id, 5).await?;
    let counts = repo::job_status_counts(db).await?;

    let mut lines = vec![
        "=== Bang tong quan gdclone-bot ===".to_string(),
        format!("Google       : {}", vi_account_status(&account)),
        match &destination {
            Some(p) => format!("Thu muc dich : {} ({})", p.label, p.destination_parent_id),
            None => "Thu muc dich : Chua dat - dung /set_destination".to_string(),
        },
    ];

    if !counts.is_empty() {
        let summary = counts
            .iter()
            .map(|c| format!("{}: {}", vi_job_status(&c.status), c.count))
            .collect::<Vec<_>>()
            .join(" | ");
        lines.push(format!("Tong job     : {summary}"));
    }

    if jobs.is_empty() {
        lines.push("Dang chay    : Khong co".to_string());
    } else {
        lines.push("Dang chay:".to_string());
        for job in &jobs {
            lines.push(format!(
                "  {} [{}] qua_quet:{} hoan_tat:{} loi:{}",
                short_job_id(&job.id),
                vi_job_status(&job.status),
                job.total_discovered,
                job.completed_items,
                job.failed_items,
            ));
        }
    }

    lines.push("(Tu cap nhat 30 giay - /preview de mo lai)".to_string());
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
            bot.send_message(chat_id, format!("Link Drive khong hop le: {err}"))
                .await?;
            return Ok(());
        }
    };

    let progress_message = bot
        .send_message(
            chat_id,
            format!("Trang thai: dang xep hang\n{}", render_progress(None, 0, 0)),
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
        "Da nhan job clone. Theo doi bang /jobs hoac /status <job_id>.",
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
                    format!("Loi: {err}"),
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
                        bot.send_message(chat_id, text)
                            .reply_markup(keyboards::confirm_clone_keyboard(&state_id))
                            .await?;
                    }
                    Err(err) => {
                        bot.send_message(chat_id, err.to_string()).await?;
                    }
                }
            } else {
                bot.send_message(chat_id, text).await?;
            }
        }
        Err(err) => {
            bot.send_message(chat_id, format!("Loi kiem tra nguon: {err}"))
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
                            "Trang thai: dang chuan bi\nThoi gian: {}\n{}",
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

    format!(
        "Trang thai  : {status}\n\
         Job         : {job_id}\n\
         Thoi gian   : {elapsed}  Toc do: {rate}  Du kien: {eta}\n\
         Qua quet    : {discovered}  Bo qua: {skipped}\n\
         {bar}",
        status = vi_job_status(&job.status),
        job_id = short_job_id(&job.id),
        elapsed = format_duration_secs(elapsed_secs),
        rate = rate_str,
        eta = eta_str,
        discovered = job.total_discovered,
        skipped = job.skipped_items,
        bar = render_progress(total, done, job.failed_items as u64),
    )
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
        return Ok("Khong tim thay job thuoc tai khoan cua ban.".to_string());
    };

    let mut lines = vec![
        format!("Job         : {}", job.id),
        format!("Loai        : {}", job.kind),
        format!("Trang thai  : {}", vi_job_status(&job.status)),
        format!("Nguon       : {}", job.source_root_id),
        format!("Thu muc dich: {}", job.destination_parent_id),
        format!("Qua quet    : {}", job.total_discovered),
        format!("Hoan tat    : {}", job.completed_items),
        format!("Loi         : {}", job.failed_items),
        format!("Bo qua      : {}", job.skipped_items),
    ];
    if let Some(error) = job.error_summary {
        lines.push(format!("Loi gan nhat: {error}"));
    }
    Ok(lines.join("\n"))
}

async fn pause_job(db: &Database, telegram_user_id: i64, job_id: &str) -> anyhow::Result<String> {
    if repo::pause_job_for_user(db, telegram_user_id, job_id).await? {
        Ok(format!("Da yeu cau tam dung job {job_id}."))
    } else {
        Ok("Khong tim thay job hoac trang thai hien tai khong cho tam dung.".to_string())
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
        Ok(format!("Da yeu cau tiep tuc job {job_id}."))
    } else {
        Ok("Khong tim thay job hoac job chua o trang thai tam dung.".to_string())
    }
}

async fn cancel_job(db: &Database, telegram_user_id: i64, job_id: &str) -> anyhow::Result<String> {
    if repo::cancel_job_for_user(db, telegram_user_id, job_id).await? {
        Ok(format!("Da yeu cau huy job {job_id}."))
    } else {
        Ok("Khong tim thay job hoac trang thai hien tai khong cho huy.".to_string())
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
            "Da xep hang lam lai job {job_id}.\n\
             Thu muc: {}  Item: {}  Thao tac: {}",
            summary.traversal_folders_requeued,
            summary.job_items_requeued,
            summary.operation_intents_replanned,
        ))
    } else {
        Ok(
            "Khong tim thay job, job khong the retry, hoac khong co phan loi de lam lai."
                .to_string(),
        )
    }
}

// ── User management ──────────────────────────────────────────────────────────

async fn grant_user(db: &Database, actor_user_id: i64, input: &str) -> anyhow::Result<String> {
    ensure_owner(db, actor_user_id).await?;
    let target = parse_telegram_user_id(input)?;
    repo::grant_operator(db, target).await?;
    Ok(format!("Da cap quyen operator cho {target}."))
}

async fn revoke_user(db: &Database, actor_user_id: i64, input: &str) -> anyhow::Result<String> {
    ensure_owner(db, actor_user_id).await?;
    let target = parse_telegram_user_id(input)?;
    if target == actor_user_id {
        anyhow::bail!("Chu so huu khong the tu thu hoi quyen cua minh");
    }
    if repo::revoke_operator(db, target).await? {
        Ok(format!("Da thu hoi quyen operator cua {target}."))
    } else {
        Ok("Khong tim thay nguoi dung hoac da bi vo hieu hoa.".to_string())
    }
}

async fn disconnect_google(
    config: &AppConfig,
    db: &Database,
    actor_user_id: i64,
) -> anyhow::Result<String> {
    ensure_owner(db, actor_user_id).await?;
    let Some(account) = repo::google_account_secret(db, "default").await? else {
        return Ok("Chua co tai khoan Google nao duoc ket noi.".to_string());
    };
    if account.status != "connected" && account.status != "reconnect_required" {
        return Ok(format!(
            "Tai khoan Google da o trang thai: {}",
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
    Ok("Da thu hoi refresh token Google va danh dau tai khoan la revoked.".to_string())
}

async fn ensure_owner(db: &Database, actor_user_id: i64) -> anyhow::Result<()> {
    if repo::is_owner(db, actor_user_id).await? {
        Ok(())
    } else {
        anyhow::bail!("Chi chu so huu moi co the quan ly nguoi dung")
    }
}

fn parse_telegram_user_id(input: &str) -> anyhow::Result<i64> {
    let value = input.trim().parse::<i64>()?;
    if value <= 0 {
        anyhow::bail!("Telegram user id phai la so duong");
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
            "Chua dat thu muc dich mac dinh.\n\
             Dung /set_destination <folder_url> de cau hinh.",
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
        return "Chua co thu muc dich nao duoc luu.\nDung /set_destination <folder_url>."
            .to_string();
    }
    let mut lines = vec!["Thu muc dich da luu:".to_string()];
    for p in profiles {
        let marker = if p.is_default { "[mac dinh]" } else { "" };
        let mut row = format!("  {} {}", p.label, marker).trim().to_string();
        if let Some(drive_id) = &p.destination_drive_id {
            row.push_str(&format!(" (Shared Drive {drive_id})"));
        } else {
            row.push_str(&format!(" (ID: {})", p.destination_parent_id));
        }
        lines.push(row);
    }
    lines.push(String::new());
    lines.push("Nhan vao ten de dat lam mac dinh. Them moi: /set_destination <url>".to_string());
    lines.join("\n")
}

/// Legacy helper kept for internal callers that only need the default.
#[allow(dead_code)]
async fn destination_summary(db: &Database) -> anyhow::Result<String> {
    let profiles = repo::list_recent_destinations(db, "default", 1).await?;
    match profiles.first() {
        Some(p) => {
            let mut lines = vec![
                "Thu muc dich mac dinh:".to_string(),
                format!("  Ten : {}", p.label),
                format!("  ID  : {}", p.destination_parent_id),
            ];
            if let Some(drive_id) = &p.destination_drive_id {
                lines.push(format!("  Shared Drive: {drive_id}"));
            }
            lines.push(String::new());
            lines.push("Thay doi: /set_destination <url>   Xoa: /clear_destination".to_string());
            Ok(lines.join("\n"))
        }
        None => Ok("Chua dat thu muc dich mac dinh.\n\
             Dung /set_destination <folder_url> de cau hinh."
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
        anyhow::bail!("Thu muc dich phai la Google Drive folder");
    }
    if file.capabilities.as_ref().and_then(|c| c.can_add_children) != Some(true) {
        anyhow::bail!("Tai khoan Google hien tai khong co quyen ghi vao thu muc dich nay");
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

    Ok(format!(
        "Da dat thu muc dich mac dinh: {}\nDrive folder ID: {}",
        file.name, file.id
    ))
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

    let item_type = if source.is_folder() {
        "Thu muc"
    } else {
        "File"
    };

    let mut lines = vec![
        format!("Nguon       : {}", source.name),
        format!("Loai        : {item_type}"),
        format!("MIME        : {}", source.mime_type),
    ];

    if let Some(size) = &source.size {
        let bytes: i64 = size.parse().unwrap_or(0);
        lines.push(format!("Kich thuoc  : {}", human_bytes(bytes)));
    }

    if let Some(drive_id) = &source.drive_id {
        lines.push(format!("Vi tri      : Shared Drive ({drive_id})"));
    } else {
        lines.push("Vi tri      : My Drive / duoc chia se".to_string());
    }

    // Capability warnings
    let caps = source.capabilities.as_ref();
    if source.is_folder() {
        if caps.and_then(|c| c.can_list_children) == Some(false) {
            lines.push(
                "CANH BAO: Khong the liet ke noi dung - thu muc co the bi han che quyen."
                    .to_string(),
            );
        }
    } else if caps.and_then(|c| c.can_copy) == Some(false) {
        lines.push("CANH BAO: Khong the sao chep - file bi han che hoac chong copy.".to_string());
    }

    if source.copy_requires_writer_permission == Some(true) {
        lines.push(
            "CANH BAO: copyRequiresWriterPermission - chi nguoi co quyen ghi moi copy duoc."
                .to_string(),
        );
    }

    if reference.resource_key.is_some() {
        lines.push("Luu y: Link dung resource key (link han che truy cap).".to_string());
    }

    if let Some(default_dest) = repo::default_destination_profile(db, "default").await? {
        let destination_preview = if source.is_folder() {
            format!("{}/{}", default_dest.label, source.name)
        } else {
            default_dest.label.clone()
        };
        lines.push(String::new());
        lines.push(format!("Thu muc dich: {destination_preview}"));
        lines.push("[Clone ngay]  [Doi dich]  [Huy]".to_string());
    } else {
        lines.push(String::new());
        lines.push("Chua co thu muc dich. Dung /set_destination <folder_url>.".to_string());
    }

    Ok(lines.join("\n"))
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
            "Ban dang co {active_count} job hoat dong. \
             Cho hoan thanh hoac huy bang /cancel <job_id>."
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
        anyhow::bail!("Nguon phai la Google Drive folder.");
    }
    let dest = drive
        .get_reference(access_token.as_str(), &dest_ref)
        .await?;
    if !dest.is_folder() {
        anyhow::bail!("Dich phai la Google Drive folder.");
    }
    if dest.capabilities.as_ref().and_then(|c| c.can_add_children) != Some(true) {
        anyhow::bail!("Khong co quyen ghi vao thu muc dich.");
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
        "Watch da tao thanh cong.\n\
         ID          : {short}\n\
         Nguon       : {src_name} ({src_id})\n\
         Dich        : {dst_name}\n\
         Clone ban dau dang chay nen - dung /watch_status {short} de theo doi.",
        short = &watch_id[..8.min(watch_id.len())],
        src_name = source.name,
        src_id = source.id,
        dst_name = dest.name,
    ))
}

async fn list_watches(db: &Database, telegram_user_id: i64) -> anyhow::Result<String> {
    let watches = repo::list_watches_for_user(db, telegram_user_id).await?;
    if watches.is_empty() {
        return Ok("Chua co watch subscription nao.".to_string());
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
        return Ok("Khong tim thay watch.".to_string());
    };
    Ok(format!(
        "Watch       : {id}\n\
         Trang thai  : {status}\n\
         Nguon       : {src}\n\
         Dich        : {dst}\n\
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
            "Policy khong hop le '{}'. Chon mot trong: versioned_copy | replace_copy | manual_confirmation",
            policy
        );
    }
    if repo::set_watch_content_update_policy(db, telegram_user_id, watch_id, policy).await? {
        Ok(format!(
            "Watch {watch_id} content_update_policy da doi thanh {policy}."
        ))
    } else {
        Ok("Khong tim thay watch.".to_string())
    }
}

// ── Helpers ──────────────────────────────────────────────────────────────────

fn short_job_id(job_id: &str) -> &str {
    job_id.get(..8).unwrap_or(job_id)
}

fn vi_account_status(status: &str) -> &str {
    match status {
        "connected" => "Da ket noi",
        "reconnect_required" => "Can dang nhap lai",
        "revoked" => "Da thu hoi",
        "disabled" => "Da tat",
        other => other,
    }
}

fn vi_job_status(status: &str) -> &str {
    match status {
        "queued" => "dang cho",
        "discovering" => "dang quet",
        "running" => "dang chay",
        "pausing" => "dang tam dung",
        "paused" => "tam dung",
        "cancelling" => "dang huy",
        "cancelled" => "da huy",
        "recovering" => "dang phuc hoi",
        "completed" => "hoan tat",
        "partially_completed" => "hoan tat mot phan",
        "failed" => "that bai",
        other => other,
    }
}

fn vi_watch_status(status: &str) -> &str {
    match status {
        "active" => "hoat dong",
        "paused" => "tam dung",
        "initializing" => "dang khoi tao",
        "catching_up" => "dang bat kip",
        "degraded" => "bi loi",
        "needs_reconcile" => "can dong bo lai",
        "stopped" => "da dung",
        other => other,
    }
}
