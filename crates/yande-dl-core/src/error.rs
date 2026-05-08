use thiserror::Error;

#[derive(Error, Debug)]
pub enum CoreError {
    #[error("network error: {0}")]
    Network(#[from] reqwest::Error),

    #[error("rate limited (HTTP 429), retry after {retry_after_secs:?}s")]
    RateLimited { retry_after_secs: Option<u64> },

    #[error("server error: HTTP {status}")]
    Server { status: u16 },

    #[error("parse error: {0}")]
    Parse(String),

    #[error("io error: {0}")]
    Io(#[from] std::io::Error),

    #[error("md5 mismatch (expected {expected}, got {actual})")]
    Md5Mismatch { expected: String, actual: String },

    #[error("provider {0} not found")]
    UnknownProvider(String),

    #[error("operation cancelled")]
    Cancelled,
}
