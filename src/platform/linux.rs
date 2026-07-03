use std::{
    env, fs,
    path::{Path, PathBuf},
};

use anyhow::Context;

use super::Platform;

pub struct LinuxPlatform;

impl Platform for LinuxPlatform {
    fn machine_key_material(&self) -> anyhow::Result<Vec<u8>> {
        let machine_id = fs::read_to_string("/etc/machine-id").context("read /etc/machine-id")?;
        Ok(machine_id.trim().as_bytes().to_vec())
    }

    fn install_service(&self, config_path: &Path) -> anyhow::Result<()> {
        let unit_dir = user_unit_dir()?;
        fs::create_dir_all(&unit_dir)?;
        let exe = env::current_exe()?;
        let unit = format!(
            "[Unit]\nDescription=gdclone-bot\nWants=network-online.target\nAfter=network-online.target\n\n[Service]\nType=simple\nExecStart={} --config {} run\nRestart=on-failure\nRestartSec=5\nTimeoutStopSec=30\nNoNewPrivileges=yes\nPrivateTmp=yes\nProtectSystem=strict\nProtectHome=read-only\nReadWritePaths=%h/.local/share/gdclone-bot\nReadWritePaths=%h/.config/gdclone-bot\n\n[Install]\nWantedBy=default.target\n",
            systemd_quote(&exe),
            systemd_quote(config_path)
        );
        fs::write(unit_dir.join("gdclone-bot.service"), unit)?;
        println!(
            "Installed user unit at {}",
            unit_dir.join("gdclone-bot.service").display()
        );
        println!(
            "Run: systemctl --user daemon-reload && systemctl --user enable --now gdclone-bot.service"
        );
        Ok(())
    }

    fn uninstall_service(&self) -> anyhow::Result<()> {
        let path = user_unit_dir()?.join("gdclone-bot.service");
        if path.exists() {
            fs::remove_file(&path)?;
        }
        println!("Removed {}", path.display());
        println!("Run: systemctl --user daemon-reload");
        Ok(())
    }
}

fn systemd_quote(path: &Path) -> String {
    format!(
        "\"{}\"",
        path.to_string_lossy()
            .replace('\\', "\\\\")
            .replace('"', "\\\"")
    )
}

fn user_unit_dir() -> anyhow::Result<PathBuf> {
    let base = directories::BaseDirs::new().context("resolve home directory")?;
    Ok(base.home_dir().join(".config/systemd/user"))
}
