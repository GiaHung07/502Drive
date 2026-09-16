use std::fs::File;
use std::io::{BufRead, BufReader};
use crate::commands::get_log_path;

#[tauri::command]
pub async fn get_recent_logs(lines: Option<usize>) -> Result<Vec<String>, String> {
    let log_path = get_log_path();
    let count = lines.unwrap_or(50);

    if !log_path.exists() {
        return Ok(vec![
            format!("[INFO] Nhật ký lưu tại: {}", log_path.display()),
            "[INFO] Hệ thống sẵn sàng ghi log khi service khởi chạy.".to_string(),
        ]);
    }

    let file = File::open(&log_path).map_err(|e| e.to_string())?;
    let reader = BufReader::new(file);
    let all_lines: Vec<String> = reader.lines().filter_map(|l| l.ok()).collect();

    let start = all_lines.len().saturating_sub(count);
    Ok(all_lines[start..].to_vec())
}
