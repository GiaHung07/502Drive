use std::time::Duration;

use crate::{
    config::AppConfig,
    drive::{client::DriveClient, token_manager::TokenManager},
    state::{db::Database, repo},
};
use teloxide::{Bot, prelude::Requester};

pub async fn run(config: &AppConfig, db: &Database) -> anyhow::Result<()> {
    println!("Drive502 doctor");
    println!("===============");
    print_check("config", true, "loaded");
    println!("  db_path    : {}", config.storage.db_path.display());
    println!("  log_dir    : {}", config.storage.log_dir.display());
    println!("  report_dir : {}", config.storage.report_dir.display());
    print_check(
        "db",
        true,
        &format!("integrity {}", db.integrity_check().await?),
    );
    let job_counts = repo::job_status_counts(db).await?;
    if job_counts.is_empty() {
        print_check("jobs", true, "none active");
    } else {
        print_check("jobs", true, "existing history");
        for item in job_counts {
            println!("  {}: {}", item.status, item.count);
        }
    }

    if telegram_configured(config) {
        print_check("telegram", true, "configured");
        check_telegram(&config.telegram.bot_token).await?;
    } else {
        print_check(
            "telegram",
            false,
            "set telegram.bot_token and telegram.owner_telegram_id in config",
        );
    }

    if oauth_configured(config) {
        print_check("google_oauth", true, "configured");
        check_google_drive(config, db).await?;
    } else {
        print_check(
            "google_oauth",
            false,
            "set google_oauth.client_id and google_oauth.client_secret, then run auth login",
        );
    }

    check_default_destination(db).await?;
    Ok(())
}

async fn check_telegram(bot_token: &str) -> anyhow::Result<()> {
    let bot = Bot::new(bot_token);
    let me = bot.get_me().await?;
    print_check("telegram.getMe", true, &format!("@{}", me.username()));

    let webhook = bot.get_webhook_info().await?;
    if webhook.url.is_some() {
        print_check(
            "telegram.webhook",
            false,
            "webhook is set; delete it or long polling will not receive updates",
        );
    } else {
        print_check("telegram.webhook", true, "unset");
    }
    println!("  pending_updates: {}", webhook.pending_update_count);
    Ok(())
}

async fn check_google_drive(config: &AppConfig, db: &Database) -> anyhow::Result<()> {
    match repo::account_status(db).await? {
        Some(status) => print_check("google_account", status == "connected", &status),
        None => {
            print_check(
                "google_account",
                false,
                "not connected; run gdclone-bot auth login",
            );
            return Ok(());
        }
    }

    let token_manager = TokenManager::new(config.clone(), db.clone());
    let access_token = token_manager.access_token("default").await?;
    print_check("google_token_refresh", true, "ok");

    let about =
        DriveClient::with_timeout(Duration::from_secs(config.engine.request_timeout_seconds))
            .about_get(access_token.as_str())
            .await?;
    let label = about
        .user
        .and_then(|user| user.email_address.or(user.display_name))
        .unwrap_or_else(|| "unknown user".to_string());
    print_check("drive.about", true, &label);
    Ok(())
}

async fn check_default_destination(db: &Database) -> anyhow::Result<()> {
    match repo::default_destination_profile(db, "default").await? {
        Some(profile) => print_check(
            "destination",
            true,
            &format!(
                "{} ({})",
                profile.label,
                short_id(&profile.destination_parent_id)
            ),
        ),
        None => print_check(
            "destination",
            false,
            "not set; open /menu -> Destination or run /set_destination <folder_url>",
        ),
    }
    Ok(())
}

fn print_check(name: &str, ok: bool, detail: &str) {
    let status = if ok { "OK" } else { "NEEDS SETUP" };
    println!("{name}: {status} - {detail}");
}

fn telegram_configured(config: &AppConfig) -> bool {
    telegram_values_configured(
        &config.telegram.bot_token,
        config.telegram.owner_telegram_id,
    )
}

fn oauth_configured(config: &AppConfig) -> bool {
    oauth_values_configured(
        &config.google_oauth.client_id,
        &config.google_oauth.client_secret,
    )
}

fn telegram_values_configured(bot_token: &str, owner_telegram_id: i64) -> bool {
    !bot_token.trim().is_empty() && bot_token != "REPLACE_ME" && owner_telegram_id > 0
}

fn oauth_values_configured(client_id: &str, client_secret: &str) -> bool {
    !client_id.trim().is_empty()
        && client_id != "REPLACE_ME.apps.googleusercontent.com"
        && !client_secret.trim().is_empty()
        && client_secret != "REPLACE_ME"
}

fn short_id(id: &str) -> &str {
    id.get(..8).unwrap_or(id)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn doctor_detects_missing_setup() {
        assert!(!telegram_values_configured("REPLACE_ME", 0));
        assert!(!telegram_values_configured("", 123));
        assert!(telegram_values_configured("123:abc", 123));

        assert!(!oauth_values_configured(
            "REPLACE_ME.apps.googleusercontent.com",
            "secret"
        ));
        assert!(!oauth_values_configured("client", "REPLACE_ME"));
        assert!(oauth_values_configured("client", "secret"));

        assert_eq!(short_id("abcdefgh1234"), "abcdefgh");
    }
}
