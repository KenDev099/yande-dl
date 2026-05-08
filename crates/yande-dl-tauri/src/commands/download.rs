use crate::events::{
    DownloadCompletedEvent, DownloadProgressEvent, NotificationEvent, EVENT_DOWNLOAD_COMPLETED,
    EVENT_DOWNLOAD_PROGRESS, EVENT_NOTIFICATION,
};
use crate::state::{ActiveJob, AppState};
use serde::Serialize;
use std::sync::Arc;
use tauri::{AppHandle, Emitter, State};
use tokio::sync::mpsc;
use tokio_util::sync::CancellationToken;
use yande_dl_config::Settings;
use yande_dl_core::downloader::Downloader;
use yande_dl_core::job::{run_job, DownloadJob, JobProgress};
use yande_dl_core::model::Rating;

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
    let (progress_tx, mut progress_rx) = mpsc::channel::<JobProgress>(16);

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

    // Forward progress events.
    let app_for_progress = app.clone();
    let job_id_for_progress = job_id.clone();
    let sub_id_for_progress = sub.id.clone();
    let active_jobs_for_progress = state.active_jobs.clone();
    tokio::spawn(async move {
        while let Some(progress) = progress_rx.recv().await {
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
