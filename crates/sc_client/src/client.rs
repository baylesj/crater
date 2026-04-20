//! High-level SoundCloud v2 client.
//!
//! Wraps a `reqwest::Client` with:
//!   - automatic `client_id` scraping + caching + rotation on 401
//!   - typed request builders for the v2 search endpoint
//!   - pagination helper that applies the client-side play-count filter
//!
//! Playlist CRUD (which requires a user OAuth token, not just a client_id)
//! lives in its own module and is not wired up in this first cut — we want
//! read-only search working end-to-end before we need the OAuth dance.

use std::sync::Arc;
use tokio::sync::RwLock;

use crate::client_id::extract_client_id;
use crate::error::{Result, ScError};
use crate::types::{SearchFilters, SearchResponse, Track};

const API_BASE: &str = "https://api-v2.soundcloud.com";

/// Default page size for v2 search. The server caps at ~50 regardless of
/// what we ask for, so we use 50 to minimize round trips.
const DEFAULT_LIMIT: u32 = 50;

/// A SoundCloud v2 API client with automatic `client_id` management.
///
/// Cheap to clone — the underlying `reqwest::Client` is already an `Arc`
/// and the `client_id` cache is behind an `Arc<RwLock<_>>`.
#[derive(Clone)]
pub struct Client {
    http: reqwest::Client,
    /// Cached client_id, refreshed on 401. `None` until first use.
    client_id: Arc<RwLock<Option<String>>>,
}

impl Client {
    /// Build a new client. The `client_id` is lazily scraped on first
    /// request rather than eagerly at construction — this lets callers
    /// construct the client offline (e.g. at server boot) without a
    /// network round trip.
    pub fn new() -> Result<Self> {
        let http = reqwest::Client::builder()
            .user_agent(
                "Mozilla/5.0 (Macintosh; Intel Mac OS X 10_15_7) \
                 AppleWebKit/537.36 (KHTML, like Gecko) Chrome/131.0.0.0 Safari/537.36",
            )
            .timeout(std::time::Duration::from_secs(30))
            .build()?;

        Ok(Self {
            http,
            client_id: Arc::new(RwLock::new(None)),
        })
    }

    /// Build a client with a pre-known `client_id` (e.g. loaded from disk
    /// cache). Skips the initial scrape on the first request.
    pub fn with_client_id(client_id: String) -> Result<Self> {
        let me = Self::new()?;
        // Blocking on a held lock is fine — we just constructed the Arc
        // and nothing else can have a reference.
        *me.client_id.try_write().expect("fresh Arc") = Some(client_id);
        Ok(me)
    }

    /// Returns the current cached client_id, scraping if necessary.
    async fn get_client_id(&self) -> Result<String> {
        // Fast path: already cached.
        if let Some(id) = self.client_id.read().await.as_ref() {
            return Ok(id.clone());
        }
        // Slow path: scrape. We take the write lock for the whole duration
        // so concurrent callers don't trigger duplicate scrapes.
        let mut guard = self.client_id.write().await;
        if let Some(id) = guard.as_ref() {
            return Ok(id.clone()); // someone else won the race
        }
        let fresh = extract_client_id(&self.http).await?;
        *guard = Some(fresh.clone());
        Ok(fresh)
    }

    /// Force a refresh of the cached client_id. Called automatically when
    /// a request returns 401, but exposed for manual use.
    pub async fn refresh_client_id(&self) -> Result<String> {
        let fresh = extract_client_id(&self.http).await?;
        *self.client_id.write().await = Some(fresh.clone());
        Ok(fresh)
    }

    /// Returns the currently cached client_id without scraping. Useful for
    /// persisting across restarts (e.g. write to disk on shutdown).
    pub async fn current_client_id(&self) -> Option<String> {
        self.client_id.read().await.clone()
    }

    /// Search for tracks matching the given filters. Returns one page of
    /// results, already filtered client-side for play-count ceiling /
    /// likes floor. Use `search_tracks_filtered` for the pagination loop
    /// that accumulates N tracks under the ceiling.
    pub async fn search_tracks(&self, filters: &SearchFilters) -> Result<SearchResponse> {
        let client_id = self.get_client_id().await?;
        let url = build_search_url(&client_id, filters);
        self.get_json::<SearchResponse>(&url).await
    }

    /// Keep paginating until we've accumulated `target` tracks that pass
    /// the client-side filter, or we hit `max_pages` pages (safety cap
    /// against fruitless searches — SoundCloud has tracks where 99% are
    /// too popular, so we need to bail eventually).
    ///
    /// Yields pages via the provided callback so callers (TUI, web UI)
    /// can stream results as they arrive rather than blocking on a full
    /// scan. The callback receives the filtered tracks from this page
    /// and the cumulative count; returning `false` halts pagination.
    pub async fn search_tracks_filtered<F>(
        &self,
        filters: &SearchFilters,
        target: usize,
        max_pages: usize,
        mut on_page: F,
    ) -> Result<Vec<Track>>
    where
        F: FnMut(&[Track], usize) -> bool,
    {
        let mut accumulated: Vec<Track> = Vec::with_capacity(target);
        let mut next_url: Option<String> = None;
        let client_id = self.get_client_id().await?;

        for page_idx in 0..max_pages {
            let url = match &next_url {
                Some(u) => u.clone(),
                None => build_search_url(&client_id, filters),
            };

            tracing::debug!(page = page_idx, url = %url, "fetching search page");
            let resp: SearchResponse = self.get_json(&url).await?;

            let before_len = accumulated.len();
            for track in resp.collection {
                if track.passes_client_filter(filters) {
                    accumulated.push(track);
                }
            }
            let new_tracks = &accumulated[before_len..];
            let keep_going = on_page(new_tracks, accumulated.len());

            if !keep_going || accumulated.len() >= target {
                break;
            }

            match resp.next_href {
                Some(href) => {
                    // SoundCloud's next_href omits client_id; inject it so the
                    // next page request doesn't need a round-trip retry.
                    let sep = if href.contains('?') { '&' } else { '?' };
                    next_url = Some(format!("{href}{sep}client_id={client_id}"));
                }
                None => {
                    tracing::info!("exhausted search results");
                    break;
                }
            }
        }

        accumulated.truncate(target);
        Ok(accumulated)
    }

    /// Perform a GET and deserialize as JSON, with automatic retry once on
    /// 401 (refresh client_id and try again). Other errors bubble up.
    async fn get_json<T: serde::de::DeserializeOwned>(&self, url: &str) -> Result<T> {
        match self.get_json_once(url).await {
            Err(ScError::AuthExpired) => {
                tracing::warn!("client_id rejected, refreshing and retrying");
                let fresh = self.refresh_client_id().await?;
                // Rewrite the URL with the new client_id. This is the
                // simplest approach — we could also store the current
                // client_id and interpolate, but a regex replace keeps
                // the `next_href` pagination path working too.
                let rewritten = replace_client_id_param(url, &fresh);
                self.get_json_once(&rewritten).await
            }
            other => other,
        }
    }

    async fn get_json_once<T: serde::de::DeserializeOwned>(&self, url: &str) -> Result<T> {
        let resp = self.http.get(url).send().await?;
        let status = resp.status();

        match status.as_u16() {
            200..=299 => {
                let text = resp.text().await?;
                let parsed = serde_json::from_str::<T>(&text)?;
                Ok(parsed)
            }
            401 | 403 => Err(ScError::AuthExpired),
            404 => Err(ScError::NotFound),
            429 => Err(ScError::RateLimited),
            _ => {
                let body = resp.text().await.unwrap_or_default();
                Err(ScError::Unexpected {
                    status: status.as_u16(),
                    body: body.chars().take(500).collect(),
                })
            }
        }
    }
}

fn build_search_url(client_id: &str, f: &SearchFilters) -> String {
    // Use a simple query builder rather than pulling in a dep.
    let mut params: Vec<(&str, String)> = Vec::new();

    if let Some(q) = &f.query {
        params.push(("q", q.clone()));
    }
    if let Some(g) = &f.genre_or_tag {
        params.push(("filter.genre_or_tag", g.clone()));
    }
    if let Some(bpm) = f.bpm_from {
        params.push(("filter.bpm[from]", bpm.to_string()));
    }
    if let Some(bpm) = f.bpm_to {
        params.push(("filter.bpm[to]", bpm.to_string()));
    }
    if let Some(d) = f.duration_from_ms {
        params.push(("filter.duration[from]", d.to_string()));
    }
    if let Some(d) = f.duration_to_ms {
        params.push(("filter.duration[to]", d.to_string()));
    }
    params.push(("limit", f.limit.unwrap_or(DEFAULT_LIMIT).to_string()));
    params.push(("client_id", client_id.to_string()));

    let qs = params
        .iter()
        .map(|(k, v)| format!("{}={}", k, urlencode(v)))
        .collect::<Vec<_>>()
        .join("&");

    format!("{API_BASE}/search/tracks?{qs}")
}

/// Minimal URL percent-encoder for query values — enough to handle spaces
/// and genre strings like "drum & bass". Pulling in `url` crate would be
/// overkill for this one call site.
fn urlencode(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    for b in s.bytes() {
        match b {
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'_' | b'.' | b'~' => {
                out.push(b as char)
            }
            _ => out.push_str(&format!("%{:02X}", b)),
        }
    }
    out
}

/// Rewrites the `client_id=XXX` query parameter in a URL with a fresh value,
/// or appends it if absent. Used when retrying after a 401.
fn replace_client_id_param(url: &str, new_id: &str) -> String {
    use once_cell::sync::Lazy;
    use regex::Regex;
    static RE: Lazy<Regex> = Lazy::new(|| Regex::new(r"client_id=[^&]+").unwrap());
    if RE.is_match(url) {
        RE.replace(url, format!("client_id={new_id}").as_str())
            .into_owned()
    } else {
        let sep = if url.contains('?') { '&' } else { '?' };
        format!("{url}{sep}client_id={new_id}")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn urlencode_handles_spaces_and_ampersand() {
        assert_eq!(urlencode("drum & bass"), "drum%20%26%20bass");
        assert_eq!(urlencode("simple"), "simple");
    }

    #[test]
    fn search_url_includes_all_filters() {
        let f = SearchFilters {
            query: Some("liquid".into()),
            genre_or_tag: Some("drum & bass".into()),
            bpm_from: Some(170),
            bpm_to: Some(178),
            max_plays: Some(1000), // client-side, should NOT appear in URL
            ..Default::default()
        };
        let url = build_search_url("test_cid", &f);
        assert!(url.contains("q=liquid"));
        assert!(url.contains("filter.genre_or_tag=drum%20%26%20bass"));
        assert!(url.contains("filter.bpm[from]=170"));
        assert!(url.contains("filter.bpm[to]=178"));
        assert!(url.contains("client_id=test_cid"));
        // max_plays is client-side only:
        assert!(!url.contains("max_plays"));
        assert!(!url.contains("playback_count"));
    }

    #[test]
    fn replace_client_id_rewrites_correctly() {
        let url = "https://api-v2.soundcloud.com/search/tracks?q=x&client_id=OLD&limit=50";
        let new_url = replace_client_id_param(url, "NEW");
        assert_eq!(
            new_url,
            "https://api-v2.soundcloud.com/search/tracks?q=x&client_id=NEW&limit=50"
        );
    }
}
