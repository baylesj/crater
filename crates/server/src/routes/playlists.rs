//! Playlist export endpoint.
//!
//! `POST /api/playlists/export` — create a SoundCloud playlist from queued
//! (or explicitly specified) tracks, then mark those tracks as exported.
//!
//! Requires a SoundCloud OAuth token saved in Settings or via CRATER_SC_OAUTH_TOKEN.

use axum::{extract::State, Json};

use crater_core::TrackStatus;

use crate::{
    error::{ApiResult, AppError},
    state::SharedState,
};

#[derive(serde::Deserialize)]
pub struct ExportRequest {
    pub name:       String,
    #[serde(default = "default_visibility")]
    pub visibility: String,
    /// Explicit track IDs to export. Defaults to all queued tracks when absent.
    pub track_ids:  Option<Vec<i64>>,
}

fn default_visibility() -> String { "private".to_owned() }

pub async fn export(
    State(state): State<SharedState>,
    Json(req): Json<ExportRequest>,
) -> ApiResult<Json<serde_json::Value>> {
    // Resolve track IDs: explicit list or all queued
    let track_ids: Vec<i64> = match req.track_ids {
        Some(ids) if !ids.is_empty() => ids,
        Some(_) => return Err(AppError::BadRequest("track_ids list is empty".into())),
        None => {
            let queued = state.crater
                .tracks_with_status(TrackStatus::Queued)
                .await?;
            if queued.is_empty() {
                return Err(AppError::BadRequest("no queued tracks to export".into()));
            }
            queued.into_iter().map(|t| t.id).collect()
        }
    };

    // Create the playlist on SoundCloud (resolves OAuth token automatically)
    let playlist = state.crater
        .create_playlist(&req.name, &req.visibility, &track_ids)
        .await?;

    let playlist_url = playlist.permalink_url.clone();
    let track_count  = track_ids.len();

    // Mark tracks exported after a successful playlist creation
    for id in &track_ids {
        state.crater.set_status(*id, TrackStatus::Exported).await?;
    }

    tracing::info!(
        playlist_url = ?playlist_url,
        track_count,
        "playlist exported"
    );

    Ok(Json(serde_json::json!({
        "playlist_sc_id": playlist.id,
        "playlist_url":   playlist_url,
        "track_count":    track_count,
    })))
}
