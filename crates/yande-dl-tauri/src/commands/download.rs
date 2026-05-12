use crate::events::{
    DownloadCompletedEvent, DownloadProgressEvent, NotificationEvent, PostStatusUpdateEvent,
    PostsDiscoveredEvent, EVENT_DOWNLOAD_COMPLETED, EVENT_DOWNLOAD_PROGRESS, EVENT_NOTIFICATION,
    EVENT_POSTS_DISCOVERED, EVENT_POST_STATUS,
};
use crate::state::{ActiveJob, AppState};
use futures::stream::{FuturesUnordered, StreamExt};
use serde::Serialize;
use std::sync::Arc;
use tauri::{AppHandle, Emitter, State};
use tokio::sync::mpsc;
use tokio_util::sync::CancellationToken;
use yande_dl_config::Settings;
use yande_dl_core::downloader::{DownloadStatus, Downloader};
use yande_dl_core::job::{run_job, DownloadJob, JobMessage, JobProgress, PostInfo, PostStatus};
use yande_dl_core::model::{Rating, SearchQuery};
use yande_dl_core::sanitize::normalize_tag;

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct StartDownloadResp {
    pub job_id: String,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ActiveJobDto {
    pub job_id: String,
    pub subscription_id: String,
    pub tag: String,
    pub current_page: u32,
    pub fetched: u32,
    pub saved: u32,
    pub skipped: u32,
    pub failed: u32,
    pub cancelled: u32,
}

fn ratings_from_settings(settings: &Settings) -> Vec<Rating> {
    settings
        .default_ratings
        .iter()
        .filter_map(|r| match r.to_lowercase().as_str() {
            "safe" => Some(Rating::Safe),
            "questionable" => Some(Rating::Questionable),
            "explicit" => Some(Rating::Explicit),
            _ => None,
        })
        .collect()
}

fn make_blacklist(blacklist: Vec<String>) -> impl Fn(&[String]) -> bool + Send + Sync + 'static {
    let normalized: Vec<String> = blacklist
        .into_iter()
        .map(|s| s.trim().to_lowercase())
        .filter(|s| !s.is_empty())
        .collect();
    move |tags: &[String]| {
        if normalized.is_empty() {
            return false;
        }
        tags.iter().any(|t| {
            let t_low = t.to_lowercase();
            normalized.iter().any(|b| b == &t_low)
        })
    }
}

#[tauri::command]
pub async fn start_download(
    state: State<'_, AppState>,
    app: AppHandle,
    subscription_id: String,
    incremental: bool,
) -> Result<StartDownloadResp, String> {
    let tags_file = state.tags.load().await.map_err(|e| e.to_string())?;
    let sub = tags_file
        .subscriptions
        .iter()
        .find(|s| s.id == subscription_id)
        .cloned()
        .ok_or_else(|| "subscription not found".to_string())?;

    {
        let active = state.active_jobs.lock().await;
        if active.values().any(|j| j.subscription_id == sub.id) {
            return Err("a download is already running for this subscription".into());
        }
    }

    let provider = state
        .providers
        .get(&sub.provider)
        .cloned()
        .ok_or_else(|| format!("unknown provider: {}", sub.provider))?;

    let settings = state.settings.load().await.map_err(|e| e.to_string())?;
    let download_root = settings
        .download_root
        .clone()
        .ok_or_else(|| "download root is not set".to_string())?;

    let downloader = Arc::new(Downloader::new(
        state.http_client.clone(),
        settings.concurrency.max(1) as usize,
        download_root,
        settings.min_delay_ms,
    ));

    let mut job = DownloadJob::new(provider, sub.tag.clone());
    if incremental {
        job.since_post_id = Some(sub.last_seen_post_id);
    }
    job.query_extra.ratings = ratings_from_settings(&settings);

    let job_id = uuid::Uuid::new_v4().to_string();
    let cancel = CancellationToken::new();
    // Capacity 128: per-post events (PostsDiscovered + PostStatus) cannot be
    // dropped or the thumbnail grid loses state. Burst per page ~50 posts × 2
    // transitions ≈ 100; 128 gives 1.3x headroom.
    let (progress_tx, mut progress_rx) = mpsc::channel::<JobMessage>(128);

    {
        let mut active = state.active_jobs.lock().await;
        active.insert(
            job_id.clone(),
            ActiveJob {
                job_id: job_id.clone(),
                subscription_id: sub.id.clone(),
                raw_tag: sub.tag.clone(),
                progress: JobProgress::default(),
                cancel: cancel.clone(),
            },
        );
    }

    let blacklist_match = make_blacklist(settings.blacklist.clone());

    // Forward progress events. Demux JobMessage → 3 different Tauri events.
    let app_for_progress = app.clone();
    let job_id_for_progress = job_id.clone();
    let sub_id_for_progress = sub.id.clone();
    let active_jobs_for_progress = state.active_jobs.clone();
    tokio::spawn(async move {
        while let Some(msg) = progress_rx.recv().await {
            match msg {
                JobMessage::PageProgress(progress) => {
                    {
                        let mut active = active_jobs_for_progress.lock().await;
                        if let Some(job) = active.get_mut(&job_id_for_progress) {
                            job.progress = progress.clone();
                        }
                    }
                    let _ = app_for_progress.emit(
                        EVENT_DOWNLOAD_PROGRESS,
                        DownloadProgressEvent {
                            job_id: job_id_for_progress.clone(),
                            subscription_id: sub_id_for_progress.clone(),
                            current_page: progress.current_page,
                            fetched: progress.fetched,
                            saved: progress.saved,
                            skipped: progress.skipped,
                            failed: progress.failed,
                            cancelled: progress.cancelled,
                        },
                    );
                }
                JobMessage::PostsDiscovered(posts) => {
                    let _ = app_for_progress.emit(
                        EVENT_POSTS_DISCOVERED,
                        PostsDiscoveredEvent {
                            job_id: job_id_for_progress.clone(),
                            subscription_id: sub_id_for_progress.clone(),
                            posts,
                        },
                    );
                }
                JobMessage::PostStatus(ev) => {
                    let _ = app_for_progress.emit(
                        EVENT_POST_STATUS,
                        PostStatusUpdateEvent {
                            job_id: job_id_for_progress.clone(),
                            subscription_id: sub_id_for_progress.clone(),
                            post_id: ev.post_id,
                            status: ev.status,
                        },
                    );
                }
            }
        }
    });

    let app_for_completion = app.clone();
    let active_jobs_for_completion = state.active_jobs.clone();
    let tags_store = state.tags.clone();
    let job_id_for_completion = job_id.clone();
    let sub_id_for_completion = sub.id.clone();

    tokio::spawn(async move {
        let outcome_result = run_job(job, downloader, blacklist_match, progress_tx, cancel).await;

        {
            let mut active = active_jobs_for_completion.lock().await;
            active.remove(&job_id_for_completion);
        }

        match outcome_result {
            Ok(outcome) => {
                if let Err(e) = tags_store
                    .update_after_run(
                        &sub_id_for_completion,
                        outcome.safe_last_post_id,
                        outcome.progress.saved as u64,
                    )
                    .await
                {
                    tracing::error!("update_after_run failed: {}", e);
                    let _ = app_for_completion.emit(
                        EVENT_NOTIFICATION,
                        NotificationEvent::warning(format!("could not save baseline: {}", e)),
                    );
                }

                let _ = app_for_completion.emit(
                    EVENT_DOWNLOAD_COMPLETED,
                    DownloadCompletedEvent {
                        job_id: job_id_for_completion,
                        subscription_id: sub_id_for_completion,
                        total_saved: outcome.progress.saved,
                        total_skipped: outcome.progress.skipped,
                        total_failed: outcome.progress.failed,
                        total_cancelled: outcome.progress.cancelled,
                        safe_last_post_id: outcome.safe_last_post_id,
                    },
                );
            }
            Err(e) => {
                tracing::error!("job failed: {}", e);
                let _ = app_for_completion.emit(
                    EVENT_NOTIFICATION,
                    NotificationEvent::error(format!("download failed: {}", e)),
                );
            }
        }
    });

    Ok(StartDownloadResp { job_id })
}

#[tauri::command]
pub async fn cancel_job(state: State<'_, AppState>, job_id: String) -> Result<(), String> {
    let active = state.active_jobs.lock().await;
    if let Some(job) = active.get(&job_id) {
        job.cancel.cancel();
        Ok(())
    } else {
        Err("job not found".into())
    }
}

/// Run a paginated search without downloading. Emits `PostsDiscovered` per page
/// so the frontend grid can render thumbnails, and caches full `Post` objects in
/// `state.recent_posts` for later use by `download_selected_posts`.
#[tauri::command]
pub async fn preview_subscription(
    state: State<'_, AppState>,
    app: AppHandle,
    subscription_id: String,
    page: Option<u32>,
    job_id: Option<String>,
) -> Result<StartDownloadResp, String> {
    let tags_file = state.tags.load().await.map_err(|e| e.to_string())?;
    let sub = tags_file
        .subscriptions
        .iter()
        .find(|s| s.id == subscription_id)
        .cloned()
        .ok_or_else(|| "subscription not found".to_string())?;

    let provider = state
        .providers
        .get(&sub.provider)
        .cloned()
        .ok_or_else(|| format!("unknown provider: {}", sub.provider))?;

    let settings = state.settings.load().await.map_err(|e| e.to_string())?;
    let download_root = settings
        .download_root
        .clone()
        .ok_or_else(|| "download root is not set".to_string())?;

    let normalized = normalize_tag(&sub.tag);
    let folder = Downloader::new(state.http_client.clone(), 1, download_root, 0)
        .folder_path(&sub.provider, &normalized);
    let known_ids = Downloader::scan_existing_post_ids(&folder, &sub.provider).await;

    let job_id = job_id.unwrap_or_else(|| uuid::Uuid::new_v4().to_string());
    let page = page.unwrap_or(1).max(1);

    let mut q = SearchQuery {
        ratings: ratings_from_settings(&settings),
        ..Default::default()
    };
    q.tags.insert(0, normalized.clone());
    q.limit = 100;

    let posts = provider.search(&q, page).await.map_err(|e| e.to_string())?;

    let post_infos: Vec<PostInfo> = posts
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

    {
        let mut cache = state.recent_posts.lock().await;
        for p in &posts {
            cache.insert((sub.id.clone(), p.post_id), p.clone());
        }
    }

    let _ = app.emit(
        EVENT_POSTS_DISCOVERED,
        PostsDiscoveredEvent {
            job_id: job_id.clone(),
            subscription_id: sub.id.clone(),
            posts: post_infos,
        },
    );

    let _ = app.emit(
        EVENT_DOWNLOAD_COMPLETED,
        DownloadCompletedEvent {
            job_id: job_id.clone(),
            subscription_id: sub.id.clone(),
            total_saved: 0,
            total_skipped: 0,
            total_failed: 0,
            total_cancelled: 0,
            safe_last_post_id: sub.last_seen_post_id,
        },
    );

    Ok(StartDownloadResp { job_id })
}

/// Download a user-selected subset of posts from `recent_posts` cache.
/// Does NOT advance `last_seen_post_id` — selected downloads are non-linear
/// (user picks specific post_ids), so bumping the baseline would skip posts.
#[tauri::command]
pub async fn download_selected_posts(
    state: State<'_, AppState>,
    app: AppHandle,
    subscription_id: String,
    post_ids: Vec<i64>,
) -> Result<StartDownloadResp, String> {
    if post_ids.is_empty() {
        return Err("no posts selected".into());
    }

    let tags_file = state.tags.load().await.map_err(|e| e.to_string())?;
    let sub = tags_file
        .subscriptions
        .iter()
        .find(|s| s.id == subscription_id)
        .cloned()
        .ok_or_else(|| "subscription not found".to_string())?;

    let settings = state.settings.load().await.map_err(|e| e.to_string())?;
    let download_root = settings
        .download_root
        .clone()
        .ok_or_else(|| "download root is not set".to_string())?;

    let posts: Vec<_> = {
        let cache = state.recent_posts.lock().await;
        let mut found = Vec::with_capacity(post_ids.len());
        for id in &post_ids {
            match cache.get(&(sub.id.clone(), *id)) {
                Some(p) => found.push(p.clone()),
                None => return Err("preview-cache miss; run preview first".into()),
            }
        }
        found
    };

    let downloader = Arc::new(Downloader::new(
        state.http_client.clone(),
        settings.concurrency.max(1) as usize,
        download_root,
        settings.min_delay_ms,
    ));

    let normalized = normalize_tag(&sub.tag);
    let folder = downloader.folder_path(&sub.provider, &normalized);
    let known_ids = Arc::new(Downloader::scan_existing_post_ids(&folder, &sub.provider).await);

    let job_id = uuid::Uuid::new_v4().to_string();
    let cancel = CancellationToken::new();

    {
        let mut active = state.active_jobs.lock().await;
        active.insert(
            job_id.clone(),
            ActiveJob {
                job_id: job_id.clone(),
                subscription_id: sub.id.clone(),
                raw_tag: sub.tag.clone(),
                progress: JobProgress::default(),
                cancel: cancel.clone(),
            },
        );
    }

    let app_clone = app.clone();
    let active_jobs = state.active_jobs.clone();
    let tags_store = state.tags.clone();
    let job_id_clone = job_id.clone();
    let sub_id_clone = sub.id.clone();

    tokio::spawn(async move {
        let mut futs = FuturesUnordered::new();
        for post in posts {
            let dl = downloader.clone();
            let known = known_ids.clone();
            let tag = normalized.clone();
            let token = cancel.clone();
            let app = app_clone.clone();
            let job_id = job_id_clone.clone();
            let sub_id = sub_id_clone.clone();
            let post_id = post.post_id;
            let will_skip = known.contains(&post_id);
            futs.push(tokio::spawn(async move {
                if !will_skip {
                    let _ = app.emit(
                        EVENT_POST_STATUS,
                        PostStatusUpdateEvent {
                            job_id: job_id.clone(),
                            subscription_id: sub_id.clone(),
                            post_id,
                            status: PostStatus::Downloading,
                        },
                    );
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
                let _ = app.emit(
                    EVENT_POST_STATUS,
                    PostStatusUpdateEvent {
                        job_id,
                        subscription_id: sub_id,
                        post_id,
                        status: final_status,
                    },
                );
                outcome
            }));
        }

        let mut progress = JobProgress::default();
        while let Some(res) = futs.next().await {
            if let Ok(outcome) = res {
                match &outcome.status {
                    DownloadStatus::Saved => progress.saved += 1,
                    DownloadStatus::SkippedDuplicate => progress.skipped += 1,
                    DownloadStatus::Cancelled => progress.cancelled += 1,
                    DownloadStatus::Failed(_) => progress.failed += 1,
                }
            } else {
                progress.failed += 1;
            }
            progress.fetched += 1;

            {
                let mut active = active_jobs.lock().await;
                if let Some(j) = active.get_mut(&job_id_clone) {
                    j.progress = progress.clone();
                }
            }
            let _ = app_clone.emit(
                EVENT_DOWNLOAD_PROGRESS,
                DownloadProgressEvent {
                    job_id: job_id_clone.clone(),
                    subscription_id: sub_id_clone.clone(),
                    current_page: 0,
                    fetched: progress.fetched,
                    saved: progress.saved,
                    skipped: progress.skipped,
                    failed: progress.failed,
                    cancelled: progress.cancelled,
                },
            );
        }

        {
            let mut active = active_jobs.lock().await;
            active.remove(&job_id_clone);
        }

        if let Err(e) = tags_store
            .bump_downloaded_count(&sub_id_clone, progress.saved as u64)
            .await
        {
            tracing::error!("bump_downloaded_count failed: {}", e);
        }

        let _ = app_clone.emit(
            EVENT_DOWNLOAD_COMPLETED,
            DownloadCompletedEvent {
                job_id: job_id_clone,
                subscription_id: sub_id_clone,
                total_saved: progress.saved,
                total_skipped: progress.skipped,
                total_failed: progress.failed,
                total_cancelled: progress.cancelled,
                safe_last_post_id: sub.last_seen_post_id,
            },
        );
    });

    Ok(StartDownloadResp { job_id })
}

#[tauri::command]
pub async fn list_active_jobs(state: State<'_, AppState>) -> Result<Vec<ActiveJobDto>, String> {
    let active = state.active_jobs.lock().await;
    let mut out: Vec<ActiveJobDto> = active
        .values()
        .map(|j| ActiveJobDto {
            job_id: j.job_id.clone(),
            subscription_id: j.subscription_id.clone(),
            tag: j.raw_tag.clone(),
            current_page: j.progress.current_page,
            fetched: j.progress.fetched,
            saved: j.progress.saved,
            skipped: j.progress.skipped,
            failed: j.progress.failed,
            cancelled: j.progress.cancelled,
        })
        .collect();
    out.sort_by(|a, b| a.tag.cmp(&b.tag));
    Ok(out)
}
