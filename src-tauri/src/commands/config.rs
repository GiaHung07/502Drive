use crate::commands::{get_config_path, get_db_path, get_project_dirs};
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::Path;

/// A value counts as configured when it is non-empty and not a `REPLACE_ME`
/// placeholder (matching the engine's `config.rs` validation).
pub(crate) fn is_configured(value: &str) -> bool {
    let trimmed = value.trim();
    !trimmed.is_empty() && !trimmed.starts_with("REPLACE_ME")
}

/// Mask a secret for display over IPC: keep the first 6 and last 4 characters
/// when the value is long enough, otherwise return a fixed marker so short
/// secrets are not (partially) revealed.
fn mask_secret(value: &str) -> String {
    let chars: Vec<char> = value.trim().chars().collect();
    if chars.len() < 12 {
        return "••••••••".to_string();
    }
    let head: String = chars[..6].iter().collect();
    let tail: String = chars[chars.len() - 4..].iter().collect();
    format!("{head}…{tail}")
}

/// Write the config file and restrict permissions to the owner on Unix so
/// bot tokens / OAuth secrets are not world- or group-readable.
pub(crate) fn write_config_file(path: &Path, contents: &str) -> Result<(), String> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).map_err(|e| e.to_string())?;
    }
    fs::write(path, contents).map_err(|e| e.to_string())?;
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        fs::set_permissions(path, fs::Permissions::from_mode(0o600)).map_err(|e| e.to_string())?;
    }
    Ok(())
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct ConfigSummary {
    pub engine_concurrency: usize,
    pub auto_confirm_clone: bool,
    pub launch_at_startup: bool,
    pub language: String,
    pub bot_token_configured: bool,
    pub bot_token: Option<String>,
    pub owner_telegram_id: i64,
    pub oauth_client_id: Option<String>,
    pub oauth_client_secret_configured: bool,
    pub db_path: String,
    pub log_dir: String,
    pub report_dir: String,
}

#[derive(Debug, Deserialize)]
pub struct WizardConfigInput {
    pub oauth_client_id: Option<String>,
    pub oauth_client_secret: Option<String>,
    pub bot_token: Option<String>,
    pub owner_telegram_id: Option<i64>,
    pub engine_concurrency: Option<usize>,
    pub auto_confirm_clone: Option<bool>,
}

#[derive(Debug, Serialize)]
pub struct TelegramBotCheckResult {
    pub ok: bool,
    pub username: Option<String>,
    pub first_name: Option<String>,
    pub bot_id: Option<i64>,
    pub error: Option<String>,
}

#[tauri::command]
pub async fn get_config_summary() -> Result<ConfigSummary, String> {
    let cfg_path = get_config_path();
    let db_path = get_db_path();

    // Matches the engine's effective write-semaphore default
    // (config.sample.toml: max_write_concurrency = 8).
    let mut concurrency = 8usize;
    let mut auto_confirm = true;
    let mut bot_token_configured = false;
    let mut bot_token: Option<String> = None;
    let mut owner_id = 0i64;
    let mut oauth_client_id: Option<String> = None;
    let mut oauth_client_secret_configured = false;

    if cfg_path.exists() {
        if let Ok(content) = fs::read_to_string(&cfg_path) {
            if let Ok(value) = content.parse::<toml::Table>() {
                if let Some(telegram) = value.get("telegram").and_then(|v| v.as_table()) {
                    if let Some(token) = telegram.get("bot_token").and_then(|v| v.as_str()) {
                        bot_token_configured = is_configured(token);
                        if bot_token_configured {
                            // Never ship the raw token to the webview — masked display only.
                            bot_token = Some(mask_secret(token));
                        }
                    }
                    if let Some(id) = telegram
                        .get("owner_telegram_id")
                        .and_then(|v| v.as_integer())
                    {
                        owner_id = id;
                    }
                }
                if let Some(oauth) = value.get("google_oauth").and_then(|v| v.as_table()) {
                    if let Some(cid) = oauth.get("client_id").and_then(|v| v.as_str()) {
                        if is_configured(cid) {
                            oauth_client_id = Some(cid.to_string());
                        }
                    }
                    if let Some(sec) = oauth.get("client_secret").and_then(|v| v.as_str()) {
                        oauth_client_secret_configured = is_configured(sec);
                    }
                }
                if let Some(engine) = value.get("engine").and_then(|v| v.as_table()) {
                    // The engine's write semaphore resolves from
                    // max_write_concurrency; `concurrency` is the legacy GUI key.
                    let engine_concurrency = engine
                        .get("max_write_concurrency")
                        .or_else(|| engine.get("concurrency"))
                        .and_then(|v| v.as_integer());
                    if let Some(c) = engine_concurrency {
                        concurrency = c.clamp(1, 32) as usize;
                    }
                    if let Some(b) = engine.get("auto_confirm_clone").and_then(|v| v.as_bool()) {
                        auto_confirm = b;
                    }
                }
            }
        }
    }

    let dirs = get_project_dirs();
    let log_dir = dirs
        .as_ref()
        .map(|d| d.data_dir().join("logs").display().to_string())
        .unwrap_or_default();
    let report_dir = dirs
        .as_ref()
        .map(|d| d.data_dir().join("reports").display().to_string())
        .unwrap_or_default();

    Ok(ConfigSummary {
        engine_concurrency: concurrency,
        auto_confirm_clone: auto_confirm,
        launch_at_startup: true,
        language: "vi".to_string(),
        bot_token_configured,
        bot_token,
        owner_telegram_id: owner_id,
        oauth_client_id,
        oauth_client_secret_configured,
        db_path: db_path.display().to_string(),
        log_dir,
        report_dir,
    })
}

#[tauri::command]
pub async fn update_config_field(field: String, value: String) -> Result<(), String> {
    let cfg_path = get_config_path();
    let content = if cfg_path.exists() {
        fs::read_to_string(&cfg_path).unwrap_or_default()
    } else {
        String::new()
    };

    let mut table: toml::Table = content.parse().unwrap_or_default();

    match field.as_str() {
        "engine_concurrency" => {
            if let Ok(num) = value.parse::<i64>() {
                let engine = table
                    .entry("engine")
                    .or_insert_with(|| toml::Value::Table(toml::Table::new()));
                if let Some(t) = engine.as_table_mut() {
                    t.insert(
                        "concurrency".to_string(),
                        toml::Value::Integer(num.clamp(1, 32)),
                    );
                    t.insert(
                        "max_write_concurrency".to_string(),
                        toml::Value::Integer(num.clamp(1, 32)),
                    );
                }
            }
        }
        "auto_confirm_clone" => {
            let b = value == "true" || value == "1";
            let engine = table
                .entry("engine")
                .or_insert_with(|| toml::Value::Table(toml::Table::new()));
            if let Some(t) = engine.as_table_mut() {
                t.insert("auto_confirm_clone".to_string(), toml::Value::Boolean(b));
            }
        }
        "owner_telegram_id" => {
            if let Ok(id) = value.parse::<i64>() {
                let tg = table
                    .entry("telegram")
                    .or_insert_with(|| toml::Value::Table(toml::Table::new()));
                if let Some(t) = tg.as_table_mut() {
                    t.insert("owner_telegram_id".to_string(), toml::Value::Integer(id));
                }
            }
        }
        "bot_token" => {
            let tg = table
                .entry("telegram")
                .or_insert_with(|| toml::Value::Table(toml::Table::new()));
            if let Some(t) = tg.as_table_mut() {
                t.insert(
                    "bot_token".to_string(),
                    toml::Value::String(value.trim().to_string()),
                );
            }
        }
        _ => {}
    }

    write_config_file(
        &cfg_path,
        &toml::to_string_pretty(&table).map_err(|e| e.to_string())?,
    )?;

    Ok(())
}

#[tauri::command]
pub async fn save_wizard_config(input: WizardConfigInput) -> Result<(), String> {
    let cfg_path = get_config_path();
    let content = if cfg_path.exists() {
        fs::read_to_string(&cfg_path).unwrap_or_default()
    } else {
        String::new()
    };

    let mut table: toml::Table = content.parse().unwrap_or_default();

    if input.oauth_client_id.is_some() || input.oauth_client_secret.is_some() {
        let oauth = table
            .entry("google_oauth")
            .or_insert_with(|| toml::Value::Table(toml::Table::new()));
        if let Some(t) = oauth.as_table_mut() {
            if let Some(cid) = input.oauth_client_id {
                let trimmed = cid.trim().to_string();
                if !trimmed.is_empty() {
                    t.insert("client_id".to_string(), toml::Value::String(trimmed));
                }
            }
            if let Some(csec) = input.oauth_client_secret {
                let trimmed = csec.trim().to_string();
                if !trimmed.is_empty() {
                    t.insert("client_secret".to_string(), toml::Value::String(trimmed));
                }
            }
            if !t.contains_key("redirect_port_start") {
                t.insert(
                    "redirect_port_start".to_string(),
                    toml::Value::Integer(51000),
                );
            }
            if !t.contains_key("redirect_port_end") {
                t.insert("redirect_port_end".to_string(), toml::Value::Integer(51100));
            }
            if !t.contains_key("scope") {
                t.insert(
                    "scope".to_string(),
                    toml::Value::String("https://www.googleapis.com/auth/drive".to_string()),
                );
            }
        }
    }

    if input.bot_token.is_some() || input.owner_telegram_id.is_some() {
        let tg = table
            .entry("telegram")
            .or_insert_with(|| toml::Value::Table(toml::Table::new()));
        if let Some(t) = tg.as_table_mut() {
            if let Some(tok) = input.bot_token {
                let trimmed = tok.trim().to_string();
                if !trimmed.is_empty() {
                    t.insert("bot_token".to_string(), toml::Value::String(trimmed));
                }
            }
            if let Some(oid) = input.owner_telegram_id {
                t.insert("owner_telegram_id".to_string(), toml::Value::Integer(oid));
            }
        }
    }

    if let Some(concurrency) = input.engine_concurrency {
        let eng = table
            .entry("engine")
            .or_insert_with(|| toml::Value::Table(toml::Table::new()));
        if let Some(t) = eng.as_table_mut() {
            t.insert(
                "concurrency".to_string(),
                toml::Value::Integer(concurrency.clamp(1, 32) as i64),
            );
            t.insert(
                "max_write_concurrency".to_string(),
                toml::Value::Integer(concurrency.clamp(1, 32) as i64),
            );
        }
    }

    if let Some(auto_confirm) = input.auto_confirm_clone {
        let eng = table
            .entry("engine")
            .or_insert_with(|| toml::Value::Table(toml::Table::new()));
        if let Some(t) = eng.as_table_mut() {
            t.insert(
                "auto_confirm_clone".to_string(),
                toml::Value::Boolean(auto_confirm),
            );
        }
    }

    write_config_file(
        &cfg_path,
        &toml::to_string_pretty(&table).map_err(|e| e.to_string())?,
    )?;

    Ok(())
}

#[tauri::command]
pub async fn verify_telegram_bot(token: String) -> Result<TelegramBotCheckResult, String> {
    let trimmed = token.trim();
    if trimmed.is_empty() {
        return Ok(TelegramBotCheckResult {
            ok: false,
            username: None,
            first_name: None,
            bot_id: None,
            error: Some("Bot token không được để trống".to_string()),
        });
    }

    let url = format!("https://api.telegram.org/bot{trimmed}/getMe");
    let output = std::process::Command::new("curl")
        .args(["-s", "--max-time", "6", &url])
        .output();

    match output {
        Ok(out) => {
            let body = String::from_utf8_lossy(&out.stdout);
            if let Ok(v) = serde_json::from_str::<serde_json::Value>(&body) {
                if v.get("ok").and_then(|b| b.as_bool()) == Some(true) {
                    let res = v.get("result");
                    let username = res
                        .and_then(|r| r.get("username"))
                        .and_then(|u| u.as_str())
                        .map(String::from);
                    let first_name = res
                        .and_then(|r| r.get("first_name"))
                        .and_then(|f| f.as_str())
                        .map(String::from);
                    let bot_id = res.and_then(|r| r.get("id")).and_then(|i| i.as_i64());
                    return Ok(TelegramBotCheckResult {
                        ok: true,
                        username,
                        first_name,
                        bot_id,
                        error: None,
                    });
                } else {
                    let desc = v
                        .get("description")
                        .and_then(|d| d.as_str())
                        .unwrap_or("Token không hợp lệ hoặc bot không tồn tại");
                    return Ok(TelegramBotCheckResult {
                        ok: false,
                        username: None,
                        first_name: None,
                        bot_id: None,
                        error: Some(desc.to_string()),
                    });
                }
            }
            Ok(TelegramBotCheckResult {
                ok: false,
                username: None,
                first_name: None,
                bot_id: None,
                error: Some("Không thể phân tích phản hồi từ máy chủ Telegram".to_string()),
            })
        }
        Err(e) => Ok(TelegramBotCheckResult {
            ok: false,
            username: None,
            first_name: None,
            bot_id: None,
            error: Some(format!("Lỗi gọi curl: {e}")),
        }),
    }
}
