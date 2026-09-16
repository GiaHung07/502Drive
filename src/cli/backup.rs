use std::{fs, path::Path};

use anyhow::Context;

use crate::config::AppConfig;

pub fn run(config_path: &Path, config: &AppConfig, output_dir: &Path) -> anyhow::Result<()> {
    fs::create_dir_all(output_dir)
        .with_context(|| format!("create backup dir {}", output_dir.display()))?;

    copy_if_exists(config_path, &output_dir.join("config.toml"))?;
    copy_if_exists(&config.storage.db_path, &output_dir.join("state.db"))?;
    copy_if_exists(
        &config.storage.config_dir.join("master.key"),
        &output_dir.join("master.key"),
    )?;
    fs::write(output_dir.join("RESTORE.vi.txt"), restore_note())?;

    println!("Backup đã tạo: {}", output_dir.display());
    println!("Gồm: config.toml, state.db, master.key, RESTORE.vi.txt");
    println!("Nên dừng bot trước khi backup nếu đang có job/watch chạy.");
    Ok(())
}

fn copy_if_exists(source: &Path, dest: &Path) -> anyhow::Result<()> {
    if !source.exists() {
        println!("Bỏ qua, chưa có: {}", source.display());
        return Ok(());
    }
    fs::copy(source, dest)
        .with_context(|| format!("copy {} -> {}", source.display(), dest.display()))?;
    println!("Đã copy: {} -> {}", source.display(), dest.display());
    Ok(())
}

fn restore_note() -> &'static str {
    "502Drive private backup\n\
     =======================\n\
     \n\
     Khôi phục trên PC/laptop khác:\n\
     \n\
     1. Cài binary gdclone-bot cùng phiên bản hoặc mới hơn.\n\
     2. Tạo thư mục config/data mặc định nếu chưa có.\n\
     3. Copy config.toml vào đường dẫn config của máy mới.\n\
     4. Copy state.db vào đúng storage.db_path trong config.toml.\n\
     5. Copy master.key vào cùng thư mục config.toml.\n\
     6. Chạy: gdclone-bot --config <config.toml> doctor\n\
     7. Nếu Google token lỗi, chạy lại: gdclone-bot --config <config.toml> auth login\n\
     \n\
     Không share thư mục backup này công khai. Nó có config, state và key giải mã token.\n"
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn restore_note_names_required_files() {
        let note = restore_note();
        assert!(note.contains("config.toml"));
        assert!(note.contains("state.db"));
        assert!(note.contains("master.key"));
        assert!(note.contains("doctor"));
    }
}
