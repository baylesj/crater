//! SoundCloud playlist write API.
//!
//! Requires a user OAuth token (not just a `client_id`) — the user must have
//! authorized crater via the PKCE flow or pasted a token in Settings.
//!
//! Only creation is implemented; update/delete are deferred until needed.

use serde::{Deserialize, Serialize};

use crate::error::{Result, ScError};

/// Minimal representation of a newly created SoundCloud playlist.
/// Full responses include many more fields; we only need what the caller
/// cares about.
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct CreatedPlaylist {
    pub id: u64,
    #[serde(default)]
    pub permalink_url: Option<String>,
    #[serde(default)]
    pub title: Option<String>,
    #[serde(default)]
    pub sharing: Option<String>,
    #[serde(default)]
    pub track_count: Option<u32>,
}

/// Create a new playlist on SoundCloud.
///
/// `sharing` is `"private"` or `"public"`. `track_ids` are SoundCloud track
/// IDs in desired playback order. `client_id` is included as the query param
/// that SC v2 expects even on authenticated endpoints.
pub async fn create(
    http:        &reqwest::Client,
    oauth_token: &str,
    client_id:   &str,
    title:       &str,
    sharing:     &str,
    track_ids:   &[u64],
) -> Result<CreatedPlaylist> {
    let tracks: Vec<serde_json::Value> = track_ids
        .iter()
        .map(|id| serde_json::json!({"id": id}))
        .collect();

    let body = serde_json::json!({
        "playlist": {
            "title":   title,
            "sharing": sharing,
            "tracks":  tracks,
        }
    });

    let resp = http
        .post(format!("https://api-v2.soundcloud.com/playlists?client_id={client_id}"))
        .header("Authorization", format!("OAuth {oauth_token}"))
        .json(&body)
        .send()
        .await?;

    let status = resp.status();
    match status.as_u16() {
        200..=299 => {
            let text = resp.text().await?;
            serde_json::from_str::<CreatedPlaylist>(&text)
                .map_err(Into::into)
        }
        401 | 403 => Err(ScError::AuthExpired),
        429       => Err(ScError::RateLimited),
        _ => {
            let body = resp.text().await.unwrap_or_default();
            Err(ScError::Unexpected {
                status: status.as_u16(),
                body:   body.chars().take(500).collect(),
            })
        }
    }
}
