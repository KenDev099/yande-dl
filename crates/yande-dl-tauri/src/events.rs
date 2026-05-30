use serde::Serialize;
use yande_dl_core::job::{PostInfo, PostStatus};

pub const EVENT_DOWNLOAD_PROGRESS: &str = "download:progress";
pub const EVENT_DOWNLOAD_COMPLETED: &str = "download:completed";
pub const EVENT_POSTS_DISCOVERED: &str = "download:postsDiscovered";
pub const EVENT_POST_STATUS: &str = "download:postStatus";
pub const EVENT_BATCH_PROGRESS: &str = "batch:progress";
pub const EVENT_BATCH_COMPLETED: &str = "batch:completed";
pub const EVENT_NOTIFICATION: &str = "notification";

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DownloadProgressEvent {
    pub job_id: String,
    pub subscription_id: String,
    pub current_page: u32,
    pub fetched: u32,
    pub saved: u32,
    pub skipped: u32,
    pub failed: u32,
    pub cancelled: u32,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PostsDiscoveredEvent {
    pub job_id: String,
    pub subscription_id: String,
    pub posts: Vec<PostInfo>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PostStatusUpdateEvent {
    pub job_id: String,
    pub subscription_id: String,
    pub post_id: i64,
    pub status: PostStatus,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DownloadCompletedEvent {
    pub job_id: String,
    pub subscription_id: String,
    pub total_saved: u32,
    pub total_skipped: u32,
    pub total_failed: u32,
    pub total_cancelled: u32,
    pub safe_last_post_id: i64,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct BatchProgressEvent {
    pub batch_id: String,
    pub current_index: u32,
    pub total: u32,
    pub current_subscription_id: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct BatchCompletedEvent {
    pub batch_id: String,
    pub processed: u32,
    pub total: u32,
    pub cancelled: bool,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct NotificationEvent {
    pub kind: String, // "info" | "success" | "warning" | "error"
    pub message: String,
}

#[allow(dead_code)] // info/success are used in v0.1-beta+
impl NotificationEvent {
    pub fn error(message: impl Into<String>) -> Self {
        Self {
            kind: "error".into(),
            message: message.into(),
        }
    }
    pub fn warning(message: impl Into<String>) -> Self {
        Self {
            kind: "warning".into(),
            message: message.into(),
        }
    }
    pub fn info(message: impl Into<String>) -> Self {
        Self {
            kind: "info".into(),
            message: message.into(),
        }
    }
    pub fn success(message: impl Into<String>) -> Self {
        Self {
            kind: "success".into(),
            message: message.into(),
        }
    }
}
