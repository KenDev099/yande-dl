use crate::state::AppState;
use serde::Serialize;
use std::path::PathBuf;
use tauri::State;
use yande_dl_config::{ImportMode, ImportReport, Subscription};

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SubscriptionDto {
    pub id: String,
    pub provider: String,
    pub provider_display_name: String,
    pub tag: String,
    pub normalized_tag: String,
    pub last_run_at: Option<i64>,
    pub last_seen_post_id: i64,
    pub total_downloaded: u64,
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
            last_run_at: sub.last_run_at,
            last_seen_post_id: sub.last_seen_post_id,
            total_downloaded: sub.total_downloaded,
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
) -> Result<SubscriptionDto, String> {
    if !state.providers.contains_key(&provider) {
        return Err(format!("unknown provider: {}", provider));
    }
    let sub = state
        .tags
        .add(&provider, &tag)
        .await
        .map_err(|e| e.to_string())?;
    let display = provider_display_name(&state, &sub.provider);
    Ok(SubscriptionDto::from(sub, display))
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
