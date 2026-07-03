use std::path::Path;

use super::Platform;

pub struct WindowsPlatform;

impl Platform for WindowsPlatform {
    fn machine_key_material(&self) -> anyhow::Result<Vec<u8>> {
        Ok(std::env::var("COMPUTERNAME")
            .unwrap_or_else(|_| "gdclone-windows-fallback".to_string())
            .into_bytes())
    }

    fn install_service(&self, _config_path: &Path) -> anyhow::Result<()> {
        anyhow::bail!("windows service install wiring is scheduled for Phase 4")
    }

    fn uninstall_service(&self) -> anyhow::Result<()> {
        anyhow::bail!("windows service uninstall wiring is scheduled for Phase 4")
    }
}
