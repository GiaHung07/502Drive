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
            "Unauthorized. Ask the owner to /grant your Telegram user id.",
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
            bot.send_message(msg.chat.id, format!("Invalid Drive link: {err}"))
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
    let chat_id = query.message.as_ref().map(|message| message.chat().id);
    if !repo::is_authorized(&db, user_id).await.unwrap_or(false) {
        if let Some(chat_id) = chat_id {
            bot.send_message(chat_id, "Unauthorized.").await?;
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
                if let Some(message) = query.message.as_ref() {
                    bot.edit_message_text(
                        chat_id,
                        message.id(),
                        "Clone request expired or does not belong to you.",
                    )
                    .await?;
                } else {
                    bot.send_message(chat_id, "Clone request expired or does not belong to you.")
                        .await?;
                }
                return Ok(());
            };
            if let Some(message) = query.message.as_ref() {
                let _ = bot
                    .edit_message_text(chat_id, message.id(), "Clone started.")
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
                bot.edit_message_text(chat_id, message.id(), "Clone request cancelled.")
                    .await?;
                return Ok(());
            }
            "Clone request cancelled.".to_string()
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
        Some(("browse", _, _)) => "Destination browser is not implemented yet.".to_string(),
        _ => "Unknown action.".to_string(),
    };

    bot.send_message(chat_id, text).await?;
    Ok(())
}

fn parse_callback_action(data: &str) -> Option<(&str, &str, &str)> {
    let mut parts = data.splitn(3, ':');
    Some((parts.next()?, parts.next()?, parts.next()?))
}

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
                "gdclone-bot is running.\nUse /account to check Google auth, then /clone <Drive URL> to start.",
            )
            .await?;
        }
        Command::Help => {
            bot.send_message(
                msg.chat.id,
                concat!(
                    "Commands:\n",
                    "/preview — dashboard\n",
                    "/account — Google auth status\n",
                    "/destination — show/change default destination\n",
                    "/set_destination <url> — set default destination folder\n",
                    "/clone <url> — start clone job\n",
                    "/clone_here <url> — clone immediately (skip confirmation)\n",
                    "/jobs — list active jobs\n",
                    "/status <job_id>\n",
                    "/pause /resume /cancel /retry <job_id>\n",
                    "/watch <src_url> <dst_url> — create watch subscription\n",
                    "/watches /watch_status /watch_pause /watch_resume /watch_policy /unwatch\n",
                    "/whoami /grant <user_id> /revoke <user_id>"
                ),
            )
            .await?;
        }
        Command::Preview => {
            spawn_preview_dashboard(bot, msg.chat.id, db, user_id).await?;
        }
        Command::Connect => {
            bot.send_message(
                msg.chat.id,
                "Google auth is local-first.\nRun `gdclone-bot auth login` on the machine running the bot, then use /account to verify.",
            )
            .await?;
        }
        Command::Account => {
            let text = match repo::account_status(&db).await {
                Ok(Some(status)) => format!("Google account status: {status}"),
                Ok(None) => {
                    "No Google account connected. Run `gdclone-bot auth login` on the bot machine."
                        .to_string()
                }
                Err(err) => format!("Could not read account status: {err}"),
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
                .map(|user| user.role)
                .unwrap_or_else(|| "unknown".to_string());
            bot.send_message(
                msg.chat.id,
                format!("Telegram user id: {user_id}\nRole: {role}"),
            )
            .await?;
        }
        Command::Destination => {
            let text = destination_summary(&db).await;
            bot.send_message(msg.chat.id, text.unwrap_or_else(|err| err.to_string()))
                .await?;
        }
        Command::ClearDestination => {
            let text = match repo::clear_default_destination(&db, "default").await {
                Ok(0) => "No default destination was configured.".to_string(),
                Ok(_) => "Default destination cleared.".to_string(),
                Err(err) => format!("Could not clear destination: {err}"),
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
        // ── Watch commands ────────────────────────────────────────────────────
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
            let text = watch_status(&db, user_id, &watch_id).await;
            bot.send_message(msg.chat.id, text.unwrap_or_else(|err| err.to_string()))
                .await?;
        }
        Command::WatchPause(watch_id) => {
            let text = match repo::pause_watch_for_user(&db, user_id, &watch_id).await {
                Ok(true) => format!("Watch {watch_id} paused."),
                Ok(false) => "Watch not found or cannot be paused.".to_string(),
                Err(err) => err.to_string(),
            };
            bot.send_message(msg.chat.id, text).await?;
        }
        Command::WatchResume(watch_id) => {
            let text = match repo::resume_watch_for_user(&db, user_id, &watch_id).await {
                Ok(true) => format!("Watch {watch_id} resumed (catching up)."),
                Ok(false) => "Watch not found or is not paused.".to_string(),
                Err(err) => err.to_string(),
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
                Ok(true) => format!("Watch {watch_id} stopped."),
                Ok(false) => "Watch not found or already stopped.".to_string(),
                Err(err) => err.to_string(),
            };
            bot.send_message(msg.chat.id, text).await?;
        }
    }
    Ok(())
}

async fn list_jobs(db: &Database, telegram_user_id: i64) -> anyhow::Result<String> {
    let jobs = repo::list_active_jobs_for_user(db, telegram_user_id, 10).await?;
    if jobs.is_empty() {
        return Ok("No active jobs.".to_string());
    }

    let mut lines = vec!["Active jobs:".to_string()];
    for job in jobs {
        lines.push(format!(
            "{}  {}  discovered={} done={} err={} skipped={}",
            short_job_id(&job.id),
            job.status,
            job.total_discovered,
            job.completed_items,
            job.failed_items,
            job.skipped_items
        ));
    }
    Ok(lines.join("\n"))
}

async fn spawn_preview_dashboard(
    bot: Bot,
    chat_id: ChatId,
    db: Database,
    telegram_user_id: i64,
) -> ResponseResult<()> {
    let text = render_preview_dashboard(&db, telegram_user_id)
        .await
        .unwrap_or_else(|err| format!("Không tạo được preview: {err}"));
    let message = bot.send_message(chat_id, text).await?;
    tokio::spawn(async move {
        let mut last_text = String::new();
        for _ in 0..15 {
            tokio::time::sleep(Duration::from_secs(2)).await;
            let text = match render_preview_dashboard(&db, telegram_user_id).await {
                Ok(text) => text,
                Err(err) => format!("Không cập nhật được preview: {err}"),
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
        .unwrap_or_else(|| "not connected".to_string());
    let destination = repo::default_destination_profile(db, "default").await?;
    let jobs = repo::list_active_jobs_for_user(db, telegram_user_id, 5).await?;
    let counts = repo::job_status_counts(db).await?;

    let mut lines = vec![
        "gdclone-bot dashboard".to_string(),
        format!("Google: {account}"),
        match destination {
            Some(profile) => format!(
                "Destination: {} ({})",
                profile.label, profile.destination_parent_id
            ),
            None => "⚠️ No default destination set. Use /set_destination <folder_url>.".to_string(),
        },
    ];

    if counts.is_empty() {
        lines.push("Jobs: none".to_string());
    } else {
        let summary = counts
            .into_iter()
            .map(|item| format!("{}={}", item.status, item.count))
            .collect::<Vec<_>>()
            .join(" ");
        lines.push(format!("Jobs: {summary}"));
    }

    if jobs.is_empty() {
        lines.push("Active: none".to_string());
    } else {
        lines.push("Active:".to_string());
        for job in jobs {
            lines.push(format!(
                "  {} {} discovered={} done={} err={} skipped={}",
                short_job_id(&job.id),
                job.status,
                job.total_discovered,
                job.completed_items,
                job.failed_items,
                job.skipped_items
            ));
        }
    }

    lines.push("(auto-refresh 30 s — /preview to re-open)".to_string());
    Ok(lines.join("\n"))
}

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
            bot.send_message(chat_id, err.to_string()).await?;
            return Ok(());
        }
    };

    let progress_message = bot
        .send_message(
            chat_id,
            format!("State: queued\n{}", render_progress(None, 0, 0)),
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
        "Clone accepted. Track with /jobs or /status <job_id>.",
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
                    err.to_string(),
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
            bot.send_message(chat_id, err.to_string()).await?;
        }
    }
    Ok(())
}

fn spawn_progress_updater(
    bot: Bot,
    chat_id: ChatId,
    message_id: MessageId,
    db: Database,
    telegram_user_id: i64,
    interval: Duration,
    mut done_rx: oneshot::Receiver<()>,
) {
    let interval = if interval.is_zero() {
        Duration::from_secs(2)
    } else {
        interval
    };
    tokio::spawn(async move {
        let started_at = Instant::now();
        let mut last_text = String::new();
        loop {
            if !last_text.is_empty() {
                tokio::select! {
                    _ = &mut done_rx => break,
                    _ = tokio::time::sleep(interval) => {}
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
                            "State: queued\nElapsed: {}\n{}",
                            format_duration_secs(elapsed_secs),
                            render_progress(None, 0, 0),
                        );
                        if text != last_text {
                            last_text = text.clone();
                            if let Err(err) = bot.edit_message_text(chat_id, message_id, text).await
                            {
                                warn!(error = %err, "edit progress message failed");
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

            if is_terminal_status(&detail.status) {
                if let Err(err) = bot.edit_message_text(chat_id, message_id, text).await {
                    warn!(error = %err, "edit progress message failed");
                }
                break;
            }

            let paused = detail.status == "paused";
            if let Err(err) = bot
                .edit_message_text(chat_id, message_id, text)
                .reply_markup(keyboards::job_control_keyboard(&detail.id, paused))
                .await
            {
                warn!(error = %err, "edit progress message failed");
            }
        }
    });
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
    let rate = if elapsed_secs > 0 {
        format!("{:.1} items/s", done as f64 / elapsed_secs as f64)
    } else {
        "—".to_string()
    };

    let eta_str = total
        .and_then(|t| estimate_eta_secs(done, t, elapsed_secs))
        .map(format_duration_secs)
        .unwrap_or_else(|| "—".to_string());

    format!(
        "State: {}\nJob:   {}\nElapsed: {}  Rate: {}  ETA: {}\nDiscovered: {}  Skipped: {}\n{}",
        job.status,
        short_job_id(&job.id),
        format_duration_secs(elapsed_secs),
        rate,
        eta_str,
        job.total_discovered,
        job.skipped_items,
        render_progress(total, done, job.failed_items as u64),
    )
}

fn is_terminal_status(status: &str) -> bool {
    matches!(
        status,
        "completed" | "partially_completed" | "failed" | "cancelled"
    )
}

async fn show_job_status(
    db: &Database,
    telegram_user_id: i64,
    job_id: &str,
) -> anyhow::Result<String> {
    let Some(job) = repo::job_detail_for_user(db, telegram_user_id, job_id).await? else {
        return Ok("Job not found for your account.".to_string());
    };

    let mut lines = vec![
        format!("Job:         {}", job.id),
        format!("Kind:        {}", job.kind),
        format!("Status:      {}", job.status),
        format!("Source:      {}", job.source_root_id),
        format!("Destination: {}", job.destination_parent_id),
        format!("Discovered:  {}", job.total_discovered),
        format!("Completed:   {}", job.completed_items),
        format!("Failed:      {}", job.failed_items),
        format!("Skipped:     {}", job.skipped_items),
    ];
    if let Some(error) = job.error_summary {
        lines.push(format!("Last error:  {error}"));
    }
    Ok(lines.join("\n"))
}

async fn pause_job(db: &Database, telegram_user_id: i64, job_id: &str) -> anyhow::Result<String> {
    if repo::pause_job_for_user(db, telegram_user_id, job_id).await? {
        Ok(format!("Pause requested for job {job_id}."))
    } else {
        Ok("Job not found or cannot be paused from its current state.".to_string())
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
        Ok(format!("Resume requested for job {job_id}."))
    } else {
        Ok("Job not found or is not paused.".to_string())
    }
}

async fn cancel_job(db: &Database, telegram_user_id: i64, job_id: &str) -> anyhow::Result<String> {
    if repo::cancel_job_for_user(db, telegram_user_id, job_id).await? {
        Ok(format!("Cancel requested for job {job_id}."))
    } else {
        Ok("Job not found or cannot be cancelled from its current state.".to_string())
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
            "Retry queued for job {job_id}. folders={} items={} operations={}",
            summary.traversal_folders_requeued,
            summary.job_items_requeued,
            summary.operation_intents_replanned
        ))
    } else {
        Ok("Job not found, not retryable, or has no failed work.".to_string())
    }
}

async fn grant_user(db: &Database, actor_user_id: i64, input: &str) -> anyhow::Result<String> {
    ensure_owner(db, actor_user_id).await?;
    let target = parse_telegram_user_id(input)?;
    repo::grant_operator(db, target).await?;
    Ok(format!("Granted operator access to {target}."))
}

async fn disconnect_google(
    config: &AppConfig,
    db: &Database,
    actor_user_id: i64,
) -> anyhow::Result<String> {
    ensure_owner(db, actor_user_id).await?;
    let Some(account) = repo::google_account_secret(db, "default").await? else {
        return Ok("No Google account connected.".to_string());
    };
    if account.status != "connected" && account.status != "reconnect_required" {
        return Ok(format!("Google account is already {}.", account.status));
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
    Ok("Google refresh token revoked and local account marked revoked.".to_string())
}

async fn revoke_user(db: &Database, actor_user_id: i64, input: &str) -> anyhow::Result<String> {
    ensure_owner(db, actor_user_id).await?;
    let target = parse_telegram_user_id(input)?;
    if target == actor_user_id {
        anyhow::bail!("Owner cannot revoke self");
    }
    if repo::revoke_operator(db, target).await? {
        Ok(format!("Revoked operator access for {target}."))
    } else {
        Ok("User not found, already disabled, or is owner.".to_string())
    }
}

async fn ensure_owner(db: &Database, actor_user_id: i64) -> anyhow::Result<()> {
    if repo::is_owner(db, actor_user_id).await? {
        Ok(())
    } else {
        anyhow::bail!("Only owner can manage authorized users")
    }
}

fn parse_telegram_user_id(input: &str) -> anyhow::Result<i64> {
    let value = input.trim().parse::<i64>()?;
    if value <= 0 {
        anyhow::bail!("Telegram user id must be positive");
    }
    Ok(value)
}

fn short_job_id(job_id: &str) -> &str {
    job_id.get(..8).unwrap_or(job_id)
}

/// Build a rich /destination summary listing default + recent destinations.
async fn destination_summary(db: &Database) -> anyhow::Result<String> {
    let default = repo::default_destination_profile(db, "default").await?;
    let mut lines: Vec<String> = Vec::new();

    match &default {
        Some(p) => {
            lines.push(format!("Default destination: {}", p.label));
            lines.push(format!("  Drive folder ID: {}", p.destination_parent_id));
            if let Some(drive_id) = &p.destination_drive_id {
                lines.push(format!("  Shared Drive:    {drive_id}"));
            }
        }
        None => {
            lines.push(
                "No default destination set.\nUse /set_destination <folder_url_or_id>.".to_string(),
            );
        }
    }

    lines.push("".to_string());
    lines.push("Change or clear: /set_destination <url>  /clear_destination".to_string());
    Ok(lines.join("\n"))
}

async fn start_clone_reference(
    config: &AppConfig,
    db: &Database,
    chat_id: i64,
    telegram_user_id: i64,
    source: crate::drive::links::DriveReference,
    progress_message_id: Option<i32>,
) -> anyhow::Result<CloneOutcome> {
    // Per-user concurrency guard: count active jobs for this user.
    let active_count = repo::active_job_count_for_user(db, telegram_user_id).await?;
    if active_count >= config.engine.max_active_jobs_per_user as i64 {
        anyhow::bail!(
            "You already have {active_count} active job(s). \
             Wait for them to finish or cancel with /cancel <job_id>."
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
        anyhow::bail!("Destination must be a Google Drive folder");
    }
    if file
        .capabilities
        .as_ref()
        .and_then(|capabilities| capabilities.can_add_children)
        != Some(true)
    {
        anyhow::bail!("Authenticated account cannot add children to this destination folder");
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
        "Default destination set: {}\nDrive item ID: {}",
        file.name, file.id
    ))
}

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

    let item_type = if source.is_folder() { "folder" } else { "file" };
    let mut lines = vec![
        format!("Source: {}", source.name),
        format!("Type: {item_type}"),
        format!("MIME: {}", source.mime_type),
    ];

    if let Some(size) = &source.size {
        lines.push(format!("Size: {size} bytes"));
    }
    if source.drive_id.is_some() {
        lines.push("Location: Shared Drive".to_string());
    } else {
        lines.push("Location: My Drive or shared-with-me".to_string());
    }
    if reference.resource_key.is_some() {
        lines.push("Warning: source link uses a resource key".to_string());
    }

    if let Some(default_dest) = repo::default_destination_profile(db, "default").await? {
        let destination_preview = if source.is_folder() {
            format!("{}/{}", default_dest.label, source.name)
        } else {
            default_dest.label
        };
        lines.push(format!("Destination: {destination_preview}"));
        lines.push("[Clone now] [Change destination] [Cancel]".to_string());
    } else {
        lines.push(
            "No default destination configured. Use /set_destination <folder_url_or_id>."
                .to_string(),
        );
    }

    Ok(lines.join("\n"))
}

// ── Watch handlers ───────────────────────────────────────────────────────────

/// `/watch <source_url_or_id> <dest_url_or_id>`
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
        anyhow::bail!("Usage: /watch <source_url_or_id> <dest_url_or_id>");
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
        anyhow::bail!("Source must be a Google Drive folder.");
    }
    let dest = drive
        .get_reference(access_token.as_str(), &dest_ref)
        .await?;
    if !dest.is_folder() {
        anyhow::bail!("Destination must be a Google Drive folder.");
    }
    if dest.capabilities.as_ref().and_then(|c| c.can_add_children) != Some(true) {
        anyhow::bail!("Cannot add children to destination folder.");
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

    // Spawn the initializer as a background task — it may take minutes for
    // large trees. We notify the user via Telegram as it progresses.
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
        "Watch created.\n\
         ID:          {short}\n\
         Source:      {src_name} ({src_id})\n\
         Destination: {dst_name}\n\
         ⌛ Initial clone starting in background.\n\
         Track with /watch_status {short}",
        short = &watch_id[..8.min(watch_id.len())],
        src_name = source.name,
        src_id = source.id,
        dst_name = dest.name,
    ))
}

async fn list_watches(db: &Database, telegram_user_id: i64) -> anyhow::Result<String> {
    let watches = repo::list_watches_for_user(db, telegram_user_id).await?;
    if watches.is_empty() {
        return Ok("No watch subscriptions.".to_string());
    }
    let mut lines = vec!["Watch subscriptions:".to_string()];
    for w in &watches {
        lines.push(format!(
            "{}  {}  src={}  dst={}",
            &w.id[..8.min(w.id.len())],
            w.status,
            w.source_root_id,
            w.destination_root_id,
        ));
    }
    Ok(lines.join("\n"))
}

async fn watch_status(
    db: &Database,
    telegram_user_id: i64,
    watch_id: &str,
) -> anyhow::Result<String> {
    let Some(w) = repo::watch_for_user(db, telegram_user_id, watch_id).await? else {
        return Ok("Watch not found.".to_string());
    };
    Ok(format!(
        "Watch: {}\nStatus: {}\nSource: {}\nDestination: {}\nContent update: {}\nDeletion: {}\nMove-out: {}\nBaseline seq: {}\nConsumed seq: {}",
        w.id,
        w.status,
        w.source_root_id,
        w.destination_root_id,
        w.content_update_policy,
        w.deletion_policy,
        w.move_out_policy,
        w.baseline_sequence,
        w.last_consumed_sequence,
    ))
}

/// `/watch_policy <watch_id> <policy>`
/// e.g. `/watch_policy abc123 versioned_copy`
async fn set_watch_policy(
    db: &Database,
    telegram_user_id: i64,
    input: &str,
) -> anyhow::Result<String> {
    let parts: Vec<&str> = input.splitn(2, char::is_whitespace).collect();
    if parts.len() < 2 {
        anyhow::bail!(
            "Usage: /watch_policy <watch_id> <versioned_copy|replace_copy|manual_confirmation>"
        );
    }
    let watch_id = parts[0];
    let policy = parts[1].trim();
    if !matches!(
        policy,
        "versioned_copy" | "replace_copy" | "manual_confirmation"
    ) {
        anyhow::bail!(
            "Unknown policy '{}'. Choose: versioned_copy | replace_copy | manual_confirmation",
            policy
        );
    }
    if repo::set_watch_content_update_policy(db, telegram_user_id, watch_id, policy).await? {
        Ok(format!(
            "Watch {watch_id} content_update_policy set to {policy}."
        ))
    } else {
        Ok("Watch not found.".to_string())
    }
}
