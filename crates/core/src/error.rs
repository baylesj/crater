//! Error types for the core crate.

use thiserror::Error;

#[derive(Debug, Error)]
pub enum CoreError {
    #[error("database error: {0}")]
    Db(#[from] sqlx::Error),

    #[error("SoundCloud error: {0}")]
    Sc(#[from] sc_client::ScError),

    #[error("JSON error: {0}")]
    Json(#[from] serde_json::Error),

    #[error("invalid cron expression '{expr}': {detail}")]
    InvalidCron { expr: String, detail: String },

    #[error("invalid ranking '{0}'")]
    InvalidRanking(String),

    #[error("invalid track status '{0}'")]
    InvalidStatus(String),

    #[error("not found")]
    NotFound,

    #[error(transparent)]
    Other(#[from] anyhow::Error),
}

pub type Result<T> = std::result::Result<T, CoreError>;
