use crate::state::AppState;
use tauri::State;
use yande_dl_config::Settings;

#[tauri::command]
pub async fn get_settings(state: State<'_, AppState>) -> Result<Settings, String> {
    state.settings.load().await.map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn update_settings(
    state: State<'_, AppState>,
    settings: Settings,
) -> Result<Settings, String> {
    state
        .settings
        .save(&settings)
        .await
        .map_err(|e| e.to_string())?;
    Ok(settings)
}
