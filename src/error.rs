#[derive(Debug, thiserror::Error)]
pub enum AppError {
    #[error("HTTP error: {0}")]
    Http(#[from] reqwest::Error),
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),
    #[error("JSON error: {0}")]
    Json(#[from] serde_json::Error),
    #[error("Request timed out")]
    Timeout,
    #[error("Request cancelled")]
    Cancelled,
    #[error("{0}")]
    Other(String),
}
