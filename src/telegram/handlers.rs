use std::{
    collections::VecDeque,
    time::{Duration, Instant},
};

use serde::{Deserialize, Serialize};
use teloxide::{
    prelude::*,
    types::{ForceReply, InlineKeyboardMarkup, InputFile, MessageId},
    utils::command::BotCommands,
};
use tokio::sync::oneshot;
use tracing::warn;

use crate::{
    config::AppConfig,
    drive::{
        auth as oauth,
        client::DriveClient,
        links::{DriveReference, parse_drive_reference},
        token_manager::TokenManager,
        types::DriveFile,
        types::FOLDER_MIME_TYPE,
    },
    engine::{
        copy::{CloneOutcome, CloneRequest, CloneService},
        recovery,
        services::{JobResolveError, JobService},
    },
    report,
    secrets::FileSecretStore,
    state::{db::Database, repo},
    telegram::{
        bot_commands,
        commands::Command,
        i18n::TextKey as T,
        keyboards,
        progress::{format_duration_secs, render_progress},
        render::*,
    },
    watch::{glob, run_initial_clone},
};

const CALLBACK_STATE_TTL_MS: i64 = 15 * 60 * 1000;
const CLONE_PLAN_ITEM_LIMIT: usize = 2_000;
const DESTINATION_BROWSER_LIMIT: usize = 20;
const WATCH_LIST_PAGE_SIZE: usize = 5;

fn ui_language(config: &AppConfig) -> keyboards::UiLanguage {
    keyboards::UiLanguage::from_code(&config.telegram.language)
}

async fn apply_user_language(config: &mut AppConfig, db: &Database, telegram_user_id: i64) {
    if let Ok(Some(language)) = repo::telegram_language_preference(db, telegram_user_id).await {
        config.telegram.language = language;
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum ReplyPrompt {
    Clone,
    CloneHere,
    SetDestination,
    Status,
    Pause,
    Resume,
    Cancel,
    Retry,
    Grant,
    Revoke,
    Watch,
    WatchStatus,
    WatchPause,
    WatchResume,
    WatchPolicy,
    Unwatch,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum WatchListMode {
    Status,
    Pause,
    Resume,
    Unwatch,
}

impl WatchListMode {
    fn as_str(self) -> &'static str {
        match self {
            Self::Status => "status",
            Self::Pause => "pause",
            Self::Resume => "resume",
            Self::Unwatch => "unwatch",
        }
    }

    fn from_str(value: &str) -> Option<Self> {
        match value {
            "status" => Some(Self::Status),
            "pause" => Some(Self::Pause),
            "resume" => Some(Self::Resume),
            "unwatch" => Some(Self::Unwatch),
            _ => None,
        }
    }
}

impl ReplyPrompt {
    fn marker_key(self) -> T {
        match self {
            Self::Clone => T::PromptMarkerClone,
            Self::CloneHere => T::PromptMarkerCloneHere,
            Self::SetDestination => T::PromptMarkerSetDestination,
            Self::Status => T::PromptMarkerStatus,
            Self::Pause => T::PromptMarkerPause,
            Self::Resume => T::PromptMarkerResume,
            Self::Cancel => T::PromptMarkerCancel,
            Self::Retry => T::PromptMarkerRetry,
            Self::Grant => T::PromptMarkerGrant,
            Self::Revoke => T::PromptMarkerRevoke,
            Self::Watch => T::PromptMarkerWatch,
            Self::WatchStatus => T::PromptMarkerWatchStatus,
            Self::WatchPause => T::PromptMarkerWatchPause,
            Self::WatchResume => T::PromptMarkerWatchResume,
            Self::WatchPolicy => T::PromptMarkerWatchPolicy,
            Self::Unwatch => T::PromptMarkerUnwatch,
        }
    }

    pub(crate) fn marker(self, lang: keyboards::UiLanguage) -> &'static str {
        lang.text(self.marker_key())
    }

    fn matches_marker(self, text: &str) -> bool {
        text.contains(self.marker(keyboards::UiLanguage::Vi))
            || text.contains(self.marker(keyboards::UiLanguage::En))
    }

    fn placeholder_key(self) -> T {
        match self {
            Self::Clone | Self::CloneHere => T::PromptPlaceholderDriveSource,
            Self::SetDestination => T::PromptPlaceholderDestinationFolder,
            Self::Status | Self::Pause | Self::Resume | Self::Cancel | Self::Retry => {
                T::PromptPlaceholderJobId
            }
            Self::Grant | Self::Revoke => T::PromptPlaceholderTelegramUserId,
            Self::Watch => T::PromptPlaceholderWatchLinks,
            Self::WatchStatus | Self::WatchPause | Self::WatchResume | Self::Unwatch => {
                T::PromptPlaceholderWatchId
            }
            Self::WatchPolicy => T::PromptPlaceholderWatchPolicy,
        }
    }

    fn placeholder(self, lang: keyboards::UiLanguage) -> &'static str {
        lang.text(self.placeholder_key())
    }
}

pub async fn handle_message(
    bot: Bot,
    msg: Message,
    mut config: AppConfig,
    db: Database,
) -> ResponseResult<()> {
    let user_id = match msg.from.as_ref() {
        Some(user) => user.id.0 as i64,
        None => return Ok(()),
    };
    apply_user_language(&mut config, &db, user_id).await;

    if !repo::is_authorized(&db, user_id).await.unwrap_or(false) {
        bot.send_message(msg.chat.id, access_denied_message(ui_language(&config)))
            .await?;
        return Ok(());
    }

    let Some(text) = msg.text().map(str::to_string) else {
        return Ok(());
    };

    if let Ok(command) = Command::parse(&text, "gdclone_bot") {
        return handle_command(bot, msg, config, db, user_id, command).await;
    }

    if let Some(prompt) = reply_prompt_kind(&msg) {
        return handle_reply_prompt(
            bot,
            msg,
            config,
            db,
            user_id,
            prompt,
            text.trim().to_string(),
        )
        .await;
    }

    match parse_drive_reference(&text) {
        Ok(reference) => {
            if matches!(
                reference.hinted_kind,
                Some(crate::drive::links::DriveItemKindHint::Folder)
            ) {
                handle_folder_link_detected(bot, msg.chat.id, config, db, user_id, text, reference)
                    .await?;
            } else {
                handle_clone_request(bot, msg.chat.id, config, db, user_id, text).await?;
            }
        }
        Err(err) => {
            bot.send_message(
                msg.chat.id,
                invalid_drive_link_text(ui_language(&config), &err),
            )
            .await?;
        }
    }
    Ok(())
}

async fn handle_reply_prompt(
    bot: Bot,
    msg: Message,
    config: AppConfig,
    db: Database,
    user_id: i64,
    prompt: ReplyPrompt,
    input: String,
) -> ResponseResult<()> {
    if input.trim().is_empty() {
        send_reply_prompt(
            &bot,
            msg.chat.id,
            prompt_text(prompt, ui_language(&config)),
            prompt,
            ui_language(&config),
        )
        .await?;
        return Ok(());
    }

    match prompt {
        ReplyPrompt::Clone => {
            handle_clone_request(bot, msg.chat.id, config, db, user_id, input).await?;
        }
        ReplyPrompt::CloneHere => {
            spawn_clone_now(bot, msg.chat.id, config, db, user_id, input).await?;
        }
        ReplyPrompt::SetDestination => {
            let text = set_destination(&config, &db, &input).await;
            bot.send_message(
                msg.chat.id,
                text.unwrap_or_else(|err| format_error_for_user(&err, ui_language(&config))),
            )
            .await?;
        }
        ReplyPrompt::Status => {
            let text = show_job_status(&db, user_id, &input, ui_language(&config)).await;
            bot.send_message(msg.chat.id, text.unwrap_or_else(|err| err.to_string()))
                .await?;
        }
        ReplyPrompt::Pause => {
            let text = pause_job(&db, user_id, &input, ui_language(&config)).await;
            bot.send_message(msg.chat.id, text.unwrap_or_else(|err| err.to_string()))
                .await?;
        }
        ReplyPrompt::Resume => {
            let text = resume_job(&config, &db, user_id, &input, ui_language(&config)).await;
            bot.send_message(msg.chat.id, text.unwrap_or_else(|err| err.to_string()))
                .await?;
        }
        ReplyPrompt::Cancel => {
            let text = cancel_job(&db, user_id, &input, ui_language(&config)).await;
            bot.send_message(msg.chat.id, text.unwrap_or_else(|err| err.to_string()))
                .await?;
        }
        ReplyPrompt::Retry => {
            let text = retry_job(&config, &db, user_id, &input, ui_language(&config)).await;
            bot.send_message(msg.chat.id, text.unwrap_or_else(|err| err.to_string()))
                .await?;
        }
        ReplyPrompt::Grant => {
            let text = grant_user(&db, user_id, &input).await;
            bot.send_message(msg.chat.id, text.unwrap_or_else(|err| err.to_string()))
                .await?;
        }
        ReplyPrompt::Revoke => {
            let text = revoke_user(&db, user_id, &input).await;
            bot.send_message(msg.chat.id, text.unwrap_or_else(|err| err.to_string()))
                .await?;
        }
        ReplyPrompt::Watch => {
            let text = start_watch(
                &bot,
                &config,
                &db,
                msg.chat.id.0,
                user_id,
                &input,
                ui_language(&config),
            )
            .await;
            bot.send_message(msg.chat.id, text.unwrap_or_else(|err| err.to_string()))
                .await?;
        }
        ReplyPrompt::WatchStatus => {
            let text =
                watch_status_detail(&config, &db, user_id, &input, ui_language(&config)).await;
            bot.send_message(msg.chat.id, text.unwrap_or_else(|err| err.to_string()))
                .await?;
        }
        ReplyPrompt::WatchPause => {
            let text = pause_watch(&db, user_id, &input, ui_language(&config)).await;
            bot.send_message(msg.chat.id, text.unwrap_or_else(|err| err.to_string()))
                .await?;
        }
        ReplyPrompt::WatchResume => {
            let text = resume_watch(&config, &db, user_id, &input, ui_language(&config)).await;
            bot.send_message(msg.chat.id, text.unwrap_or_else(|err| err.to_string()))
                .await?;
        }
        ReplyPrompt::WatchPolicy => {
            let text = set_watch_policy(&db, user_id, &input, ui_language(&config)).await;
            bot.send_message(msg.chat.id, text.unwrap_or_else(|err| err.to_string()))
                .await?;
        }
        ReplyPrompt::Unwatch => {
            let text = stop_watch(&db, user_id, &input, ui_language(&config)).await;
            bot.send_message(msg.chat.id, text.unwrap_or_else(|err| err.to_string()))
                .await?;
        }
    }
    Ok(())
}

fn reply_prompt_kind(msg: &Message) -> Option<ReplyPrompt> {
    let text = msg.reply_to_message()?.text()?;
    [
        ReplyPrompt::Clone,
        ReplyPrompt::CloneHere,
        ReplyPrompt::SetDestination,
        ReplyPrompt::Status,
        ReplyPrompt::Pause,
        ReplyPrompt::Resume,
        ReplyPrompt::Cancel,
        ReplyPrompt::Retry,
        ReplyPrompt::Grant,
        ReplyPrompt::Revoke,
        ReplyPrompt::Watch,
        ReplyPrompt::WatchStatus,
        ReplyPrompt::WatchPause,
        ReplyPrompt::WatchResume,
        ReplyPrompt::WatchPolicy,
        ReplyPrompt::Unwatch,
    ]
    .into_iter()
    .find(|prompt| prompt.matches_marker(text))
}

pub async fn handle_callback_query(
    bot: Bot,
    query: CallbackQuery,
    mut config: AppConfig,
    db: Database,
) -> ResponseResult<()> {
    if let Err(err) = bot.answer_callback_query(query.id.clone()).await {
        warn!(error = %err, "answer callback query failed");
    }

    let user_id = query.from.id.0 as i64;
    apply_user_language(&mut config, &db, user_id).await;
    let chat_id = query.message.as_ref().map(|m| m.chat().id);
    if !repo::is_authorized(&db, user_id).await.unwrap_or(false) {
        if let Some(chat_id) = chat_id {
            bot.send_message(chat_id, access_denied_short(ui_language(&config)))
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
        Some(("menu", "open", "home")) => {
            let text = render_home_dashboard(&config, &db, user_id, ui_language(&config)).await;
            edit_or_send_with_keyboard(
                &bot,
                chat_id,
                query.message.as_ref().map(|m| m.id()),
                text.unwrap_or_else(|err| load_home_error(ui_language(&config), &err)),
                Some(keyboards::main_menu_keyboard(
                    config.watch.enabled,
                    ui_language(&config),
                )),
            )
            .await?;
            return Ok(());
        }
        Some(("menu", "open", "account")) => {
            let text = account_summary(&config, &db, ui_language(&config))
                .await
                .unwrap_or_else(|err| format!("Lỗi đọc trạng thái tài khoản: {err}"));
            edit_or_send_with_keyboard(
                &bot,
                chat_id,
                query.message.as_ref().map(|m| m.id()),
                text,
                Some(keyboards::account_keyboard(ui_language(&config))),
            )
            .await?;
            return Ok(());
        }
        Some(("lang", "open", "panel")) => {
            let lang = ui_language(&config);
            edit_or_send_with_keyboard(
                &bot,
                chat_id,
                query.message.as_ref().map(|m| m.id()),
                language_panel_text(lang),
                Some(keyboards::language_keyboard(lang)),
            )
            .await?;
            return Ok(());
        }
        Some(("lang", "set", code)) => {
            let Some(lang) = language_from_callback(code) else {
                let lang = ui_language(&config);
                edit_or_send_with_keyboard(
                    &bot,
                    chat_id,
                    query.message.as_ref().map(|m| m.id()),
                    language_invalid_text(lang),
                    Some(keyboards::language_keyboard(lang)),
                )
                .await?;
                return Ok(());
            };
            let code = match lang {
                keyboards::UiLanguage::Vi => "vi",
                keyboards::UiLanguage::En => "en",
            };
            if let Err(err) = repo::set_telegram_language_preference(&db, user_id, code).await {
                edit_or_send_with_keyboard(
                    &bot,
                    chat_id,
                    query.message.as_ref().map(|m| m.id()),
                    language_save_error(ui_language(&config), &err),
                    Some(keyboards::language_keyboard(ui_language(&config))),
                )
                .await?;
                return Ok(());
            }
            config.telegram.language = code.to_string();
            let _ = bot
                .set_my_commands(bot_commands(config.watch.enabled, lang))
                .await;
            edit_or_send_with_keyboard(
                &bot,
                chat_id,
                query.message.as_ref().map(|m| m.id()),
                language_changed_text(lang),
                Some(keyboards::language_keyboard(lang)),
            )
            .await?;
            return Ok(());
        }
        Some(("menu", "open", "destination")) => {
            let (text, keyboard) =
                render_destination_panel(&db, user_id, chat_id.0, ui_language(&config)).await;
            edit_or_send_with_keyboard(
                &bot,
                chat_id,
                query.message.as_ref().map(|m| m.id()),
                text,
                keyboard,
            )
            .await?;
            return Ok(());
        }
        Some(("menu", "open", "jobs")) => {
            let result = render_jobs_panel(&db, user_id, ui_language(&config)).await;
            match result {
                Ok((text, keyboard)) => {
                    edit_or_send_with_keyboard(
                        &bot,
                        chat_id,
                        query.message.as_ref().map(|m| m.id()),
                        text,
                        Some(keyboard),
                    )
                    .await?;
                }
                Err(err) => {
                    edit_or_send_with_keyboard(
                        &bot,
                        chat_id,
                        query.message.as_ref().map(|m| m.id()),
                        load_jobs_error(ui_language(&config), &err),
                        Some(keyboards::back_home_keyboard(ui_language(&config))),
                    )
                    .await?;
                }
            }
            return Ok(());
        }
        Some(("menu", "open", "watches")) => {
            let result = render_watch_panel(&config, &db, user_id, 0, WatchListMode::Status).await;
            match result {
                Ok((text, keyboard)) => {
                    let keyboard = (!keyboard.inline_keyboard.is_empty()).then_some(keyboard);
                    edit_or_send_with_keyboard(
                        &bot,
                        chat_id,
                        query.message.as_ref().map(|m| m.id()),
                        text,
                        keyboard
                            .or_else(|| Some(keyboards::back_home_keyboard(ui_language(&config)))),
                    )
                    .await?;
                }
                Err(err) => {
                    edit_or_send_with_keyboard(
                        &bot,
                        chat_id,
                        query.message.as_ref().map(|m| m.id()),
                        load_watch_error(ui_language(&config), &err),
                        Some(keyboards::back_home_keyboard(ui_language(&config))),
                    )
                    .await?;
                }
            }
            return Ok(());
        }
        Some(("menu", "prompt", "clone")) => {
            send_reply_prompt(
                &bot,
                chat_id,
                clone_prompt(ui_language(&config)),
                ReplyPrompt::Clone,
                ui_language(&config),
            )
            .await?;
            return Ok(());
        }
        Some(("menu", "prompt", "clone_here")) => {
            send_reply_prompt(
                &bot,
                chat_id,
                clone_here_prompt(ui_language(&config)),
                ReplyPrompt::CloneHere,
                ui_language(&config),
            )
            .await?;
            return Ok(());
        }
        Some(("menu", "prompt", "watch")) => {
            send_reply_prompt(
                &bot,
                chat_id,
                watch_prompt(ui_language(&config)),
                ReplyPrompt::Watch,
                ui_language(&config),
            )
            .await?;
            return Ok(());
        }
        Some(("menu", "prompt", "set_destination")) => {
            send_reply_prompt(
                &bot,
                chat_id,
                set_destination_prompt(ui_language(&config)),
                ReplyPrompt::SetDestination,
                ui_language(&config),
            )
            .await?;
            return Ok(());
        }
        Some(("clone", "confirm", source)) => {
            let Some(source) =
                repo::consume_callback_state(&db, source, user_id, chat_id.0, "clone_confirm")
                    .await
                    .unwrap_or(None)
            else {
                let msg_text = clone_request_expired(ui_language(&config));
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
                    .edit_message_text(chat_id, message.id(), clone_starting(ui_language(&config)))
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
                bot.edit_message_text(
                    chat_id,
                    message.id(),
                    clone_request_cancelled(ui_language(&config)),
                )
                .await?;
                return Ok(());
            }
            clone_request_cancelled(ui_language(&config)).to_string()
        }
        Some(("smart", "clone", state_id)) => {
            let Some(source) =
                repo::consume_callback_state(&db, state_id, user_id, chat_id.0, "smart_link")
                    .await
                    .unwrap_or(None)
            else {
                let msg_text = clone_request_expired(ui_language(&config));
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
                    .edit_message_text(chat_id, message.id(), clone_starting(ui_language(&config)))
                    .await;
            }
            spawn_clone_now(bot.clone(), chat_id, config, db, user_id, source).await?;
            return Ok(());
        }
        Some(("smart", "sync", state_id)) => {
            let Some(source) =
                repo::consume_callback_state(&db, state_id, user_id, chat_id.0, "smart_link")
                    .await
                    .unwrap_or(None)
            else {
                let msg_text = clone_request_expired(ui_language(&config));
                if let Some(message) = query.message.as_ref() {
                    bot.edit_message_text(chat_id, message.id(), msg_text)
                        .await?;
                } else {
                    bot.send_message(chat_id, msg_text).await?;
                }
                return Ok(());
            };
            let res = start_watch(
                &bot,
                &config,
                &db,
                chat_id.0,
                user_id,
                &source,
                ui_language(&config),
            )
            .await;
            let msg_text = res.unwrap_or_else(|err| err.to_string());
            if let Some(message) = query.message.as_ref() {
                bot.edit_message_text(chat_id, message.id(), msg_text)
                    .await?;
            } else {
                bot.send_message(chat_id, msg_text).await?;
            }
            return Ok(());
        }
        Some(("smart", "cancel", state_id)) => {
            let _ =
                repo::consume_callback_state(&db, state_id, user_id, chat_id.0, "smart_link").await;
            if let Some(message) = query.message.as_ref() {
                bot.edit_message_text(
                    chat_id,
                    message.id(),
                    clone_request_cancelled(ui_language(&config)),
                )
                .await?;
                return Ok(());
            }
            clone_request_cancelled(ui_language(&config)).to_string()
        }
        Some(("job", "status", job_id)) => {
            let result = job_status_panel(&db, user_id, job_id, ui_language(&config)).await;
            match result {
                Ok((text, keyboard)) => {
                    edit_or_send_with_keyboard(
                        &bot,
                        chat_id,
                        query.message.as_ref().map(|m| m.id()),
                        text,
                        Some(keyboard),
                    )
                    .await?;
                }
                Err(err) => {
                    edit_or_send_with_keyboard(
                        &bot,
                        chat_id,
                        query.message.as_ref().map(|m| m.id()),
                        err.to_string(),
                        Some(keyboards::back_home_keyboard(ui_language(&config))),
                    )
                    .await?;
                }
            }
            return Ok(());
        }
        Some(("job", "pause", job_id)) => {
            edit_job_after_action(
                &bot,
                chat_id,
                query.message.as_ref().map(|m| m.id()),
                &db,
                user_id,
                job_id,
                ui_language(&config),
                pause_job(&db, user_id, job_id, ui_language(&config)).await,
            )
            .await?;
            return Ok(());
        }
        Some(("job", "resume", job_id)) => {
            let result = resume_job(&config, &db, user_id, job_id, ui_language(&config)).await;
            edit_job_after_action(
                &bot,
                chat_id,
                query.message.as_ref().map(|m| m.id()),
                &db,
                user_id,
                job_id,
                ui_language(&config),
                result,
            )
            .await?;
            return Ok(());
        }
        Some(("job", "retry", job_id)) => {
            let result = retry_job(&config, &db, user_id, job_id, ui_language(&config)).await;
            edit_job_after_action(
                &bot,
                chat_id,
                query.message.as_ref().map(|m| m.id()),
                &db,
                user_id,
                job_id,
                ui_language(&config),
                result,
            )
            .await?;
            return Ok(());
        }
        Some(("job", "report", job_id)) => {
            send_report_for_job_id(&bot, chat_id, &config, &db, user_id, job_id).await?;
            return Ok(());
        }
        Some(("job", "cancel", job_id)) => {
            let lang = ui_language(&config);
            edit_or_send_with_keyboard(
                &bot,
                chat_id,
                query.message.as_ref().map(|m| m.id()),
                format!(
                    "{}\n━━━━━━━━━━━━\nJob: {}\n\n{}",
                    lang.text(T::ConfirmCancelJobTitle),
                    short_job_id(job_id),
                    lang.text(T::ConfirmCancelJobBody)
                ),
                Some(keyboards::job_cancel_confirm_keyboard(job_id, lang)),
            )
            .await?;
            return Ok(());
        }
        Some(("job", "cancel_confirm", job_id)) => {
            edit_job_after_action(
                &bot,
                chat_id,
                query.message.as_ref().map(|m| m.id()),
                &db,
                user_id,
                job_id,
                ui_language(&config),
                cancel_job(&db, user_id, job_id, ui_language(&config)).await,
            )
            .await?;
            return Ok(());
        }
        Some(("dest", "select", profile_id)) => {
            match repo::set_default_destination_by_id(&db, "default", profile_id).await {
                Ok(true) => {
                    // Re-render the destination panel in place.
                    let (text, keyboard) =
                        render_destination_panel(&db, user_id, chat_id.0, ui_language(&config))
                            .await;
                    if let Some(message) = query.message.as_ref() {
                        if let Some(keyboard) = keyboard {
                            let _ = bot
                                .edit_message_text(chat_id, message.id(), &text)
                                .reply_markup(keyboard)
                                .await;
                        } else {
                            let _ = bot.edit_message_text(chat_id, message.id(), &text).await;
                        }
                        return Ok(());
                    }
                    text
                }
                Ok(false) => destination_not_found(ui_language(&config)).to_string(),
                Err(err) => destination_switch_error(ui_language(&config), err),
            }
        }
        Some(("browse", "open", state_id)) => {
            let result = open_destination_browser(&config, &db, user_id, chat_id.0, state_id).await;
            match result {
                Ok((text, keyboard)) => {
                    let keyboard = (!keyboard.inline_keyboard.is_empty()).then_some(keyboard);
                    edit_or_send_with_keyboard(
                        &bot,
                        chat_id,
                        query.message.as_ref().map(|m| m.id()),
                        text,
                        keyboard,
                    )
                    .await?;
                }
                Err(err) => {
                    edit_or_send_with_keyboard(
                        &bot,
                        chat_id,
                        query.message.as_ref().map(|m| m.id()),
                        browse_folder_error(ui_language(&config), &err),
                        None,
                    )
                    .await?;
                }
            }
            return Ok(());
        }
        Some(("browse", "pick", state_id)) => {
            let result =
                pick_destination_from_browser(&config, &db, user_id, chat_id.0, state_id).await;
            edit_or_send_with_keyboard(
                &bot,
                chat_id,
                query.message.as_ref().map(|m| m.id()),
                result.unwrap_or_else(|err| pick_destination_error(ui_language(&config), &err)),
                None,
            )
            .await?;
            return Ok(());
        }
        Some(("watch", "list", payload)) => {
            let (mode, page) =
                parse_watch_list_payload(payload).unwrap_or((WatchListMode::Status, 0));
            let result = render_watch_panel(&config, &db, user_id, page, mode).await;
            match result {
                Ok((text, keyboard)) => {
                    let keyboard = (!keyboard.inline_keyboard.is_empty()).then_some(keyboard);
                    edit_or_send_with_keyboard(
                        &bot,
                        chat_id,
                        query.message.as_ref().map(|m| m.id()),
                        text,
                        keyboard,
                    )
                    .await?;
                }
                Err(err) => {
                    edit_or_send_with_keyboard(
                        &bot,
                        chat_id,
                        query.message.as_ref().map(|m| m.id()),
                        load_watch_list_error(ui_language(&config), &err),
                        None,
                    )
                    .await?;
                }
            }
            return Ok(());
        }
        Some(("watch", "status", watch_id)) => {
            let result = watch_status_panel(&config, &db, user_id, watch_id).await;
            match result {
                Ok((text, keyboard)) => {
                    let keyboard = (!keyboard.inline_keyboard.is_empty()).then_some(keyboard);
                    edit_or_send_with_keyboard(
                        &bot,
                        chat_id,
                        query.message.as_ref().map(|m| m.id()),
                        text,
                        keyboard,
                    )
                    .await?;
                }
                Err(err) => {
                    edit_or_send_with_keyboard(
                        &bot,
                        chat_id,
                        query.message.as_ref().map(|m| m.id()),
                        err.to_string(),
                        None,
                    )
                    .await?;
                }
            }
            return Ok(());
        }
        Some(("watch", "pause", watch_id)) => {
            edit_watch_after_action(
                &bot,
                chat_id,
                query.message.as_ref().map(|m| m.id()),
                &config,
                &db,
                user_id,
                watch_id,
                pause_watch(&db, user_id, watch_id, ui_language(&config)).await,
            )
            .await?;
            return Ok(());
        }
        Some(("watch", "resume", watch_id)) => {
            let result = resume_watch(&config, &db, user_id, watch_id, ui_language(&config)).await;
            edit_watch_after_action(
                &bot,
                chat_id,
                query.message.as_ref().map(|m| m.id()),
                &config,
                &db,
                user_id,
                watch_id,
                result,
            )
            .await?;
            return Ok(());
        }
        Some(("watch", "unwatch", watch_id)) => {
            let lang = ui_language(&config);
            edit_or_send_with_keyboard(
                &bot,
                chat_id,
                query.message.as_ref().map(|m| m.id()),
                format!(
                    "{}\n━━━━━━━━━━━━\nWatch: {}\n\n{}",
                    lang.text(T::ConfirmUnwatchTitle),
                    short_id(watch_id),
                    lang.text(T::ConfirmUnwatchBody)
                ),
                Some(keyboards::watch_unwatch_confirm_keyboard(watch_id, lang)),
            )
            .await?;
            return Ok(());
        }
        Some(("watch", "unwatch_confirm", watch_id)) => {
            edit_watch_after_action(
                &bot,
                chat_id,
                query.message.as_ref().map(|m| m.id()),
                &config,
                &db,
                user_id,
                watch_id,
                stop_watch(&db, user_id, watch_id, ui_language(&config)).await,
            )
            .await?;
            return Ok(());
        }
        Some(("watch", "pol", payload)) => {
            let result =
                set_watch_policy_callback(&db, user_id, payload, ui_language(&config)).await;
            let watch_id = payload.split_once(':').map_or(payload, |(id, _)| id);
            edit_watch_after_action(
                &bot,
                chat_id,
                query.message.as_ref().map(|m| m.id()),
                &config,
                &db,
                user_id,
                watch_id,
                result,
            )
            .await?;
            return Ok(());
        }
        _ => unknown_action(ui_language(&config)).to_string(),
    };

    bot.send_message(chat_id, text).await?;
    Ok(())
}

fn parse_callback_action(data: &str) -> Option<(&str, &str, &str)> {
    let mut parts = data.splitn(3, ':');
    Some((parts.next()?, parts.next()?, parts.next()?))
}

async fn send_reply_prompt(
    bot: &Bot,
    chat_id: ChatId,
    text: impl Into<String>,
    prompt: ReplyPrompt,
    lang: keyboards::UiLanguage,
) -> ResponseResult<()> {
    bot.send_message(chat_id, text)
        .reply_markup(
            ForceReply::new()
                .input_field_placeholder(Some(prompt.placeholder(lang).to_string()))
                .selective(),
        )
        .await?;
    Ok(())
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
        Command::Start | Command::Menu => {
            let text = render_home_dashboard(&config, &db, user_id, ui_language(&config))
                .await
                .unwrap_or_else(|err| load_home_error(ui_language(&config), &err));
            bot.send_message(msg.chat.id, text)
                .reply_markup(keyboards::main_menu_keyboard(
                    config.watch.enabled,
                    ui_language(&config),
                ))
                .await?;
        }
        Command::Help => {
            let text = render_help_text(ui_language(&config));
            bot.send_message(msg.chat.id, text)
                .reply_markup(keyboards::main_menu_keyboard(
                    config.watch.enabled,
                    ui_language(&config),
                ))
                .await?;
        }
        Command::Preview => {
            spawn_preview_dashboard(bot, msg.chat.id, db, user_id, ui_language(&config)).await?;
        }
        Command::Connect => {
            let connect_text = match ui_language(&config) {
                keyboards::UiLanguage::Vi => {
                    "Google Auth chạy local-first.\n\
                     \n\
                     Chạy lệnh sau trên máy đang chạy bot:\n\
                     \n\
                       502drive auth login\n\
                     \n\
                     Sau đó dùng /account để kiểm tra kết nối."
                }
                keyboards::UiLanguage::En => {
                    "Google Auth runs local-first.\n\
                     \n\
                     Run this command on the machine running the bot:\n\
                     \n\
                       502drive auth login\n\
                     \n\
                     Then use /account to verify the connection."
                }
            };
            bot.send_message(msg.chat.id, connect_text).await?;
        }
        Command::Account => {
            let text = account_summary(&config, &db, ui_language(&config))
                .await
                .unwrap_or_else(|err| match ui_language(&config) {
                    keyboards::UiLanguage::Vi => format!("Lỗi đọc trạng thái tài khoản: {err}"),
                    keyboards::UiLanguage::En => format!("Error reading account status: {err}"),
                });
            bot.send_message(msg.chat.id, text)
                .reply_markup(keyboards::account_keyboard(ui_language(&config)))
                .await?;
        }
        Command::Disconnect => {
            let text = disconnect_google(&config, &db, user_id).await;
            bot.send_message(msg.chat.id, text.unwrap_or_else(|err| err.to_string()))
                .await?;
        }
        Command::Clone(input) => {
            if input.trim().is_empty() {
                send_reply_prompt(
                    &bot,
                    msg.chat.id,
                    clone_prompt(ui_language(&config)),
                    ReplyPrompt::Clone,
                    ui_language(&config),
                )
                .await?;
                return Ok(());
            }
            handle_clone_request(bot, msg.chat.id, config, db, user_id, input).await?;
        }
        Command::CloneHere(input) => {
            if input.trim().is_empty() {
                send_reply_prompt(
                    &bot,
                    msg.chat.id,
                    clone_here_prompt(ui_language(&config)),
                    ReplyPrompt::CloneHere,
                    ui_language(&config),
                )
                .await?;
                return Ok(());
            }
            spawn_clone_now(bot, msg.chat.id, config, db, user_id, input).await?;
        }
        Command::Whoami => {
            let lang = ui_language(&config);
            let role = repo::authorized_user(&db, user_id)
                .await
                .ok()
                .flatten()
                .map(|u| u.role)
                .unwrap_or_else(|| match lang {
                    keyboards::UiLanguage::Vi => "không rõ".to_string(),
                    keyboards::UiLanguage::En => "unknown".to_string(),
                });
            let whoami_text = match lang {
                keyboards::UiLanguage::Vi => format!(
                    "NGƯỜI DÙNG\n\
                     ━━━━━━━━━\n\
                     Telegram ID : {user_id}\n\
                     Quyền       : {role}"
                ),
                keyboards::UiLanguage::En => format!(
                    "USER\n\
                     ━━━━━━━━━\n\
                     Telegram ID : {user_id}\n\
                     Role        : {role}"
                ),
            };
            bot.send_message(msg.chat.id, whoami_text).await?;
        }
        Command::Destination => {
            spawn_destination_panel(bot, msg.chat.id, db, user_id, ui_language(&config)).await?;
        }
        Command::ClearDestination => {
            let lang = ui_language(&config);
            let text = match repo::clear_default_destination(&db, "default").await {
                Ok(0) => match lang {
                    keyboards::UiLanguage::Vi => {
                        "Chưa có thư mục đích mặc định để xoá.".to_string()
                    }
                    keyboards::UiLanguage::En => {
                        "No default destination folder to clear.".to_string()
                    }
                },
                Ok(_) => match lang {
                    keyboards::UiLanguage::Vi => "Đã xoá thư mục đích mặc định.".to_string(),
                    keyboards::UiLanguage::En => "Cleared default destination folder.".to_string(),
                },
                Err(err) => match lang {
                    keyboards::UiLanguage::Vi => format!("Lỗi xoá thư mục đích: {err}"),
                    keyboards::UiLanguage::En => format!("Error clearing destination: {err}"),
                },
            };
            bot.send_message(msg.chat.id, text).await?;
        }
        Command::SetDestination(input) => {
            if input.trim().is_empty() {
                send_reply_prompt(
                    &bot,
                    msg.chat.id,
                    set_destination_prompt(ui_language(&config)),
                    ReplyPrompt::SetDestination,
                    ui_language(&config),
                )
                .await?;
                return Ok(());
            }
            let text = set_destination(&config, &db, &input).await;
            bot.send_message(
                msg.chat.id,
                text.unwrap_or_else(|err| format_error_for_user(&err, ui_language(&config))),
            )
            .await?;
        }
        Command::Jobs => match render_jobs_panel(&db, user_id, ui_language(&config)).await {
            Ok((text, keyboard)) => {
                bot.send_message(msg.chat.id, text)
                    .reply_markup(keyboard)
                    .await?;
            }
            Err(err) => {
                bot.send_message(msg.chat.id, load_jobs_error(ui_language(&config), &err))
                    .reply_markup(keyboards::back_home_keyboard(ui_language(&config)))
                    .await?;
            }
        },
        Command::Status(job_id) => {
            if job_id.trim().is_empty() {
                match render_jobs_panel(&db, user_id, ui_language(&config)).await {
                    Ok((text, keyboard)) => {
                        bot.send_message(msg.chat.id, text)
                            .reply_markup(keyboard)
                            .await?;
                    }
                    Err(err) => {
                        bot.send_message(msg.chat.id, load_jobs_error(ui_language(&config), &err))
                            .reply_markup(keyboards::back_home_keyboard(ui_language(&config)))
                            .await?;
                    }
                }
                return Ok(());
            }
            match job_status_panel(&db, user_id, &job_id, ui_language(&config)).await {
                Ok((text, keyboard)) => {
                    bot.send_message(msg.chat.id, text)
                        .reply_markup(keyboard)
                        .await?;
                }
                Err(err) => {
                    bot.send_message(msg.chat.id, err.to_string()).await?;
                }
            }
        }
        Command::Pause(job_id) => {
            if job_id.trim().is_empty() {
                match render_jobs_panel(&db, user_id, ui_language(&config)).await {
                    Ok((text, keyboard)) => {
                        bot.send_message(msg.chat.id, text)
                            .reply_markup(keyboard)
                            .await?;
                    }
                    Err(err) => {
                        bot.send_message(msg.chat.id, load_jobs_error(ui_language(&config), &err))
                            .reply_markup(keyboards::back_home_keyboard(ui_language(&config)))
                            .await?;
                    }
                }
                return Ok(());
            }
            let text = pause_job(&db, user_id, &job_id, ui_language(&config)).await;
            bot.send_message(msg.chat.id, text.unwrap_or_else(|err| err.to_string()))
                .await?;
        }
        Command::Resume(job_id) => {
            if job_id.trim().is_empty() {
                match render_jobs_panel(&db, user_id, ui_language(&config)).await {
                    Ok((text, keyboard)) => {
                        bot.send_message(msg.chat.id, text)
                            .reply_markup(keyboard)
                            .await?;
                    }
                    Err(err) => {
                        bot.send_message(msg.chat.id, load_jobs_error(ui_language(&config), &err))
                            .reply_markup(keyboards::back_home_keyboard(ui_language(&config)))
                            .await?;
                    }
                }
                return Ok(());
            }
            let text = resume_job(&config, &db, user_id, &job_id, ui_language(&config)).await;
            bot.send_message(msg.chat.id, text.unwrap_or_else(|err| err.to_string()))
                .await?;
        }
        Command::Cancel(job_id) => {
            if job_id.trim().is_empty() {
                match render_jobs_panel(&db, user_id, ui_language(&config)).await {
                    Ok((text, keyboard)) => {
                        bot.send_message(msg.chat.id, text)
                            .reply_markup(keyboard)
                            .await?;
                    }
                    Err(err) => {
                        bot.send_message(msg.chat.id, load_jobs_error(ui_language(&config), &err))
                            .reply_markup(keyboards::back_home_keyboard(ui_language(&config)))
                            .await?;
                    }
                }
                return Ok(());
            }
            let text = cancel_job(&db, user_id, &job_id, ui_language(&config)).await;
            bot.send_message(msg.chat.id, text.unwrap_or_else(|err| err.to_string()))
                .await?;
        }
        Command::Retry(job_id) => {
            if job_id.trim().is_empty() {
                send_reply_prompt(
                    &bot,
                    msg.chat.id,
                    job_id_prompt("/retry", ui_language(&config)),
                    ReplyPrompt::Retry,
                    ui_language(&config),
                )
                .await?;
                return Ok(());
            }
            let text = retry_job(&config, &db, user_id, &job_id, ui_language(&config)).await;
            bot.send_message(msg.chat.id, text.unwrap_or_else(|err| err.to_string()))
                .await?;
        }
        Command::LastReport => {
            send_last_report(&bot, msg.chat.id, &config, &db, user_id).await?;
        }
        Command::Grant(input) => {
            if input.trim().is_empty() {
                send_reply_prompt(
                    &bot,
                    msg.chat.id,
                    user_id_prompt("/grant", ui_language(&config)),
                    ReplyPrompt::Grant,
                    ui_language(&config),
                )
                .await?;
                return Ok(());
            }
            let text = grant_user(&db, user_id, &input).await;
            bot.send_message(msg.chat.id, text.unwrap_or_else(|err| err.to_string()))
                .await?;
        }
        Command::Revoke(input) => {
            if input.trim().is_empty() {
                send_reply_prompt(
                    &bot,
                    msg.chat.id,
                    user_id_prompt("/revoke", ui_language(&config)),
                    ReplyPrompt::Revoke,
                    ui_language(&config),
                )
                .await?;
                return Ok(());
            }
            let text = revoke_user(&db, user_id, &input).await;
            bot.send_message(msg.chat.id, text.unwrap_or_else(|err| err.to_string()))
                .await?;
        }
        // ── Sync & Watch ──────────────────────────────────────────────────────
        Command::Sync(input) => {
            if input.trim().is_empty() {
                send_reply_prompt(
                    &bot,
                    msg.chat.id,
                    watch_prompt(ui_language(&config)),
                    ReplyPrompt::Watch,
                    ui_language(&config),
                )
                .await?;
                return Ok(());
            }
            let text = start_watch(
                &bot,
                &config,
                &db,
                msg.chat.id.0,
                user_id,
                &input,
                ui_language(&config),
            )
            .await;
            bot.send_message(msg.chat.id, text.unwrap_or_else(|err| err.to_string()))
                .await?;
        }
        Command::Watch(input) => {
            if input.trim().is_empty() {
                send_reply_prompt(
                    &bot,
                    msg.chat.id,
                    watch_prompt(ui_language(&config)),
                    ReplyPrompt::Watch,
                    ui_language(&config),
                )
                .await?;
                return Ok(());
            }
            let text = start_watch(
                &bot,
                &config,
                &db,
                msg.chat.id.0,
                user_id,
                &input,
                ui_language(&config),
            )
            .await;
            bot.send_message(msg.chat.id, text.unwrap_or_else(|err| err.to_string()))
                .await?;
        }
        Command::Watches => {
            spawn_watch_panel(
                bot,
                msg.chat.id,
                config,
                db,
                user_id,
                0,
                WatchListMode::Status,
            )
            .await?;
        }
        Command::WatchStatus(watch_id) => {
            if watch_id.trim().is_empty() {
                spawn_watch_panel(
                    bot,
                    msg.chat.id,
                    config,
                    db,
                    user_id,
                    0,
                    WatchListMode::Status,
                )
                .await?;
                return Ok(());
            }
            match watch_status_panel(&config, &db, user_id, &watch_id).await {
                Ok((text, keyboard)) => {
                    if keyboard.inline_keyboard.is_empty() {
                        bot.send_message(msg.chat.id, text).await?;
                    } else {
                        bot.send_message(msg.chat.id, text)
                            .reply_markup(keyboard)
                            .await?;
                    }
                }
                Err(err) => {
                    bot.send_message(msg.chat.id, err.to_string()).await?;
                }
            }
        }
        Command::WatchPause(watch_id) => {
            if watch_id.trim().is_empty() {
                spawn_watch_panel(
                    bot,
                    msg.chat.id,
                    config,
                    db,
                    user_id,
                    0,
                    WatchListMode::Pause,
                )
                .await?;
                return Ok(());
            }
            let text = pause_watch(&db, user_id, &watch_id, ui_language(&config)).await;
            bot.send_message(msg.chat.id, text.unwrap_or_else(|err| err.to_string()))
                .await?;
        }
        Command::WatchResume(watch_id) => {
            if watch_id.trim().is_empty() {
                spawn_watch_panel(
                    bot,
                    msg.chat.id,
                    config,
                    db,
                    user_id,
                    0,
                    WatchListMode::Resume,
                )
                .await?;
                return Ok(());
            }
            let text = resume_watch(&config, &db, user_id, &watch_id, ui_language(&config)).await;
            bot.send_message(msg.chat.id, text.unwrap_or_else(|err| err.to_string()))
                .await?;
        }
        Command::WatchPolicy(input) => {
            if input.trim().is_empty() {
                spawn_watch_panel(
                    bot,
                    msg.chat.id,
                    config,
                    db,
                    user_id,
                    0,
                    WatchListMode::Status,
                )
                .await?;
                return Ok(());
            }
            let text = set_watch_policy(&db, user_id, &input, ui_language(&config)).await;
            bot.send_message(msg.chat.id, text.unwrap_or_else(|err| err.to_string()))
                .await?;
        }
        Command::WatchFilter(input) => {
            if input.trim().is_empty() {
                bot.send_message(msg.chat.id, watch_filter_usage_text(ui_language(&config)))
                    .await?;
                return Ok(());
            }
            let text = manage_watch_filter(&db, user_id, &input, ui_language(&config)).await;
            bot.send_message(msg.chat.id, text.unwrap_or_else(|err| err.to_string()))
                .await?;
        }
        Command::Unwatch(watch_id) => {
            if watch_id.trim().is_empty() {
                spawn_watch_panel(
                    bot,
                    msg.chat.id,
                    config,
                    db,
                    user_id,
                    0,
                    WatchListMode::Unwatch,
                )
                .await?;
                return Ok(());
            }
            let text = stop_watch(&db, user_id, &watch_id, ui_language(&config)).await;
            bot.send_message(msg.chat.id, text.unwrap_or_else(|err| err.to_string()))
                .await?;
        }
    }
    Ok(())
}

// ── Job listing ──────────────────────────────────────────────────────────────

async fn account_summary(
    config: &AppConfig,
    db: &Database,
    lang: keyboards::UiLanguage,
) -> anyhow::Result<String> {
    let status = repo::account_status(db)
        .await?
        .unwrap_or_else(|| "chưa kết nối".to_string());
    let destination = repo::default_destination_profile(db, "default").await?;

    let mut lines = vec![
        account_title(lang).to_string(),
        "━━━━━━━━━━━━━━".to_string(),
    ];
    push_field(
        &mut lines,
        account_status_field(lang),
        account_status_label(lang, &status),
    );

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
                push_field(&mut lines, account_name_field(lang), &name);
            }
        }
    } else {
        lines.push(String::new());
        lines.push(account_login_hint(lang).to_string());
        lines.push("  502drive auth login".to_string());
    }

    lines.push(String::new());
    lines.push(account_destination_title(lang).to_string());
    lines.push("━━━━━━━━━━━━".to_string());
    match destination {
        Some(dest) => {
            push_field(&mut lines, account_name_field(lang), &dest.label);
            push_field(
                &mut lines,
                account_parent_id_field(lang),
                &dest.destination_parent_id,
            );
            if let Some(drive_id) = dest.destination_drive_id {
                push_field(&mut lines, "Drive", &format!("Shared Drive ({drive_id})"));
            }
        }
        None => {
            lines.push(account_destination_missing(lang).to_string());
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
    lang: keyboards::UiLanguage,
) -> ResponseResult<()> {
    let text = render_preview_dashboard(&db, telegram_user_id, lang)
        .await
        .unwrap_or_else(|err| preview_load_error(lang, &err));
    let message = bot.send_message(chat_id, text).await?;
    tokio::spawn(async move {
        let mut last_text = String::new();
        for _ in 0..15 {
            tokio::time::sleep(Duration::from_secs(2)).await;
            let text = match render_preview_dashboard(&db, telegram_user_id, lang).await {
                Ok(t) => t,
                Err(err) => preview_update_error(lang, &err),
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

// ── Clone flow ───────────────────────────────────────────────────────────────

async fn send_clone_outcome(
    bot: &Bot,
    chat_id: ChatId,
    db: &Database,
    telegram_user_id: i64,
    lang: keyboards::UiLanguage,
    outcome: CloneOutcome,
    progress_message_id: Option<MessageId>,
) -> ResponseResult<()> {
    let message = render_clone_outcome(db, telegram_user_id, lang, &outcome)
        .await
        .unwrap_or_else(|_| outcome.message.clone());
    send_or_edit_clone_message(bot, chat_id, progress_message_id, message).await?;
    if let Some(paths) = outcome.report_paths {
        bot.send_document(chat_id, InputFile::file(paths.json))
            .await?;
        bot.send_document(chat_id, InputFile::file(paths.csv))
            .await?;
    }
    Ok(())
}

async fn render_clone_outcome(
    db: &Database,
    telegram_user_id: i64,
    lang: keyboards::UiLanguage,
    outcome: &CloneOutcome,
) -> anyhow::Result<String> {
    if let Some(job) = repo::job_detail_for_user(db, telegram_user_id, &outcome.job_id).await? {
        return Ok(render_clone_outcome_from_job(lang, &job));
    }
    Ok(outcome.message.clone())
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
            bot.send_message(chat_id, report_none_text(ui_language(config)))
                .await?;
            return Ok(());
        }
        Err(err) => {
            bot.send_message(chat_id, report_latest_read_error(ui_language(config), &err))
                .await?;
            return Ok(());
        }
    };

    let paths = match report::write_job_reports(db, &config.storage.report_dir, &job.id).await {
        Ok(paths) => paths,
        Err(err) => {
            bot.send_message(chat_id, report_write_error(ui_language(config), &err))
                .await?;
            return Ok(());
        }
    };

    bot.send_message(
        chat_id,
        render_report_summary(
            ui_language(config),
            T::ReportLatestTitle,
            &job.id,
            &job.status,
            job.total_discovered,
            job.completed_items,
            job.failed_items,
            job.skipped_items,
        ),
    )
    .await?;
    bot.send_document(chat_id, InputFile::file(paths.json))
        .await?;
    bot.send_document(chat_id, InputFile::file(paths.csv))
        .await?;
    Ok(())
}

async fn send_report_for_job_id(
    bot: &Bot,
    chat_id: ChatId,
    config: &AppConfig,
    db: &Database,
    telegram_user_id: i64,
    job_id: &str,
) -> ResponseResult<()> {
    let job = match repo::job_detail_for_user(db, telegram_user_id, job_id).await {
        Ok(Some(job)) => job,
        Ok(None) => {
            bot.send_message(chat_id, report_job_not_found(ui_language(config)))
                .await?;
            return Ok(());
        }
        Err(err) => {
            bot.send_message(chat_id, report_job_read_error(ui_language(config), &err))
                .await?;
            return Ok(());
        }
    };
    let paths = match report::write_job_reports(db, &config.storage.report_dir, &job.id).await {
        Ok(paths) => paths,
        Err(err) => {
            bot.send_message(chat_id, report_write_error(ui_language(config), &err))
                .await?;
            return Ok(());
        }
    };
    bot.send_message(
        chat_id,
        render_report_summary(
            ui_language(config),
            T::ReportJobTitle,
            &job.id,
            &job.status,
            job.total_discovered,
            job.completed_items,
            job.failed_items,
            job.skipped_items,
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

async fn edit_or_send_with_keyboard(
    bot: &Bot,
    chat_id: ChatId,
    message_id: Option<MessageId>,
    text: String,
    keyboard: Option<InlineKeyboardMarkup>,
) -> ResponseResult<()> {
    if let Some(message_id) = message_id {
        let edited = if let Some(keyboard) = keyboard.clone() {
            bot.edit_message_text(chat_id, message_id, text.clone())
                .reply_markup(keyboard)
                .await
        } else {
            bot.edit_message_text(chat_id, message_id, text.clone())
                .await
        };
        if edited.is_ok() {
            return Ok(());
        }
    }
    if let Some(keyboard) = keyboard {
        bot.send_message(chat_id, text)
            .reply_markup(keyboard)
            .await?;
    } else {
        bot.send_message(chat_id, text).await?;
    }
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
            bot.send_message(chat_id, invalid_drive_link_text(ui_language(&config), &err))
                .await?;
            return Ok(());
        }
    };

    let lang = ui_language(&config);
    let progress_message = bot
        .send_message(
            chat_id,
            format!(
                "{}: {}\n{}",
                progress_status_field(lang),
                progress_queued_label(lang),
                render_progress(lang, None, 0, 0)
            ),
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
        lang,
    );

    bot.send_message(chat_id, clone_job_accepted_text(lang))
        .await?;

    let outcome_db = db.clone();
    let outcome_lang = ui_language(&config);
    tokio::spawn(async move {
        let outcome = start_clone_reference(
            &config,
            &db,
            chat_id.0,
            telegram_user_id,
            source,
            Some(progress_message_id.0),
            outcome_lang,
        )
        .await;
        match outcome {
            Ok(outcome) => {
                let _ = progress_done_tx.send(());
                if let Err(err) = send_clone_outcome(
                    &bot,
                    chat_id,
                    &outcome_db,
                    telegram_user_id,
                    outcome_lang,
                    outcome,
                    Some(progress_message_id),
                )
                .await
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
                    clone_run_error(outcome_lang, &err),
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

async fn handle_folder_link_detected(
    bot: Bot,
    chat_id: ChatId,
    config: AppConfig,
    db: Database,
    telegram_user_id: i64,
    input: String,
    reference: crate::drive::links::DriveReference,
) -> ResponseResult<()> {
    let lang = ui_language(&config);
    let default_dest = repo::default_destination_profile(&db, "default")
        .await
        .ok()
        .flatten();

    let dest_info = match &default_dest {
        Some(p) => format!("{} (ID: {})", p.label, p.destination_parent_id),
        None => match lang {
            keyboards::UiLanguage::Vi => {
                "Chưa đặt (dùng /set_destination hoặc nút bên dưới)".to_string()
            }
            keyboards::UiLanguage::En => {
                "Not set (use /set_destination or button below)".to_string()
            }
        },
    };

    let text = match lang {
        keyboards::UiLanguage::Vi => format!(
            "Phát hiện liên kết thư mục Google Drive:
• Nguồn: {}
• Thư mục đích: {}

Chọn tác vụ bạn muốn thực hiện:",
            reference.file_id, dest_info
        ),
        keyboards::UiLanguage::En => format!(
            "Detected Google Drive folder link:
• Source: {}
• Destination: {}

Select an action:",
            reference.file_id, dest_info
        ),
    };

    let _ = repo::delete_expired_callback_states(&db).await;
    match repo::create_callback_state(
        &db,
        repo::NewCallbackState {
            telegram_user_id,
            chat_id: chat_id.0,
            action: "smart_link".to_string(),
            payload: input,
            ttl_ms: CALLBACK_STATE_TTL_MS,
        },
    )
    .await
    {
        Ok(state_id) => {
            bot.send_message(chat_id, text)
                .reply_markup(keyboards::smart_link_action_keyboard(&state_id, lang))
                .await?;
        }
        Err(err) => {
            bot.send_message(chat_id, err.to_string()).await?;
        }
    }
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
        .send_message(chat_id, clone_loading_text(ui_language(&config)))
        .await?;

    match inspect_clone_source(&config, &db, &input, ui_language(&config)).await {
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
                            .reply_markup(keyboards::confirm_clone_keyboard(
                                &state_id,
                                ui_language(&config),
                            ))
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
                clone_inspect_error(ui_language(&config), &err),
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
    lang: keyboards::UiLanguage,
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
                            "{}: {}\n{}: {}\n{}",
                            progress_status_field(lang),
                            progress_preparing_label(lang),
                            progress_elapsed_field(lang),
                            format_duration_secs(elapsed_secs),
                            render_progress(lang, None, 0, 0),
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

            let text = render_job_progress(&detail, elapsed_secs, lang);
            if text == last_text {
                continue;
            }
            last_text = text.clone();

            let paused = detail.status == "paused";
            let edit_result = if is_terminal_status(&detail.status) {
                bot.edit_message_text(chat_id, message_id, text).await
            } else {
                bot.edit_message_text(chat_id, message_id, text)
                    .reply_markup(keyboards::job_control_keyboard(&detail.id, paused, lang))
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

// ── Job detail / control ─────────────────────────────────────────────────────

async fn show_job_status(
    db: &Database,
    telegram_user_id: i64,
    job_id: &str,
    lang: keyboards::UiLanguage,
) -> anyhow::Result<String> {
    let job = resolve_job_for_user(db, telegram_user_id, job_id, lang).await?;
    Ok(render_job_detail(&job, lang))
}

async fn job_status_panel(
    db: &Database,
    telegram_user_id: i64,
    job_id: &str,
    lang: keyboards::UiLanguage,
) -> anyhow::Result<(String, InlineKeyboardMarkup)> {
    let job = resolve_job_for_user(db, telegram_user_id, job_id, lang).await?;
    let text = render_job_detail(&job, lang);
    Ok((
        text,
        keyboards::job_detail_keyboard(&job.id, &job.status, lang),
    ))
}

async fn edit_job_after_action(
    bot: &Bot,
    chat_id: ChatId,
    message_id: Option<MessageId>,
    db: &Database,
    telegram_user_id: i64,
    job_id: &str,
    lang: keyboards::UiLanguage,
    action_result: anyhow::Result<String>,
) -> ResponseResult<()> {
    let action_text = action_result.unwrap_or_else(|err| err.to_string());
    match job_status_panel(db, telegram_user_id, job_id, lang).await {
        Ok((detail, keyboard)) => {
            edit_or_send_with_keyboard(
                bot,
                chat_id,
                message_id,
                format!("{action_text}\n\n{detail}"),
                Some(keyboard),
            )
            .await?;
        }
        Err(_) => {
            edit_or_send_with_keyboard(
                bot,
                chat_id,
                message_id,
                action_text,
                Some(keyboards::back_home_keyboard(lang)),
            )
            .await?;
        }
    }
    Ok(())
}

async fn resolve_job_for_user(
    db: &Database,
    telegram_user_id: i64,
    job_id: &str,
    lang: keyboards::UiLanguage,
) -> anyhow::Result<repo::JobDetail> {
    // The resolution logic (empty → latest active, full id, 8-char prefix)
    // lives in the shared services layer; this wrapper only maps error
    // variants to localized text.
    match JobService::resolve_user_input(db, telegram_user_id, job_id).await {
        Ok(job) => Ok(job),
        Err(JobResolveError::Db(err)) => Err(err),
        Err(JobResolveError::NoActiveJob) => anyhow::bail!(job_no_active_text(lang)),
        Err(JobResolveError::ActiveJobMissing) => {
            anyhow::bail!(job_active_missing_text(lang))
        }
        Err(JobResolveError::NotFound) => anyhow::bail!(job_not_found_text(lang)),
        Err(JobResolveError::Ambiguous) => anyhow::bail!(job_prefix_ambiguous_text(lang)),
    }
}

async fn resolve_job_id_for_user(
    db: &Database,
    telegram_user_id: i64,
    job_id: &str,
    lang: keyboards::UiLanguage,
) -> anyhow::Result<String> {
    Ok(resolve_job_for_user(db, telegram_user_id, job_id, lang)
        .await?
        .id)
}

async fn pause_job(
    db: &Database,
    telegram_user_id: i64,
    job_id: &str,
    lang: keyboards::UiLanguage,
) -> anyhow::Result<String> {
    let job_id = resolve_job_id_for_user(db, telegram_user_id, job_id, lang).await?;
    if repo::pause_job_for_user(db, telegram_user_id, &job_id).await? {
        Ok(job_pause_requested_text(lang, &job_id))
    } else {
        Ok(job_pause_failed_text(lang).to_string())
    }
}

async fn resume_job(
    config: &AppConfig,
    db: &Database,
    telegram_user_id: i64,
    job_id: &str,
    lang: keyboards::UiLanguage,
) -> anyhow::Result<String> {
    let job_id = resolve_job_id_for_user(db, telegram_user_id, job_id, lang).await?;
    if repo::resume_job_for_user(db, telegram_user_id, &job_id).await? {
        let _resume_worker = recovery::spawn_startup_resume_worker(
            config.clone(),
            db.clone(),
            DriveClient::with_timeout(Duration::from_secs(config.engine.request_timeout_seconds)),
        );
        Ok(job_resume_requested_text(lang, &job_id))
    } else {
        Ok(job_resume_failed_text(lang).to_string())
    }
}

async fn cancel_job(
    db: &Database,
    telegram_user_id: i64,
    job_id: &str,
    lang: keyboards::UiLanguage,
) -> anyhow::Result<String> {
    let job_id = resolve_job_id_for_user(db, telegram_user_id, job_id, lang).await?;
    if repo::cancel_job_for_user(db, telegram_user_id, &job_id).await? {
        Ok(job_cancel_requested_text(lang, &job_id))
    } else {
        Ok(job_cancel_failed_text(lang).to_string())
    }
}

async fn retry_job(
    config: &AppConfig,
    db: &Database,
    telegram_user_id: i64,
    job_id: &str,
    lang: keyboards::UiLanguage,
) -> anyhow::Result<String> {
    let job_id = resolve_job_id_for_user(db, telegram_user_id, job_id, lang).await?;
    if let Some(summary) = repo::retry_failed_job_for_user(db, telegram_user_id, &job_id).await? {
        let _resume_worker = recovery::spawn_startup_resume_worker(
            config.clone(),
            db.clone(),
            DriveClient::with_timeout(Duration::from_secs(config.engine.request_timeout_seconds)),
        );
        Ok(job_retry_requested_text(lang, &job_id, &summary))
    } else {
        Ok(job_retry_failed_text(lang).to_string())
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
async fn spawn_destination_panel(
    bot: Bot,
    chat_id: ChatId,
    db: Database,
    telegram_user_id: i64,
    lang: keyboards::UiLanguage,
) -> ResponseResult<()> {
    let (text, keyboard) = render_destination_panel(&db, telegram_user_id, chat_id.0, lang).await;
    if let Some(keyboard) = keyboard {
        bot.send_message(chat_id, text)
            .reply_markup(keyboard)
            .await?;
    } else {
        bot.send_message(chat_id, text).await?;
    }
    Ok(())
}

async fn render_destination_panel(
    db: &Database,
    telegram_user_id: i64,
    chat_id: i64,
    lang: keyboards::UiLanguage,
) -> (String, Option<InlineKeyboardMarkup>) {
    let profiles = repo::list_recent_destinations(db, "default", 5)
        .await
        .unwrap_or_default();
    let text = render_destination_list(&profiles, lang);
    let my_drive_state = create_destination_browser_state(
        db,
        telegram_user_id,
        chat_id,
        DestinationBrowseTarget::root(),
    )
    .await
    .ok();
    let shared_drives_state = create_destination_browser_state(
        db,
        telegram_user_id,
        chat_id,
        DestinationBrowseTarget::shared_drives(),
    )
    .await
    .ok();

    if let (Some(my_drive_state), Some(shared_drives_state)) = (my_drive_state, shared_drives_state)
    {
        (
            text,
            Some(keyboards::destination_panel_keyboard(
                &profiles,
                &my_drive_state,
                &shared_drives_state,
                lang,
            )),
        )
    } else {
        (text, Some(keyboards::back_home_keyboard(lang)))
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct DestinationBrowseTarget {
    file_id: String,
    #[serde(default)]
    resource_key: Option<String>,
    #[serde(default)]
    page_token: Option<String>,
    #[serde(default)]
    drive_id: Option<String>,
    #[serde(default)]
    shared_drives: bool,
}

impl DestinationBrowseTarget {
    fn root() -> Self {
        Self {
            file_id: "root".to_string(),
            resource_key: None,
            page_token: None,
            drive_id: None,
            shared_drives: false,
        }
    }

    fn shared_drives() -> Self {
        Self {
            file_id: String::new(),
            resource_key: None,
            page_token: None,
            drive_id: None,
            shared_drives: true,
        }
    }

    fn reference(&self) -> DriveReference {
        DriveReference {
            file_id: self.file_id.clone(),
            resource_key: self.resource_key.clone(),
            hinted_kind: None,
        }
    }
}

async fn create_destination_browser_state(
    db: &Database,
    telegram_user_id: i64,
    chat_id: i64,
    target: DestinationBrowseTarget,
) -> anyhow::Result<String> {
    repo::create_callback_state(
        db,
        repo::NewCallbackState {
            telegram_user_id,
            chat_id,
            action: "destination_browse".to_string(),
            payload: serde_json::to_string(&target)?,
            ttl_ms: CALLBACK_STATE_TTL_MS,
        },
    )
    .await
}

async fn consume_destination_browser_state(
    db: &Database,
    telegram_user_id: i64,
    chat_id: i64,
    state_id: &str,
) -> anyhow::Result<DestinationBrowseTarget> {
    let Some(payload) = repo::consume_callback_state(
        db,
        state_id,
        telegram_user_id,
        chat_id,
        "destination_browse",
    )
    .await?
    else {
        anyhow::bail!("Phiên duyệt thư mục đã hết hạn. Mở lại bằng /destination.");
    };
    Ok(serde_json::from_str(&payload)?)
}

async fn open_destination_browser(
    config: &AppConfig,
    db: &Database,
    telegram_user_id: i64,
    chat_id: i64,
    state_id: &str,
) -> anyhow::Result<(String, InlineKeyboardMarkup)> {
    let target = consume_destination_browser_state(db, telegram_user_id, chat_id, state_id).await?;
    let token_manager = TokenManager::new(config.clone(), db.clone());
    let access_token = token_manager.access_token("default").await?;
    let drive =
        DriveClient::with_timeout(Duration::from_secs(config.engine.request_timeout_seconds));
    if target.shared_drives {
        return open_shared_drives_browser(
            &drive,
            access_token.as_str(),
            db,
            telegram_user_id,
            chat_id,
            target.page_token.as_deref(),
            ui_language(config),
        )
        .await;
    }
    let folder = drive
        .get_reference(access_token.as_str(), &target.reference())
        .await?;
    if !folder.is_folder() {
        anyhow::bail!("Mục này không phải thư mục Drive.");
    }

    let page = drive
        .list_child_folders_page_size(
            access_token.as_str(),
            &folder.id,
            target
                .resource_key
                .as_deref()
                .or(folder.resource_key.as_deref()),
            target.page_token.as_deref(),
            DESTINATION_BROWSER_LIMIT,
            target.drive_id.as_deref().or(folder.drive_id.as_deref()),
        )
        .await?;
    let mut folders: Vec<DriveFile> = page
        .files
        .into_iter()
        .filter(|file| file.is_folder())
        .collect();
    folders.sort_by_key(|file| file.name.to_lowercase());
    // ponytail: page through Drive's native page token; add search if huge folders need it.
    let has_next = page.next_page_token.is_some();

    let can_pick = folder
        .capabilities
        .as_ref()
        .and_then(|c| c.can_add_children)
        == Some(true);
    let pick_state = if can_pick {
        Some(
            create_destination_browser_state(
                db,
                telegram_user_id,
                chat_id,
                DestinationBrowseTarget {
                    file_id: folder.id.clone(),
                    resource_key: target.resource_key.clone().or(folder.resource_key.clone()),
                    page_token: None,
                    drive_id: target.drive_id.clone().or(folder.drive_id.clone()),
                    shared_drives: false,
                },
            )
            .await?,
        )
    } else {
        None
    };
    let parent_state = if let Some(parent_id) = folder.parents.first() {
        Some(
            create_destination_browser_state(
                db,
                telegram_user_id,
                chat_id,
                DestinationBrowseTarget {
                    file_id: parent_id.clone(),
                    resource_key: None,
                    page_token: None,
                    drive_id: target.drive_id.clone().or(folder.drive_id.clone()),
                    shared_drives: false,
                },
            )
            .await?,
        )
    } else {
        None
    };
    let mut child_states = Vec::with_capacity(folders.len());
    for child in folders {
        let state_id = create_destination_browser_state(
            db,
            telegram_user_id,
            chat_id,
            DestinationBrowseTarget {
                file_id: child.id.clone(),
                resource_key: child.resource_key.clone(),
                page_token: None,
                drive_id: child.drive_id.clone().or_else(|| target.drive_id.clone()),
                shared_drives: false,
            },
        )
        .await?;
        child_states.push((child.name, state_id));
    }
    let next_state = if let Some(next_page_token) = page.next_page_token {
        Some(
            create_destination_browser_state(
                db,
                telegram_user_id,
                chat_id,
                DestinationBrowseTarget {
                    file_id: folder.id.clone(),
                    resource_key: target.resource_key.clone().or(folder.resource_key.clone()),
                    page_token: Some(next_page_token),
                    drive_id: target.drive_id.clone().or(folder.drive_id.clone()),
                    shared_drives: false,
                },
            )
            .await?,
        )
    } else {
        None
    };

    let mut lines = vec![
        destination_browser_title(ui_language(config)).to_string(),
        "━━━━━━━━━━━━━━".to_string(),
    ];
    push_field(
        &mut lines,
        destination_name_field(ui_language(config)),
        &folder.name,
    );
    push_field(
        &mut lines,
        destination_folder_id_field(ui_language(config)),
        &folder.id,
    );
    push_field(
        &mut lines,
        destination_writable_field(ui_language(config)),
        capability_text_lang(Some(can_pick), ui_language(config)),
    );
    if let Some(drive_id) = &folder.drive_id {
        push_field(&mut lines, "Drive", &format!("Shared Drive ({drive_id})"));
    } else {
        push_field(
            &mut lines,
            "Drive",
            destination_my_drive_label(ui_language(config)),
        );
    }
    lines.push(String::new());
    if child_states.is_empty() {
        lines.push(destination_no_child_folders(ui_language(config)).to_string());
    } else {
        lines.push(destination_child_count(
            ui_language(config),
            child_states.len(),
        ));
    }
    if has_next {
        lines.push(destination_folder_page_hint(ui_language(config)).to_string());
    }

    Ok((
        lines.join("\n"),
        keyboards::destination_browser_keyboard(
            pick_state.as_deref(),
            parent_state.as_deref(),
            next_state.as_deref(),
            &child_states,
            ui_language(config),
        ),
    ))
}

async fn open_shared_drives_browser(
    drive: &DriveClient,
    access_token: &str,
    db: &Database,
    telegram_user_id: i64,
    chat_id: i64,
    page_token: Option<&str>,
    lang: keyboards::UiLanguage,
) -> anyhow::Result<(String, InlineKeyboardMarkup)> {
    let page = drive
        .list_shared_drives_page_size(access_token, page_token, DESTINATION_BROWSER_LIMIT)
        .await?;
    let has_next = page.next_page_token.is_some();
    let mut drive_states = Vec::with_capacity(page.drives.len());
    for shared_drive in page.drives {
        let state_id = create_destination_browser_state(
            db,
            telegram_user_id,
            chat_id,
            DestinationBrowseTarget {
                file_id: shared_drive.id.clone(),
                resource_key: None,
                page_token: None,
                drive_id: Some(shared_drive.id),
                shared_drives: false,
            },
        )
        .await?;
        drive_states.push((shared_drive.name, state_id));
    }
    let next_state = if let Some(next_page_token) = page.next_page_token {
        Some(
            create_destination_browser_state(
                db,
                telegram_user_id,
                chat_id,
                DestinationBrowseTarget {
                    file_id: String::new(),
                    resource_key: None,
                    page_token: Some(next_page_token),
                    drive_id: None,
                    shared_drives: true,
                },
            )
            .await?,
        )
    } else {
        None
    };

    let mut lines = vec![
        destination_shared_drives_title(lang).to_string(),
        "━━━━━━━━━━━━━".to_string(),
    ];
    if drive_states.is_empty() {
        lines.push(destination_no_shared_drives(lang).to_string());
    } else {
        lines.push(destination_shared_drive_count(lang, drive_states.len()));
    }
    if has_next {
        lines.push(destination_drive_page_hint(lang).to_string());
    }

    Ok((
        lines.join("\n"),
        keyboards::destination_browser_keyboard(
            None,
            None,
            next_state.as_deref(),
            &drive_states,
            lang,
        ),
    ))
}

async fn pick_destination_from_browser(
    config: &AppConfig,
    db: &Database,
    telegram_user_id: i64,
    chat_id: i64,
    state_id: &str,
) -> anyhow::Result<String> {
    let target = consume_destination_browser_state(db, telegram_user_id, chat_id, state_id).await?;
    let token_manager = TokenManager::new(config.clone(), db.clone());
    let access_token = token_manager.access_token("default").await?;
    let drive =
        DriveClient::with_timeout(Duration::from_secs(config.engine.request_timeout_seconds));
    let folder = drive
        .get_reference(access_token.as_str(), &target.reference())
        .await?;
    save_destination_profile(db, &folder, target.resource_key).await
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
    save_destination_profile(db, &file, reference.resource_key).await
}

async fn save_destination_profile(
    db: &Database,
    file: &DriveFile,
    resource_key: Option<String>,
) -> anyhow::Result<String> {
    if file.mime_type != FOLDER_MIME_TYPE {
        anyhow::bail!("Thư mục đích phải là thư mục Google Drive");
    }
    if file.capabilities.as_ref().and_then(|c| c.can_add_children) != Some(true) {
        anyhow::bail!("Tài khoản Google hiện tại không có quyền ghi vào thư mục đích này");
    }
    repo::upsert_destination_profile(
        db,
        destination_profile_from_file(file, resource_key.clone()),
    )
    .await?;
    Ok(render_destination_saved(file, resource_key))
}

fn destination_profile_from_file(
    file: &DriveFile,
    resource_key: Option<String>,
) -> repo::NewDestinationProfile {
    repo::NewDestinationProfile {
        google_account_id: "default".to_string(),
        label: file.name.clone(),
        destination_parent_id: file.id.clone(),
        destination_drive_id: file.drive_id.clone(),
        destination_resource_key: resource_key.or(file.resource_key.clone()),
        is_default: true,
    }
}

// ── Clone source inspect ─────────────────────────────────────────────────────

async fn inspect_clone_source(
    config: &AppConfig,
    db: &Database,
    input: &str,
    lang: keyboards::UiLanguage,
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

    let item_type = clone_item_type(lang, source.is_folder());

    let mut lines = vec![
        clone_info_title(lang).to_string(),
        "━━━━━━━━━━━━━━".to_string(),
    ];
    push_field(
        &mut lines,
        "Google",
        account_email
            .as_deref()
            .unwrap_or(clone_missing_email(lang)),
    );
    push_field(&mut lines, clone_source_name_field(lang), &source.name);
    push_field(&mut lines, clone_source_id_field(lang), &source.id);
    push_field(&mut lines, clone_type_field(lang), item_type);
    push_field(&mut lines, "MIME", &source.mime_type);
    push_field(
        &mut lines,
        "Resource key",
        capability_text_lang(
            Some(reference.resource_key.is_some() || source.resource_key.is_some()),
            lang,
        ),
    );

    if let Some(size) = &source.size {
        let bytes: i64 = size.parse().unwrap_or(0);
        push_field(&mut lines, clone_size_field(lang), &human_bytes(bytes));
    }

    if let Some(drive_id) = &source.drive_id {
        push_field(
            &mut lines,
            clone_location_field(lang),
            &format!("Shared Drive ({drive_id})"),
        );
    } else {
        push_field(
            &mut lines,
            clone_location_field(lang),
            destination_my_drive_label(lang),
        );
    }
    if let Some(modified_time) = &source.modified_time {
        push_field(&mut lines, clone_modified_field(lang), modified_time);
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
    push_field(
        &mut lines,
        clone_readable_field(lang),
        capability_text_lang(can_read, lang),
    );
    if source.is_folder() {
        if caps.and_then(|c| c.can_list_children) == Some(false) {
            lines.push(clone_warn_cannot_list(lang).to_string());
        }
    } else if caps.and_then(|c| c.can_copy) == Some(false) {
        lines.push(clone_warn_cannot_copy(lang).to_string());
    }

    if source.copy_requires_writer_permission == Some(true) {
        lines.push(clone_warn_writer_required(lang).to_string());
    }

    if reference.resource_key.is_some() {
        lines.push(clone_resource_key_note(lang).to_string());
    }

    match build_clone_plan(
        &drive,
        access_token.as_str(),
        &source,
        reference.resource_key.as_deref(),
    )
    .await
    {
        Ok(plan) => {
            lines.push(String::new());
            lines.push(clone_plan_title(lang).to_string());
            lines.push("━━━━━━━━".to_string());
            lines.extend(plan.render_lines(lang));
        }
        Err(err) => {
            lines.push(String::new());
            lines.push(clone_plan_title(lang).to_string());
            lines.push("━━━━━━━━".to_string());
            lines.push(clone_plan_unavailable(lang).to_string());
            lines.push(format!(
                "{}: {}",
                clone_reason_field(lang),
                first_line(&format_error_for_user(&err, lang))
            ));
        }
    }

    if let Some(default_dest) = repo::default_destination_profile(db, "default").await? {
        let destination_preview = if source.is_folder() {
            format!("{}/{}", default_dest.label, source.name)
        } else {
            default_dest.label.clone()
        };
        lines.push(String::new());
        lines.push(clone_destination_title(lang).to_string());
        lines.push("━━━━━━".to_string());
        push_field(
            &mut lines,
            destination_name_field(lang),
            &destination_preview,
        );
        push_field(
            &mut lines,
            account_parent_id_field(lang),
            &default_dest.destination_parent_id,
        );
        push_field(
            &mut lines,
            "Resource key",
            capability_text_lang(Some(default_dest.destination_resource_key.is_some()), lang),
        );
        if let Some(drive_id) = &default_dest.destination_drive_id {
            push_field(&mut lines, "Drive", &format!("Shared Drive ({drive_id})"));
        }
        lines.push(String::new());
        lines.push(clone_confirm_hint(lang).to_string());
    } else {
        lines.push(String::new());
        lines.push(clone_missing_destination(lang).to_string());
    }

    Ok(lines.join("\n"))
}

fn first_line(text: &str) -> &str {
    text.lines().next().unwrap_or(text)
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

// ── Clone engine call ────────────────────────────────────────────────────────

async fn start_clone_reference(
    config: &AppConfig,
    db: &Database,
    chat_id: i64,
    telegram_user_id: i64,
    source: crate::drive::links::DriveReference,
    progress_message_id: Option<i32>,
    lang: keyboards::UiLanguage,
) -> anyhow::Result<CloneOutcome> {
    // Per-user concurrency guard.
    let active_count = repo::active_job_count_for_user(db, telegram_user_id).await?;
    if active_count >= config.engine.max_active_jobs_per_user as i64 {
        anyhow::bail!(clone_active_limit_text(lang, active_count));
    }
    let service = CloneService::new(
        config.clone(),
        db.clone(),
        DriveClient::with_timeout(Duration::from_secs(config.engine.request_timeout_seconds)),
    );
    service
        .start_one_shot(CloneRequest {
            chat_id,
            telegram_user_id,
            source,
            progress_message_id,
            name_override: None,
            destination_parent_id: None,
            duplicate_policy: None,
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
    lang: keyboards::UiLanguage,
) -> anyhow::Result<String> {
    ensure_watch_enabled(config.watch.enabled, lang)?;
    let parts: Vec<&str> = input.split_whitespace().collect();
    if parts.is_empty() {
        anyhow::bail!(watch_usage_text(lang));
    }
    let source_ref = parse_drive_reference(parts[0])?;
    let dest_ref = if parts.len() >= 2 {
        parse_drive_reference(parts[1])?
    } else {
        match repo::default_destination_profile(db, "default").await? {
            Some(profile) => parse_drive_reference(&profile.destination_parent_id)?,
            None => {
                let msg = match lang {
                    keyboards::UiLanguage::Vi => {
                        "Chưa cấu hình thư mục đích mặc định.\nVui lòng chỉ định: /sync <nguồn> <đích>\nHoặc cài đặt thư mục đích trước bằng lệnh /set_destination <link_đích>."
                    }
                    keyboards::UiLanguage::En => {
                        "No default destination configured.\nPlease provide: /sync <source> <destination>\nOr set a default destination first using /set_destination <dest_link>."
                    }
                };
                anyhow::bail!(msg);
            }
        }
    };

    let drive = crate::watch::service::drive_client_for(config);

    let created = crate::watch::service::create_watch(
        config,
        db,
        &drive,
        crate::watch::service::CreateWatchParams {
            source: source_ref,
            destination: dest_ref,
            telegram_user_id,
            chat_id,
            exclude_globs: Vec::new(),
        },
    )
    .await
    .map_err(|err| match err {
        crate::watch::service::CreateWatchError::SourceNotFolder => {
            anyhow::anyhow!(watch_source_must_be_folder(lang))
        }
        crate::watch::service::CreateWatchError::DestinationNotFolder => {
            anyhow::anyhow!(watch_destination_must_be_folder(lang))
        }
        crate::watch::service::CreateWatchError::DestinationNotWritable => {
            anyhow::anyhow!(watch_destination_not_writable(lang))
        }
        crate::watch::service::CreateWatchError::Other(err) => err,
    })?;

    {
        let config2 = config.clone();
        let db2 = db.clone();
        let bot2 = bot.clone();
        let chat_id2 = chat_id;
        let watch_id2 = created.watch_id.clone();
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
            // Local shared client for this initialization flow (the shared
            // process-wide client lives in main.rs and is not threaded into
            // the Telegram handler layer).
            let drive2 = DriveClient::with_timeout(Duration::from_secs(
                config2.engine.request_timeout_seconds,
            ));
            if let Err(err) =
                run_initial_clone(config2, db2, drive2, watch_id2.clone(), notify).await
            {
                tracing::error!(watch_id = watch_id2, error = %err, "watch initializer failed");
            }
        });
    }

    Ok(watch_created_text(
        lang,
        &created.watch_id,
        &created.source_name,
        &created.source_id,
        &created.destination_name,
    ))
}

fn ensure_watch_enabled(enabled: bool, lang: keyboards::UiLanguage) -> anyhow::Result<()> {
    if enabled {
        Ok(())
    } else {
        anyhow::bail!(watch_disabled_text(lang))
    }
}

async fn spawn_watch_panel(
    bot: Bot,
    chat_id: ChatId,
    config: AppConfig,
    db: Database,
    telegram_user_id: i64,
    page: usize,
    mode: WatchListMode,
) -> ResponseResult<()> {
    match render_watch_panel(&config, &db, telegram_user_id, page, mode).await {
        Ok((text, keyboard)) => {
            if keyboard.inline_keyboard.is_empty() {
                bot.send_message(chat_id, text).await?;
            } else {
                bot.send_message(chat_id, text)
                    .reply_markup(keyboard)
                    .await?;
            }
        }
        Err(err) => {
            bot.send_message(chat_id, load_watch_list_error(ui_language(&config), &err))
                .await?;
        }
    }
    Ok(())
}

async fn render_watch_panel(
    config: &AppConfig,
    db: &Database,
    telegram_user_id: i64,
    page: usize,
    mode: WatchListMode,
) -> anyhow::Result<(String, InlineKeyboardMarkup)> {
    let lang = ui_language(config);
    let watches = repo::list_watches_for_user(db, telegram_user_id).await?;
    if watches.is_empty() {
        return Ok((
            watch_empty_text(lang).to_string(),
            InlineKeyboardMarkup::new(Vec::<Vec<teloxide::types::InlineKeyboardButton>>::new()),
        ));
    }

    let total_pages = watches.len().div_ceil(WATCH_LIST_PAGE_SIZE);
    let page = page.min(total_pages.saturating_sub(1));
    let start = page * WATCH_LIST_PAGE_SIZE;
    let end = (start + WATCH_LIST_PAGE_SIZE).min(watches.len());
    let page_watches = &watches[start..end];
    let token_manager = TokenManager::new(config.clone(), db.clone());
    let access_token = token_manager.access_token("default").await.ok();
    let drive =
        DriveClient::with_timeout(Duration::from_secs(config.engine.request_timeout_seconds));

    let mut lines = vec![watch_title().to_string(), "━━━━━━━━━━".to_string()];
    lines.push(watch_list_mode_title(mode, lang).to_string());
    lines.push(format!(
        "{} {}/{} · {} watch",
        watch_page_label(lang),
        page + 1,
        total_pages,
        watches.len()
    ));
    let mut buttons = Vec::with_capacity(page_watches.len());
    for watch in page_watches {
        let (source, destination) =
            watch_folder_labels(&drive, access_token.as_ref().map(|t| t.as_str()), watch).await;
        let backlog = repo::watch_backlog(db, &watch.id)
            .await?
            .map_or(0, |b| b.pending_events);
        lines.push(String::new());
        push_field(
            &mut lines,
            watch_name_field(lang),
            &format!("{source} -> {destination}"),
        );
        push_field(
            &mut lines,
            destination_short_id_field(lang),
            short_id(&watch.id),
        );
        push_field(
            &mut lines,
            progress_status_field(lang),
            watch_status_label(lang, &watch.status),
        );
        push_field(&mut lines, watch_pending_field(lang), &backlog.to_string());
        push_field(
            &mut lines,
            "Policy",
            content_update_policy_label(lang, &watch.content_update_policy),
        );
        buttons.push((
            watch.id.clone(),
            format!(
                "{} -> {} · {}",
                source,
                destination,
                watch_status_label(lang, &watch.status)
            ),
        ));
    }

    Ok((
        lines.join("\n"),
        keyboards::watch_list_keyboard(
            &buttons,
            mode.as_str(),
            page,
            page > 0,
            end < watches.len(),
            lang,
        ),
    ))
}

fn parse_watch_list_payload(payload: &str) -> Option<(WatchListMode, usize)> {
    let (mode, page) = payload.split_once(':')?;
    Some((WatchListMode::from_str(mode)?, page.parse().ok()?))
}

async fn watch_status_panel(
    config: &AppConfig,
    db: &Database,
    telegram_user_id: i64,
    watch_id: &str,
) -> anyhow::Result<(String, InlineKeyboardMarkup)> {
    let watch = resolve_watch_for_user(db, telegram_user_id, watch_id, ui_language(config)).await?;
    let lang = ui_language(config);
    let text = watch_status_detail(config, db, telegram_user_id, &watch.id, lang).await?;
    let keyboard = keyboards::watch_detail_keyboard(&watch.id, &watch.status, ui_language(config));
    Ok((text, keyboard))
}

async fn edit_watch_after_action(
    bot: &Bot,
    chat_id: ChatId,
    message_id: Option<MessageId>,
    config: &AppConfig,
    db: &Database,
    telegram_user_id: i64,
    watch_id: &str,
    action_result: anyhow::Result<String>,
) -> ResponseResult<()> {
    let action_text = action_result.unwrap_or_else(|err| err.to_string());
    match watch_status_panel(config, db, telegram_user_id, watch_id).await {
        Ok((detail, keyboard)) => {
            let keyboard = (!keyboard.inline_keyboard.is_empty()).then_some(keyboard);
            edit_or_send_with_keyboard(
                bot,
                chat_id,
                message_id,
                format!("{action_text}\n\n{detail}"),
                keyboard,
            )
            .await?;
        }
        Err(_) => {
            edit_or_send_with_keyboard(
                bot,
                chat_id,
                message_id,
                action_text,
                Some(keyboards::back_home_keyboard(ui_language(config))),
            )
            .await?;
        }
    }
    Ok(())
}

async fn set_watch_policy_callback(
    db: &Database,
    telegram_user_id: i64,
    payload: &str,
    lang: keyboards::UiLanguage,
) -> anyhow::Result<String> {
    let Some((watch_id, policy)) = payload.split_once(':') else {
        anyhow::bail!(watch_policy_invalid(lang));
    };
    let policy = match policy {
        "v" => "versioned_copy",
        "r" => "replace_copy",
        "m" => "manual_confirmation",
        _ => anyhow::bail!(watch_policy_invalid(lang)),
    };
    let watch_id = resolve_watch_id_for_user(db, telegram_user_id, watch_id, lang).await?;
    if repo::set_watch_content_update_policy(db, telegram_user_id, &watch_id, policy).await? {
        Ok(watch_policy_changed_text(lang, &watch_id, policy))
    } else {
        Ok(watch_not_found_text(lang).to_string())
    }
}

async fn watch_status_detail(
    config: &AppConfig,
    db: &Database,
    telegram_user_id: i64,
    watch_id: &str,
    lang: keyboards::UiLanguage,
) -> anyhow::Result<String> {
    let w = resolve_watch_for_user(db, telegram_user_id, watch_id, lang).await?;
    let token_manager = TokenManager::new(config.clone(), db.clone());
    let access_token = token_manager.access_token("default").await.ok();
    let drive =
        DriveClient::with_timeout(Duration::from_secs(config.engine.request_timeout_seconds));
    let (source, destination) =
        watch_folder_labels(&drive, access_token.as_ref().map(|t| t.as_str()), &w).await;
    let backlog = repo::watch_backlog(db, &w.id).await?;
    let cursor_seq = backlog
        .as_ref()
        .map_or(w.last_consumed_sequence, |b| b.cursor_last_event_sequence);
    let pending_events = backlog.as_ref().map_or(0, |b| b.pending_events);
    Ok(render_watch_detail(
        lang,
        &w,
        &source,
        &destination,
        cursor_seq,
        pending_events,
    ))
}

async fn watch_folder_labels(
    drive: &DriveClient,
    access_token: Option<&str>,
    watch: &repo::WatchSubscription,
) -> (String, String) {
    let source = watch_folder_label(
        drive,
        access_token,
        &watch.source_root_id,
        watch.source_resource_key.as_deref(),
    )
    .await;
    let destination =
        watch_folder_label(drive, access_token, &watch.destination_root_id, None).await;
    (source, destination)
}

async fn watch_folder_label(
    drive: &DriveClient,
    access_token: Option<&str>,
    folder_id: &str,
    resource_key: Option<&str>,
) -> String {
    let Some(access_token) = access_token else {
        return short_id(folder_id).to_string();
    };
    let reference = DriveReference {
        file_id: folder_id.to_string(),
        resource_key: resource_key.map(str::to_string),
        hinted_kind: None,
    };
    drive
        .get_reference(access_token, &reference)
        .await
        .map(|file| file.name)
        .unwrap_or_else(|_| short_id(folder_id).to_string())
}

async fn resolve_watch_for_user(
    db: &Database,
    telegram_user_id: i64,
    watch_id: &str,
    lang: keyboards::UiLanguage,
) -> anyhow::Result<repo::WatchSubscription> {
    let watch_id = watch_id.trim();
    if let Some(watch) = repo::watch_for_user(db, telegram_user_id, watch_id).await? {
        return Ok(watch);
    }

    let matches = repo::watches_for_user_prefix(db, telegram_user_id, watch_id, 2).await?;
    match matches.as_slice() {
        [watch] => Ok(watch.clone()),
        [] => anyhow::bail!(watch_not_found_text(lang)),
        _ => anyhow::bail!(watch_prefix_ambiguous_text(lang)),
    }
}

async fn resolve_watch_id_for_user(
    db: &Database,
    telegram_user_id: i64,
    watch_id: &str,
    lang: keyboards::UiLanguage,
) -> anyhow::Result<String> {
    Ok(resolve_watch_for_user(db, telegram_user_id, watch_id, lang)
        .await?
        .id)
}

async fn pause_watch(
    db: &Database,
    telegram_user_id: i64,
    watch_id: &str,
    lang: keyboards::UiLanguage,
) -> anyhow::Result<String> {
    let watch_id = resolve_watch_id_for_user(db, telegram_user_id, watch_id, lang).await?;
    if repo::pause_watch_for_user(db, telegram_user_id, &watch_id).await? {
        Ok(watch_paused_text(lang, &watch_id))
    } else {
        Ok(watch_pause_failed_text(lang).to_string())
    }
}

async fn resume_watch(
    config: &AppConfig,
    db: &Database,
    telegram_user_id: i64,
    watch_id: &str,
    lang: keyboards::UiLanguage,
) -> anyhow::Result<String> {
    let watch_id = resolve_watch_id_for_user(db, telegram_user_id, watch_id, lang).await?;
    match repo::resume_watch_for_user(
        db,
        telegram_user_id,
        &watch_id,
        config.watch.max_backlog_events_per_watch,
    )
    .await?
    {
        repo::WatchResumeResult::Resumed => Ok(watch_resumed_text(lang, &watch_id)),
        repo::WatchResumeResult::NeedsReconcile { pending_events } => {
            Ok(watch_needs_reconcile_text(
                lang,
                &watch_id,
                pending_events,
                config.watch.max_backlog_events_per_watch,
            ))
        }
        repo::WatchResumeResult::NotResumable => Ok(watch_resume_failed_text(lang).to_string()),
    }
}

async fn stop_watch(
    db: &Database,
    telegram_user_id: i64,
    watch_id: &str,
    lang: keyboards::UiLanguage,
) -> anyhow::Result<String> {
    let watch_id = resolve_watch_id_for_user(db, telegram_user_id, watch_id, lang).await?;
    if repo::stop_watch_for_user(db, telegram_user_id, &watch_id).await? {
        Ok(watch_stopped_text(lang, &watch_id))
    } else {
        Ok(watch_stop_failed_text(lang).to_string())
    }
}

async fn set_watch_policy(
    db: &Database,
    telegram_user_id: i64,
    input: &str,
    lang: keyboards::UiLanguage,
) -> anyhow::Result<String> {
    let parts: Vec<&str> = input.splitn(2, char::is_whitespace).collect();
    if parts.len() < 2 {
        anyhow::bail!(watch_policy_usage_text(lang));
    }
    let watch_id = parts[0];
    let policy = parts[1].trim();
    if !matches!(
        policy,
        "versioned_copy" | "replace_copy" | "manual_confirmation"
    ) {
        anyhow::bail!(watch_policy_invalid_value(lang, policy));
    }
    let watch_id = resolve_watch_id_for_user(db, telegram_user_id, watch_id, lang).await?;
    if repo::set_watch_content_update_policy(db, telegram_user_id, &watch_id, policy).await? {
        Ok(watch_policy_changed_text(lang, &watch_id, policy))
    } else {
        Ok(watch_not_found_text(lang).to_string())
    }
}

async fn ensure_owner_or_operator(db: &Database, actor_user_id: i64) -> anyhow::Result<()> {
    let role = repo::authorized_user(db, actor_user_id)
        .await?
        .filter(|user| user.enabled)
        .map(|user| user.role)
        .unwrap_or_default();
    if matches!(role.as_str(), "owner" | "operator") {
        Ok(())
    } else {
        anyhow::bail!("Chỉ owner/operator mới có thể quản lý bộ lọc watch")
    }
}

async fn manage_watch_filter(
    db: &Database,
    telegram_user_id: i64,
    input: &str,
    lang: keyboards::UiLanguage,
) -> anyhow::Result<String> {
    ensure_owner_or_operator(db, telegram_user_id).await?;
    let parts: Vec<&str> = input.trim().splitn(3, char::is_whitespace).collect();
    let watch_id = parts.first().copied().unwrap_or_default();

    match (parts.first(), parts.get(1).copied()) {
        (_, Some("list")) if !watch_id.is_empty() => {
            let watch = resolve_watch_for_user(db, telegram_user_id, watch_id, lang).await?;
            Ok(render_watch_filter_list(lang, &watch))
        }
        (_, Some("clear")) if !watch_id.is_empty() => {
            let watch_id = resolve_watch_id_for_user(db, telegram_user_id, watch_id, lang).await?;
            repo::set_watch_exclude_globs(db, telegram_user_id, &watch_id, "[]").await?;
            Ok(watch_filter_cleared_text(lang, &watch_id))
        }
        (_, Some(action @ ("add" | "remove"))) if !watch_id.is_empty() => {
            let Some(raw_glob) = parts.get(2) else {
                anyhow::bail!(watch_filter_usage_text(lang));
            };
            let glob = glob::normalize_glob(raw_glob).map_err(anyhow::Error::msg)?;
            let watch = resolve_watch_for_user(db, telegram_user_id, watch_id, lang).await?;
            let mut globs = glob::parse_glob_list(&watch.exclude_globs);

            if action == "add" {
                if globs.len() >= glob::MAX_GLOBS_PER_WATCH {
                    anyhow::bail!(
                        "Tối đa {} glob cho mỗi watch. Dùng remove hoặc clear trước.",
                        glob::MAX_GLOBS_PER_WATCH
                    );
                }
                if globs.iter().any(|existing| *existing == glob) {
                    return Ok(watch_filter_exists_text(lang, &glob, &watch.id));
                }
                globs.push(glob.clone());
                repo::set_watch_exclude_globs(
                    db,
                    telegram_user_id,
                    &watch.id,
                    &glob::serialize_glob_list(&globs),
                )
                .await?;
                Ok(watch_filter_added_text(lang, &glob, &watch.id))
            } else {
                let before = globs.len();
                globs.retain(|existing| *existing != glob);
                if globs.len() == before {
                    return Ok(watch_filter_missing_text(lang, &glob, &watch.id));
                }
                repo::set_watch_exclude_globs(
                    db,
                    telegram_user_id,
                    &watch.id,
                    &glob::serialize_glob_list(&globs),
                )
                .await?;
                Ok(watch_filter_removed_text(lang, &glob, &watch.id))
            }
        }
        _ => anyhow::bail!(watch_filter_usage_text(lang)),
    }
}

// ── Helpers ──────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use crate::drive::client::DriveApiError;
    use crate::drive::types::DriveFile;
    use crate::watch::errors::format_drive_error;
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
    fn clone_plan_lines_follow_language() {
        let plan = ClonePlan {
            folders: 2,
            files: 3,
            shortcuts: 1,
            google_native: 1,
            known_bytes: 2048,
            warning_count: 1,
            scanned_items: 5,
            truncated: true,
        };
        let en = plan.render_lines(keyboards::UiLanguage::En);
        assert!(en.iter().any(|line| line == "Scanned      : 5 items"));
        assert!(en.iter().any(|line| line == "Folders      : 2"));
        assert!(en.iter().any(|line| line == "Files        : 3"));
        assert!(en.iter().any(|line| line == "Known size   : 2.0 KB"));
        assert!(
            en.iter()
                .any(|line| line == "Warnings     : 1 items need attention")
        );
        assert!(
            en.iter()
                .any(|line| line == "Note         : scanned only the first 5 items")
        );

        let vi = plan.render_lines(keyboards::UiLanguage::Vi);
        assert!(vi.iter().any(|line| line == "Đã quét      : 5 item"));
        assert!(vi.iter().any(|line| line == "Dung lượng rõ: 2.0 KB"));
    }

    #[test]
    fn clone_outcome_summary_follows_language() {
        let job = repo::JobDetail {
            id: "abcdefgh-job".to_string(),
            kind: "one_shot".to_string(),
            status: "failed".to_string(),
            source_root_id: "source".to_string(),
            destination_parent_id: "dest".to_string(),
            total_discovered: 10,
            completed_items: 7,
            failed_items: 2,
            skipped_items: 1,
            error_summary: Some("permission denied".to_string()),
            created_at_ms: 0,
            updated_at_ms: 0,
        };
        let en = render_clone_outcome_from_job(keyboards::UiLanguage::En, &job);
        assert!(en.contains("CLONE FAILED"));
        assert!(en.contains("Status: failed"));
        assert!(en.contains("Scanned: 10"));
        assert!(en.contains("Completed: 7"));
        assert!(en.contains("Failed: 2"));
        assert!(en.contains("Skipped: 1"));
        assert!(en.contains("Main error: permission denied"));
        assert!(en.contains("retry failed items"));

        let vi = render_clone_outcome_from_job(keyboards::UiLanguage::Vi, &job);
        assert!(vi.contains("CLONE LỖI"));
        assert!(vi.contains("Trạng thái: thất bại"));
        assert!(vi.contains("Retry lỗi"));
    }

    #[test]
    fn job_detail_and_progress_follow_language() {
        let job = repo::JobDetail {
            id: "abcdefgh-job".to_string(),
            kind: "one_shot".to_string(),
            status: "running".to_string(),
            source_root_id: "source".to_string(),
            destination_parent_id: "dest".to_string(),
            total_discovered: 10,
            completed_items: 4,
            failed_items: 1,
            skipped_items: 2,
            error_summary: Some("rate limited".to_string()),
            created_at_ms: 0,
            updated_at_ms: 0,
        };

        let en_detail = render_job_detail(&job, keyboards::UiLanguage::En);
        assert!(en_detail.contains("JOB DETAIL"));
        assert!(en_detail.contains("Status: running"));
        assert!(en_detail.contains("Latest error: rate limited"));

        let en_progress = render_job_progress(&job, 2, keyboards::UiLanguage::En);
        assert!(en_progress.contains("CLONE PROGRESS"));
        assert!(en_progress.contains("Status: running"));
        assert!(en_progress.contains("Rate: 2.0 item/s"));
        assert!(en_progress.contains("Skipped: 2"));

        let vi_detail = render_job_detail(&job, keyboards::UiLanguage::Vi);
        assert!(vi_detail.contains("CHI TIẾT JOB"));
        assert!(vi_detail.contains("Trạng thái: đang chạy"));
        assert!(vi_detail.contains("Lỗi gần nhất: rate limited"));
    }

    #[test]
    fn clone_and_report_errors_follow_language() {
        let err = anyhow::anyhow!("bad id");
        assert!(
            invalid_drive_link_text(keyboards::UiLanguage::En, &err).contains("Invalid Drive link")
        );
        assert!(
            invalid_drive_link_text(keyboards::UiLanguage::Vi, &err)
                .contains("Link Drive không hợp lệ")
        );
        assert!(clone_job_accepted_text(keyboards::UiLanguage::En).contains("/jobs"));
        assert!(clone_active_limit_text(keyboards::UiLanguage::En, 2).contains("active jobs"));
        assert_eq!(
            report_job_not_found(keyboards::UiLanguage::En),
            "Job not found."
        );
        assert!(
            report_write_error(keyboards::UiLanguage::En, "disk")
                .contains("Could not create report")
        );
    }

    #[test]
    fn job_action_messages_follow_language() {
        let summary = repo::RetryFailedSummary {
            traversal_folders_requeued: 1,
            job_items_requeued: 2,
            operation_intents_replanned: 3,
        };

        assert!(
            job_pause_requested_text(keyboards::UiLanguage::En, "abcdefgh-job")
                .contains("Pause requested")
        );
        assert!(job_resume_failed_text(keyboards::UiLanguage::En).contains("not paused"));
        assert!(
            job_cancel_requested_text(keyboards::UiLanguage::Vi, "abcdefgh-job").contains("huỷ")
        );
        assert!(
            job_retry_requested_text(keyboards::UiLanguage::En, "abcdefgh-job", &summary)
                .contains("Folders: 1")
        );
        assert!(job_prefix_ambiguous_text(keyboards::UiLanguage::En).contains("More than one job"));
    }

    #[test]
    fn watch_policies_are_translated_for_telegram() {
        assert_eq!(
            vi_content_update_policy("versioned_copy"),
            "tạo bản copy mới"
        );
        assert_eq!(
            vi_deletion_policy("preserve_destination"),
            "giữ bản copy ở đích"
        );
        assert_eq!(
            vi_move_out_policy("detach"),
            "tách khỏi watch, không xoá bản copy"
        );
    }

    #[test]
    fn watch_detail_follows_language() {
        let watch = repo::WatchSubscription {
            id: "abcdefgh-watch".to_string(),
            google_account_id: "default".to_string(),
            cursor_id: "cursor".to_string(),
            telegram_user_id: 1,
            chat_id: 2,
            source_root_id: "source".to_string(),
            source_resource_key: None,
            source_drive_id: None,
            destination_root_id: "dest".to_string(),
            destination_drive_id: None,
            status: "catching_up".to_string(),
            content_update_policy: "versioned_copy".to_string(),
            deletion_policy: "preserve_destination".to_string(),
            move_out_policy: "detach".to_string(),
            exclude_globs: "[]".to_string(),
            baseline_sequence: 3,
            last_consumed_sequence: 4,
            created_at_ms: 0,
            updated_at_ms: 0,
        };

        let en = render_watch_detail(keyboards::UiLanguage::En, &watch, "Source", "Dest", 9, 5);
        assert!(en.contains("Status: catching up"));
        assert!(en.contains("Source folder: Source (source)"));
        assert!(en.contains("Pending changes: 5"));
        assert!(en.contains("create a new copy"));

        let vi = render_watch_detail(keyboards::UiLanguage::Vi, &watch, "Nguồn", "Đích", 9, 5);
        assert!(vi.contains("Trạng thái: đang bắt kịp"));
        assert!(vi.contains("Nguồn (folder cần lưu): Nguồn (source)"));
        assert!(vi.contains("Số thay đổi còn chờ: 5"));
        assert!(vi.contains("tạo bản copy mới"));
    }

    #[test]
    fn report_summary_gives_next_action() {
        let failed = render_report_summary(
            keyboards::UiLanguage::Vi,
            T::ReportJobTitle,
            "abcdefgh-job",
            "failed",
            10,
            7,
            2,
            1,
        );
        assert!(failed.contains("Retry lỗi"));
        assert!(failed.contains("Bỏ qua: 1"));

        let clean = render_report_summary(
            keyboards::UiLanguage::Vi,
            T::ReportJobTitle,
            "abcdefgh-job",
            "completed",
            10,
            10,
            0,
            0,
        );
        assert!(clean.contains("Job đã sạch lỗi"));

        let english = render_report_summary(
            keyboards::UiLanguage::En,
            T::ReportLatestTitle,
            "abcdefgh-job",
            "partially_completed",
            10,
            7,
            0,
            3,
        );
        assert!(english.contains("LATEST REPORT"));
        assert!(english.contains("Status: partially completed"));
        assert!(english.contains("Skipped: 3"));
    }

    #[test]
    fn help_text_follows_language() {
        let en = render_help_text(keyboards::UiLanguage::En);
        assert!(en.contains("502Drive"));
        assert!(en.contains("/connect"));
        assert!(en.contains("/clone_here"));
        assert!(en.contains("/watch_filter"));
        assert!(en.contains("Admin: /grant /revoke"));
        assert!(!en.contains("Bạn có thể"));

        let vi = render_help_text(keyboards::UiLanguage::Vi);
        assert!(vi.contains("Bạn có thể"));
        assert!(vi.contains("/watch_filter"));
        assert!(vi.contains("Quản trị: /grant /revoke"));
    }

    #[test]
    fn home_labels_follow_language() {
        assert_eq!(
            account_status_label(keyboards::UiLanguage::En, "chưa kết nối"),
            "Not connected"
        );
        assert_eq!(
            account_status_label(keyboards::UiLanguage::Vi, "connected"),
            "Đã kết nối"
        );
        let en = keyboards::UiLanguage::En;
        assert_eq!(en.text(T::HomeDefaultDestination), "Default destination");
        assert_eq!(en.text(T::HomeStatusConnected), "Connected");
        assert_eq!(en.text(T::HomeStatusNotConnected), "Not connected");
        assert_eq!(en.text(T::HomeStatusReconnect), "Reconnect needed");
        assert_eq!(en.text(T::HomeBotReady), "Ready");
        assert_eq!(en.text(T::HomeJobsRunning), "Running");
        assert_eq!(en.text(T::HomeWatching), "Watching");
        assert_eq!(en.text(T::HomeNeedsAttention), "Needs attention");
        assert_eq!(
            en.text(T::HomeHint),
            "Send a Google Drive link to get started quickly."
        );
    }

    #[test]
    fn account_labels_follow_language() {
        assert_eq!(account_title(keyboards::UiLanguage::En), "GOOGLE ACCOUNT");
        assert_eq!(account_status_field(keyboards::UiLanguage::En), "Status");
        assert_eq!(account_name_field(keyboards::UiLanguage::En), "Name");
        assert_eq!(
            account_login_hint(keyboards::UiLanguage::En),
            "Run this on the machine running the bot:"
        );
        assert_eq!(
            account_destination_title(keyboards::UiLanguage::En),
            "DESTINATION FOLDER"
        );
        assert_eq!(
            account_destination_missing(keyboards::UiLanguage::En),
            "Not set. Use /set_destination <folder_url>."
        );
    }

    #[test]
    fn jobs_labels_follow_language() {
        assert_eq!(jobs_title(keyboards::UiLanguage::En), "ACTIVE JOBS");
        assert_eq!(
            jobs_empty_text(keyboards::UiLanguage::En),
            "ACTIVE JOBS\n━━━━━━━━━━\nNo active jobs."
        );
        assert_eq!(jobs_status_field(keyboards::UiLanguage::En), "Status");
        assert_eq!(jobs_progress_field(keyboards::UiLanguage::En), "Progress");
        assert_eq!(
            jobs_progress_text(keyboards::UiLanguage::En, 2, 5, 1),
            "2 done / 5 scanned / 1 failed"
        );
        assert_eq!(
            jobs_progress_text(keyboards::UiLanguage::Vi, 2, 5, 1),
            "2 xong / 5 quét / 1 lỗi"
        );
    }

    #[test]
    fn destination_list_follows_language() {
        assert_eq!(
            render_destination_list(&[], keyboards::UiLanguage::En),
            "No saved destination folders yet.\nUse /set_destination <folder_url>."
        );

        let profiles = vec![repo::DestinationProfile {
            id: "profile".to_string(),
            google_account_id: "default".to_string(),
            label: "Backup".to_string(),
            destination_parent_id: "abcdefghijklmnop".to_string(),
            destination_drive_id: None,
            destination_resource_key: None,
            is_default: true,
        }];
        let text = render_destination_list(&profiles, keyboards::UiLanguage::En);
        assert!(text.contains("DESTINATIONS"));
        assert!(text.contains("Backup [default]"));
        assert!(text.contains("Location: My Drive / shared"));
        assert!(text.contains("Short ID: abcdefgh"));
        assert!(!text.contains("mặc định"));
    }

    #[test]
    fn destination_browser_labels_follow_language() {
        let en = keyboards::UiLanguage::En;
        assert_eq!(destination_browser_title(en), "CHOOSE DESTINATION FOLDER");
        assert_eq!(destination_writable_field(en), "Writable");
        assert_eq!(capability_text_lang(Some(true), en), "Yes");
        assert_eq!(capability_text_lang(Some(false), en), "No");
        assert_eq!(capability_text_lang(None, en), "Unknown");
        assert_eq!(
            destination_no_child_folders(en),
            "No child folders on this page."
        );
        assert_eq!(destination_child_count(en, 3), "Child folders: 3");
        assert_eq!(
            destination_folder_page_hint(en),
            "Showing 20 folders per page."
        );
        assert_eq!(
            destination_no_shared_drives(en),
            "No Shared Drives found for this account."
        );
        assert_eq!(destination_shared_drive_count(en, 2), "Shared Drives: 2");
        assert_eq!(
            destination_drive_page_hint(en),
            "Showing 20 drives per page."
        );
    }

    #[test]
    fn clone_preview_labels_follow_language() {
        let en = keyboards::UiLanguage::En;
        assert!(clone_loading_text(en).contains("Checking Drive source"));
        assert_eq!(clone_info_title(en), "CLONE PREVIEW");
        assert_eq!(clone_missing_email(en), "email unavailable");
        assert_eq!(clone_source_name_field(en), "Source name");
        assert_eq!(clone_item_type(en, true), "Folder");
        assert_eq!(clone_readable_field(en), "Readable/copyable");
        assert!(clone_warn_cannot_copy(en).contains("WARNING"));
        assert_eq!(clone_plan_title(en), "PLAN");
        assert_eq!(clone_reason_field(en), "Reason");
        assert_eq!(clone_destination_title(en), "DESTINATION");
        assert_eq!(
            clone_missing_destination(en),
            "No destination folder yet. Use /set_destination <folder_url>."
        );
    }

    #[test]
    fn preview_destination_label_uses_short_id_and_drive_marker() {
        let profile = repo::DestinationProfile {
            id: "profile".to_string(),
            google_account_id: "default".to_string(),
            label: "Đích".to_string(),
            destination_parent_id: "abcdefghijklmnop".to_string(),
            destination_drive_id: Some("shared-drive".to_string()),
            destination_resource_key: None,
            is_default: true,
        };
        assert_eq!(preview_destination_label(&profile), "Đích [SD:abcdefgh]");
    }

    #[test]
    fn missing_argument_prompts_are_actionable() {
        let vi = keyboards::UiLanguage::Vi;
        let en = keyboards::UiLanguage::En;

        assert!(clone_prompt(vi).contains("ô trả lời"));
        assert!(clone_prompt(vi).contains(ReplyPrompt::Clone.marker(vi)));
        assert!(set_destination_prompt(vi).contains("Đích là folder"));
        assert!(watch_prompt(vi).contains("nguồn rồi đích"));
        assert!(watch_prompt(vi).contains(ReplyPrompt::Watch.marker(vi)));
        assert!(watch_id_prompt("/watch_status", vi).contains("/watches"));
        assert!(watch_policy_prompt(vi).contains("versioned_copy"));
        assert!(watch_policy_prompt(vi).contains(ReplyPrompt::WatchPolicy.marker(vi)));
        assert_eq!(ReplyPrompt::Clone.placeholder(vi), "Dán link nguồn Drive");

        assert!(clone_prompt(en).contains("reply to this message"));
        assert!(watch_prompt(en).contains("source then destination"));
        assert_eq!(
            ReplyPrompt::Clone.placeholder(en),
            "Paste source Drive link"
        );
    }

    #[test]
    fn reply_prompt_markers_are_unique() {
        let prompts = [
            ReplyPrompt::Clone,
            ReplyPrompt::CloneHere,
            ReplyPrompt::SetDestination,
            ReplyPrompt::Status,
            ReplyPrompt::Pause,
            ReplyPrompt::Resume,
            ReplyPrompt::Cancel,
            ReplyPrompt::Retry,
            ReplyPrompt::Grant,
            ReplyPrompt::Revoke,
            ReplyPrompt::Watch,
            ReplyPrompt::WatchStatus,
            ReplyPrompt::WatchPause,
            ReplyPrompt::WatchResume,
            ReplyPrompt::WatchPolicy,
            ReplyPrompt::Unwatch,
        ];
        for (idx, prompt) in prompts.iter().enumerate() {
            assert!(!prompt.marker(keyboards::UiLanguage::Vi).is_empty());
            assert!(!prompt.marker(keyboards::UiLanguage::En).is_empty());
            assert_eq!(
                prompts
                    .iter()
                    .filter(|other| {
                        other.marker(keyboards::UiLanguage::Vi)
                            == prompt.marker(keyboards::UiLanguage::Vi)
                    })
                    .count(),
                1,
                "duplicate marker at index {idx}"
            );
        }
    }

    #[test]
    fn drive_errors_are_translated_for_telegram() {
        let permission = DriveApiError::Api {
            status: StatusCode::FORBIDDEN,
            reason: Some("insufficientPermissions".to_string()),
            message: "The user does not have sufficient permissions".to_string(),
            retry_after: None,
        };
        let text = format_drive_error(&permission, keyboards::UiLanguage::Vi);
        assert!(text.contains("không đủ quyền"));
        assert!(text.contains("HTTP: 403"));

        let not_found = DriveApiError::Api {
            status: StatusCode::NOT_FOUND,
            reason: Some("notFound".to_string()),
            message: "File not found".to_string(),
            retry_after: None,
        };
        let text = format_drive_error(&not_found, keyboards::UiLanguage::Vi);
        assert!(text.contains("resource key"));

        let english = format_drive_error(&permission, keyboards::UiLanguage::En);
        assert!(english.contains("does not have access"));
        assert!(english.contains("Details: The user"));
        assert!(!english.contains("Chi tiết"));

        let rate = DriveApiError::Api {
            status: StatusCode::TOO_MANY_REQUESTS,
            reason: Some("rateLimitExceeded".to_string()),
            message: "slow down".to_string(),
            retry_after: None,
        };
        assert!(format_drive_error(&rate, keyboards::UiLanguage::En).contains("rate limiting"));
    }

    #[test]
    fn action_status_text_follows_language() {
        let en = keyboards::UiLanguage::En;
        assert_eq!(
            access_denied_message(en),
            "Access denied. Contact the owner to be allowed."
        );
        assert_eq!(access_denied_short(en), "Access denied.");
        assert_eq!(
            clone_request_expired(en),
            "Clone request expired or does not belong to you."
        );
        assert_eq!(clone_starting(en), "Starting clone...");
        assert_eq!(clone_request_cancelled(en), "Clone request cancelled.");
        assert_eq!(destination_not_found(en), "Destination folder not found.");
        assert_eq!(unknown_action(en), "Unknown action.");

        let err = anyhow::anyhow!("boom");
        assert_eq!(load_home_error(en, &err), "Could not load home: boom");
        assert_eq!(load_jobs_error(en, &err), "Could not load jobs: boom");
        assert_eq!(load_watch_error(en, &err), "Could not load watches: boom");
        assert_eq!(
            load_watch_list_error(en, &err),
            "Could not load watch list: boom"
        );
        assert!(browse_folder_error(en, &err).starts_with("Could not browse folder:"));
        assert!(pick_destination_error(en, &err).starts_with("Could not set destination folder:"));
        assert!(clone_run_error(en, &err).starts_with("Clone failed:"));
        assert!(clone_inspect_error(en, &err).starts_with("Could not inspect source:"));
    }

    #[test]
    fn watch_command_requires_enabled_config() {
        assert!(ensure_watch_enabled(true, keyboards::UiLanguage::Vi).is_ok());
        let err = ensure_watch_enabled(false, keyboards::UiLanguage::Vi)
            .unwrap_err()
            .to_string();
        assert!(err.contains("watch.enabled = true"));
    }
}
