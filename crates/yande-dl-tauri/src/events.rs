use serde::Serialize;

pub const EVENT_DOWNLOAD_PROGRESS: &str = "download:progress";
pub const EVENT_DOWNLOAD_COMPLETED: &str = "download:completed";
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
