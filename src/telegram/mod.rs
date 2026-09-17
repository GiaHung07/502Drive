pub mod commands;
pub mod handlers;
pub mod i18n;
pub mod keyboards;
pub mod progress;
pub mod render;
pub mod session;

use teloxide::prelude::*;
use teloxide::types::BotCommand;

use crate::{
    config::AppConfig,
    state::db::Database,
    telegram::i18n::{TextKey as T, UiLanguage},
};

pub async fn run(config: AppConfig, db: Database) -> anyhow::Result<()> {
    let bot = Bot::new(config.telegram.bot_token.clone());
    let lang = UiLanguage::from_code(&config.telegram.language);
    bot.set_my_commands(bot_commands(config.watch.enabled, lang))
        .await?;

    if let Err(err) = handlers::resume_running_job_progress_updaters(&bot, &db, &config).await {
        tracing::warn!(error = %err, "failed to resume running job progress updaters");
    }

    let handler = dptree::entry()
        .branch(Update::filter_message().endpoint({
            let db = db.clone();
            let config = config.clone();
            move |bot: Bot, msg: Message| {
                let db = db.clone();
                let config = config.clone();
                async move {
                    handlers::handle_message(bot, msg, config, db).await?;
                    respond(())
                }
            }
        }))
        .branch(Update::filter_callback_query().endpoint({
            let db = db.clone();
            let config = config.clone();
            move |bot: Bot, query: CallbackQuery| {
                let db = db.clone();
                let config = config.clone();
                async move {
                    handlers::handle_callback_query(bot, query, config, db).await?;
                    respond(())
                }
            }
        }));

    Dispatcher::builder(bot, handler)
        .enable_ctrlc_handler()
        .build()
        .dispatch()
        .await;

    Ok(())
}

pub(crate) fn bot_commands(watch_enabled: bool, lang: UiLanguage) -> Vec<BotCommand> {
    // Full public command menu. /grant and /revoke stay out of the menu on
    // purpose — they are admin-only and documented in /help instead.
    let mut commands = vec![
        ("start", lang.text(T::CommandStart)),
        ("menu", lang.text(T::CommandMenu)),
        ("help", lang.text(T::CommandHelp)),
        ("clone", lang.text(T::CommandClone)),
        ("clone_here", lang.text(T::CommandCloneHere)),
        ("connect", lang.text(T::CommandConnect)),
        ("account", lang.text(T::CommandAccount)),
        ("preview", lang.text(T::CommandPreview)),
        ("disconnect", lang.text(T::CommandDisconnect)),
        ("jobs", lang.text(T::CommandJobs)),
        ("status", lang.text(T::CommandStatus)),
        ("pause", lang.text(T::CommandPause)),
        ("resume", lang.text(T::CommandResume)),
        ("cancel", lang.text(T::CommandCancel)),
        ("retry", lang.text(T::CommandRetry)),
        ("last_report", lang.text(T::CommandLastReport)),
        ("whoami", lang.text(T::CommandWhoami)),
        ("destination", lang.text(T::CommandDestination)),
        ("set_destination", lang.text(T::CommandSetDestination)),
        ("clear_destination", lang.text(T::CommandClearDestination)),
    ];
    if watch_enabled {
        commands.extend([
            ("sync", lang.text(T::CommandSync)),
            ("watch", lang.text(T::CommandWatch)),
            ("watches", lang.text(T::CommandWatches)),
            ("watch_status", lang.text(T::CommandWatchStatus)),
            ("watch_pause", lang.text(T::CommandWatchPause)),
            ("watch_resume", lang.text(T::CommandWatchResume)),
            ("watch_policy", lang.text(T::CommandWatchPolicy)),
            ("watch_filter", lang.text(T::CommandWatchFilter)),
            ("unwatch", lang.text(T::CommandUnwatch)),
        ]);
    }
    commands
        .into_iter()
        .map(|(command, description)| BotCommand {
            command: command.to_string(),
            description: description.to_string(),
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::{UiLanguage, bot_commands};

    #[test]
    fn bot_commands_follow_selected_language() {
        let vi = bot_commands(true, UiLanguage::Vi);
        assert_eq!(vi[0].description, "Mở bảng điều khiển");
        assert!(vi.iter().any(|command| command.command == "watches"));
        assert!(vi.iter().any(|command| command.command == "watch_filter"));
        assert!(vi.iter().any(|command| command.command == "pause"));
        assert!(vi.iter().any(|command| command.command == "watch_policy"));
        assert!(!vi.iter().any(|command| command.command == "grant"));
        assert!(!vi.iter().any(|command| command.command == "revoke"));
        assert!(
            vi.iter()
                .all(|command| command.description.chars().count() <= 256)
        );

        let en = bot_commands(true, UiLanguage::En);
        assert_eq!(en[0].description, "Open control center");
        assert!(en.iter().any(|command| command.command == "watch"
            && command.description == "Watch source folder into destination"));
    }

    #[test]
    fn watch_commands_hidden_when_watch_disabled() {
        let disabled = bot_commands(false, UiLanguage::Vi);
        assert!(
            disabled
                .iter()
                .any(|command| command.command == "clone_here")
        );
        assert!(disabled.iter().any(|command| command.command == "connect"));
        assert!(!disabled.iter().any(|command| command.command == "sync"));
        assert!(
            !disabled
                .iter()
                .any(|command| command.command == "watch_filter")
        );
    }
}
