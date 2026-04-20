//! Search session — the dedup-aware pagination primitive.
//!
//! A `Session` wraps a set of `SearchFilters` and a reference to the DB +
//! sc_client. Calling `next_batch` fetches pages from SoundCloud, upserts
//! every track (refreshing play counts), then filters out any track the user
//! has already rejected or exported. Hearted and queued tracks are included
//! so the UI can badge them.

use sqlx::SqlitePool;
use uuid::Uuid;

use crate::error::Result;
use crate::tracks::{get_track, upsert_track, StoredTrack};

pub struct Session {
    pub id: Uuid,
    filters: sc_client::SearchFilters,
    db: SqlitePool,
    sc: sc_client::Client,
}

pub struct SessionBatch {
    pub tracks:        Vec<StoredTrack>,
    /// Pages fetched from SoundCloud in this call.
    pub pages_scanned: u32,
    /// Raw tracks returned by sc_client before client-side filtering.
    pub total_scanned: u32,
    /// True if the search result set was exhausted (no more pages).
    pub exhausted:     bool,
}

impl Session {
    pub(crate) fn new(
        filters: sc_client::SearchFilters,
        db:      SqlitePool,
        sc:      sc_client::Client,
    ) -> Self {
        Self { id: Uuid::new_v4(), filters, db, sc }
    }

    /// Fetch the next batch of unsuppressed tracks.
    ///
    /// Scans up to `max_pages` SoundCloud pages to accumulate at least `min`
    /// tracks not previously rejected/exported. May return fewer if the
    /// search is exhausted.
    pub async fn next_batch(&mut self, min: usize, max_pages: u32) -> Result<SessionBatch> {
        let mut pages_scanned: u32 = 0;
        let mut total_scanned: u32 = 0;

        // Ask sc_client for more than `min` raw tracks because many will be
        // suppressed by the dedup filter below.
        let raw_tracks = self
            .sc
            .search_tracks_filtered(
                &self.filters,
                min * 5,
                max_pages as usize,
                |page, _| {
                    pages_scanned  += 1;
                    total_scanned  += page.len() as u32;
                    true
                },
            )
            .await?;

        let exhausted = raw_tracks.len() < min;

        // Upsert all fetched tracks (refreshes play counts in DB).
        for track in &raw_tracks {
            upsert_track(&self.db, track).await?;
        }

        // Filter out rejected/exported; include hearted/queued with badge info.
        let mut fresh = Vec::with_capacity(min);
        for track in &raw_tracks {
            if fresh.len() >= min {
                break;
            }
            if let Some(stored) = get_track(&self.db, track.id as i64).await? {
                if !stored.is_suppressed() {
                    fresh.push(stored);
                }
            }
        }

        Ok(SessionBatch {
            tracks: fresh,
            pages_scanned,
            total_scanned,
            exhausted,
        })
    }
}
