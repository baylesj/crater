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
use crate::oauth::{client_credentials, CachedToken, OAuthConfig};
use crate::types::{SearchFilters, SearchResponse, Track};

const API_BASE: &str = "https://api-v2.soundcloud.com";

/// Response from a transcoding resolution request: `{"url": "<cdn_url>"}`.
#[derive(serde::Deserialize)]
struct StreamUrlResponse {
    url: String,
}

/// Default page size for v2 search. The server caps at ~50 regardless of
/// what we ask for, so we use 50 to minimize round trips.
const DEFAULT_LIMIT: u32 = 50;

/// A SoundCloud v2 API client with automatic credential management.
///
/// Supports two auth modes:
/// - **Official**: uses `client_id` + `client_secret` for client credentials
///   tokens. Tokens auto-refresh when expired. No scraping required.
/// - **Scrape fallback**: extracts `client_id` from SoundCloud's homepage JS
///   bundles. Used when official credentials aren't configured.
///
/// Cheap to clone — the underlying `reqwest::Client` and all caches are
/// behind `Arc`.
#[derive(Clone)]
pub struct Client {
    http:      reqwest::Client,
    /// Official API credentials, if configured.
    oauth_cfg: Option<Arc<OAuthConfig>>,
    /// Client credentials token (official flow). `None` until first use or
    /// when using scrape fallback.
    cc_token:  Arc<RwLock<Option<CachedToken>>>,
    /// Scraped client_id (fallback). `None` until first use when no official
    /// credentials are configured.
    client_id: Arc<RwLock<Option<String>>>,
}

impl Client {
    fn build_http() -> Result<reqwest::Client> {
        reqwest::Client::builder()
            .user_agent(
                "Mozilla/5.0 (Macintosh; Intel Mac OS X 10_15_7) \
                 AppleWebKit/537.36 (KHTML, like Gecko) Chrome/131.0.0.0 Safari/537.36",
            )
            .timeout(std::time::Duration::from_secs(30))
            .build()
            .map_err(Into::into)
    }

    /// Build a client using the official API credentials (preferred).
    /// Tokens are obtained via client credentials flow, lazily on first use.
    pub fn with_oauth(oauth_cfg: OAuthConfig) -> Result<Self> {
        Ok(Self {
            http:      Self::build_http()?,
            oauth_cfg: Some(Arc::new(oauth_cfg)),
            cc_token:  Arc::new(RwLock::new(None)),
            client_id: Arc::new(RwLock::new(None)),
        })
    }

    /// Build a client using the scrape fallback (no official credentials).
    /// The `client_id` is lazily scraped from SoundCloud's homepage JS.
    pub fn new() -> Result<Self> {
        Ok(Self {
            http:      Self::build_http()?,
            oauth_cfg: None,
            cc_token:  Arc::new(RwLock::new(None)),
            client_id: Arc::new(RwLock::new(None)),
        })
    }

    /// Build a scrape-fallback client with a pre-known `client_id`.
    pub fn with_client_id(client_id: String) -> Result<Self> {
        let me = Self::new()?;
        *me.client_id.try_write().expect("fresh Arc") = Some(client_id);
        Ok(me)
    }

    /// Returns a valid `client_id` (official mode) or scraped id (fallback),
    /// refreshing/scraping as needed. This is what all API calls use for the
    /// `client_id=` query parameter.
    async fn get_client_id(&self) -> Result<String> {
        if let Some(cfg) = &self.oauth_cfg {
            // Official mode: use/refresh the client credentials token.
            return self.get_cc_token(cfg).await;
        }
        // Scrape fallback.
        if let Some(id) = self.client_id.read().await.as_ref() {
            return Ok(id.clone());
        }
        let mut guard = self.client_id.write().await;
        if let Some(id) = guard.as_ref() {
            return Ok(id.clone());
        }
        let fresh = extract_client_id(&self.http).await?;
        *guard = Some(fresh.clone());
        Ok(fresh)
    }

    /// Get a valid client credentials access token, fetching or refreshing
    /// as needed.
    async fn get_cc_token(&self, cfg: &OAuthConfig) -> Result<String> {
        // Fast path: valid cached token.
        if let Some(t) = self.cc_token.read().await.as_ref() {
            if !t.is_expired() {
                return Ok(t.access_token.clone());
            }
        }
        // Slow path: fetch new token. Hold write lock to avoid thundering herd.
        let mut guard = self.cc_token.write().await;
        if let Some(t) = guard.as_ref() {
            if !t.is_expired() {
                return Ok(t.access_token.clone());
            }
        }
        tracing::info!("fetching client credentials token from SoundCloud");
        let resp  = client_credentials(&self.http, cfg).await?;
        let token = CachedToken::from_response(resp);
        let id    = token.access_token.clone();
        *guard    = Some(token);
        Ok(id)
    }

    /// Force a refresh. In official mode refreshes the CC token; in scrape
    /// mode re-scrapes the homepage. Called automatically on 401.
    pub async fn refresh_client_id(&self) -> Result<String> {
        if let Some(cfg) = &self.oauth_cfg {
            *self.cc_token.write().await = None; // force re-fetch
            return self.get_cc_token(cfg).await;
        }
        let fresh = extract_client_id(&self.http).await?;
        *self.client_id.write().await = Some(fresh.clone());
        Ok(fresh)
    }

    /// Returns the currently cached client_id/token without making requests.
    pub async fn current_client_id(&self) -> Option<String> {
        if self.oauth_cfg.is_some() {
            return self.cc_token.read().await.as_ref().map(|t| t.access_token.clone());
        }
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

    /// Resolve a SoundCloud transcoding API URL to the actual CDN manifest URL.
    ///
    /// The transcoding URL (from `Track.media.transcodings[].url`) is an API
    /// endpoint that requires `client_id`. It returns `{"url": "<cdn_url>"}`.
    /// The CDN URL is a signed HLS manifest URL on `cf-hls-media.sndcdn.com`.
    pub async fn resolve_stream_url(&self, transcoding_api_url: &str) -> Result<String> {
        let client_id  = self.get_client_id().await?;
        let url        = format!("{transcoding_api_url}?client_id={client_id}");
        let response   = self.get_json::<StreamUrlResponse>(&url).await?;
        Ok(response.url)
    }

    /// Fetch a single track by its SoundCloud ID.
    ///
    /// Used when the track's `raw_json` was stored before the `media` field
    /// was added to the schema and therefore lacks transcoding info.
    pub async fn fetch_track(&self, track_id: u64) -> Result<Track> {
        let client_id = self.get_client_id().await?;
        let url       = format!("{API_BASE}/tracks/{track_id}?client_id={client_id}");
        self.get_json::<Track>(&url).await
    }

    /// Create a SoundCloud playlist owned by the authenticated user.
    ///
    /// `sharing` is `"private"` or `"public"`. Track IDs are in desired
    /// playback order. Requires a user OAuth token; the `client_id` is
    /// resolved automatically (same as all other API calls).
    pub async fn create_playlist(
        &self,
        oauth_token: &str,
        title:       &str,
        sharing:     &str,
        track_ids:   &[u64],
    ) -> Result<crate::playlist::CreatedPlaylist> {
        let client_id = self.get_client_id().await?;
        crate::playlist::create(&self.http, oauth_token, &client_id, title, sharing, track_ids).await
    }

    /// Fetch raw bytes from a URL without authentication.
    ///
    /// Used by the server to proxy HLS segment bytes from the CDN. CDN URLs
    /// are pre-signed and do not require `client_id`.
    pub async fn fetch_bytes(&self, url: &str) -> Result<Vec<u8>> {
        let resp = self.http.get(url).send().await?;
        if !resp.status().is_success() {
            let status = resp.status().as_u16();
            let body   = resp.text().await.unwrap_or_default();
            return Err(ScError::Unexpected {
                status,
                body: body.chars().take(200).collect(),
            });
        }
        Ok(resp.bytes().await?.to_vec())
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

    // `q` is required by the v2 API even when filtering by genre/BPM only.
    // Use "*" as a wildcard when the caller didn't specify a text query.
    params.push(("q", f.query.clone().unwrap_or_else(|| "*".to_owned())));
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
    fn search_url_uses_wildcard_when_no_query() {
        let f = SearchFilters {
            genre_or_tag: Some("jungle".into()),
            ..Default::default()
        };
        let url = build_search_url("cid", &f);
        // Must always include q= so SC doesn't return 400.
        assert!(url.contains("q=%2A") || url.contains("q=*"), "q param missing: {url}");
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
