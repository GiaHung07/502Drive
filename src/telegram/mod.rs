pub mod commands;
pub mod handlers;
pub mod keyboards;
pub mod progress;

use teloxide::prelude::*;
use teloxide::types::BotCommand;

use crate::{config::AppConfig, state::db::Database};

pub async fn run(config: AppConfig, db: Database) -> anyhow::Result<()> {
    let bot = Bot::new(config.telegram.bot_token.clone());
    bot.set_my_commands(core_bot_commands()).await?;

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

fn core_bot_commands() -> Vec<BotCommand> {
    [
        ("start", "Mở hướng dẫn nhanh"),
        ("help", "Xem các lệnh chính"),
        ("account", "Xem tài khoản Google"),
        ("destination", "Xem/đổi thư mục đích"),
        ("set_destination", "Đặt thư mục đích mặc định"),
        ("clone", "Kiểm tra và clone Drive URL"),
        ("clone_here", "Clone ngay vào đích mặc định"),
        ("jobs", "Xem job đang chạy"),
        ("status", "Xem chi tiết job"),
        ("pause", "Tạm dừng job"),
        ("resume", "Tiếp tục job"),
        ("cancel", "Huỷ job"),
        ("retry", "Làm lại item lỗi"),
        ("preview", "Bảng tổng quan realtime"),
    ]
    .into_iter()
    .map(|(command, description)| BotCommand {
        command: command.to_string(),
        description: description.to_string(),
    })
    .collect()
}
