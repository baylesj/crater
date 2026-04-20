//! Playlist export endpoint.
//!
//! `POST /api/playlists/export` requires an OAuth token. Returns a clear error
//! when the token is absent so the UI can show a useful message rather than a
//! generic 500.

use axum::{extract::State, Json};

use crate::{
    error::{ApiResult, AppError},
    state::SharedState,
};

#[derive(serde::Deserialize)]
pub struct ExportRequest {
    pub name:       String,
    #[serde(default = "default_visibility")]
    pub visibility: String,
    /// Explicit track IDs to export. Defaults to all queued tracks.
    pub track_ids:  Option<Vec<i64>>,
}

fn default_visibility() -> String { "private".to_owned() }

pub async fn export(
    State(state): State<SharedState>,
    Json(req): Json<ExportRequest>,
) -> ApiResult<Json<serde_json::Value>> {
    // Fail early with a clear message — OAuth capture is a manual step
    // (see docs/04-oauth-capture.md). The endpoint is wired so the UI gets
    // a proper error instead of 404.
    if state.crater.oauth_token.is_none() {
        return Err(AppError::BadRequest(
            "SoundCloud OAuth token not configured. \
             Follow docs/04-oauth-capture.md to capture your token, \
             then set CRATER_SC_OAUTH_TOKEN.".into(),
        ));
    }

    // Resolve track IDs: explicit list or all queued tracks
    let track_ids = match req.track_ids {
        Some(ids) => ids,
        None => {
            state.crater
                .tracks_with_status(crater_core::TrackStatus::Queued)
                .await?
                .into_iter()
                .map(|t| t.id)
                .collect()
        }
    };

    if track_ids.is_empty() {
        return Err(AppError::BadRequest("no tracks to export".into()));
    }

    // Playlist creation not yet implemented — requires sc_client::playlist module.
    // This stub is here so the UI wires up correctly; the OAuth check above
    // gives actionable feedback in the meantime.
    Err(AppError::BadRequest(
        "playlist export requires OAuth token and sc_client playlist support (coming soon)".into()
    ))
}
