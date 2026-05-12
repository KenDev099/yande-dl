use crate::atomic_write::atomic_write_json;
use crate::paths::AppPaths;
use anyhow::{bail, Result};
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Subscription {
    pub id: String,
    pub provider: String,
    /// User's original input — displayed verbatim in the UI.
    pub tag: String,
    /// `normalize_tag(tag)`. Used for folder names and dedup keying.
    pub normalized_tag: String,
    /// Optional user-defined alias (e.g. Chinese name). Shown as primary label
    /// when set; falls back to `tag` otherwise.
    #[serde(default)]
    pub display_name: Option<String>,
    pub last_run_at: Option<i64>,
    pub last_seen_post_id: i64,
    pub total_downloaded: u64,
    pub created_at: i64,
}

impl Subscription {
    pub fn new(provider: &str, raw_tag: &str) -> Self {
        Self::new_with_display_name(provider, raw_tag, None)
    }

    pub fn new_with_display_name(
        provider: &str,
        raw_tag: &str,
        display_name: Option<String>,
    ) -> Self {
        let normalized = yande_dl_core::sanitize::normalize_tag(raw_tag);
        Self {
            id: Uuid::new_v4().to_string(),
            provider: provider.into(),
            tag: raw_tag.trim().to_string(),
            normalized_tag: normalized,
            display_name: normalize_display_name(display_name),
            last_run_at: None,
            last_seen_post_id: 0,
            total_downloaded: 0,
            created_at: chrono::Utc::now().timestamp(),
        }
    }
}

/// Trim + collapse-to-None on empty so the on-disk form is consistent.
fn normalize_display_name(input: Option<String>) -> Option<String> {
    input
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty())
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TagsFile {
    pub version: u32,
    pub subscriptions: Vec<Subscription>,
}

impl Default for TagsFile {
    fn default() -> Self {
        Self {
            version: 1,
            subscriptions: vec![],
        }
    }
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ImportMode {
    Replace,
    Merge,
}

#[derive(Debug, Default, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ImportReport {
    pub added: usize,
    pub skipped: usize,
    pub removed: usize,
}

#[derive(Debug, Clone)]
pub struct TagsStore {
    path: PathBuf,
}

impl TagsStore {
    pub fn new(paths: &AppPaths) -> Self {
        Self {
            path: paths.tags_file.clone(),
        }
    }

    /// Load the tags file. If parsing fails, back up the broken file and
    /// return an empty default — the user does not lose access to the app.
    pub async fn load(&self) -> Result<TagsFile> {
        match tokio::fs::read_to_string(&self.path).await {
            Ok(s) => match serde_json::from_str::<TagsFile>(&s) {
                Ok(f) => Ok(f),
                Err(e) => {
                    let backup = self.path.with_file_name(format!(
                        "tags.json.broken.{}",
                        chrono::Utc::now().timestamp()
                    ));
                    let _ = tokio::fs::rename(&self.path, &backup).await;
                    tracing::error!("tags.json corrupted, backed up to {:?}: {}", backup, e);
                    Ok(TagsFile::default())
                }
            },
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(TagsFile::default()),
            Err(e) => Err(e.into()),
        }
    }

    pub async fn save(&self, file: &TagsFile) -> Result<()> {
        atomic_write_json(&self.path, file).await
    }

    pub async fn add(&self, provider: &str, raw_tag: &str) -> Result<Subscription> {
        self.add_with_display_name(provider, raw_tag, None).await
    }

    /// Add with an optional display name. If a subscription for this
    /// (provider, normalized_tag) already exists, the existing one is returned
    /// unchanged — the display name is NOT overwritten on dedup hits. Use
    /// `update_display_name` to rename.
    pub async fn add_with_display_name(
        &self,
        provider: &str,
        raw_tag: &str,
        display_name: Option<String>,
    ) -> Result<Subscription> {
        if raw_tag.trim().is_empty() {
            bail!("tag must not be empty");
        }
        let mut file = self.load().await?;
        let normalized = yande_dl_core::sanitize::normalize_tag(raw_tag);

        if let Some(existing) = file
            .subscriptions
            .iter()
            .find(|s| s.provider == provider && s.normalized_tag == normalized)
        {
            return Ok(existing.clone());
        }

        let sub = Subscription::new_with_display_name(provider, raw_tag, display_name);
        file.subscriptions.push(sub.clone());
        self.save(&file).await?;
        Ok(sub)
    }

    /// Set or clear the display name for a subscription. Passing `None` (or
    /// an empty/whitespace string) clears the alias.
    pub async fn update_display_name(
        &self,
        id: &str,
        display_name: Option<String>,
    ) -> Result<Option<Subscription>> {
        let mut file = self.load().await?;
        let normalized = normalize_display_name(display_name);
        if let Some(s) = file.subscriptions.iter_mut().find(|s| s.id == id) {
            s.display_name = normalized;
            let updated = s.clone();
            self.save(&file).await?;
            Ok(Some(updated))
        } else {
            Ok(None)
        }
    }

    pub async fn remove(&self, id: &str) -> Result<bool> {
        let mut file = self.load().await?;
        let before = file.subscriptions.len();
        file.subscriptions.retain(|s| s.id != id);
        let removed = file.subscriptions.len() < before;
        if removed {
            self.save(&file).await?;
        }
        Ok(removed)
    }

    /// Increment `total_downloaded` by `delta` without touching baseline.
    /// Used by "download selected posts" — those downloads are non-linear
    /// (user picks specific post_ids), so the incremental baseline must
    /// not move (would break invariant #3).
    pub async fn bump_downloaded_count(&self, id: &str, delta: u64) -> Result<()> {
        if delta == 0 {
            return Ok(());
        }
        let mut file = self.load().await?;
        if let Some(s) = file.subscriptions.iter_mut().find(|s| s.id == id) {
            s.total_downloaded += delta;
            s.last_run_at = Some(chrono::Utc::now().timestamp());
            self.save(&file).await?;
        }
        Ok(())
    }

    /// Update post-run bookkeeping. `safe_last_post_id` only ever advances
    /// (we keep `max(prev, new)`). `added_count` should be the number of
    /// images saved in this run (skipped duplicates do not count).
    pub async fn update_after_run(
        &self,
        id: &str,
        safe_last_post_id: i64,
        added_count: u64,
    ) -> Result<()> {
        let mut file = self.load().await?;
        if let Some(s) = file.subscriptions.iter_mut().find(|s| s.id == id) {
            s.last_run_at = Some(chrono::Utc::now().timestamp());
            s.last_seen_post_id = safe_last_post_id.max(s.last_seen_post_id);
            s.total_downloaded += added_count;
            self.save(&file).await?;
        }
        Ok(())
    }

    pub async fn export_to(&self, dest: &Path) -> Result<()> {
        let file = self.load().await?;
        atomic_write_json(dest, &file).await
    }

    pub async fn import_from(&self, source: &Path, mode: ImportMode) -> Result<ImportReport> {
        let raw = tokio::fs::read_to_string(source).await?;
        let imported: TagsFile = serde_json::from_str(&raw)?;
        let mut current = self.load().await?;
        let mut report = ImportReport::default();

        match mode {
            ImportMode::Replace => {
                report.removed = current.subscriptions.len();
                current.subscriptions = imported.subscriptions;
                report.added = current.subscriptions.len();
            }
            ImportMode::Merge => {
                for s in imported.subscriptions {
                    let dup = current
                        .subscriptions
                        .iter()
                        .any(|x| x.provider == s.provider && x.normalized_tag == s.normalized_tag);
                    if dup {
                        report.skipped += 1;
                    } else {
                        current.subscriptions.push(s);
                        report.added += 1;
                    }
                }
            }
        }
        self.save(&current).await?;
        Ok(report)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    fn store(dir: &TempDir) -> TagsStore {
        let paths = AppPaths::under(dir.path().to_path_buf());
        TagsStore::new(&paths)
    }

    #[tokio::test]
    async fn load_returns_default_when_missing() {
        let dir = TempDir::new().unwrap();
        let s = store(&dir);
        let f = s.load().await.unwrap();
        assert_eq!(f.version, 1);
        assert!(f.subscriptions.is_empty());
    }

    #[tokio::test]
    async fn add_and_persist() {
        let dir = TempDir::new().unwrap();
        let s = store(&dir);

        let sub = s.add("yande", "stella_sora").await.unwrap();
        assert_eq!(sub.provider, "yande");
        assert_eq!(sub.tag, "stella_sora");
        assert_eq!(sub.normalized_tag, "stella_sora");

        let reload = s.load().await.unwrap();
        assert_eq!(reload.subscriptions.len(), 1);
        assert_eq!(reload.subscriptions[0].id, sub.id);
    }

    #[tokio::test]
    async fn add_dedups_by_normalized_tag() {
        let dir = TempDir::new().unwrap();
        let s = store(&dir);

        let a = s.add("yande", "Stella_Sora").await.unwrap();
        let b = s.add("yande", "stella_sora").await.unwrap();
        let c = s.add("yande", "  stella_sora  ").await.unwrap();
        assert_eq!(a.id, b.id);
        assert_eq!(b.id, c.id);

        let f = s.load().await.unwrap();
        assert_eq!(f.subscriptions.len(), 1);
    }

    #[tokio::test]
    async fn add_treats_provider_as_part_of_key() {
        let dir = TempDir::new().unwrap();
        let s = store(&dir);

        let a = s.add("yande", "stella_sora").await.unwrap();
        let b = s.add("konachan", "stella_sora").await.unwrap();
        assert_ne!(a.id, b.id);

        let f = s.load().await.unwrap();
        assert_eq!(f.subscriptions.len(), 2);
    }

    #[tokio::test]
    async fn add_rejects_empty_tag() {
        let dir = TempDir::new().unwrap();
        let s = store(&dir);
        assert!(s.add("yande", "").await.is_err());
        assert!(s.add("yande", "   ").await.is_err());
    }

    #[tokio::test]
    async fn remove_works() {
        let dir = TempDir::new().unwrap();
        let s = store(&dir);

        let sub = s.add("yande", "foo").await.unwrap();
        assert!(s.remove(&sub.id).await.unwrap());
        assert!(!s.remove(&sub.id).await.unwrap());

        let f = s.load().await.unwrap();
        assert!(f.subscriptions.is_empty());
    }

    #[tokio::test]
    async fn update_after_run_advances_baseline_and_count() {
        let dir = TempDir::new().unwrap();
        let s = store(&dir);

        let sub = s.add("yande", "foo").await.unwrap();
        s.update_after_run(&sub.id, 100, 5).await.unwrap();
        s.update_after_run(&sub.id, 80, 2).await.unwrap(); // baseline regress; should be ignored

        let f = s.load().await.unwrap();
        let got = &f.subscriptions[0];
        assert_eq!(got.last_seen_post_id, 100);
        assert_eq!(got.total_downloaded, 7);
        assert!(got.last_run_at.is_some());
    }

    #[tokio::test]
    async fn corrupted_file_is_recovered() {
        let dir = TempDir::new().unwrap();
        let s = store(&dir);

        // Write a corrupt file.
        tokio::fs::write(&s.path, b"not json").await.unwrap();

        let f = s.load().await.unwrap();
        assert!(f.subscriptions.is_empty());

        // The corrupt file should have been backed up.
        let mut entries = tokio::fs::read_dir(dir.path()).await.unwrap();
        let mut found_backup = false;
        while let Some(e) = entries.next_entry().await.unwrap() {
            let name = e.file_name().to_string_lossy().to_string();
            if name.starts_with("tags.json.broken.") {
                found_backup = true;
            }
        }
        assert!(found_backup, "expected backup file");
    }

    #[tokio::test]
    async fn import_replace_swaps_everything() {
        let dir = TempDir::new().unwrap();
        let s = store(&dir);

        s.add("yande", "old_a").await.unwrap();
        s.add("yande", "old_b").await.unwrap();

        // Build an import file with one different subscription.
        let import_src = dir.path().join("import.json");
        let imported = TagsFile {
            version: 1,
            subscriptions: vec![Subscription::new("konachan", "imported_one")],
        };
        atomic_write_json(&import_src, &imported).await.unwrap();

        let report = s
            .import_from(&import_src, ImportMode::Replace)
            .await
            .unwrap();
        assert_eq!(report.removed, 2);
        assert_eq!(report.added, 1);
        assert_eq!(report.skipped, 0);

        let f = s.load().await.unwrap();
        assert_eq!(f.subscriptions.len(), 1);
        assert_eq!(f.subscriptions[0].provider, "konachan");
    }

    #[tokio::test]
    async fn import_merge_skips_duplicates() {
        let dir = TempDir::new().unwrap();
        let s = store(&dir);

        s.add("yande", "shared").await.unwrap();
        s.add("yande", "kept_a").await.unwrap();

        let import_src = dir.path().join("import.json");
        let imported = TagsFile {
            version: 1,
            subscriptions: vec![
                Subscription::new("yande", "shared"),     // dup
                Subscription::new("yande", "added_b"),    // new
                Subscription::new("konachan", "added_c"), // new (different provider)
            ],
        };
        atomic_write_json(&import_src, &imported).await.unwrap();

        let report = s.import_from(&import_src, ImportMode::Merge).await.unwrap();
        assert_eq!(report.added, 2);
        assert_eq!(report.skipped, 1);
        assert_eq!(report.removed, 0);

        let f = s.load().await.unwrap();
        assert_eq!(f.subscriptions.len(), 4);
    }

    #[tokio::test]
    async fn export_then_import_round_trip() {
        let dir = TempDir::new().unwrap();
        let s = store(&dir);
        s.add("yande", "alpha").await.unwrap();
        s.add("yande", "beta").await.unwrap();

        let dest = dir.path().join("backup.json");
        s.export_to(&dest).await.unwrap();

        // Wipe and re-import.
        s.save(&TagsFile::default()).await.unwrap();
        let report = s.import_from(&dest, ImportMode::Replace).await.unwrap();
        assert_eq!(report.added, 2);

        let f = s.load().await.unwrap();
        assert_eq!(f.subscriptions.len(), 2);
    }

    #[tokio::test]
    async fn camel_case_serialization() {
        let dir = TempDir::new().unwrap();
        let s = store(&dir);
        s.add("yande", "foo").await.unwrap();

        let raw = tokio::fs::read_to_string(&s.path).await.unwrap();
        // Basic check: snake_case fields should be camelCase on disk.
        assert!(raw.contains("normalizedTag"));
        assert!(raw.contains("lastRunAt"));
        assert!(raw.contains("lastSeenPostId"));
        assert!(raw.contains("totalDownloaded"));
        assert!(raw.contains("createdAt"));
        assert!(!raw.contains("normalized_tag"));
    }

    #[tokio::test]
    async fn add_with_display_name_persists_alias() {
        let dir = TempDir::new().unwrap();
        let s = store(&dir);

        let sub = s
            .add_with_display_name("yande", "cirno", Some("琪露諾".into()))
            .await
            .unwrap();
        assert_eq!(sub.display_name.as_deref(), Some("琪露諾"));

        let reload = s.load().await.unwrap();
        assert_eq!(
            reload.subscriptions[0].display_name.as_deref(),
            Some("琪露諾")
        );
    }

    #[tokio::test]
    async fn add_with_display_name_trims_and_drops_empty() {
        let dir = TempDir::new().unwrap();
        let s = store(&dir);

        let a = s
            .add_with_display_name("yande", "foo", Some("  ".into()))
            .await
            .unwrap();
        assert!(a.display_name.is_none());

        let b = s
            .add_with_display_name("yande", "bar", Some("  名字  ".into()))
            .await
            .unwrap();
        assert_eq!(b.display_name.as_deref(), Some("名字"));
    }

    #[tokio::test]
    async fn update_display_name_sets_and_clears() {
        let dir = TempDir::new().unwrap();
        let s = store(&dir);

        let sub = s.add("yande", "foo").await.unwrap();
        assert!(sub.display_name.is_none());

        let updated = s
            .update_display_name(&sub.id, Some("中文名".into()))
            .await
            .unwrap()
            .unwrap();
        assert_eq!(updated.display_name.as_deref(), Some("中文名"));

        let cleared = s.update_display_name(&sub.id, None).await.unwrap().unwrap();
        assert!(cleared.display_name.is_none());
    }

    #[tokio::test]
    async fn update_display_name_returns_none_for_unknown_id() {
        let dir = TempDir::new().unwrap();
        let s = store(&dir);
        let result = s
            .update_display_name("does-not-exist", Some("x".into()))
            .await
            .unwrap();
        assert!(result.is_none());
    }

    #[tokio::test]
    async fn loads_legacy_file_without_display_name_field() {
        // Simulate a v0.1 tags.json that pre-dates the displayName field.
        let dir = TempDir::new().unwrap();
        let s = store(&dir);
        let legacy = r#"{
            "version": 1,
            "subscriptions": [{
                "id": "abc",
                "provider": "yande",
                "tag": "foo",
                "normalizedTag": "foo",
                "lastRunAt": null,
                "lastSeenPostId": 0,
                "totalDownloaded": 0,
                "createdAt": 1700000000
            }]
        }"#;
        tokio::fs::write(&s.path, legacy).await.unwrap();

        let f = s.load().await.unwrap();
        assert_eq!(f.subscriptions.len(), 1);
        assert!(f.subscriptions[0].display_name.is_none());
        assert_eq!(f.subscriptions[0].tag, "foo");
    }

    #[tokio::test]
    async fn bump_downloaded_count_does_not_touch_baseline() {
        let dir = TempDir::new().unwrap();
        let s = store(&dir);

        let sub = s.add("yande", "foo").await.unwrap();
        s.update_after_run(&sub.id, 100, 3).await.unwrap();
        s.bump_downloaded_count(&sub.id, 4).await.unwrap();

        let f = s.load().await.unwrap();
        let got = &f.subscriptions[0];
        assert_eq!(got.last_seen_post_id, 100); // unchanged
        assert_eq!(got.total_downloaded, 7);
        assert!(got.last_run_at.is_some());
    }

    #[tokio::test]
    async fn bump_downloaded_count_zero_is_noop() {
        let dir = TempDir::new().unwrap();
        let s = store(&dir);
        let sub = s.add("yande", "foo").await.unwrap();
        s.bump_downloaded_count(&sub.id, 0).await.unwrap();
        let f = s.load().await.unwrap();
        assert_eq!(f.subscriptions[0].total_downloaded, 0);
        assert!(f.subscriptions[0].last_run_at.is_none());
    }

    #[tokio::test]
    async fn add_with_existing_normalized_tag_keeps_original_display_name() {
        let dir = TempDir::new().unwrap();
        let s = store(&dir);

        let first = s
            .add_with_display_name("yande", "cirno", Some("琪露諾".into()))
            .await
            .unwrap();
        // Try to re-add with a different alias — should hit dedup and return
        // the original unchanged.
        let second = s
            .add_with_display_name("yande", "Cirno", Some("changed".into()))
            .await
            .unwrap();
        assert_eq!(first.id, second.id);
        assert_eq!(second.display_name.as_deref(), Some("琪露諾"));
    }
}
