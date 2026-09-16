use std::{env, path::Path, process::Command};

use super::Platform;

pub struct WindowsPlatform;

impl Platform for WindowsPlatform {
    fn machine_key_material(&self) -> anyhow::Result<Vec<u8>> {
        Ok(std::env::var("COMPUTERNAME")
            .unwrap_or_else(|_| "502drive-windows-fallback".to_string())
            .into_bytes())
    }

    fn install_service(&self, config_path: &Path) -> anyhow::Result<()> {
        let exe = env::current_exe()?;
        let task_run = format!(
            "\"{}\" --config \"{}\"",
            exe.display(),
            config_path.display()
        );
        run_schtasks(&[
            "/Create",
            "/TN",
            "502Drive",
            "/TR",
            task_run.as_str(),
            "/SC",
            "ONLOGON",
            "/RL",
            "LIMITED",
            "/F",
        ])?;
        println!("Installed Windows logon task '502Drive'.");
        println!("Run now: schtasks /Run /TN 502Drive");
        println!(
            "Remove: 502drive --config \"{}\" service-uninstall",
            config_path.display()
        );
        Ok(())
    }

    fn uninstall_service(&self) -> anyhow::Result<()> {
        let _ = run_schtasks(&["/End", "/TN", "502Drive"]);
        run_schtasks(&["/Delete", "/TN", "502Drive", "/F"])?;
        println!("Removed Windows logon task '502Drive'.");
        Ok(())
    }
}

fn run_schtasks(args: &[&str]) -> anyhow::Result<()> {
    let output = Command::new("schtasks").args(args).output()?;
    if output.status.success() {
        return Ok(());
    }
    let stderr = String::from_utf8_lossy(&output.stderr);
    let stdout = String::from_utf8_lossy(&output.stdout);
    anyhow::bail!(
        "schtasks failed: {}{}",
        stderr.trim(),
        if stdout.trim().is_empty() {
            String::new()
        } else {
            format!("\n{}", stdout.trim())
        }
    )
}
