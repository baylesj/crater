//! HLS audio stream proxy.
//!
//! SoundCloud's CDN doesn't send permissive CORS headers for LAN origins, and
//! stream URLs require a `client_id` the browser doesn't have. We proxy both
//! the HLS manifest and every segment through the server.
//!
//! Flow:
//!   1. `GET /api/stream/:id`         → resolve CDN manifest URL via sc_client,
//!                                      fetch manifest, rewrite segment URLs to
//!                                      point at `/api/stream/:id/seg?url=…`,
//!                                      serve with HLS content-type.
//!   2. `GET /api/stream/:id/seg`     → proxy individual segment bytes from CDN.
//!
//! The manifest is fetched fresh on every play request; CDN signed-URL TTLs
//! are short (~30 min) so caching would be wrong anyway.

use axum::{
    extract::{Path, Query, State},
    http::{header, StatusCode},
    response::{IntoResponse, Response},
};

use crate::{error::AppError, state::SharedState};

// ── Manifest ──────────────────────────────────────────────────────────────────

pub async fn stream(
    State(state): State<SharedState>,
    Path(id): Path<i64>,
) -> Result<Response, AppError> {
    // Resolve transcoding URL → CDN manifest URL.
    let manifest_url = state.crater.stream_url(id).await?;

    // Fetch the HLS manifest from the CDN.
    let manifest_bytes = state.crater.fetch_bytes(&manifest_url).await?;
    let manifest_text  = String::from_utf8_lossy(&manifest_bytes);

    // Rewrite segment URLs. HLS manifests have two kinds of lines:
    //   - Lines starting with `#`   → metadata, pass through unchanged
    //   - Non-empty other lines     → segment URIs (absolute CDN URLs)
    let proxy_base = format!("/api/stream/{id}/seg?url=");
    let rewritten: String = manifest_text
        .lines()
        .map(|line| {
            if line.starts_with('#') || line.is_empty() {
                line.to_owned()
            } else {
                // Percent-encode the CDN segment URL so it survives as a
                // query parameter.
                format!("{proxy_base}{}", pct_encode(line))
            }
        })
        .collect::<Vec<_>>()
        .join("\n");

    Ok((
        StatusCode::OK,
        [
            (header::CONTENT_TYPE, "application/vnd.apple.mpegurl"),
            // Let the browser (and hls.js) cache manifests briefly.
            (header::CACHE_CONTROL, "public, max-age=10"),
        ],
        rewritten,
    )
        .into_response())
}

// ── Segment proxy ─────────────────────────────────────────────────────────────

#[derive(serde::Deserialize)]
pub struct SegQuery {
    /// Percent-encoded CDN segment URL.
    pub url: String,
}

pub async fn segment(
    State(state): State<SharedState>,
    Path(_id): Path<i64>,
    Query(q): Query<SegQuery>,
) -> Result<Response, AppError> {
    let bytes = state.crater.fetch_bytes(&q.url).await?;
    Ok((
        StatusCode::OK,
        [
            // Segments are either AAC in MPEG-TS or raw Opus in OGG.
            // `application/octet-stream` is a safe fallback the browser accepts.
            (header::CONTENT_TYPE, "application/octet-stream"),
            (header::CACHE_CONTROL, "public, max-age=300"),
        ],
        bytes,
    )
        .into_response())
}

// ── Utilities ─────────────────────────────────────────────────────────────────

/// Percent-encode all bytes that aren't safe in a query-parameter value.
/// The decoder on the receiving end is axum's `Query` extractor, which
/// handles standard percent-decoding automatically.
fn pct_encode(s: &str) -> String {
    let mut out = String::with_capacity(s.len() * 2);
    for b in s.bytes() {
        match b {
            // RFC 3986 unreserved characters — safe to leave as-is.
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9'
            | b'-' | b'_' | b'.' | b'~' => out.push(b as char),
            _ => {
                out.push('%');
                out.push(char::from_digit((b >> 4) as u32, 16).unwrap().to_ascii_uppercase());
                out.push(char::from_digit((b & 0xf) as u32, 16).unwrap().to_ascii_uppercase());
            }
        }
    }
    out
}
