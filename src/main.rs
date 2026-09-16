use std::path::PathBuf;

use anyhow::Context;
use clap::{Parser, Subcommand};
use gdclone_bot::{
    cli,
    config::AppConfig,
    engine::recovery,
    platform, report,
    state::db::Database,
    telegram,
    watch::{NotifyReceiver, spawn_all_pollers},
};
use tracing_subscriber::{EnvFilter, layer::SubscriberExt, util::SubscriberInitExt};

#[derive(Debug, Parser)]
#[command(name = "502drive")]
struct Cli {
    #[arg(short, long)]
    config: Option<PathBuf>,

    #[command(subcommand)]
    command: Command,
}

#[derive(Debug, Subcommand)]
enum Command {
    Run,
    Doctor,
    Status,
    Backup {
        output_dir: PathBuf,
    },
    Auth {
        #[command(subcommand)]
        command: AuthCommand,
    },
    ServiceInstall,
    ServiceUninstall,
}

#[derive(Debug, Subcommand)]
enum AuthCommand {
    Login,
    Status,
    Revoke,
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let cli = Cli::parse();
    let config_path = match &cli.config {
        Some(path) => gdclone_bot::config::resolve_path(path)?,
        None => gdclone_bot::config::default_config_path()?,
    };
    let config = AppConfig::load(&config_path)?;
    init_tracing(&config)?;

    match cli.command {
        Command::Run => run_bot(config).await,
        Command::Doctor => {
            let db = open_db(&config).await?;
            cli::doctor::run(&config, &db).await
        }
        Command::Status => {
            let db = open_db(&config).await?;
            cli::status::run(&db).await
        }
        Command::Backup { output_dir } => cli::backup::run(&config_path, &config, &output_dir),
        Command::Auth { command } => {
            let db = open_db(&config).await?;
            match command {
                AuthCommand::Login => cli::auth::login(&config, &db).await,
                AuthCommand::Status => cli::auth::status(&db).await,
                AuthCommand::Revoke => cli::auth::revoke(&config, &db).await,
            }
        }
        Command::ServiceInstall => platform::current().install_service(&config_path),
        Command::ServiceUninstall => platform::current().uninstall_service(),
    }
}

async fn run_bot(config: AppConfig) -> anyhow::Result<()> {
    config.validate_for_run()?;
    let db = open_db(&config).await?;
    db.ensure_owner(config.telegram.owner_telegram_id).await?;
    match report::cleanup_old_reports(
        &config.storage.report_dir,
        config.security.report_retention_days,
    ) {
        Ok(removed) if removed > 0 => tracing::info!(removed, "old reports cleaned up"),
        Ok(_) => {}
        Err(err) => tracing::warn!(error = %err, "report cleanup failed"),
    }
    recovery::recover_on_startup(&db).await?;
    let _resume_worker = recovery::spawn_startup_resume_worker(config.clone(), db.clone());
    // Recover any watches that were stuck in 'initializing' at the time of the last crash.
    let _init_recovery = recovery::recover_initializing_watches(config.clone(), db.clone());

    // Start watch/sync infrastructure.
    let (_stop_tx, _poller_handles) = if config.watch.enabled {
        let (tx, handles, notify_rx) = spawn_all_pollers(config.clone(), db.clone());
        tracing::info!("watch pollers started");
        // Forward watch notifications to Telegram in a background task.
        // The actual Bot handle is created by the telegram layer; we route via
        // a shared mpsc channel. We accept an error here (e.g. if bot not yet ready)
        // since the initializer may fire before the bot is up.
        spawn_notify_forwarder(notify_rx, &config);
        (Some(tx), handles)
    } else {
        tracing::info!("watch.enabled=false — pollers not started");
        (None, vec![])
    };

    telegram::run(config, db).await
    // _stop_tx drops here → broadcasts shutdown to pollers
}

/// Drain the watch notification channel and send messages via Telegram.
/// This uses a simple reqwest call to avoid circular bot dependency in main.
fn spawn_notify_forwarder(mut notify_rx: NotifyReceiver, config: &AppConfig) {
    let bot_token = config.telegram.bot_token.clone();
    tokio::spawn(async move {
        while let Some((chat_id, text)) = notify_rx.recv().await {
            let url = format!("https://api.telegram.org/bot{}/sendMessage", bot_token);
            let body = serde_json::json!({
                "chat_id": chat_id,
                "text": text,
            });
            // Best-effort; ignore errors (bot may not be running yet).
            let _ = reqwest::Client::new().post(&url).json(&body).send().await;
        }
    });
}

async fn open_db(config: &AppConfig) -> anyhow::Result<Database> {
    let db = Database::open(&config.storage.db_path).await?;
    db.run_migrations().await?;
    Ok(db)
}

fn init_tracing(config: &AppConfig) -> anyhow::Result<()> {
    std::fs::create_dir_all(&config.storage.log_dir)
        .with_context(|| format!("create log dir {}", config.storage.log_dir.display()))?;

    let file_appender =
        tracing_appender::rolling::daily(&config.storage.log_dir, "gdclone-bot.log");
    let (file_writer, guard) = tracing_appender::non_blocking(file_appender);
    Box::leak(Box::new(guard));

    let filter =
        EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("gdclone_bot=info"));
    tracing_subscriber::registry()
        .with(filter)
        .with(tracing_subscriber::fmt::layer().with_writer(std::io::stdout))
        .with(
            tracing_subscriber::fmt::layer()
                .json()
                .with_writer(file_writer)
                .with_ansi(false),
        )
        .init();

    Ok(())
}
