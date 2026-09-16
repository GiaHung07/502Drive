use std::{
    fs,
    path::Path,
    sync::{Mutex, MutexGuard},
};

use gdclone_bot::config::{AppConfig, default_config_path};
use uuid::Uuid;

static ENV_LOCK: Mutex<()> = Mutex::new(());
const CONFIG_ENV_KEYS: &[&str] = &[
    "GDCLONE__TELEGRAM__BOT_TOKEN",
    "GDCLONE__TELEGRAM__OWNER_TELEGRAM_ID",
    "GDCLONE__TELEGRAM__PROGRESS_EDIT_MIN_INTERVAL_MS",
    "GDCLONE__TELEGRAM__LANGUAGE",
    "GDCLONE__DESTINATION__AUTO_CONFIRM_CLONE",
    "GDCLONE__DESTINATION__WRAP_SINGLE_FILE_IN_FOLDER",
    "GDCLONE__GOOGLE_OAUTH__CLIENT_ID",
    "GDCLONE__GOOGLE_OAUTH__CLIENT_SECRET",
    "GDCLONE__GOOGLE_OAUTH__SCOPE",
    "GDCLONE__ENGINE__MAX_ACTIVE_JOBS",
    "GDCLONE__ENGINE__MAX_ACTIVE_JOBS_PER_USER",
    "GDCLONE__ENGINE__INITIAL_WRITE_CONCURRENCY",
    "GDCLONE__ENGINE__MAX_RETRY_ATTEMPTS",
    "GDCLONE__ENGINE__RETRY_BASE_DELAY_MS",
    "GDCLONE__ENGINE__RETRY_MAX_DELAY_MS",
    "GDCLONE__ENGINE__REQUEST_TIMEOUT_SECONDS",
    "GDCLONE__STORAGE__DB_PATH",
    "GDCLONE__STORAGE__LOG_DIR",
    "GDCLONE__STORAGE__REPORT_DIR",
];

#[test]
fn rejects_invalid_policy_values() {
    let _guard = EnvGuard::new(&[]);
    let path = temp_config_path();
    fs::write(
        &path,
        sample_config().replace(
            "same_name_policy = \"keep_both\"",
            "same_name_policy = \"merge_by_name\"",
        ),
    )
    .unwrap();

    let error = AppConfig::load(&path).unwrap_err().to_string();
    assert!(error.contains("destination.same_name_policy"));

    let _ = fs::remove_file(path);
}

#[test]
fn sample_config_shape_loads() {
    let _guard = EnvGuard::new(&[]);
    let path = temp_config_path();
    fs::write(&path, sample_config()).unwrap();

    let config = AppConfig::load(&path).unwrap();
    assert_eq!(config.engine.default_duplicate_policy, "skip_same_source");
    assert_eq!(config.telegram.language, "vi");

    let _ = fs::remove_file(path);
}

#[test]
fn default_config_path_uses_project_config_dir() {
    let path = default_config_path().unwrap();
    assert_eq!(
        path.file_name().and_then(|v| v.to_str()),
        Some("config.toml")
    );
    assert!(path.components().any(|c| c.as_os_str() == "gdclone-bot"));
}

#[test]
fn env_overrides_operational_fields() {
    let _guard = EnvGuard::new(&[
        ("GDCLONE__TELEGRAM__PROGRESS_EDIT_MIN_INTERVAL_MS", "1500"),
        ("GDCLONE__TELEGRAM__LANGUAGE", "en"),
        ("GDCLONE__DESTINATION__AUTO_CONFIRM_CLONE", "true"),
        ("GDCLONE__ENGINE__INITIAL_WRITE_CONCURRENCY", "3"),
        ("GDCLONE__ENGINE__MAX_RETRY_ATTEMPTS", "4"),
        ("GDCLONE__STORAGE__DB_PATH", "/tmp/gdclone-env-test.db"),
    ]);
    let path = temp_config_path();
    fs::write(&path, sample_config()).unwrap();

    let config = AppConfig::load(&path).unwrap();
    assert_eq!(config.telegram.progress_edit_min_interval_ms, 1500);
    assert_eq!(config.telegram.language, "en");
    assert!(config.destination.auto_confirm_clone);
    assert_eq!(config.engine.initial_write_concurrency, 3);
    assert_eq!(config.engine.max_retry_attempts, 4);
    assert_eq!(
        config.storage.db_path,
        Path::new("/tmp/gdclone-env-test.db")
    );

    let _ = fs::remove_file(path);
}

fn temp_config_path() -> std::path::PathBuf {
    std::env::temp_dir().join(format!("gdclone-config-test-{}.toml", Uuid::new_v4()))
}

fn sample_config() -> String {
    fs::read_to_string(Path::new(env!("CARGO_MANIFEST_DIR")).join("config.sample.toml")).unwrap()
}

struct EnvGuard {
    previous: Vec<(String, Option<String>)>,
    _guard: MutexGuard<'static, ()>,
}

impl EnvGuard {
    fn new(values: &[(&str, &str)]) -> Self {
        let guard = ENV_LOCK.lock().unwrap();
        let previous = CONFIG_ENV_KEYS
            .iter()
            .map(|key| ((*key).to_string(), std::env::var(key).ok()))
            .collect::<Vec<_>>();
        for key in CONFIG_ENV_KEYS {
            // SAFETY: These tests serialize environment access with ENV_LOCK.
            unsafe {
                std::env::remove_var(key);
            }
        }
        for (key, value) in values {
            // SAFETY: These tests serialize environment access with ENV_LOCK.
            unsafe {
                std::env::set_var(key, value);
            }
        }
        Self {
            previous,
            _guard: guard,
        }
    }
}

impl Drop for EnvGuard {
    fn drop(&mut self) {
        for (key, value) in &self.previous {
            // SAFETY: These tests serialize environment access with ENV_LOCK.
            unsafe {
                match value {
                    Some(value) => std::env::set_var(key, value),
                    None => std::env::remove_var(key),
                }
            }
        }
    }
}
