//! Social-graph endpoints: who liked or reposted a track, and what a user
//! liked or reposted.
//!
//! SoundCloud search ranks by keyword relevance and popularity, so it can't
//! surface tracks that only a handful of people know about. Those people can
//! be found, though: they are the likers and reposters of tracks you already
//! love. These calls are the raw material for the crawler in `core::dig`.
//!
//! All four endpoints are cursor-paginated like search. `max` caps the number
//! of items returned (not pages), and every call stops at the first page that
//! takes it over the cap, so the request count is `ceil(max / 50)` at most.

use serde::Deserialize;

use crate::client::{Client, API_BASE};
use crate::error::Result;
use crate::types::{Track, User};

const PAGE_LIMIT: u32 = 50;

#[derive(Deserialize)]
struct Page<T> {
    #[serde(default = "Vec::new")]
    collection: Vec<T>,
    #[serde(default)]
    next_href:  Option<String>,
}

/// One entry of `/users/{id}/track_likes`. The track is absent when it has
/// since been deleted or made private.
#[derive(Deserialize)]
struct LikeItem {
    #[serde(default)]
    track: Option<Track>,
}

/// One entry of `/stream/users/{id}/reposts`. Playlist reposts have no
/// `track`, and are skipped.
#[derive(Deserialize)]
struct RepostItem {
    #[serde(default)]
    track: Option<Track>,
}

impl Client {
    /// Users who liked `track_id`, most recent first.
    pub async fn track_likers(&self, track_id: u64, max: usize) -> Result<Vec<User>> {
        self.paginate(&format!("/tracks/{track_id}/likers"), max).await
    }

    /// Users who reposted `track_id`, most recent first.
    pub async fn track_reposters(&self, track_id: u64, max: usize) -> Result<Vec<User>> {
        self.paginate(&format!("/tracks/{track_id}/reposters"), max).await
    }

    /// Tracks `user_id` liked, most recent first.
    pub async fn user_track_likes(&self, user_id: u64, max: usize) -> Result<Vec<Track>> {
        let items: Vec<LikeItem> =
            self.paginate(&format!("/users/{user_id}/track_likes"), max).await?;
        Ok(items.into_iter().filter_map(|i| i.track).collect())
    }

    /// Tracks `user_id` reposted, most recent first.
    pub async fn user_track_reposts(&self, user_id: u64, max: usize) -> Result<Vec<Track>> {
        let items: Vec<RepostItem> =
            self.paginate(&format!("/stream/users/{user_id}/reposts"), max).await?;
        Ok(items.into_iter().filter_map(|i| i.track).collect())
    }

    async fn paginate<T: serde::de::DeserializeOwned>(
        &self,
        path: &str,
        max:  usize,
    ) -> Result<Vec<T>> {
        let client_id = self.get_client_id().await?;
        let mut url = format!("{API_BASE}{path}?limit={PAGE_LIMIT}&client_id={client_id}");
        let mut out = Vec::new();
        loop {
            let page: Page<T> = self.get_json(&url).await?;
            let empty = page.collection.is_empty();
            out.extend(page.collection);
            if out.len() >= max || empty {
                break;
            }
            match page.next_href {
                Some(href) => {
                    let sep = if href.contains('?') { '&' } else { '?' };
                    url = format!("{href}{sep}client_id={client_id}");
                }
                None => break,
            }
        }
        out.truncate(max);
        Ok(out)
    }
}
