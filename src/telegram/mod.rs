pub mod commands;
pub mod handlers;
pub mod keyboards;
pub mod progress;

use teloxide::prelude::*;
use teloxide::utils::command::BotCommands;

use crate::{config::AppConfig, state::db::Database, telegram::commands::Command};

pub async fn run(config: AppConfig, db: Database) -> anyhow::Result<()> {
    let bot = Bot::new(config.telegram.bot_token.clone());
    bot.set_my_commands(Command::bot_commands()).await?;

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
