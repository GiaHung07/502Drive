use std::process::Command;

#[tauri::command]
pub async fn start_service() -> Result<(), String> {
    #[cfg(target_os = "linux")]
    {
        let output = Command::new("systemctl")
            .args(["--user", "start", "gdclone-bot"])
            .output()
            .map_err(|e| e.to_string())?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            return Err(format!("systemctl start failed: {stderr}"));
        }
        return Ok(());
    }

    #[cfg(not(target_os = "linux"))]
    {
        Ok(())
    }
}

#[tauri::command]
pub async fn restart_service() -> Result<(), String> {
    #[cfg(target_os = "linux")]
    {
        let output = Command::new("systemctl")
            .args(["--user", "restart", "gdclone-bot"])
            .output()
            .map_err(|e| e.to_string())?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            return Err(format!("systemctl restart failed: {stderr}"));
        }
        return Ok(());
    }

    #[cfg(not(target_os = "linux"))]
    {
        Ok(())
    }
}

#[tauri::command]
pub async fn open_telegram_bot() -> Result<(), String> {
    open::that("https://t.me/Drive502_Bot").map_err(|e| e.to_string())?;
    Ok(())
}

#[tauri::command]
pub async fn trigger_auth_login() -> Result<(), String> {
    let title = "502Drive - Đăng nhập Google";
    let cmd = "export PATH=\"$HOME/.local/bin:$PATH\"; which 502drive && 502drive auth login || ~/.local/bin/502drive auth login; echo; read -p 'Nhấn Enter để đóng...' -r";

    if std::path::Path::new("/usr/bin/ptyxis").exists() {
        Command::new("ptyxis")
            .args(["--title", title, "--", "bash", "-c", cmd])
            .spawn()
            .map_err(|e| format!("Failed to spawn ptyxis: {e}"))?;
    } else if std::path::Path::new("/usr/bin/gnome-terminal").exists() {
        Command::new("gnome-terminal")
            .args(["--title", title, "--", "bash", "-c", cmd])
            .spawn()
            .map_err(|e| format!("Failed to spawn gnome-terminal: {e}"))?;
    } else if std::path::Path::new("/usr/bin/xterm").exists() {
        Command::new("xterm")
            .args(["-title", title, "-e", "bash", "-c", cmd])
            .spawn()
            .map_err(|e| format!("Failed to spawn xterm: {e}"))?;
    } else {
        Command::new("bash")
            .args([
                "-c",
                "export PATH=\"$HOME/.local/bin:$PATH\"; 502drive auth login",
            ])
            .spawn()
            .map_err(|e| format!("Failed to spawn 502drive auth login: {e}"))?;
    }

    Ok(())
}

#[tauri::command]
pub async fn trigger_auth_revoke() -> Result<(), String> {
    Command::new("bash")
        .args([
            "-c",
            "export PATH=\"$HOME/.local/bin:$PATH\"; 502drive auth revoke",
        ])
        .output()
        .map_err(|e| format!("Failed to run 502drive auth revoke: {e}"))?;

    Ok(())
}
