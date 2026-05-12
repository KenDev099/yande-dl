use crate::state::AppState;
use std::path::PathBuf;
use tauri::{AppHandle, State};
use tauri_plugin_opener::OpenerExt;

#[tauri::command]
pub async fn open_folder(
    app: AppHandle,
    state: State<'_, AppState>,
    path: Option<PathBuf>,
) -> Result<(), String> {
    let target = match path {
        Some(p) => p,
        None => state
            .settings
            .load()
            .await
            .map_err(|e| e.to_string())?
            .download_root
            .ok_or_else(|| "download root is not set".to_string())?,
    };

    if !target.exists() {
        std::fs::create_dir_all(&target).map_err(|e| e.to_string())?;
    }

    app.opener()
        .open_path(target.to_string_lossy().to_string(), None::<&str>)
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn open_url(app: AppHandle, url: String) -> Result<(), String> {
    if !(url.starts_with("http://") || url.starts_with("https://")) {
        return Err("only http(s) URLs are allowed".into());
    }
    app.opener()
        .open_url(url, None::<&str>)
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn open_post_url(
    app: AppHandle,
    state: State<'_, AppState>,
    provider: String,
    post_id: i64,
) -> Result<(), String> {
    let base = match provider.as_str() {
        "yande" => "https://yande.re/post/show/",
        "konachan" => "https://konachan.com/post/show/",
        _ => return Err(format!("unknown provider: {}", provider)),
    };
    let _ = state; // unused but reserved for future per-provider URL templates
    let url = format!("{}{}", base, post_id);
    app.opener()
        .open_url(url, None::<&str>)
        .map_err(|e| e.to_string())
}
