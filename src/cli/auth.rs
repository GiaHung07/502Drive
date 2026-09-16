use std::time::Duration;

use anyhow::{Context, bail};

use crate::{
    config::AppConfig,
    drive::{auth as oauth, client::DriveClient},
    secrets::FileSecretStore,
    state::{db::Database, repo},
};

pub async fn login(config: &AppConfig, db: &Database) -> anyhow::Result<()> {
    config.validate_for_auth()?;
    let port = oauth::choose_redirect_port(&config.google_oauth).await?;
    let start = oauth::build_authorization_url(&config.google_oauth, port)?;

    println!("Open this URL on this machine to authorize 502drive:\n");
    println!("{}", start.auth_url);
    if let Err(err) = open::that(&start.auth_url) {
        eprintln!("Could not open browser automatically: {err}");
    }

    let code = oauth::wait_for_loopback_code(start.redirect_port, &start.csrf_state).await?;
    let token = oauth::exchange_code(
        &config.google_oauth,
        start.redirect_port,
        &code,
        &start.pkce_verifier,
        Duration::from_secs(config.engine.request_timeout_seconds),
    )
    .await?;
    let refresh_token = token
        .refresh_token
        .as_deref()
        .context("Google did not return a refresh token; revoke app access and retry login")?;

    let drive =
        DriveClient::with_timeout(Duration::from_secs(config.engine.request_timeout_seconds));
    let about = drive.about_get(&token.access_token).await?;

    let secret_store = FileSecretStore::new(&config.storage.config_dir);
    let master_key = secret_store.load_or_create_key().await?;
    let ciphertext = oauth::encrypt_token(refresh_token, &master_key)?;
    let scopes_json = serde_json::to_string(&vec![
        token
            .scope
            .unwrap_or_else(|| config.google_oauth.scope.clone()),
    ])?;
    let label = about
        .user
        .as_ref()
        .and_then(|user| user.email_address.as_deref())
        .unwrap_or("default");
    repo::upsert_google_account(
        db,
        label,
        about
            .user
            .as_ref()
            .and_then(|user| user.email_address.as_deref()),
        ciphertext,
        secret_store.backend_name(),
        &scopes_json,
    )
    .await?;

    println!("Google account connected: {label}");
    Ok(())
}

pub async fn status(db: &Database) -> anyhow::Result<()> {
    match repo::account_status(db).await? {
        Some(status) => println!("Google account status: {status}"),
        None => println!("No Google account connected. Run: 502drive auth login"),
    }
    Ok(())
}

pub async fn revoke(config: &AppConfig, db: &Database) -> anyhow::Result<()> {
    let Some(account) = repo::google_account_secret(db, "default").await? else {
        println!("No Google account connected.");
        return Ok(());
    };
    if account.status != "connected" && account.status != "reconnect_required" {
        bail!("Google account is already {}", account.status);
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
    println!("Google refresh token revoked and local account marked revoked.");
    Ok(())
}
