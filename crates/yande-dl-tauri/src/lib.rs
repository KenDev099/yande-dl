use tauri::Manager;
use tracing_subscriber::EnvFilter;

mod commands;
mod events;
mod http;
mod setup;
mod state;

pub use state::AppState;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    let filter = EnvFilter::try_from_env("KURA_LOG").unwrap_or_else(|_| EnvFilter::new("info"));
    tracing_subscriber::fmt().with_env_filter(filter).init();

    tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_opener::init())
        .setup(|app| {
            let state = setup::build_state()?;
            app.manage(state);
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            commands::subscriptions::list_subscriptions,
            commands::subscriptions::add_subscription,
            commands::subscriptions::remove_subscription,
            commands::subscriptions::export_subscriptions,
            commands::subscriptions::import_subscriptions,
            commands::download::start_download,
            commands::download::cancel_job,
            commands::download::list_active_jobs,
            commands::settings::get_settings,
            commands::settings::update_settings,
            commands::system::open_folder,
            commands::system::open_post_url,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
