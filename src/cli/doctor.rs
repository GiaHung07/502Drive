use std::time::Duration;

use crate::{
    config::AppConfig,
    drive::{client::DriveClient, token_manager::TokenManager},
    state::{db::Database, repo},
};
use teloxide::{Bot, prelude::Requester};

pub async fn run(config: &AppConfig, db: &Database) -> anyhow::Result<()> {
    println!("config: ok");
    println!("db_path: {}", config.storage.db_path.display());
    println!("log_dir: {}", config.storage.log_dir.display());
    println!("report_dir: {}", config.storage.report_dir.display());
    println!("db_integrity: {}", db.integrity_check().await?);
    let job_counts = repo::job_status_counts(db).await?;
    if job_counts.is_empty() {
        println!("jobs: none");
    } else {
        for item in job_counts {
            println!("jobs.{}: {}", item.status, item.count);
        }
    }
    if config.telegram.bot_token == "REPLACE_ME" || config.telegram.bot_token.trim().is_empty() {
        println!("telegram: not configured");
    } else {
        println!("telegram: configured");
        check_telegram(&config.telegram.bot_token).await?;
    }
    if config.google_oauth.client_id == "REPLACE_ME.apps.googleusercontent.com"
        || config.google_oauth.client_secret == "REPLACE_ME"
    {
        println!("google_oauth: not configured");
    } else {
        println!("google_oauth: configured");
        check_google_drive(config, db).await?;
    }
    Ok(())
}

async fn check_telegram(bot_token: &str) -> anyhow::Result<()> {
    let bot = Bot::new(bot_token);
    let me = bot.get_me().await?;
    println!("telegram.getMe: @{}", me.username());

    let webhook = bot.get_webhook_info().await?;
    if webhook.url.is_some() {
        println!(
            "telegram.webhook: set; long polling will not receive updates until it is deleted"
        );
    } else {
        println!("telegram.webhook: unset");
    }
    println!("telegram.pending_updates: {}", webhook.pending_update_count);
    Ok(())
}

async fn check_google_drive(config: &AppConfig, db: &Database) -> anyhow::Result<()> {
    match repo::account_status(db).await? {
        Some(status) => println!("google_account: {status}"),
        None => {
            println!("google_account: not connected");
            return Ok(());
        }
    }

    let token_manager = TokenManager::new(config.clone(), db.clone());
    let access_token = token_manager.access_token("default").await?;
    println!("google_token_refresh: ok");

    let about =
        DriveClient::with_timeout(Duration::from_secs(config.engine.request_timeout_seconds))
            .about_get(access_token.as_str())
            .await?;
    let label = about
        .user
        .and_then(|user| user.email_address.or(user.display_name))
        .unwrap_or_else(|| "unknown user".to_string());
    println!("drive.about: {label}");
    Ok(())
}
