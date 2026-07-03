use std::path::{Path, PathBuf};

use crate::config;

pub trait Platform: Send + Sync {
    fn machine_key_material(&self) -> anyhow::Result<Vec<u8>>;
    fn default_data_dir(&self) -> anyhow::Result<PathBuf> {
        config::default_data_dir()
    }
    fn install_service(&self, config_path: &Path) -> anyhow::Result<()>;
    fn uninstall_service(&self) -> anyhow::Result<()>;
}

#[cfg(target_os = "linux")]
pub mod linux;
#[cfg(windows)]
pub mod windows;

#[cfg(target_os = "linux")]
pub fn current() -> Box<dyn Platform> {
    Box::new(linux::LinuxPlatform)
}

#[cfg(windows)]
pub fn current() -> Box<dyn Platform> {
    Box::new(windows::WindowsPlatform)
}

#[cfg(not(any(target_os = "linux", windows)))]
pub fn current() -> Box<dyn Platform> {
    Box::new(UnsupportedPlatform)
}

#[cfg(not(any(target_os = "linux", windows)))]
struct UnsupportedPlatform;

#[cfg(not(any(target_os = "linux", windows)))]
impl Platform for UnsupportedPlatform {
    fn machine_key_material(&self) -> anyhow::Result<Vec<u8>> {
        anyhow::bail!("unsupported platform")
    }

    fn install_service(&self, _config_path: &Path) -> anyhow::Result<()> {
        anyhow::bail!("service install is unsupported on this platform")
    }

    fn uninstall_service(&self) -> anyhow::Result<()> {
        anyhow::bail!("service uninstall is unsupported on this platform")
    }
}
