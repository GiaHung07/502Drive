use std::{collections::HashMap, time::Duration};

use aes_gcm::{
    Aes256Gcm, Nonce,
    aead::{Aead, KeyInit, OsRng, rand_core::RngCore},
};
use anyhow::{Context, bail};
use base64::{Engine, engine::general_purpose::URL_SAFE_NO_PAD};
use oauth2::{AuthUrl, ClientId, CsrfToken, PkceCodeChallenge, RedirectUrl, Scope};
use reqwest::{Client, redirect::Policy};
use serde::Deserialize;
use sha2::{Digest, Sha256};
use tokio::{
    io::{AsyncReadExt, AsyncWriteExt},
    net::TcpListener,
};

use crate::config::GoogleOAuthConfig;

pub const DRIVE_SCOPE: &str = "https://www.googleapis.com/auth/drive";

#[derive(Debug, Clone)]
pub struct OAuthStart {
    pub auth_url: String,
    pub csrf_state: String,
    pub redirect_port: u16,
    pub pkce_verifier: String,
}

#[derive(Debug, Clone, Deserialize)]
pub struct TokenResponse {
    pub access_token: String,
    #[serde(default)]
    pub refresh_token: Option<String>,
    pub expires_in: Option<i64>,
    #[serde(default)]
    pub scope: Option<String>,
    #[serde(default)]
    pub token_type: Option<String>,
}

pub fn build_authorization_url(
    config: &GoogleOAuthConfig,
    redirect_port: u16,
) -> anyhow::Result<OAuthStart> {
    let redirect = format!("http://127.0.0.1:{redirect_port}/oauth2/callback");
    let (pkce_challenge, pkce_verifier) = PkceCodeChallenge::new_random_sha256();
    let client = oauth2::basic::BasicClient::new(ClientId::new(config.client_id.clone()))
        .set_auth_uri(AuthUrl::new(
            "https://accounts.google.com/o/oauth2/v2/auth".to_string(),
        )?)
        .set_redirect_uri(RedirectUrl::new(redirect)?);

    let (url, csrf) = client
        .authorize_url(CsrfToken::new_random)
        .add_scope(Scope::new(config.scope.clone()))
        .set_pkce_challenge(pkce_challenge)
        .add_extra_param("access_type", "offline")
        .add_extra_param("include_granted_scopes", "true")
        .add_extra_param("prompt", "consent")
        .url();

    Ok(OAuthStart {
        auth_url: url.to_string(),
        csrf_state: csrf.secret().to_string(),
        redirect_port,
        pkce_verifier: pkce_verifier.secret().to_string(),
    })
}

pub async fn choose_redirect_port(config: &GoogleOAuthConfig) -> anyhow::Result<u16> {
    for port in config.redirect_port_start..=config.redirect_port_end {
        if TcpListener::bind(("127.0.0.1", port)).await.is_ok() {
            return Ok(port);
        }
    }
    bail!(
        "no free OAuth redirect port in {}..={}",
        config.redirect_port_start,
        config.redirect_port_end
    )
}

pub async fn wait_for_loopback_code(port: u16, expected_state: &str) -> anyhow::Result<String> {
    let listener = TcpListener::bind(("127.0.0.1", port))
        .await
        .with_context(|| format!("bind OAuth loopback port {port}"))?;
    let (mut stream, _) = tokio::time::timeout(Duration::from_secs(300), listener.accept())
        .await
        .context("OAuth loopback timed out")??;

    let mut buffer = vec![0_u8; 8192];
    let n = stream.read(&mut buffer).await?;
    let request = String::from_utf8_lossy(&buffer[..n]);
    let first_line = request
        .lines()
        .next()
        .context("empty OAuth callback request")?;
    let path = first_line
        .split_whitespace()
        .nth(1)
        .context("malformed OAuth callback request")?;
    let callback_url = format!("http://127.0.0.1:{port}{path}");
    let url = url::Url::parse(&callback_url)?;
    let query: HashMap<_, _> = url.query_pairs().into_owned().collect();

    let response = b"HTTP/1.1 200 OK\r\nContent-Type: text/html; charset=utf-8\r\nConnection: close\r\n\r\n<!doctype html><title>gdclone-bot</title><h1>Authorization completed</h1><p>You may close this tab.</p>\n";
    stream.write_all(response).await?;

    if query.get("state").map(String::as_str) != Some(expected_state) {
        bail!("OAuth state mismatch");
    }
    query
        .get("code")
        .cloned()
        .context("OAuth callback missing code")
}

pub async fn exchange_code(
    config: &GoogleOAuthConfig,
    redirect_port: u16,
    code: &str,
    pkce_verifier: &str,
    timeout: Duration,
) -> anyhow::Result<TokenResponse> {
    let redirect_uri = format!("http://127.0.0.1:{redirect_port}/oauth2/callback");
    let response = oauth_http_client(timeout)?
        .post("https://oauth2.googleapis.com/token")
        .form(&[
            ("code", code),
            ("client_id", config.client_id.as_str()),
            ("client_secret", config.client_secret.as_str()),
            ("redirect_uri", redirect_uri.as_str()),
            ("grant_type", "authorization_code"),
            ("code_verifier", pkce_verifier),
        ])
        .send()
        .await?
        .error_for_status()?;

    Ok(response.json().await?)
}

#[derive(Debug, thiserror::Error)]
pub enum RefreshTokenError {
    #[error("Google OAuth refresh token is invalid or expired")]
    InvalidGrant,
    #[error("Google OAuth token endpoint returned {status}: {body}")]
    Endpoint {
        status: reqwest::StatusCode,
        body: String,
    },
    #[error(transparent)]
    Transport(#[from] anyhow::Error),
}

pub async fn refresh_access_token(
    config: &GoogleOAuthConfig,
    refresh_token: &str,
    timeout: Duration,
) -> Result<TokenResponse, RefreshTokenError> {
    let response = oauth_http_client(timeout)
        .map_err(RefreshTokenError::Transport)?
        .post("https://oauth2.googleapis.com/token")
        .form(&[
            ("client_id", config.client_id.as_str()),
            ("client_secret", config.client_secret.as_str()),
            ("refresh_token", refresh_token),
            ("grant_type", "refresh_token"),
        ])
        .send()
        .await
        .map_err(|err| RefreshTokenError::Transport(err.into()))?;

    let status = response.status();
    if status.is_success() {
        return response
            .json()
            .await
            .map_err(|err| RefreshTokenError::Transport(err.into()));
    }

    let body = response.text().await.unwrap_or_default();
    if body.contains("\"invalid_grant\"")
        || body.contains("\"unauthorized_client\"")
        || body.contains("\"invalid_client\"")
        || status == reqwest::StatusCode::UNAUTHORIZED
    {
        return Err(RefreshTokenError::InvalidGrant);
    }
    Err(RefreshTokenError::Endpoint { status, body })
}

pub async fn revoke_refresh_token(token: &str, timeout: Duration) -> anyhow::Result<()> {
    oauth_http_client(timeout)?
        .post("https://oauth2.googleapis.com/revoke")
        .form(&[("token", token)])
        .send()
        .await?
        .error_for_status()?;
    Ok(())
}

pub fn encrypt_token(plaintext: &str, master_key: &[u8; 32]) -> anyhow::Result<Vec<u8>> {
    let cipher = Aes256Gcm::new_from_slice(master_key).context("initialize token cipher")?;
    let mut nonce_bytes = [0_u8; 12];
    OsRng.fill_bytes(&mut nonce_bytes);
    let ciphertext = cipher
        .encrypt(Nonce::from_slice(&nonce_bytes), plaintext.as_bytes())
        .map_err(|_| anyhow::anyhow!("encrypt token"))?;
    let mut out = nonce_bytes.to_vec();
    out.extend(ciphertext);
    Ok(out)
}

pub fn decrypt_token(ciphertext: &[u8], master_key: &[u8; 32]) -> anyhow::Result<String> {
    if ciphertext.len() < 13 {
        bail!("encrypted token is too short");
    }
    let (nonce, body) = ciphertext.split_at(12);
    let cipher = Aes256Gcm::new_from_slice(master_key).context("initialize token cipher")?;
    let plaintext = cipher
        .decrypt(Nonce::from_slice(nonce), body)
        .map_err(|_| anyhow::anyhow!("decrypt token"))?;
    Ok(String::from_utf8(plaintext)?)
}

pub fn token_fingerprint(token: &str) -> String {
    let digest = Sha256::digest(token.as_bytes());
    URL_SAFE_NO_PAD.encode(&digest[..12])
}

fn oauth_http_client(timeout: Duration) -> anyhow::Result<Client> {
    Ok(Client::builder()
        .redirect(Policy::none())
        .timeout(timeout)
        .build()?)
}
