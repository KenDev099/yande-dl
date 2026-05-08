use crate::error::CoreError;
use crate::model::Post;
use crate::retry::{with_backoff, RetryPolicy};
use crate::sanitize::safe_folder_segment;
use md5::{Digest, Md5};
use reqwest::Client;
use std::collections::HashSet;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::Semaphore;
use tokio_util::sync::CancellationToken;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DownloadStatus {
    Saved,
    SkippedDuplicate,
    Cancelled,
    Failed(String),
}

#[derive(Debug, Clone)]
pub struct DownloadOutcome {
    pub post_id: i64,
    pub status: DownloadStatus,
    pub file_path: Option<PathBuf>,
    pub bytes: Option<u64>,
}

pub struct Downloader {
    client: Client,
    permits: Arc<Semaphore>,
    root_dir: PathBuf,
    min_delay_ms: u64,
}

impl Downloader {
    pub fn new(client: Client, concurrency: usize, root_dir: PathBuf, min_delay_ms: u64) -> Self {
        Self {
            client,
            permits: Arc::new(Semaphore::new(concurrency.max(1))),
            root_dir,
            min_delay_ms,
        }
    }

    /// `<root>/_<provider> <safe_tag>/`
    pub fn folder_path(&self, provider_id: &str, normalized_tag: &str) -> PathBuf {
        let folder = format!("_{} {}", provider_id, safe_folder_segment(normalized_tag));
        self.root_dir.join(folder)
    }

    /// `<folder>/<provider>_<post_id>.<ext>`. The extension does NOT participate
    /// in dedup — `scan_existing_post_ids` looks only at the prefix.
    pub fn target_path(&self, post: &Post, normalized_tag: &str) -> PathBuf {
        let folder = self.folder_path(&post.provider_id, normalized_tag);
        let ext = Path::new(&post.variants.original.url)
            .extension()
            .and_then(|s| s.to_str())
            .unwrap_or("bin")
            .to_string();
        folder.join(format!("{}_{}.{}", post.provider_id, post.post_id, ext))
    }

    /// Scan `folder` once at job start. Returns the set of `post_id`s already
    /// on disk (extension-agnostic). The dedup-by-folder-scan invariant.
    ///
    /// This also opportunistically reaps stale `.part` files (older than 24h
    /// AND smaller than 100 KB) — a best-effort cleanup so partials from
    /// crashed runs do not accumulate.
    pub async fn scan_existing_post_ids(folder: &Path, provider_id: &str) -> HashSet<i64> {
        let mut ids = HashSet::new();
        let prefix = format!("{}_", provider_id);

        let mut entries = match tokio::fs::read_dir(folder).await {
            Ok(e) => e,
            Err(_) => return ids, // folder doesn't exist == empty set
        };

        let now = std::time::SystemTime::now();

        while let Ok(Some(entry)) = entries.next_entry().await {
            let name = entry.file_name();
            let name = name.to_string_lossy().to_string();

            // Reap stale `.part` files. Errors here are non-fatal.
            if name.ends_with(".part") {
                if let Ok(meta) = entry.metadata().await {
                    let stale_age = meta
                        .modified()
                        .ok()
                        .and_then(|m| now.duration_since(m).ok())
                        .map(|d| d.as_secs() > 24 * 3600)
                        .unwrap_or(false);
                    if stale_age && meta.len() < 100_000 {
                        let _ = tokio::fs::remove_file(entry.path()).await;
                    }
                }
                continue;
            }

            if let Some(rest) = name.strip_prefix(&prefix) {
                if let Some(id_str) = rest.split('.').next() {
                    if let Ok(id) = id_str.parse::<i64>() {
                        ids.insert(id);
                    }
                }
            }
        }
        ids
    }

    /// Download a single post. Honors cancellation throughout, including
    /// during the in-flight body read (via `tokio::select!`).
    pub async fn download_post<F>(
        &self,
        post: &Post,
        normalized_tag: &str,
        already_have: F,
        cancel: &CancellationToken,
    ) -> DownloadOutcome
    where
        F: Fn(i64) -> bool,
    {
        if cancel.is_cancelled() {
            return DownloadOutcome {
                post_id: post.post_id,
                status: DownloadStatus::Cancelled,
                file_path: None,
                bytes: None,
            };
        }

        if already_have(post.post_id) {
            return DownloadOutcome {
                post_id: post.post_id,
                status: DownloadStatus::SkippedDuplicate,
                file_path: None,
                bytes: None,
            };
        }

        let permit = match self.permits.clone().acquire_owned().await {
            Ok(p) => p,
            Err(e) => {
                return DownloadOutcome {
                    post_id: post.post_id,
                    status: DownloadStatus::Failed(format!("semaphore closed: {}", e)),
                    file_path: None,
                    bytes: None,
                };
            }
        };

        let result = tokio::select! {
            biased;
            _ = cancel.cancelled() => Err(CoreError::Cancelled),
            r = self.do_download_with_retry(post, normalized_tag) => r,
        };

        if self.min_delay_ms > 0 {
            tokio::time::sleep(Duration::from_millis(self.min_delay_ms)).await;
        }
        drop(permit);

        match result {
            Ok((path, bytes)) => DownloadOutcome {
                post_id: post.post_id,
                status: DownloadStatus::Saved,
                file_path: Some(path),
                bytes: Some(bytes),
            },
            Err(CoreError::Cancelled) => DownloadOutcome {
                post_id: post.post_id,
                status: DownloadStatus::Cancelled,
                file_path: None,
                bytes: None,
            },
            Err(e) => DownloadOutcome {
                post_id: post.post_id,
                status: DownloadStatus::Failed(e.to_string()),
                file_path: None,
                bytes: None,
            },
        }
    }

    /// Network errors retry under `RetryPolicy::standard`. An MD5 mismatch
    /// is retried once more under `RetryPolicy::fast` — a single transmission
    /// flip is plausible, but two in a row likely means upstream corruption
    /// and is not worth more attempts.
    async fn do_download_with_retry(
        &self,
        post: &Post,
        normalized_tag: &str,
    ) -> Result<(PathBuf, u64), CoreError> {
        let first = with_backoff(&RetryPolicy::standard(), || {
            self.do_download(post, normalized_tag)
        })
        .await;
        match first {
            Err(CoreError::Md5Mismatch { .. }) => {
                with_backoff(&RetryPolicy::fast(), || {
                    self.do_download(post, normalized_tag)
                })
                .await
            }
            other => other,
        }
    }

    async fn do_download(
        &self,
        post: &Post,
        normalized_tag: &str,
    ) -> Result<(PathBuf, u64), CoreError> {
        let path = self.target_path(post, normalized_tag);
        if let Some(parent) = path.parent() {
            tokio::fs::create_dir_all(parent).await?;
        }

        // v0.1: always download the original. md5 is for original only.
        let url = &post.variants.original.url;
        let resp = self.client.get(url).send().await?;
        if !resp.status().is_success() {
            return Err(CoreError::Server {
                status: resp.status().as_u16(),
            });
        }
        let bytes = resp.bytes().await?;

        let mut hasher = Md5::new();
        hasher.update(&bytes);
        let computed = format!("{:x}", hasher.finalize());
        if computed != post.md5 {
            return Err(CoreError::Md5Mismatch {
                expected: post.md5.clone(),
                actual: computed,
            });
        }

        // Atomic write: tmp + rename.
        let tmp_path = path.with_extension(format!(
            "{}.part",
            path.extension().and_then(|s| s.to_str()).unwrap_or("tmp")
        ));
        tokio::fs::write(&tmp_path, &bytes).await?;
        tokio::fs::rename(&tmp_path, &path).await?;

        Ok((path, bytes.len() as u64))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::Duration;
    use tempfile::TempDir;

    #[tokio::test]
    async fn scan_returns_empty_for_missing_folder() {
        let dir = TempDir::new().unwrap();
        let folder = dir.path().join("nope");
        let ids = Downloader::scan_existing_post_ids(&folder, "yande").await;
        assert!(ids.is_empty());
    }

    #[tokio::test]
    async fn scan_picks_up_provider_prefixed_files() {
        let dir = TempDir::new().unwrap();
        let folder = dir.path();

        // Files that should be detected:
        tokio::fs::write(folder.join("yande_100.png"), b"x")
            .await
            .unwrap();
        tokio::fs::write(folder.join("yande_101.jpg"), b"x")
            .await
            .unwrap();
        tokio::fs::write(folder.join("yande_102.gif"), b"x")
            .await
            .unwrap();

        // Files that should NOT be detected:
        tokio::fs::write(folder.join("konachan_999.png"), b"x")
            .await
            .unwrap();
        tokio::fs::write(folder.join("yande_abc.png"), b"x")
            .await
            .unwrap();
        tokio::fs::write(folder.join("foo.png"), b"x")
            .await
            .unwrap();
        tokio::fs::write(folder.join("yande_103.png.part"), b"x")
            .await
            .unwrap();

        let ids = Downloader::scan_existing_post_ids(folder, "yande").await;
        assert_eq!(ids.len(), 3);
        assert!(ids.contains(&100));
        assert!(ids.contains(&101));
        assert!(ids.contains(&102));
        assert!(!ids.contains(&103)); // .part is excluded
    }

    #[tokio::test]
    async fn scan_treats_extension_agnostically() {
        // Even if a post was originally saved as .png and re-listed as .jpg,
        // dedup must still trigger.
        let dir = TempDir::new().unwrap();
        tokio::fs::write(dir.path().join("yande_500.png"), b"x")
            .await
            .unwrap();
        let ids = Downloader::scan_existing_post_ids(dir.path(), "yande").await;
        assert!(ids.contains(&500));
    }

    fn make_client() -> Client {
        Client::builder()
            .timeout(Duration::from_secs(5))
            .build()
            .unwrap()
    }

    fn make_downloader(root: PathBuf) -> Downloader {
        Downloader::new(make_client(), 2, root, 0)
    }

    fn make_post(provider: &str, id: i64, md5: &str, ext: &str, file_url: String) -> Post {
        use crate::model::{ImageVariant, PostVariants, Rating};
        Post {
            provider_id: provider.into(),
            post_id: id,
            md5: md5.into(),
            rating: Rating::Safe,
            score: 0,
            width: 100,
            height: 100,
            tags: vec![],
            artist: None,
            source_url: None,
            created_at: None,
            variants: PostVariants {
                original: ImageVariant {
                    url: file_url,
                    width: Some(100),
                    height: Some(100),
                    size_bytes: None,
                    mime: Some(format!("image/{}", ext)),
                },
                preview: ImageVariant {
                    url: "http://x/p".into(),
                    width: Some(50),
                    height: Some(50),
                    size_bytes: None,
                    mime: None,
                },
                sample: None,
                jpeg: None,
            },
            extra: Default::default(),
        }
    }

    fn md5_hex(bytes: &[u8]) -> String {
        let mut h = Md5::new();
        h.update(bytes);
        format!("{:x}", h.finalize())
    }

    #[tokio::test]
    async fn target_path_uses_provider_post_id_and_extension() {
        let dir = TempDir::new().unwrap();
        let dl = make_downloader(dir.path().to_path_buf());
        let post = make_post("yande", 42, "x", "png", "http://example.com/foo.png".into());
        let p = dl.target_path(&post, "stella_sora");
        assert!(p.ends_with("_yande stella_sora/yande_42.png"));
    }

    #[tokio::test]
    async fn already_have_short_circuits_to_skipped() {
        let dir = TempDir::new().unwrap();
        let dl = make_downloader(dir.path().to_path_buf());
        let post = make_post("yande", 1, "x", "png", "http://localhost:1/foo.png".into());
        let cancel = CancellationToken::new();
        let outcome = dl.download_post(&post, "tag", |_| true, &cancel).await;
        assert_eq!(outcome.status, DownloadStatus::SkippedDuplicate);
    }

    #[tokio::test]
    async fn cancel_before_call_returns_cancelled() {
        let dir = TempDir::new().unwrap();
        let dl = make_downloader(dir.path().to_path_buf());
        let post = make_post("yande", 1, "x", "png", "http://localhost:1/foo.png".into());
        let cancel = CancellationToken::new();
        cancel.cancel();
        let outcome = dl.download_post(&post, "tag", |_| false, &cancel).await;
        assert_eq!(outcome.status, DownloadStatus::Cancelled);
    }

    #[tokio::test]
    async fn download_succeeds_with_correct_md5() {
        use wiremock::matchers::method;
        use wiremock::{Mock, MockServer, ResponseTemplate};

        let body = b"hello world";
        let server = MockServer::start().await;
        Mock::given(method("GET"))
            .respond_with(ResponseTemplate::new(200).set_body_bytes(body.to_vec()))
            .mount(&server)
            .await;

        let dir = TempDir::new().unwrap();
        let dl = make_downloader(dir.path().to_path_buf());
        let url = format!("{}/file.png", server.uri());
        let post = make_post("yande", 7, &md5_hex(body), "png", url);

        let cancel = CancellationToken::new();
        let outcome = dl.download_post(&post, "tag", |_| false, &cancel).await;
        assert_eq!(outcome.status, DownloadStatus::Saved);
        assert!(outcome.file_path.unwrap().exists());
        assert_eq!(outcome.bytes, Some(body.len() as u64));
    }

    #[tokio::test]
    async fn download_fails_on_md5_mismatch_after_fast_retry() {
        use wiremock::matchers::method;
        use wiremock::{Mock, MockServer, ResponseTemplate};

        // Server always returns the wrong bytes; both retries should fail.
        let server = MockServer::start().await;
        Mock::given(method("GET"))
            .respond_with(ResponseTemplate::new(200).set_body_bytes(b"corrupted".to_vec()))
            .mount(&server)
            .await;

        let dir = TempDir::new().unwrap();
        let dl = make_downloader(dir.path().to_path_buf());
        let url = format!("{}/file.png", server.uri());
        // Wrong md5 on purpose — server bytes hash to something else.
        let post = make_post("yande", 9, "00000000000000000000000000000000", "png", url);

        let cancel = CancellationToken::new();
        let outcome = dl.download_post(&post, "tag", |_| false, &cancel).await;
        match outcome.status {
            DownloadStatus::Failed(msg) => assert!(msg.contains("md5"), "unexpected: {}", msg),
            other => panic!("expected Failed(md5...), got {:?}", other),
        }
    }

    #[tokio::test]
    async fn cancel_mid_download_returns_quickly() {
        use wiremock::matchers::method;
        use wiremock::{Mock, MockServer, ResponseTemplate};

        // Server delays 5 seconds before responding.
        let server = MockServer::start().await;
        Mock::given(method("GET"))
            .respond_with(
                ResponseTemplate::new(200)
                    .set_body_bytes(vec![0u8; 1024])
                    .set_delay(Duration::from_secs(5)),
            )
            .mount(&server)
            .await;

        let dir = TempDir::new().unwrap();
        let dl = make_downloader(dir.path().to_path_buf());
        let url = format!("{}/file.png", server.uri());
        let post = make_post("yande", 11, "deadbeef", "png", url);

        let cancel = CancellationToken::new();
        let cancel_clone = cancel.clone();
        tokio::spawn(async move {
            tokio::time::sleep(Duration::from_millis(100)).await;
            cancel_clone.cancel();
        });

        let started = std::time::Instant::now();
        let outcome = dl.download_post(&post, "tag", |_| false, &cancel).await;
        let elapsed = started.elapsed();

        assert_eq!(outcome.status, DownloadStatus::Cancelled);
        assert!(
            elapsed < Duration::from_secs(2),
            "should cancel quickly, took {:?}",
            elapsed
        );
    }
}
