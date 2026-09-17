//! Google account services — disconnect flow shared by telegram and GUI.
//!
//! Extracted from the telegram `/disconnect` handler: status guard, token
//! decryption, OAuth revocation, and the repo status flip. Authorization
//! (owner check) and user-facing messaging stay with the callers.

use std::time::Duration;

use crate::config::AppConfig;
use crate::drive::auth as oauth;
use crate::secrets::FileSecretStore;
use crate::state::db::Database;
use crate::state::repo;

/// Outcome of [`AccountService::disconnect`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DisconnectOutcome {
    /// No Google account has ever been connected.
    NoAccount,
    /// The account exists but is not in a revocable state (`status`).
    AlreadyDisconnected { status: String },
    /// Refresh token revoked and the account marked `revoked`.
    Disconnected,
}

pub struct AccountService;

impl AccountService {
    /// Disconnect the (single, hardcoded `"default"`) Google account:
    /// decrypt the stored refresh token, revoke it at Google, and mark the
    /// account `revoked` in the repo.
    pub async fn disconnect(
        config: &AppConfig,
        db: &Database,
    ) -> anyhow::Result<DisconnectOutcome> {
        let Some(account) = repo::google_account_secret(db, "default").await? else {
            return Ok(DisconnectOutcome::NoAccount);
        };
        if account.status != "connected" && account.status != "reconnect_required" {
            return Ok(DisconnectOutcome::AlreadyDisconnected {
                status: account.status,
            });
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
        Ok(DisconnectOutcome::Disconnected)
    }
}
