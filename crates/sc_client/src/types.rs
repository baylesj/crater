//! Domain types for SoundCloud v2 API responses.
//!
//! These mirror the shape of what `api-v2.soundcloud.com` returns, with
//! `#[serde(default)]` on most fields so that schema drift (SoundCloud
//! adding/removing fields) doesn't break deserialization. We only mark
//! `id` as required since it's the primary key and nothing works without it.
//!
//! The raw JSON is preserved separately by the caller (the `core` crate
//! stores the full response in SQLite) so future fields can be extracted
//! without a re-fetch.

use serde::{Deserialize, Serialize};

/// A SoundCloud track. Only `id` is guaranteed; everything else is optional
/// because the v2 API occasionally returns sparse records.
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct Track {
    pub id: u64,

    #[serde(default)]
    pub title: Option<String>,

    /// Full public URL, e.g. `https://soundcloud.com/artist/track-name`.
    #[serde(default)]
    pub permalink_url: Option<String>,

    /// Duration in milliseconds.
    #[serde(default)]
    pub duration: Option<u64>,

    /// BPM as reported by the uploader. Often missing or unreliable — we
    /// still filter on it when present but don't require it.
    #[serde(default)]
    pub bpm: Option<f64>,

    /// Primary genre tag. Note this is a single string, not a list; richer
    /// tagging lives in `tag_list` (space-separated).
    #[serde(default)]
    pub genre: Option<String>,

    /// Space-separated tags, possibly quoted for multi-word tags
    /// (e.g. `drum\ and\ bass liquid neurofunk`). Parse with care.
    #[serde(default)]
    pub tag_list: Option<String>,

    /// Play count. This is the field we filter on for the "undiscovered"
    /// criteria. Missing means we treat it as unknown/skipped.
    #[serde(default)]
    pub playback_count: Option<u64>,

    #[serde(default)]
    pub likes_count: Option<u64>,

    #[serde(default)]
    pub reposts_count: Option<u64>,

    #[serde(default)]
    pub comment_count: Option<u64>,

    /// ISO 8601 timestamp. Kept as string to avoid a chrono dep in this
    /// crate; the `core` crate parses it when writing to SQLite.
    #[serde(default)]
    pub created_at: Option<String>,

    /// Embedded uploader info — v2 inlines this rather than requiring a
    /// second request.
    #[serde(default)]
    pub user: Option<User>,

    /// "allow" | "snip" | "blocked". We filter to "allow" when streaming.
    /// Cover art URL. The API returns the `-large` (100×100) variant by default;
    /// callers can replace the size suffix to request other sizes.
    #[serde(default)]
    pub artwork_url: Option<String>,

    #[serde(default)]
    pub access: Option<String>,

    /// Audio stream info. Present on search results; used to resolve the HLS
    /// manifest URL for playback.
    #[serde(default)]
    pub media: Option<Media>,
}

/// Container for audio transcodings (encoding variants) for a track.
#[derive(Debug, Clone, Default, Deserialize, Serialize)]
pub struct Media {
    #[serde(default)]
    pub transcodings: Vec<Transcoding>,
}

/// A single audio encoding offered by SoundCloud (HLS opus, HLS mp3,
/// progressive mp3, etc.).
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct Transcoding {
    /// API endpoint URL — must be resolved with `client_id` to get the CDN
    /// manifest URL. Not a direct CDN URL.
    #[serde(default)]
    pub url: Option<String>,
    #[serde(default)]
    pub preset: Option<String>,
    #[serde(default)]
    pub duration: Option<u64>,
    /// True when the track is only available as a 30-second snip (geo-blocked
    /// or label-restricted). Snipped transcodings are skipped for playback.
    #[serde(default)]
    pub snipped: bool,
    #[serde(default)]
    pub format: Option<TranscodingFormat>,
    #[serde(default)]
    pub quality: Option<String>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct TranscodingFormat {
    /// "hls" | "progressive"
    #[serde(default)]
    pub protocol: Option<String>,
    /// e.g. `"audio/ogg; codecs=\"opus\""` or `"audio/mpeg"`
    #[serde(default)]
    pub mime_type: Option<String>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct User {
    pub id: u64,
    #[serde(default)]
    pub username: Option<String>,
    #[serde(default)]
    pub permalink_url: Option<String>,
    #[serde(default)]
    pub followers_count: Option<u64>,
    #[serde(default)]
    pub avatar_url: Option<String>,
}

/// The shape of a paginated v2 search response. `next_href` is a
/// fully-qualified URL (with query params including `client_id`) pointing
/// at the next page, or `None` when exhausted.
#[derive(Debug, Clone, Deserialize)]
pub struct SearchResponse {
    pub collection: Vec<Track>,
    #[serde(default)]
    pub next_href: Option<String>,
    #[serde(default)]
    pub total_results: Option<u64>,
}

/// Filters we support at search time. Fields map to v2 query params where
/// possible; play-count filtering is client-side since v2 doesn't expose it.
///
/// The semantics for BPM/duration ranges follow v2's inclusive convention:
/// `bpm_from = 170, bpm_to = 178` matches tracks with `170 <= bpm <= 178`.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct SearchFilters {
    /// Free-text query (title, description, tags). Empty means "browse" —
    /// v2 accepts an empty `q` when filters narrow enough.
    pub query: Option<String>,

    /// Genre-or-tag filter. v2 uses the param name `filter.genre_or_tag`
    /// and matches either the primary genre or any tag.
    pub genre_or_tag: Option<String>,

    pub bpm_from: Option<u32>,
    pub bpm_to: Option<u32>,

    pub duration_from_ms: Option<u64>,
    pub duration_to_ms: Option<u64>,

    /// Hard ceiling on `playback_count`. Applied client-side after
    /// fetching each page. This is the core "undiscovered" filter.
    pub max_plays: Option<u64>,

    /// Optional floor on likes — useful for filtering out pure noise
    /// (bots, unfinished sketches with zero engagement).
    pub min_likes: Option<u64>,

    /// Page size. v2 caps this around 50 in practice.
    pub limit: Option<u32>,
}

/// Pick the best HLS transcoding URL from a track's media field.
///
/// Preference order: HLS opus → HLS mp3. Progressive (non-HLS) and snipped
/// transcodings are skipped. Returns the API endpoint URL (not a CDN URL —
/// it must be resolved via `Client::resolve_stream_url`).
pub fn pick_hls_transcoding(track: &Track) -> Option<&str> {
    let transcodings = track.media.as_ref()?.transcodings.as_slice();
    // Two passes: prefer opus first, fall back to mp3.
    for prefer_opus in [true, false] {
        for t in transcodings {
            if t.snipped { continue; }
            let protocol = t.format.as_ref().and_then(|f| f.protocol.as_deref());
            if protocol != Some("hls") { continue; }
            let is_opus = t.format.as_ref()
                .and_then(|f| f.mime_type.as_deref())
                .map(|m| m.contains("ogg") || m.contains("opus"))
                .unwrap_or(false);
            if prefer_opus == is_opus {
                if let Some(url) = t.url.as_deref() {
                    return Some(url);
                }
            }
        }
    }
    None
}

impl Track {
    /// Returns true iff this track passes the client-side portion of the
    /// filter (play count ceiling, likes floor). Server-side filters
    /// (genre, BPM, duration) are applied by the query itself.
    pub fn passes_client_filter(&self, filters: &SearchFilters) -> bool {
        if let Some(max) = filters.max_plays {
            match self.playback_count {
                Some(plays) if plays > max => return false,
                None => return false, // unknown play count — exclude, safer
                _ => {}
            }
        }
        if let Some(min) = filters.min_likes {
            if self.likes_count.unwrap_or(0) < min {
                return false;
            }
        }
        // Duration is a server-side param but we enforce it client-side too
        // since the API occasionally returns tracks outside the range.
        if let Some(max_ms) = filters.duration_to_ms {
            if self.duration.unwrap_or(u64::MAX) > max_ms {
                return false;
            }
        }
        if let Some(min_ms) = filters.duration_from_ms {
            if self.duration.unwrap_or(0) < min_ms {
                return false;
            }
        }
        true
    }

    /// Like/play ratio — a rough "hidden gem" signal. High ratio with
    /// low plays suggests listeners who found it liked it.
    /// Returns None if we can't compute it.
    pub fn engagement_ratio(&self) -> Option<f64> {
        match (self.likes_count, self.playback_count) {
            (Some(likes), Some(plays)) if plays > 0 => Some(likes as f64 / plays as f64),
            _ => None,
        }
    }
}
