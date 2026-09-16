pub mod commands;
pub mod handlers;
pub mod i18n;
pub mod keyboards;
pub mod progress;

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
    let mut commands = vec![
        ("start", lang.text(T::CommandStart)),
        ("menu", lang.text(T::CommandMenu)),
        ("help", lang.text(T::CommandHelp)),
        ("clone", lang.text(T::CommandClone)),
        ("destination", lang.text(T::CommandDestination)),
        ("jobs", lang.text(T::CommandJobs)),
        ("last_report", lang.text(T::CommandLastReport)),
        ("account", lang.text(T::CommandAccount)),
        ("preview", lang.text(T::CommandPreview)),
    ];
    if watch_enabled {
        commands.extend([
            ("watches", lang.text(T::CommandWatches)),
            ("watch", lang.text(T::CommandWatch)),
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
        assert!(!vi.iter().any(|command| command.command == "pause"));
        assert!(!vi.iter().any(|command| command.command == "watch_policy"));

        let en = bot_commands(true, UiLanguage::En);
        assert_eq!(en[0].description, "Open control center");
        assert!(en.iter().any(|command| command.command == "watch"
            && command.description == "Watch source folder into destination"));
    }
}
