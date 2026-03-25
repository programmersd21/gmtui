use thiserror::Error;

#[derive(Debug, Error)]
pub enum GmtuiError {
    #[error("auth error: {0}")]
    Auth(String),
    #[error("api error: {0}")]
    Api(String),
    #[error("config error: {0}")]
    Config(String),
    #[error(transparent)]
    Io(#[from] std::io::Error),
    #[error(transparent)]
    Json(#[from] serde_json::Error),
}

pub type Result<T> = std::result::Result<T, GmtuiError>;
