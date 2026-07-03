use std::{
    sync::Arc,
    time::{Duration, Instant},
};

use anyhow::{Context, bail};
use tokio::sync::Mutex;

use crate::{
    config::AppConfig,
    drive::auth::{self, RefreshTokenError},
    secrets::FileSecretStore,
    state::{db::Database, repo},
};

#[derive(Debug, Clone)]
pub struct AccessToken {
    value: String,
    expires_at: Instant,
}

impl AccessToken {
    pub fn as_str(&self) -> &str {
        &self.value
    }

    fn is_valid(&self) -> bool {
        Instant::now() + Duration::from_secs(60) < self.expires_at
    }
}

#[derive(Clone)]
pub struct TokenManager {
    config: AppConfig,
    db: Database,
    cache: Arc<Mutex<Option<AccessToken>>>,
    refresh_lock: Arc<Mutex<()>>,
}

impl TokenManager {
    pub fn new(config: AppConfig, db: Database) -> Self {
        Self {
            config,
            db,
            cache: Arc::new(Mutex::new(None)),
            refresh_lock: Arc::new(Mutex::new(())),
        }
    }

    pub async fn access_token(&self, account_id: &str) -> anyhow::Result<AccessToken> {
        if let Some(token) = self
            .cache
            .lock()
            .await
            .as_ref()
            .filter(|token| token.is_valid())
        {
            return Ok(token.clone());
        }

        let _guard = self.refresh_lock.lock().await;
        if let Some(token) = self
            .cache
            .lock()
            .await
            .as_ref()
            .filter(|token| token.is_valid())
        {
            return Ok(token.clone());
        }

        let account = repo::google_account_secret(&self.db, account_id)
            .await?
            .with_context(|| format!("Google account '{account_id}' is not connected"))?;
        if account.status != "connected" {
            bail!("Google account '{account_id}' is {}", account.status);
        }

        let secret_store = FileSecretStore::new(&self.config.storage.config_dir);
        let master_key = secret_store.load_or_create_key().await?;
        let refresh_token = auth::decrypt_token(&account.refresh_token_ciphertext, &master_key)?;

        let token = match auth::refresh_access_token(
            &self.config.google_oauth,
            &refresh_token,
            Duration::from_secs(self.config.engine.request_timeout_seconds),
        )
        .await
        {
            Ok(token) => token,
            Err(RefreshTokenError::InvalidGrant) => {
                repo::mark_account_reconnect_required(&self.db, account_id).await?;
                bail!("Google refresh token expired or was revoked. Run: gdclone-bot auth login");
            }
            Err(err) => return Err(err.into()),
        };

        let expires_in = token.expires_in.unwrap_or(3600).max(60) as u64;
        let access_token = AccessToken {
            value: token.access_token,
            expires_at: Instant::now() + Duration::from_secs(expires_in),
        };
        *self.cache.lock().await = Some(access_token.clone());
        Ok(access_token)
    }

    pub async fn invalidate(&self) {
        *self.cache.lock().await = None;
    }
}
