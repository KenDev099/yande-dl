use anyhow::{anyhow, Result};
use directories::ProjectDirs;
use std::path::PathBuf;

#[derive(Debug, Clone)]
pub struct AppPaths {
    pub config_dir: PathBuf,
    pub tags_file: PathBuf,
    pub settings_file: PathBuf,
}

impl AppPaths {
    pub fn resolve() -> Result<Self> {
        let dirs = ProjectDirs::from("dev", "kura", "yande-dl")
            .ok_or_else(|| anyhow!("cannot determine app data directory"))?;
        let config_dir = dirs.config_dir().to_path_buf();
        std::fs::create_dir_all(&config_dir)?;
        Ok(Self {
            tags_file: config_dir.join("tags.json"),
            settings_file: config_dir.join("settings.json"),
            config_dir,
        })
    }

    /// Construct paths under a given root. Used in tests to avoid touching
    /// the real user config directory.
    pub fn under(root: PathBuf) -> Self {
        Self {
            tags_file: root.join("tags.json"),
            settings_file: root.join("settings.json"),
            config_dir: root,
        }
    }
}
