use crate::state::AppState;
use serde::Serialize;
use std::path::PathBuf;
use tauri::State;
use yande_dl_config::{ImportMode, ImportReport, Subscription};
use yande_dl_core::downloader::Downloader;
use yande_dl_core::sanitize::normalize_tag;

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SubscriptionDto {
    pub id: String,
    pub provider: String,
    pub provider_display_name: String,
    pub tag: String,
    pub normalized_tag: String,
    pub display_name: Option<String>,
    pub last_run_at: Option<i64>,
    pub last_seen_post_id: i64,
    pub created_at: i64,
}

impl SubscriptionDto {
    fn from(sub: Subscription, provider_display_name: String) -> Self {
        Self {
            id: sub.id,
            provider: sub.provider,
            provider_display_name,
            tag: sub.tag,
            normalized_tag: sub.normalized_tag,
            display_name: sub.display_name,
            last_run_at: sub.last_run_at,
            last_seen_post_id: sub.last_seen_post_id,
            created_at: sub.created_at,
        }
    }
}

fn provider_display_name(state: &AppState, provider_id: &str) -> String {
    state
        .providers
        .get(provider_id)
        .map(|p| p.display_name().to_string())
        .unwrap_or_else(|| provider_id.to_string())
}

#[tauri::command]
pub async fn list_subscriptions(
    state: State<'_, AppState>,
) -> Result<Vec<SubscriptionDto>, String> {
    let file = state.tags.load().await.map_err(|e| e.to_string())?;
    Ok(file
        .subscriptions
        .into_iter()
        .map(|s| {
            let display = provider_display_name(&state, &s.provider);
            SubscriptionDto::from(s, display)
        })
        .collect())
}

#[tauri::command]
pub async fn add_subscription(
    state: State<'_, AppState>,
    provider: String,
    tag: String,
    display_name: Option<String>,
) -> Result<SubscriptionDto, String> {
    if !state.providers.contains_key(&provider) {
        return Err(format!("unknown provider: {}", provider));
    }
    let sub = state
        .tags
        .add_with_display_name(&provider, &tag, display_name)
        .await
        .map_err(|e| e.to_string())?;
    let display = provider_display_name(&state, &sub.provider);
    Ok(SubscriptionDto::from(sub, display))
}

#[tauri::command]
pub async fn update_subscription_display_name(
    state: State<'_, AppState>,
    id: String,
    display_name: Option<String>,
) -> Result<SubscriptionDto, String> {
    let updated = state
        .tags
        .update_display_name(&id, display_name)
        .await
        .map_err(|e| e.to_string())?
        .ok_or_else(|| "subscription not found".to_string())?;
    let display = provider_display_name(&state, &updated.provider);
    Ok(SubscriptionDto::from(updated, display))
}

#[tauri::command]
pub async fn remove_subscription(state: State<'_, AppState>, id: String) -> Result<(), String> {
    state.tags.remove(&id).await.map_err(|e| e.to_string())?;
    Ok(())
}

#[tauri::command]
pub async fn export_subscriptions(state: State<'_, AppState>, dest: PathBuf) -> Result<(), String> {
    state.tags.export_to(&dest).await.map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn import_subscriptions(
    state: State<'_, AppState>,
    source: PathBuf,
    mode: ImportMode,
) -> Result<ImportReport, String> {
    state
        .tags
        .import_from(&source, mode)
        .await
        .map_err(|e| e.to_string())
}

/// Count of files already on disk for this subscription. This is the truthful
/// answer (per invariant #1: filesystem is the source of truth) and replaces
/// the legacy `total_downloaded` counter which drifted under repeated runs.
#[tauri::command]
pub async fn count_downloaded_files(
    state: State<'_, AppState>,
    subscription_id: String,
) -> Result<u64, String> {
    let file = state.tags.load().await.map_err(|e| e.to_string())?;
    let sub = file
        .subscriptions
        .into_iter()
        .find(|s| s.id == subscription_id)
        .ok_or_else(|| "subscription not found".to_string())?;

    let settings = state.settings.load().await.map_err(|e| e.to_string())?;
    let root = match settings.download_root {
        Some(p) => p,
        None => return Ok(0),
    };

    let folder = Downloader::new(state.http_client.clone(), 1, root, 0)
        .folder_path(&sub.provider, &normalize_tag(&sub.tag));
    let ids = Downloader::scan_existing_post_ids(&folder, &sub.provider).await;
    Ok(ids.len() as u64)
}
