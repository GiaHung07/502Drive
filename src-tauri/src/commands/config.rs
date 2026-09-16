use serde::{Deserialize, Serialize};
use std::fs;
use crate::commands::{get_config_path, get_db_path, get_project_dirs};

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct ConfigSummary {
    pub engine_concurrency: usize,
    pub auto_confirm_clone: bool,
    pub launch_at_startup: bool,
    pub language: String,
    pub bot_token_configured: bool,
    pub owner_telegram_id: i64,
    pub db_path: String,
    pub log_dir: String,
    pub report_dir: String,
}

#[tauri::command]
pub async fn get_config_summary() -> Result<ConfigSummary, String> {
    let cfg_path = get_config_path();
    let db_path = get_db_path();

    let mut concurrency = 8usize;
    let mut auto_confirm = true;
    let mut bot_token_configured = false;
    let mut owner_id = 0i64;

    if cfg_path.exists() {
        if let Ok(content) = fs::read_to_string(&cfg_path) {
            if let Ok(value) = content.parse::<toml::Table>() {
                if let Some(telegram) = value.get("telegram").and_then(|v| v.as_table()) {
                    if let Some(token) = telegram.get("bot_token").and_then(|v| v.as_str()) {
                        bot_token_configured = !token.is_empty();
                    }
                    if let Some(id) = telegram.get("owner_telegram_id").and_then(|v| v.as_integer()) {
                        owner_id = id;
                    }
                }
                if let Some(engine) = value.get("engine").and_then(|v| v.as_table()) {
                    if let Some(c) = engine.get("concurrency").and_then(|v| v.as_integer()) {
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
    let log_dir = dirs.as_ref().map(|d| d.data_dir().join("logs").display().to_string()).unwrap_or_default();
    let report_dir = dirs.as_ref().map(|d| d.data_dir().join("reports").display().to_string()).unwrap_or_default();

    Ok(ConfigSummary {
        engine_concurrency: concurrency,
        auto_confirm_clone: auto_confirm,
        launch_at_startup: true,
        language: "vi".to_string(),
        bot_token_configured,
        owner_telegram_id: owner_id,
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
                let engine = table.entry("engine").or_insert_with(|| toml::Value::Table(toml::Table::new()));
                if let Some(t) = engine.as_table_mut() {
                    t.insert("concurrency".to_string(), toml::Value::Integer(num.clamp(1, 32)));
                }
            }
        }
        "auto_confirm_clone" => {
            let b = value == "true" || value == "1";
            let engine = table.entry("engine").or_insert_with(|| toml::Value::Table(toml::Table::new()));
            if let Some(t) = engine.as_table_mut() {
                t.insert("auto_confirm_clone".to_string(), toml::Value::Boolean(b));
            }
        }
        _ => {}
    }

    if let Some(parent) = cfg_path.parent() {
        fs::create_dir_all(parent).map_err(|e| e.to_string())?;
    }
    fs::write(&cfg_path, toml::to_string_pretty(&table).map_err(|e| e.to_string())?)
        .map_err(|e| e.to_string())?;

    Ok(())
}
