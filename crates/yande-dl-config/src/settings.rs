use crate::atomic_write::{atomic_write_json, read_json_or_default};
use crate::paths::AppPaths;
use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Settings {
    pub version: u32,
    pub download_root: Option<PathBuf>,
    pub concurrency: u32,
    pub min_delay_ms: u64,
    /// Subset of `["safe", "questionable", "explicit"]`.
    pub default_ratings: Vec<String>,
    /// `"dark"` | `"light"` | `"system"`.
    pub theme: String,
    pub age_confirmed: bool,
    pub blacklist: Vec<String>,
}

impl Default for Settings {
    fn default() -> Self {
        Self {
            version: 1,
            download_root: None,
            concurrency: 3,
            min_delay_ms: 300,
            default_ratings: vec!["safe".into()],
            theme: "dark".into(),
            age_confirmed: false,
            blacklist: vec![],
        }
    }
}

#[derive(Debug, Clone)]
pub struct SettingsStore {
    path: PathBuf,
}

impl SettingsStore {
    pub fn new(paths: &AppPaths) -> Self {
        Self {
            path: paths.settings_file.clone(),
        }
    }

    pub async fn load(&self) -> Result<Settings> {
        read_json_or_default(&self.path).await
    }

    pub async fn save(&self, settings: &Settings) -> Result<()> {
        atomic_write_json(&self.path, settings).await
    }

    pub async fn update<F>(&self, mutator: F) -> Result<Settings>
    where
        F: FnOnce(&mut Settings),
    {
        let mut s = self.load().await?;
        mutator(&mut s);
        self.save(&s).await?;
        Ok(s)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    fn store(dir: &TempDir) -> SettingsStore {
        let paths = AppPaths::under(dir.path().to_path_buf());
        SettingsStore::new(&paths)
    }

    #[tokio::test]
    async fn load_returns_default_when_missing() {
        let dir = TempDir::new().unwrap();
        let s = store(&dir);
        let v = s.load().await.unwrap();
        assert_eq!(v.concurrency, 3);
        assert_eq!(v.theme, "dark");
        assert!(!v.age_confirmed);
        assert_eq!(v.default_ratings, vec!["safe".to_string()]);
    }

    #[tokio::test]
    async fn save_and_load_round_trip() {
        let dir = TempDir::new().unwrap();
        let s = store(&dir);

        let v = Settings {
            download_root: Some(PathBuf::from("/tmp/yande-dl")),
            concurrency: 5,
            age_confirmed: true,
            blacklist: vec!["loli".into(), "shota".into()],
            ..Settings::default()
        };
        s.save(&v).await.unwrap();

        let back = s.load().await.unwrap();
        assert_eq!(back.download_root, Some(PathBuf::from("/tmp/yande-dl")));
        assert_eq!(back.concurrency, 5);
        assert!(back.age_confirmed);
        assert_eq!(back.blacklist, vec!["loli".to_string(), "shota".into()]);
    }

    #[tokio::test]
    async fn update_mutates_and_persists() {
        let dir = TempDir::new().unwrap();
        let s = store(&dir);

        let updated = s
            .update(|s| {
                s.concurrency = 7;
                s.theme = "light".into();
            })
            .await
            .unwrap();
        assert_eq!(updated.concurrency, 7);
        assert_eq!(updated.theme, "light");

        let back = s.load().await.unwrap();
        assert_eq!(back.concurrency, 7);
        assert_eq!(back.theme, "light");
    }

    #[tokio::test]
    async fn camel_case_serialization() {
        let dir = TempDir::new().unwrap();
        let s = store(&dir);
        s.save(&Settings::default()).await.unwrap();
        let raw = tokio::fs::read_to_string(&s.path).await.unwrap();
        assert!(raw.contains("downloadRoot"));
        assert!(raw.contains("minDelayMs"));
        assert!(raw.contains("defaultRatings"));
        assert!(raw.contains("ageConfirmed"));
        assert!(!raw.contains("download_root"));
    }
}
