//! Unofficial SoundCloud v2 API client for crater.
//!
//! Scope: what this crate does, and deliberately does not do.
//!
//! ## Does
//! - Scrapes a public `client_id` from SoundCloud's web bundles (rotated
//!   every few weeks, auto-refreshed on 401).
//! - Searches tracks with server-side filters (genre, BPM, duration) and
//!   client-side filters (play-count ceiling, likes floor).
//! - Paginates v2 responses and streams filtered results page-by-page via
//!   a callback, so UIs can render progressively rather than block.
//!
//! ## Does not (yet)
//! - Playlist CRUD — needs a user OAuth token, which requires a manual
//!   auth step. Will live in a separate `playlist` module in a later pass.
//! - Streaming URL resolution — the v2 `/tracks/:id/streams` endpoint
//!   requires authentication and returns HLS manifests; deferred until
//!   we have the audio-preview UI ready to consume them.
//! - Writes of any kind.
//!
//! ## Ethical scope
//! This client only reads public metadata. Do not use it to scrape audio,
//! circumvent paywalls, or operate at scale against SoundCloud. It exists
//! to let one person find obscure tracks more efficiently than the web UI
//! allows, which is in the spirit of SoundCloud's platform.

mod client;
mod client_id;
mod error;
mod types;

pub use client::Client;
pub use client_id::extract_client_id;
pub use error::{Result, ScError};
pub use types::{SearchFilters, SearchResponse, Track, User};
