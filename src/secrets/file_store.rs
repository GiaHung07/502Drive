use std::path::{Path, PathBuf};

use anyhow::{Context, bail};
use rand::RngCore;

#[derive(Debug, Clone)]
pub struct FileSecretStore {
    key_path: PathBuf,
}

impl FileSecretStore {
    pub fn new(config_dir: &Path) -> Self {
        Self {
            key_path: config_dir.join("master.key"),
        }
    }

    pub async fn load_or_create_key(&self) -> anyhow::Result<[u8; 32]> {
        if self.key_path.exists() {
            let data = tokio::fs::read(&self.key_path)
                .await
                .with_context(|| format!("read {}", self.key_path.display()))?;
            if data.len() != 32 {
                bail!("{} must contain exactly 32 bytes", self.key_path.display());
            }
            let mut key = [0_u8; 32];
            key.copy_from_slice(&data);
            return Ok(key);
        }

        if let Some(parent) = self.key_path.parent() {
            tokio::fs::create_dir_all(parent).await?;
        }
        let mut key = [0_u8; 32];
        rand::rng().fill_bytes(&mut key);
        write_new_key(&self.key_path, &key).await?;
        Ok(key)
    }

    pub fn backend_name(&self) -> &'static str {
        "file"
    }
}

#[cfg(unix)]
async fn write_new_key(path: &Path, key: &[u8; 32]) -> anyhow::Result<()> {
    use std::os::unix::fs::OpenOptionsExt;

    let path = path.to_path_buf();
    let key = *key;
    tokio::task::spawn_blocking(move || {
        let mut opts = std::fs::OpenOptions::new();
        opts.write(true).create_new(true).mode(0o600);
        std::io::Write::write_all(&mut opts.open(path)?, &key)?;
        Ok::<_, std::io::Error>(())
    })
    .await??;
    Ok(())
}

#[cfg(not(unix))]
async fn write_new_key(path: &Path, key: &[u8; 32]) -> anyhow::Result<()> {
    tokio::fs::write(path, key).await?;
    Ok(())
}
