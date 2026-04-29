//! Search session — the dedup-aware pagination primitive.
//!
//! A `Session` wraps a set of `SearchFilters` and a reference to the DB +
//! sc_client. Calling `next_batch` fetches pages from SoundCloud, upserts
//! every track (refreshing play counts), then filters out any track the user
//! has already rejected or exported. Hearted and queued tracks are included
//! so the UI can badge them.
//!
//! Tracks are delivered to the caller via an `on_track` callback as each page
//! arrives rather than after the full scan completes. This lets the server
//! broadcast WS events in real time while still upsert-refreshing every track.

use std::time::Duration;

use sqlx::SqlitePool;
use uuid::Uuid;

use crate::error::Result;
use crate::tracks::{get_track, upsert_track, StoredTrack};

/// How long to wait between page fetches. Keeps us well within SC's informal
/// rate limits while having negligible impact on perceived latency per track.
const PAGE_DELAY: Duration = Duration::from_millis(300);

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
    /// True when SC returned no `next_href` (search result set exhausted).
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

    /// Fetch the next batch of unsuppressed tracks, streaming results as each
    /// page arrives.
    ///
    /// Scans up to `max_pages` SoundCloud pages to accumulate at least `min`
    /// tracks not previously rejected/exported. May return fewer if the search
    /// is exhausted. The `on_track` callback fires for each qualifying track
    /// immediately after its page is upserted, so callers can emit WS events
    /// without waiting for the full scan to finish.
    pub async fn next_batch<F>(
        &mut self,
        min:       usize,
        max_pages: u32,
        mut on_track: F,
    ) -> Result<SessionBatch>
    where
        F: FnMut(StoredTrack, u32 /* pages_scanned */, u32 /* total_scanned */),
    {
        let mut pages_scanned: u32 = 0;
        let mut total_scanned: u32 = 0;
        let mut accepted: Vec<StoredTrack> = Vec::new();
        let mut exhausted = false;
        let mut next_href: Option<String> = None;

        while pages_scanned < max_pages && accepted.len() < min {
            // Fetch the next page: first call uses the filter-built URL;
            // subsequent calls follow SC's next_href cursor.
            let resp = match next_href.take() {
                None       => self.sc.search_tracks(&self.filters).await?,
                Some(href) => self.sc.fetch_search_page(&href).await?,
            };

            pages_scanned += 1;
            total_scanned += resp.collection.len() as u32;

            // Upsert the full page (refreshes play counts for dedup check).
            for track in &resp.collection {
                upsert_track(&self.db, track).await?;
            }

            // Deliver qualifying tracks to the caller immediately.
            for track in &resp.collection {
                if accepted.len() >= min {
                    break;
                }
                if track.passes_client_filter(&self.filters) {
                    if let Some(stored) = get_track(&self.db, track.id as i64).await? {
                        if !stored.is_suppressed() {
                            on_track(stored.clone(), pages_scanned, total_scanned);
                            accepted.push(stored);
                        }
                    }
                }
            }

            match resp.next_href {
                None => {
                    exhausted = true;
                    break;
                }
                Some(href) => {
                    if accepted.len() < min && pages_scanned < max_pages {
                        tokio::time::sleep(PAGE_DELAY).await;
                        next_href = Some(href);
                    }
                }
            }
        }

        Ok(SessionBatch {
            tracks: accepted,
            pages_scanned,
            total_scanned,
            exhausted,
        })
    }
}
