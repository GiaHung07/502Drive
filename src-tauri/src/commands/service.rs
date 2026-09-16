use std::process::Command;

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
    // Try launching 502drive auth login
    Command::new("502drive")
        .args(["auth", "login"])
        .spawn()
        .map_err(|e| format!("Failed to launch 502drive auth login: {e}"))?;

    Ok(())
}

#[tauri::command]
pub async fn trigger_auth_revoke() -> Result<(), String> {
    Command::new("502drive")
        .args(["auth", "revoke"])
        .output()
        .map_err(|e| format!("Failed to run 502drive auth revoke: {e}"))?;

    Ok(())
}
