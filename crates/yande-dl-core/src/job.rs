use crate::downloader::{DownloadOutcome, DownloadStatus, Downloader};
use crate::error::CoreError;
use crate::model::SearchQuery;
use crate::provider::ImageProvider;
use crate::sanitize::normalize_tag;
use futures::stream::{FuturesUnordered, StreamExt};
use serde::{Deserialize, Serialize};
use std::collections::HashSet;
use std::sync::Arc;
use tokio::sync::mpsc;
use tokio_util::sync::CancellationToken;

pub struct DownloadJob {
    pub raw_tag: String,
    pub provider: Arc<dyn ImageProvider>,
    /// `None` = full sweep. Otherwise the new-post baseline.
    pub since_post_id: Option<i64>,
    pub max_pages: u32,
    /// After we cross `since_post_id`, keep paging this many more pages to
    /// recover from any earlier transient failures. Default: 2.
    pub incremental_lookback_pages: u32,
    pub query_extra: SearchQuery,
}

/// Lifecycle state of a single post during a job. Emitted via `JobMessage::PostStatus`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum PostStatus {
    Queued,
    Downloading,
    Saved,
    Skipped,
    Failed,
    Cancelled,
}

/// Per-post metadata pushed to the frontend so it can render a thumbnail grid.
/// Sample/preview URLs come from `Post.variants` (the provider already parses them).
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PostInfo {
    pub post_id: i64,
    pub sample_url: Option<String>,
    pub preview_url: String,
    pub original_url: String,
    pub width: u32,
    pub height: u32,
    pub status: PostStatus,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PostStatusEvent {
    pub post_id: i64,
    pub status: PostStatus,
}

/// Messages emitted by `run_job` on a single channel. The page-level aggregate
/// is stale-tolerant (try_send may drop); per-post messages are not (use
/// `send().await`). The forwarder in the Tauri layer demuxes by variant.
#[derive(Debug, Clone)]
pub enum JobMessage {
    /// Page-level aggregate counts. Stale-tolerant — earlier values can be
    /// dropped by `try_send` without loss of correctness.
    PageProgress(JobProgress),
    /// Emitted once per page after candidate filtering. Carries thumbnail
    /// URLs and initial status (`Queued` for unknown posts, `Skipped` for
    /// ones already on disk).
    PostsDiscovered(Vec<PostInfo>),
    /// Emitted on every status transition of an individual post.
    PostStatus(PostStatusEvent),
}

impl DownloadJob {
    pub fn new(provider: Arc<dyn ImageProvider>, raw_tag: impl Into<String>) -> Self {
        Self {
            raw_tag: raw_tag.into(),
            provider,
            since_post_id: None,
            max_pages: 500,
            incremental_lookback_pages: 2,
            query_extra: SearchQuery::default(),
        }
    }
}

#[derive(Debug, Clone, Default)]
pub struct JobProgress {
    pub current_page: u32,
    pub fetched: u32,
    pub saved: u32,
    pub skipped: u32,
    pub failed: u32,
    pub cancelled: u32,
}

#[derive(Debug)]
pub struct JobOutcome {
    pub progress: JobProgress,
    /// `last_seen_post_id` value the caller can safely persist.  Failed posts
    /// will *not* advance past their own id, so the next incremental run
    /// retries them.
    pub safe_last_post_id: i64,
    pub outcomes: Vec<DownloadOutcome>,
}

pub async fn run_job<B>(
    job: DownloadJob,
    downloader: Arc<Downloader>,
    blacklist_match: B,
    progress_tx: mpsc::Sender<JobMessage>,
    cancel: CancellationToken,
) -> Result<JobOutcome, CoreError>
where
    B: Fn(&[String]) -> bool + Send + Sync + 'static,
{
    let normalized = normalize_tag(&job.raw_tag);
    let folder = downloader.folder_path(job.provider.id(), &normalized);

    let known_ids: Arc<HashSet<i64>> =
        Arc::new(Downloader::scan_existing_post_ids(&folder, job.provider.id()).await);

    tracing::info!(
        tag = %normalized,
        known = known_ids.len(),
        "scanned existing posts"
    );

    let baseline = job.since_post_id.unwrap_or(0);
    let mut page: u32 = 1;
    let mut progress = JobProgress::default();
    let mut outcomes: Vec<DownloadOutcome> = Vec::new();
    let mut pages_after_caught_up: u32 = 0;

    let mut id_status: Vec<(i64, bool)> = Vec::new();

    loop {
        if cancel.is_cancelled() {
            break;
        }
        if page > job.max_pages {
            tracing::warn!(tag = %normalized, max = job.max_pages, "reached max_pages");
            break;
        }

        let mut q = job.query_extra.clone();
        q.tags.insert(0, normalized.clone());
        if q.limit == 0 {
            q.limit = 100;
        }

        let posts = job.provider.search(&q, page).await?;
        if posts.is_empty() {
            break;
        }

        // Two filters that must NOT be conflated:
        //   - baseline check is the termination condition
        //   - blacklist is a per-page filter
        let any_above_baseline = posts.iter().any(|p| p.post_id > baseline);

        let candidates: Vec<_> = posts
            .iter()
            .filter(|p| p.post_id > baseline)
            .filter(|p| !blacklist_match(&p.tags))
            .cloned()
            .collect();

        if !any_above_baseline && job.since_post_id.is_some() {
            pages_after_caught_up += 1;
            tracing::debug!(
                tag = %normalized,
                page,
                lookback = pages_after_caught_up,
                "caught up; in lookback window"
            );
            if pages_after_caught_up >= job.incremental_lookback_pages {
                tracing::info!("incremental update finished after lookback");
                break;
            }
            page += 1;
            continue;
        }

        // Snapshot all candidates with thumbnail URLs + initial status so the
        // frontend can render the grid immediately. Posts already on disk get
        // `Skipped` here (no Downloading event will follow for them).
        if !candidates.is_empty() {
            let post_infos: Vec<PostInfo> = candidates
                .iter()
                .map(|p| PostInfo {
                    post_id: p.post_id,
                    sample_url: p.variants.sample.as_ref().map(|v| v.url.clone()),
                    preview_url: p.variants.preview.url.clone(),
                    original_url: p.variants.original.url.clone(),
                    width: p.width,
                    height: p.height,
                    status: if known_ids.contains(&p.post_id) {
                        PostStatus::Skipped
                    } else {
                        PostStatus::Queued
                    },
                })
                .collect();
            // Not stale-tolerant: the grid relies on knowing all posts. If the
            // channel is full, wait — the consumer is fast (just emits Tauri events).
            let _ = progress_tx
                .send(JobMessage::PostsDiscovered(post_infos))
                .await;
        }

        let mut futs = FuturesUnordered::new();
        for post in candidates {
            let dl = downloader.clone();
            let known = known_ids.clone();
            let tag = normalized.clone();
            let token = cancel.clone();
            let tx = progress_tx.clone();
            let post_id = post.post_id;
            let will_skip = known.contains(&post_id);
            futs.push(tokio::spawn(async move {
                if !will_skip {
                    let _ = tx
                        .send(JobMessage::PostStatus(PostStatusEvent {
                            post_id,
                            status: PostStatus::Downloading,
                        }))
                        .await;
                }
                let outcome = dl
                    .download_post(&post, &tag, |id| known.contains(&id), &token)
                    .await;
                let final_status = match &outcome.status {
                    DownloadStatus::Saved => PostStatus::Saved,
                    DownloadStatus::SkippedDuplicate => PostStatus::Skipped,
                    DownloadStatus::Cancelled => PostStatus::Cancelled,
                    DownloadStatus::Failed(_) => PostStatus::Failed,
                };
                let _ = tx
                    .send(JobMessage::PostStatus(PostStatusEvent {
                        post_id,
                        status: final_status,
                    }))
                    .await;
                outcome
            }));
        }

        while let Some(res) = futs.next().await {
            match res {
                Ok(outcome) => {
                    let post_id = outcome.post_id;
                    match &outcome.status {
                        DownloadStatus::Saved => {
                            progress.saved += 1;
                            id_status.push((post_id, true));
                        }
                        DownloadStatus::SkippedDuplicate => {
                            progress.skipped += 1;
                            id_status.push((post_id, true));
                        }
                        DownloadStatus::Cancelled => progress.cancelled += 1,
                        DownloadStatus::Failed(_) => {
                            progress.failed += 1;
                            id_status.push((post_id, false));
                        }
                    }
                    outcomes.push(outcome);
                }
                Err(e) => {
                    tracing::error!("download task panicked: {}", e);
                    progress.failed += 1;
                }
            }
        }

        progress.fetched += posts.len() as u32;
        progress.current_page = page;

        // Bounded channel + try_send: page-aggregate progress is stale-tolerant;
        // dropping updates is preferable to backpressuring the runner.
        let _ = progress_tx.try_send(JobMessage::PageProgress(progress.clone()));

        page += 1;
    }

    let safe_last = compute_safe_baseline(baseline, &id_status);

    Ok(JobOutcome {
        progress,
        safe_last_post_id: safe_last,
        outcomes,
    })
}

/// `safe_last_post_id` algorithm:
///
/// - If any post failed, take the maximum `post_id` strictly less than the
///   minimum failed id, among posts that succeeded or were skipped.
/// - Otherwise, take the maximum `post_id` of all successful/skipped posts.
/// - If nothing was processed, keep the original `baseline`.
/// - The result never goes backward (always `>= baseline`).
pub fn compute_safe_baseline(baseline: i64, id_status: &[(i64, bool)]) -> i64 {
    let min_failed: Option<i64> = id_status
        .iter()
        .filter(|(_, ok)| !ok)
        .map(|(id, _)| *id)
        .min();

    let candidates = id_status.iter().filter(|(_, ok)| *ok).map(|(id, _)| *id);

    let max_ok = match min_failed {
        Some(m) => candidates.filter(|id| *id < m).max(),
        None => candidates.max(),
    };

    max_ok.unwrap_or(baseline).max(baseline)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::{Capabilities, ImageVariant, Post, PostVariants, Rating};
    use async_trait::async_trait;
    use reqwest::Client;
    use std::path::PathBuf;
    use std::sync::Mutex;
    use tempfile::TempDir;

    // ---- compute_safe_baseline unit tests ----

    #[test]
    fn safe_baseline_no_failures() {
        let s = vec![(100, true), (102, true), (105, true)];
        assert_eq!(compute_safe_baseline(0, &s), 105);
    }

    #[test]
    fn safe_baseline_with_one_failure() {
        let s = vec![(99, true), (100, false), (101, true), (105, true)];
        assert_eq!(compute_safe_baseline(0, &s), 99);
    }

    #[test]
    fn safe_baseline_first_post_failed() {
        let s = vec![(50, false), (51, true), (52, true)];
        assert_eq!(compute_safe_baseline(40, &s), 40);
    }

    #[test]
    fn safe_baseline_empty() {
        assert_eq!(compute_safe_baseline(123, &[]), 123);
    }

    #[test]
    fn safe_baseline_never_regresses() {
        let s = vec![(10, true)];
        assert_eq!(compute_safe_baseline(50, &s), 50);
    }

    // ---- run_job integration tests with a mock provider ----

    struct MockProvider {
        pages: Mutex<Vec<Vec<Post>>>,
    }

    impl MockProvider {
        fn new(pages: Vec<Vec<Post>>) -> Arc<Self> {
            Arc::new(Self {
                pages: Mutex::new(pages),
            })
        }
    }

    #[async_trait]
    impl ImageProvider for MockProvider {
        fn id(&self) -> &str {
            "mock"
        }
        fn display_name(&self) -> &str {
            "Mock"
        }
        fn capabilities(&self) -> Capabilities {
            Capabilities {
                max_results_per_page: 100,
                uses_md5: true,
                default_sort_desc_by_id: true,
            }
        }
        async fn search(&self, _q: &SearchQuery, page: u32) -> Result<Vec<Post>, CoreError> {
            let pages = self.pages.lock().unwrap();
            let idx = (page as usize).saturating_sub(1);
            Ok(pages.get(idx).cloned().unwrap_or_default())
        }
    }

    fn fake_post(id: i64) -> Post {
        Post {
            provider_id: "mock".into(),
            post_id: id,
            md5: "0".repeat(32),
            rating: Rating::Safe,
            score: 0,
            width: 1,
            height: 1,
            tags: vec![],
            artist: None,
            source_url: None,
            created_at: None,
            variants: PostVariants {
                original: ImageVariant {
                    url: "http://nowhere/x.png".into(),
                    width: Some(1),
                    height: Some(1),
                    size_bytes: None,
                    mime: Some("image/png".into()),
                },
                preview: ImageVariant {
                    url: "http://nowhere/p.png".into(),
                    width: Some(1),
                    height: Some(1),
                    size_bytes: None,
                    mime: None,
                },
                sample: None,
                jpeg: None,
            },
            extra: Default::default(),
        }
    }

    fn make_dl(root: PathBuf) -> Arc<Downloader> {
        Arc::new(Downloader::new(
            Client::builder().build().unwrap(),
            2,
            root,
            0,
        ))
    }

    fn fake_post_with_tags(id: i64, tags: &[&str]) -> Post {
        let mut p = fake_post(id);
        p.tags = tags.iter().map(|s| (*s).to_string()).collect();
        p
    }

    /// All-Saved happy path is awkward to test without a real HTTP server
    /// for every post. Instead we test the loop control and bookkeeping by
    /// pre-populating the dedup set so every post becomes SkippedDuplicate.
    #[tokio::test]
    async fn full_sweep_terminates_on_empty_page() {
        let tmp = TempDir::new().unwrap();
        let dl = make_dl(tmp.path().to_path_buf());

        // Pre-fill folder so all posts are dedup-skipped.
        let folder = dl.folder_path("mock", "anything");
        tokio::fs::create_dir_all(&folder).await.unwrap();
        for id in [1, 2, 3, 4] {
            tokio::fs::write(folder.join(format!("mock_{}.png", id)), b"x")
                .await
                .unwrap();
        }

        let provider = MockProvider::new(vec![
            vec![fake_post(4), fake_post(3)],
            vec![fake_post(2), fake_post(1)],
            vec![],
        ]);
        let job = DownloadJob::new(provider, "anything");
        let (tx, _rx) = mpsc::channel(16);
        let cancel = CancellationToken::new();

        let outcome = run_job(job, dl, |_| false, tx, cancel).await.unwrap();
        assert_eq!(outcome.progress.skipped, 4);
        assert_eq!(outcome.progress.fetched, 4);
        assert_eq!(outcome.safe_last_post_id, 4);
    }

    #[tokio::test]
    async fn incremental_lookback_keeps_paging_after_caught_up() {
        let tmp = TempDir::new().unwrap();
        let dl = make_dl(tmp.path().to_path_buf());

        // Pre-fill ALL above-baseline posts so they dedup-skip rather than
        // touch the (fake) network.
        let folder = dl.folder_path("mock", "tag");
        tokio::fs::create_dir_all(&folder).await.unwrap();
        for id in 105..=115 {
            tokio::fs::write(folder.join(format!("mock_{}.png", id)), b"x")
                .await
                .unwrap();
        }

        // Page 1: posts 110-115 (above baseline 100) → candidates exist
        // Page 2: posts 105-109 (above baseline) → candidates exist
        // Page 3: posts 95-99   (all <= baseline 100) → caught-up, lookback=1
        // Page 4: posts 90-94   (all <= baseline 100) → caught-up, lookback=2 → break
        let provider = MockProvider::new(vec![
            (110..=115).rev().map(fake_post).collect(),
            (105..=109).rev().map(fake_post).collect(),
            (95..=99).rev().map(fake_post).collect(),
            (90..=94).rev().map(fake_post).collect(),
            vec![], // would terminate anyway
        ]);

        let mut job = DownloadJob::new(provider, "tag");
        job.since_post_id = Some(100);
        job.incremental_lookback_pages = 2;

        let (tx, _rx) = mpsc::channel(16);
        let outcome = run_job(job, dl, |_| false, tx, CancellationToken::new())
            .await
            .unwrap();

        // current_page tracks the last page where candidates were dispatched
        // (page 1 + page 2). Pages 3 and 4 are in lookback (continue early)
        // and never advance current_page.
        assert_eq!(outcome.progress.current_page, 2);
        assert_eq!(outcome.progress.skipped, 11);
        assert_eq!(outcome.progress.failed, 0);
    }

    #[tokio::test]
    async fn blacklist_does_not_terminate_when_new_ids_exist() {
        let tmp = TempDir::new().unwrap();
        let dl = make_dl(tmp.path().to_path_buf());

        let folder = dl.folder_path("mock", "tag");
        tokio::fs::create_dir_all(&folder).await.unwrap();
        // Pre-fill these so the test does not actually hit the network.
        for id in [201, 202, 203, 204] {
            tokio::fs::write(folder.join(format!("mock_{}.png", id)), b"x")
                .await
                .unwrap();
        }

        // Page 1 has new posts above baseline 100; ALL of them are blacklisted.
        // The runner must NOT terminate just because candidates is empty —
        // any_above_baseline is true.
        let page1: Vec<Post> = vec![201, 202, 203, 204]
            .into_iter()
            .map(|id| fake_post_with_tags(id, &["loli"]))
            .collect();
        // Page 2: empty → terminates normally.
        let provider = MockProvider::new(vec![page1, vec![]]);

        let mut job = DownloadJob::new(provider, "tag");
        job.since_post_id = Some(100);

        let (tx, _rx) = mpsc::channel(16);
        let blacklist = |tags: &[String]| tags.iter().any(|t| t == "loli");
        let outcome = run_job(job, dl, blacklist, tx, CancellationToken::new())
            .await
            .unwrap();

        // All 4 page-1 posts were filtered out, but we should have advanced
        // to page 2 (which is empty and terminates).
        assert_eq!(outcome.progress.fetched, 4);
        assert_eq!(outcome.progress.saved, 0);
        assert_eq!(outcome.progress.skipped, 0);
    }

    #[tokio::test]
    async fn job_respects_cancellation() {
        let tmp = TempDir::new().unwrap();
        let dl = make_dl(tmp.path().to_path_buf());

        let provider = MockProvider::new(vec![
            (1..=10).rev().map(fake_post).collect(),
            (11..=20).rev().map(fake_post).collect(),
        ]);
        let job = DownloadJob::new(provider, "anything");
        let (tx, _rx) = mpsc::channel(16);
        let cancel = CancellationToken::new();
        cancel.cancel();

        let outcome = run_job(job, dl, |_| false, tx, cancel).await.unwrap();
        assert_eq!(outcome.progress.fetched, 0);
    }
}
