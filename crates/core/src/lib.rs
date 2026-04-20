//! `crater-core` — business logic and data layer.
//!
//! The `Crater` struct is the single facade the `server` crate talks to.
//! It owns the SQLite pool and the sc_client instance; all other modules
//! are internal implementation details exposed only through this API.
//!
//! `pub use sc_client` re-exports the client crate so `server` only needs to
//! depend on `crater-core`, not both.

pub use sc_client;

mod db;
pub mod digest_runner;
pub mod digests;
pub mod error;
pub mod filters;
pub mod session;
pub mod tracks;

pub use digest_runner::{DigestRun, RunStatus};
pub use digests::{Digest, DigestSpec, PlaylistVisibility};
pub use error::{CoreError, Result};
pub use filters::Ranking;
pub use session::{Session, SessionBatch};
pub use tracks::{StoredTrack, TrackStatus};

use std::path::PathBuf;
use sqlx::SqlitePool;

// ── Config ───────────────────────────────────────────────────────────────────

pub struct Config {
    /// Directory where `crater.db` lives. Created on first run if absent.
    pub data_dir:          PathBuf,
    /// SoundCloud OAuth token for playlist export. Nil until the user runs
    /// the DevTools capture flow (see docs/04-oauth-capture.md).
    pub sc_oauth_token:    Option<String>,
    /// Cached `client_id` from a previous run. Skips the initial scrape.
    pub cached_client_id:  Option<String>,
}

// ── Crater facade ────────────────────────────────────────────────────────────

pub struct Crater {
    pub(crate) db:          SqlitePool,
    pub(crate) sc:          sc_client::Client,
    pub(crate) oauth_token: Option<String>,
}

impl Crater {
    pub async fn new(config: Config) -> Result<Self> {
        let db = db::open(&config.data_dir).await?;
        let sc = match config.cached_client_id {
            Some(id) => sc_client::Client::with_client_id(id)?,
            None     => sc_client::Client::new()?,
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

    pub async fn delete_digest(&self, id: i64) -> Result<()> {
        digests::delete_digest(&self.db, id).await
    }

    // ── Digest execution ─────────────────────────────────────────────────

    pub async fn run_digest(&self, id: i64) -> Result<DigestRun> {
        digest_runner::run_digest(&self.db, &self.sc, id, self.oauth_token.as_deref()).await
    }
}
