use std::{
    env,
    path::{Path, PathBuf},
};

use anyhow::{Context, bail};
use directories::ProjectDirs;
use serde::Deserialize;

#[derive(Debug, Clone, Deserialize)]
pub struct AppConfig {
    pub telegram: TelegramConfig,
    pub destination: DestinationConfig,
    pub google_oauth: GoogleOAuthConfig,
    pub engine: EngineConfig,
    pub watch: WatchConfig,
    pub storage: StorageConfig,
    pub security: SecurityConfig,
    pub platform: PlatformConfig,
}

#[derive(Debug, Clone, Deserialize)]
pub struct TelegramConfig {
    pub bot_token: String,
    pub owner_telegram_id: i64,
    pub progress_edit_min_interval_ms: u64,
    #[serde(default = "default_telegram_language")]
    pub language: String,
}

#[derive(Debug, Clone, Deserialize)]
pub struct DestinationConfig {
    pub auto_use_default: bool,
    pub auto_confirm_clone: bool,
    pub wrap_single_file_in_folder: bool,
    pub root_name_policy: String,
    pub same_name_policy: String,
}

#[derive(Debug, Clone, Deserialize)]
pub struct GoogleOAuthConfig {
    pub client_id: String,
    pub client_secret: String,
    pub redirect_port_start: u16,
    pub redirect_port_end: u16,
    pub scope: String,
}

#[derive(Debug, Clone, Deserialize)]
pub struct EngineConfig {
    pub max_active_jobs: usize,
    pub max_active_jobs_per_user: usize,
    pub initial_write_concurrency: usize,
    pub min_write_concurrency: usize,
    pub max_write_concurrency: usize,
    pub list_concurrency: usize,
    pub max_retry_attempts: u32,
    pub retry_base_delay_ms: u64,
    pub retry_max_delay_ms: u64,
    pub request_timeout_seconds: u64,
    pub default_duplicate_policy: String,
    pub default_shortcut_policy: String,
}

#[derive(Debug, Clone, Deserialize)]
pub struct WatchConfig {
    pub enabled: bool,
    #[serde(default = "default_active_poll_seconds")]
    pub active_poll_seconds: u64,
    /// Minimum interval (ms) between fallback source scans per watch
    /// (`scan_missing_children`). The scan also runs once right after a watch
    /// finishes initialization; between scans it is gated by this knob.
    #[serde(default = "default_scan_interval_ms")]
    pub scan_interval_ms: u64,
    pub warm_idle_poll_seconds: u64,
    pub cold_idle_poll_seconds: u64,
    pub warm_idle_after_seconds: u64,
    pub cold_idle_after_seconds: u64,
    pub max_backlog_events_per_watch: u64,
    pub raw_event_retention_days: u64,
    pub default_content_update_policy: String,
    pub default_deletion_policy: String,
    pub default_move_out_policy: String,
}

#[derive(Debug, Clone, Deserialize)]
pub struct StorageConfig {
    pub db_path: PathBuf,
    pub log_dir: PathBuf,
    pub report_dir: PathBuf,
    pub secret_backend: String,
    #[serde(skip)]
    pub config_dir: PathBuf,
}

#[derive(Debug, Clone, Deserialize)]
pub struct SecurityConfig {
    pub redact_file_names_in_info_logs: bool,
    pub report_retention_days: u64,
    pub allow_operators: bool,
}

#[derive(Debug, Clone, Deserialize)]
pub struct PlatformConfig {
    pub startup_mode: String,
}

impl AppConfig {
    pub fn load(path: &Path) -> anyhow::Result<Self> {
        let raw = std::fs::read_to_string(path)
            .with_context(|| format!("read config {}", path.display()))?;
        let mut config: AppConfig =
            toml::from_str(&raw).with_context(|| format!("parse config {}", path.display()))?;

        apply_env_overrides(&mut config)?;
        config.resolve_paths(path)?;
        config.validate_shape()?;

        Ok(config)
    }

    pub fn validate_for_run(&self) -> anyhow::Result<()> {
        self.validate_telegram()?;
        self.validate_oauth_client()?;
        if self.telegram.owner_telegram_id <= 0 {
            bail!("telegram.owner_telegram_id must be configured");
        }
        Ok(())
    }

    pub fn validate_for_auth(&self) -> anyhow::Result<()> {
        self.validate_oauth_client()
    }

    fn validate_telegram(&self) -> anyhow::Result<()> {
        if self.telegram.bot_token.trim().is_empty() || self.telegram.bot_token == "REPLACE_ME" {
            bail!("telegram.bot_token is not configured");
        }
        Ok(())
    }

    fn validate_oauth_client(&self) -> anyhow::Result<()> {
        if self.google_oauth.client_id.trim().is_empty()
            || self.google_oauth.client_id == "REPLACE_ME.apps.googleusercontent.com"
        {
            bail!("google_oauth.client_id is not configured");
        }
        if self.google_oauth.client_secret.trim().is_empty()
            || self.google_oauth.client_secret == "REPLACE_ME"
        {
            bail!("google_oauth.client_secret is not configured");
        }
        Ok(())
    }

    fn validate_shape(&self) -> anyhow::Result<()> {
        if self.google_oauth.redirect_port_start > self.google_oauth.redirect_port_end {
            bail!("google_oauth.redirect_port_start must be <= redirect_port_end");
        }
        if self.telegram.progress_edit_min_interval_ms == 0 {
            bail!("telegram.progress_edit_min_interval_ms must be > 0");
        }
        validate_choice("telegram.language", &self.telegram.language, &["vi", "en"])?;
        validate_choice(
            "destination.root_name_policy",
            &self.destination.root_name_policy,
            &["preserve"],
        )?;
        validate_choice(
            "destination.same_name_policy",
            &self.destination.same_name_policy,
            &["keep_both", "reuse_selected", "append_suffix"],
        )?;
        if self.engine.min_write_concurrency == 0
            || self.engine.initial_write_concurrency == 0
            || self.engine.max_write_concurrency < self.engine.min_write_concurrency
            || self.engine.initial_write_concurrency > self.engine.max_write_concurrency
        {
            bail!("engine write concurrency bounds are invalid");
        }
        if self.engine.list_concurrency == 0 {
            bail!("engine.list_concurrency must be > 0");
        }
        if self.engine.max_active_jobs == 0 || self.engine.max_active_jobs_per_user == 0 {
            bail!("engine active job limits must be > 0");
        }
        if self.engine.max_retry_attempts == 0 {
            bail!("engine.max_retry_attempts must be > 0");
        }
        if self.engine.retry_base_delay_ms == 0 || self.engine.retry_max_delay_ms == 0 {
            bail!("engine retry delays must be > 0");
        }
        if self.engine.retry_base_delay_ms > self.engine.retry_max_delay_ms {
            bail!("engine.retry_base_delay_ms must be <= retry_max_delay_ms");
        }
        if self.engine.request_timeout_seconds == 0 {
            bail!("engine.request_timeout_seconds must be > 0");
        }
        if self.watch.active_poll_seconds == 0 || self.watch.scan_interval_ms == 0 {
            bail!("watch.active_poll_seconds and watch.scan_interval_ms must be > 0");
        }
        validate_choice(
            "engine.default_duplicate_policy",
            &self.engine.default_duplicate_policy,
            &["keep_both", "skip_same_source", "replace_safe"],
        )?;
        validate_choice(
            "engine.default_shortcut_policy",
            &self.engine.default_shortcut_policy,
            &["preserve", "remap_internal", "resolve"],
        )?;
        validate_choice(
            "watch.default_content_update_policy",
            &self.watch.default_content_update_policy,
            &["versioned_copy", "replace_copy", "manual_confirmation"],
        )?;
        validate_choice(
            "watch.default_deletion_policy",
            &self.watch.default_deletion_policy,
            &["preserve_destination", "manual_confirmation"],
        )?;
        validate_choice(
            "watch.default_move_out_policy",
            &self.watch.default_move_out_policy,
            &["detach", "keep_following"],
        )?;
        validate_choice(
            "storage.secret_backend",
            &self.storage.secret_backend,
            &["auto"],
        )?;
        validate_choice(
            "platform.startup_mode",
            &self.platform.startup_mode,
            &["manual"],
        )?;
        Ok(())
    }

    fn resolve_paths(&mut self, config_path: &Path) -> anyhow::Result<()> {
        // The secret-store directory follows the config file's parent: for the
        // default desktop path this equals the ProjectDirs config dir, and for
        // container deployments (--config /config/config.toml) it keeps
        // master.key on the mounted volume instead of the ephemeral container
        // home. Fall back to ProjectDirs for bare relative filenames.
        let explicit_parent = config_path
            .parent()
            .filter(|p| !p.as_os_str().is_empty())
            .map(|p| p.to_path_buf());
        self.storage.config_dir = match explicit_parent {
            Some(parent) => parent,
            None => project_dirs()?.config_dir().to_path_buf(),
        };
        let dirs = project_dirs()?;

        if self.storage.db_path.as_os_str().is_empty() {
            self.storage.db_path = dirs.data_dir().join("state.db");
        } else {
            self.storage.db_path = resolve_path(&self.storage.db_path)?;
        }
        if self.storage.log_dir.as_os_str().is_empty() {
            self.storage.log_dir = dirs.data_dir().join("logs");
        } else {
            self.storage.log_dir = resolve_path(&self.storage.log_dir)?;
        }
        if self.storage.report_dir.as_os_str().is_empty() {
            self.storage.report_dir = dirs.data_dir().join("reports");
        } else {
            self.storage.report_dir = resolve_path(&self.storage.report_dir)?;
        }
        Ok(())
    }
}

fn validate_choice(field: &str, value: &str, allowed: &[&str]) -> anyhow::Result<()> {
    if allowed.contains(&value) {
        Ok(())
    } else {
        bail!("{field} must be one of: {}", allowed.join(", "))
    }
}

fn apply_env_overrides(config: &mut AppConfig) -> anyhow::Result<()> {
    env_string(
        "GDCLONE__TELEGRAM__BOT_TOKEN",
        &mut config.telegram.bot_token,
    );
    env_parse(
        "GDCLONE__TELEGRAM__OWNER_TELEGRAM_ID",
        &mut config.telegram.owner_telegram_id,
    )?;
    env_parse(
        "GDCLONE__TELEGRAM__PROGRESS_EDIT_MIN_INTERVAL_MS",
        &mut config.telegram.progress_edit_min_interval_ms,
    )?;
    env_string("GDCLONE__TELEGRAM__LANGUAGE", &mut config.telegram.language);
    env_parse(
        "GDCLONE__DESTINATION__AUTO_CONFIRM_CLONE",
        &mut config.destination.auto_confirm_clone,
    )?;
    env_parse(
        "GDCLONE__DESTINATION__WRAP_SINGLE_FILE_IN_FOLDER",
        &mut config.destination.wrap_single_file_in_folder,
    )?;
    env_string(
        "GDCLONE__GOOGLE_OAUTH__CLIENT_ID",
        &mut config.google_oauth.client_id,
    );
    env_string(
        "GDCLONE__GOOGLE_OAUTH__CLIENT_SECRET",
        &mut config.google_oauth.client_secret,
    );
    env_string(
        "GDCLONE__GOOGLE_OAUTH__SCOPE",
        &mut config.google_oauth.scope,
    );
    env_parse(
        "GDCLONE__ENGINE__MAX_ACTIVE_JOBS",
        &mut config.engine.max_active_jobs,
    )?;
    env_parse(
        "GDCLONE__ENGINE__MAX_ACTIVE_JOBS_PER_USER",
        &mut config.engine.max_active_jobs_per_user,
    )?;
    env_parse(
        "GDCLONE__ENGINE__INITIAL_WRITE_CONCURRENCY",
        &mut config.engine.initial_write_concurrency,
    )?;
    env_parse(
        "GDCLONE__ENGINE__MAX_RETRY_ATTEMPTS",
        &mut config.engine.max_retry_attempts,
    )?;
    env_parse(
        "GDCLONE__ENGINE__RETRY_BASE_DELAY_MS",
        &mut config.engine.retry_base_delay_ms,
    )?;
    env_parse(
        "GDCLONE__ENGINE__RETRY_MAX_DELAY_MS",
        &mut config.engine.retry_max_delay_ms,
    )?;
    env_parse(
        "GDCLONE__ENGINE__REQUEST_TIMEOUT_SECONDS",
        &mut config.engine.request_timeout_seconds,
    )?;
    env_path("GDCLONE__STORAGE__DB_PATH", &mut config.storage.db_path);
    env_path("GDCLONE__STORAGE__LOG_DIR", &mut config.storage.log_dir);
    env_path(
        "GDCLONE__STORAGE__REPORT_DIR",
        &mut config.storage.report_dir,
    );
    Ok(())
}

fn default_telegram_language() -> String {
    "vi".to_string()
}

fn default_active_poll_seconds() -> u64 {
    10
}

fn default_scan_interval_ms() -> u64 {
    600_000
}

fn env_string(key: &str, target: &mut String) {
    if let Ok(value) = env::var(key) {
        *target = value;
    }
}

fn env_parse<T>(key: &str, target: &mut T) -> anyhow::Result<()>
where
    T: std::str::FromStr,
    T::Err: std::error::Error + Send + Sync + 'static,
{
    if let Ok(value) = env::var(key) {
        *target = value.parse().with_context(|| format!("parse {key}"))?;
    }
    Ok(())
}

fn env_path(key: &str, target: &mut PathBuf) {
    if let Ok(value) = env::var(key) {
        *target = PathBuf::from(value);
    }
}

pub fn default_data_dir() -> anyhow::Result<PathBuf> {
    Ok(project_dirs()?.data_dir().to_path_buf())
}

pub fn default_config_path() -> anyhow::Result<PathBuf> {
    Ok(project_dirs()?.config_dir().join("config.toml"))
}

pub fn project_dirs() -> anyhow::Result<ProjectDirs> {
    ProjectDirs::from("dev", "gdclone", "gdclone-bot")
        .context("cannot resolve platform project directories")
}

pub fn resolve_path(path: &Path) -> anyhow::Result<PathBuf> {
    let text = path.to_string_lossy();
    if text == "~" || text.starts_with("~/") {
        let home = directories::BaseDirs::new()
            .context("cannot resolve home directory")?
            .home_dir()
            .to_path_buf();
        if text == "~" {
            return Ok(home);
        }
        return Ok(home.join(&text[2..]));
    }
    if path.is_absolute() {
        Ok(path.to_path_buf())
    } else {
        Ok(env::current_dir()?.join(path))
    }
}
