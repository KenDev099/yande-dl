use crate::http::build_client;
use crate::state::AppState;
use anyhow::Result;
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::Mutex;
use yande_dl_config::{AppPaths, SettingsStore, TagsStore};
use yande_dl_core::provider::ImageProvider;
use yande_dl_providers::MoebooruProvider;

pub fn build_state() -> Result<AppState> {
    let paths = AppPaths::resolve()?;
    let tags = Arc::new(TagsStore::new(&paths));
    let settings = Arc::new(SettingsStore::new(&paths));

    let http_client = build_client();

    let mut providers: HashMap<String, Arc<dyn ImageProvider>> = HashMap::new();
    providers.insert(
        "yande".into(),
        Arc::new(MoebooruProvider::yandere(http_client.clone())),
    );
    providers.insert(
        "konachan".into(),
        Arc::new(MoebooruProvider::konachan(http_client.clone())),
    );

    Ok(AppState {
        tags,
        settings,
        providers,
        http_client,
        active_jobs: Arc::new(Mutex::new(HashMap::new())),
        active_batch: Arc::new(Mutex::new(None)),
        recent_posts: Arc::new(Mutex::new(HashMap::new())),
    })
}
