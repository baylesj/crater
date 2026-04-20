//! Scrapes the public `client_id` token from SoundCloud's web bundles.
//!
//! SoundCloud's web client embeds a 32-character `client_id` in one of its
//! JS bundles. We fetch `soundcloud.com`, find the asset URLs matching
//! `https://a-v2.sndcdn.com/assets/*.js`, then grep each bundle for the
//! `client_id:"XXXX"` pattern. The working id is typically in one of the
//! last few bundles (confirmed by multiple public scrapers as of 2026).
//!
//! This token rotates every few weeks, so callers should catch
//! `ScError::AuthExpired` on any request and refresh by calling
//! `extract_client_id()` again. The `Client` wrapper does this automatically.

use once_cell::sync::Lazy;
use regex::Regex;

use crate::error::{Result, ScError};

const SOUNDCLOUD_HOMEPAGE: &str = "https://soundcloud.com";

/// Matches `https://a-v2.sndcdn.com/assets/<hash>-<hash>.js` — the JS bundle
/// URLs embedded in SoundCloud's homepage HTML.
static ASSET_URL_RE: Lazy<Regex> =
    Lazy::new(|| Regex::new(r#"https://a-v2\.sndcdn\.com/assets/[a-zA-Z0-9._-]+\.js"#).unwrap());

/// Matches `client_id:"XXXXXXXX..."` — the token as emitted by the minified
/// JS. Current token length is 32 chars, but we allow 20-64 for resilience.
static CLIENT_ID_RE: Lazy<Regex> =
    Lazy::new(|| Regex::new(r#"client_id\s*:\s*"([a-zA-Z0-9]{20,64})""#).unwrap());

/// Fetches the SoundCloud homepage, finds the bundled JS files, and searches
/// them for an embedded `client_id`. Tries the last ~5 bundles first since
/// empirically the id lives in a late-loaded bundle.
pub async fn extract_client_id(http: &reqwest::Client) -> Result<String> {
    let homepage = http
        .get(SOUNDCLOUD_HOMEPAGE)
        .header(
            "User-Agent",
            "Mozilla/5.0 (Macintosh; Intel Mac OS X 10_15_7) \
             AppleWebKit/537.36 (KHTML, like Gecko) Chrome/131.0.0.0 Safari/537.36",
        )
        .send()
        .await?
        .error_for_status()?
        .text()
        .await?;

    let mut asset_urls: Vec<&str> = ASSET_URL_RE
        .find_iter(&homepage)
        .map(|m| m.as_str())
        .collect();

    // Deduplicate while preserving order (a URL might appear multiple times).
    let mut seen = std::collections::HashSet::new();
    asset_urls.retain(|url| seen.insert(*url));

    if asset_urls.is_empty() {
        return Err(ScError::ClientIdExtractionFailed(
            "no a-v2.sndcdn.com/assets/*.js URLs found in homepage HTML".into(),
        ));
    }

    tracing::debug!(count = asset_urls.len(), "found JS bundle URLs");

    // Check the last ~5 bundles first — the client_id usually lives in a
    // late-loaded bundle. Reverse iteration is the standard heuristic in
    // public scrapers.
    for url in asset_urls.iter().rev().take(8) {
        tracing::trace!(url = %url, "scanning JS bundle for client_id");
        match http.get(*url).send().await {
            Ok(resp) => {
                if !resp.status().is_success() {
                    continue;
                }
                let body = match resp.text().await {
                    Ok(b) => b,
                    Err(e) => {
                        tracing::warn!(error = %e, "failed to read bundle body, skipping");
                        continue;
                    }
                };
                if let Some(caps) = CLIENT_ID_RE.captures(&body) {
                    let id = caps[1].to_string();
                    tracing::info!(
                        client_id_prefix = &id[..6.min(id.len())],
                        "extracted client_id"
                    );
                    return Ok(id);
                }
            }
            Err(e) => {
                tracing::warn!(error = %e, url = %url, "bundle fetch failed, continuing");
            }
        }
    }

    Err(ScError::ClientIdExtractionFailed(format!(
        "scanned {} bundles, no client_id pattern matched",
        asset_urls.len().min(8)
    )))
}
