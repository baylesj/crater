//! Error types for the SoundCloud v2 API client.
//!
//! We surface specific variants for cases the caller needs to react to
//! differently: e.g. a 401 probably means the scraped `client_id` rotated,
//! a 429 means we need to back off, a 404 on a track is expected normal flow.

use thiserror::Error;

#[derive(Debug, Error)]
pub enum ScError {
    #[error("HTTP error: {0}")]
    Http(#[from] reqwest::Error),

    #[error("Failed to deserialize response: {0}")]
    Deserialize(#[from] serde_json::Error),

    /// The scraped `client_id` is no longer valid. The caller should refresh
    /// it (re-scrape the SoundCloud JS bundles) and retry.
    #[error("SoundCloud rejected our client_id (401/403) — needs refresh")]
    AuthExpired,

    /// SoundCloud is rate-limiting us. Back off and retry later.
    #[error("Rate limited by SoundCloud (429)")]
    RateLimited,

    #[error("Resource not found (404)")]
    NotFound,

    /// Could not locate a `client_id` in the scraped SoundCloud JS bundles.
    /// If this happens, SoundCloud has likely changed the format of their
    /// web bundle and our regex needs updating.
    #[error("Could not extract client_id from SoundCloud: {0}")]
    ClientIdExtractionFailed(String),

    #[error("Unexpected SoundCloud response ({status}): {body}")]
    Unexpected { status: u16, body: String },
}

pub type Result<T> = std::result::Result<T, ScError>;
