//! `crater-core` — business logic and data layer.
//!
//! The `Crater` struct is the single facade the `server` crate talks to.
//! It owns the SQLite pool and the sc_client instance; all other modules
//! are internal implementation details exposed only through this API.
//!
//! `pub use sc_client` re-exports the client crate so `server` only needs to
//! depend on `crater-core`, not both.

pub use sc_client;
// Convenience re-exports so dependents don't need to depend on sc_client directly.
pub use sc_client::{SearchFilters, Track};

pub mod db;
pub mod digest_runner;
pub mod digests;
pub mod error;
pub mod filters;
pub mod session;
pub mod tracks;

pub use digest_runner::{DigestRun, RunStatus};
pub use digests::{Digest, DigestRunRow, DigestSpec, PlaylistVisibility};
pub use error::{CoreError, Result};
pub use filters::Ranking;
pub use session::{Session, SessionBatch};
pub use tracks::{
    clear_status, get_track, set_status, tracks_with_status, upsert_track, StoredTrack, TrackStatus,
};

use std::path::PathBuf;
use sqlx::SqlitePool;

// ── Config ───────────────────────────────────────────────────────────────────

pub struct Config {
    /// Directory where `crater.db` lives. Created on first run if absent.
    pub data_dir:         PathBuf,
    /// Official API credentials. When present, used for client credentials
    /// token flow instead of scraping. Also required for PKCE playlist export.
    pub sc_oauth_cfg:     Option<sc_client::OAuthConfig>,
    /// Fallback: manually captured OAuth token for playlist export.
    pub sc_oauth_token:   Option<String>,
    /// Cached scraped `client_id` (used only when `sc_oauth_cfg` is absent).
    pub cached_client_id: Option<String>,
}

// ── Crater facade ────────────────────────────────────────────────────────────

pub struct Crater {
    pub(crate) db:        SqlitePool,
    pub(crate) sc:        sc_client::Client,
    pub oauth_token:      Option<String>,
}

impl Crater {
    pub async fn new(config: Config) -> Result<Self> {
        let db = db::open(&config.data_dir).await?;
        let sc = if let Some(oauth_cfg) = config.sc_oauth_cfg {
            sc_client::Client::with_oauth(oauth_cfg)?
        } else if let Some(id) = config.cached_client_id {
            sc_client::Client::with_client_id(id)?
        } else {
            sc_client::Client::new()?
        };
        Ok(Self { db, sc, oauth_token: config.sc_oauth_token })
    }

    // ── Track operations ─────────────────────────────────────────────────

    pub async fn upsert_track(&self, track: &sc_client::Track) -> Result<()> {
        tracks::upsert_track(&self.db, track).await
    }

    pub async fn set_status(&self, track_id: i64, status: TrackStatus) -> Result<()> {
        tracks::set_status(&self.db, track_id, &status, None).await
    }

    pub async fn set_status_with_note(
        &self,
        track_id: i64,
        status:   TrackStatus,
        note:     &str,
    ) -> Result<()> {
        tracks::set_status(&self.db, track_id, &status, Some(note)).await
    }

    pub async fn clear_status(&self, track_id: i64) -> Result<()> {
        tracks::clear_status(&self.db, track_id).await
    }

    pub async fn get_track(&self, track_id: i64) -> Result<Option<StoredTrack>> {
        tracks::get_track(&self.db, track_id).await
    }

    pub async fn tracks_with_status(&self, status: TrackStatus) -> Result<Vec<StoredTrack>> {
        tracks::tracks_with_status(&self.db, &status).await
    }

    // ── Key-value store ──────────────────────────────────────────────────

    pub async fn set_kv(&self, key: &str, value: &str) -> Result<()> {
        sqlx::query(
            "INSERT INTO kv (k, v, updated_at) VALUES (?, ?, CURRENT_TIMESTAMP)
             ON CONFLICT(k) DO UPDATE SET v=excluded.v, updated_at=CURRENT_TIMESTAMP"
        )
        .bind(key)
        .bind(value)
        .execute(&self.db)
        .await?;
        Ok(())
    }

    pub async fn get_kv(&self, key: &str) -> Result<Option<String>> {
        let row: Option<(String,)> = sqlx::query_as("SELECT v FROM kv WHERE k = ?")
            .bind(key)
            .fetch_optional(&self.db)
            .await?;
        Ok(row.map(|(v,)| v))
    }

    // ── Audio streaming ──────────────────────────────────────────────────

    /// Resolve the HLS manifest URL for a stored track.
    ///
    /// Tries to extract the transcoding URL from cached `raw_json`. If the
    /// track predates the `media` field being captured (or SC returned a sparse
    /// response), re-fetches the track from the API and updates the DB cache.
    pub async fn stream_url(&self, track_id: i64) -> Result<String> {
        let stored = tracks::get_track(&self.db, track_id).await?
            .ok_or(CoreError::NotFound)?;

        // Try cached raw_json first — avoids an extra round-trip.
        let transcoding_url = stored.raw_json
            .as_deref()
            .and_then(|j| serde_json::from_str::<sc_client::Track>(j).ok())
            .and_then(|t| sc_client::pick_hls_transcoding(&t).map(str::to_owned));

        let transcoding_url = match transcoding_url {
            Some(url) => url,
            None => {
                // raw_json missing media field — re-fetch and update cache.
                tracing::info!(track_id, "media.transcodings absent, re-fetching from SC");
                let fresh = self.sc.fetch_track(track_id as u64).await
                    .map_err(CoreError::Sc)?;
                tracks::upsert_track(&self.db, &fresh).await?;
                sc_client::pick_hls_transcoding(&fresh)
                    .map(str::to_owned)
                    .ok_or_else(|| CoreError::Other(
                        anyhow::anyhow!("no HLS transcoding available for track {track_id}")
                    ))?
            }
        };

        self.sc.resolve_stream_url(&transcoding_url).await
            .map_err(CoreError::Sc)
    }

    /// Fetch raw bytes from a pre-signed CDN URL (HLS segments, manifests).
    pub async fn fetch_bytes(&self, url: &str) -> Result<Vec<u8>> {
        self.sc.fetch_bytes(url).await.map_err(CoreError::Sc)
    }

    // ── OAuth token ──────────────────────────────────────────────────────

    /// In-memory field (env var at startup) first, kv store (saved via
    /// settings page / PKCE flow) as fallback.
    pub async fn resolve_oauth_token(&self) -> Result<Option<String>> {
        if let Some(t) = &self.oauth_token {
            return Ok(Some(t.clone()));
        }
        self.get_kv("sc_access_token").await
    }

    // ── Playlist export ──────────────────────────────────────────────────

    /// Create a SoundCloud playlist from stored track IDs (which equal SC
    /// track IDs). Resolves the OAuth token automatically.
    /// Caller marks tracks exported after a successful return.
    pub async fn create_playlist(
        &self,
        title:     &str,
        sharing:   &str,
        track_ids: &[i64],
    ) -> Result<sc_client::CreatedPlaylist> {
        let token = self.resolve_oauth_token().await?
            .ok_or_else(|| CoreError::Other(anyhow::anyhow!(
                "SoundCloud OAuth token not configured — save a token in Settings \
                 or set CRATER_SC_OAUTH_TOKEN"
            )))?;
        let sc_ids: Vec<u64> = track_ids.iter().map(|&id| id as u64).collect();
        self.sc
            .create_playlist(&token, title, sharing, &sc_ids)
            .await
            .map_err(CoreError::Sc)
    }

    // ── Session ──────────────────────────────────────────────────────────

    pub fn new_session(&self, filters: sc_client::SearchFilters) -> Session {
        Session::new(filters, self.db.clone(), self.sc.clone())
    }

    // ── Digest CRUD ──────────────────────────────────────────────────────

    pub async fn create_digest(&self, spec: DigestSpec) -> Result<Digest> {
        digests::create_digest(&self.db, &spec).await
    }

    pub async fn list_digests(&self) -> Result<Vec<Digest>> {
        digests::list_digests(&self.db).await
    }

    pub async fn get_digest(&self, id: i64) -> Result<Option<Digest>> {
        digests::get_digest(&self.db, id).await
    }

    pub async fn update_digest(&self, id: i64, spec: DigestSpec) -> Result<Digest> {
        digests::update_digest(&self.db, id, &spec).await
    }

    pub async fn set_digest_enabled(&self, id: i64, enabled: bool) -> Result<()> {
        digests::set_digest_enabled(&self.db, id, enabled).await
    }

    pub async fn delete_digest(&self, id: i64) -> Result<()> {
        digests::delete_digest(&self.db, id).await
    }

    // ── Digest execution ─────────────────────────────────────────────────

    pub async fn run_digest(&self, id: i64) -> Result<DigestRun> {
        let oauth_token = self.resolve_oauth_token().await?;
        digest_runner::run_digest(&self.db, &self.sc, id, oauth_token.as_deref()).await
    }

    pub async fn list_digest_runs(&self, digest_id: i64, limit: i64) -> Result<Vec<DigestRunRow>> {
        digests::list_digest_runs(&self.db, digest_id, limit).await
    }
}
