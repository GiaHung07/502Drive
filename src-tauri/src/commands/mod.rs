pub mod config;
pub mod doctor;
pub mod features;
pub mod jobs;
pub mod logs;
pub mod service;
pub mod status;
pub mod watches;

use directories::ProjectDirs;
use std::path::PathBuf;

pub fn get_project_dirs() -> Option<ProjectDirs> {
    ProjectDirs::from("dev", "gdclone", "gdclone-bot")
}

pub fn get_db_path() -> PathBuf {
    if let Some(dirs) = get_project_dirs() {
        dirs.data_dir().join("state.db")
    } else {
        PathBuf::from("state.db")
    }
}

pub fn get_config_path() -> PathBuf {
    if let Some(dirs) = get_project_dirs() {
        dirs.config_dir().join("config.toml")
    } else {
        PathBuf::from("config.toml")
    }
}

pub fn get_log_path() -> PathBuf {
    if let Some(dirs) = get_project_dirs() {
        dirs.data_dir().join("logs").join("502drive.log")
    } else {
        PathBuf::from("502drive.log")
    }
}
