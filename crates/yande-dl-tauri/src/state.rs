use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::Mutex;
use tokio_util::sync::CancellationToken;
use yande_dl_config::{SettingsStore, TagsStore};
use yande_dl_core::job::JobProgress;
use yande_dl_core::provider::ImageProvider;

#[derive(Debug, Clone)]
pub struct ActiveJob {
    pub job_id: String,
    pub subscription_id: String,
    pub raw_tag: String,
    pub progress: JobProgress,
    pub cancel: CancellationToken,
}

pub struct AppState {
    pub tags: Arc<TagsStore>,
    pub settings: Arc<SettingsStore>,
    pub providers: HashMap<String, Arc<dyn ImageProvider>>,
    pub http_client: reqwest::Client,
    pub active_jobs: Arc<Mutex<HashMap<String, ActiveJob>>>,
}
