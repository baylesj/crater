//! Track status API.
//!
//! GET  /api/tracks?status=queued   list by status
//! GET  /api/tracks/:id             single track
//! POST /api/tracks/:id/status      set or clear status

use axum::{
    extract::{Path, Query, State},
    Json,
};
use crater_core::TrackStatus;
use std::str::FromStr;

use crate::{
    error::{ApiResult, AppError},
    state::SharedState,
};

// ── Request / response types ──────────────────────────────────────────────────

#[derive(Debug, serde::Deserialize)]
pub struct ListQuery {
    pub status: Option<String>,
}

#[derive(Debug, serde::Deserialize)]
pub struct SetStatusRequest {
    /// `null` to clear status.
    pub status: Option<String>,
    pub note:   Option<String>,
}

// ── Handlers ──────────────────────────────────────────────────────────────────

pub async fn list(
    State(state): State<SharedState>,
    Query(q): Query<ListQuery>,
) -> ApiResult<Json<serde_json::Value>> {
    let tracks = match q.status {
        Some(s) => {
            let status = TrackStatus::from_str(&s)
                .map_err(|_| AppError::BadRequest(format!("unknown status '{s}'")))?;
            state.crater.tracks_with_status(status).await?
        }
        None => return Err(AppError::BadRequest("?status= required".into())),
    };
    Ok(Json(serde_json::to_value(tracks).unwrap()))
}

pub async fn get(
    State(state): State<SharedState>,
    Path(id): Path<i64>,
) -> ApiResult<Json<serde_json::Value>> {
    let track = state.crater.get_track(id).await?.ok_or(AppError::NotFound)?;
    Ok(Json(serde_json::to_value(track).unwrap()))
}

pub async fn set_status(
    State(state): State<SharedState>,
    Path(id): Path<i64>,
    Json(req): Json<SetStatusRequest>,
) -> ApiResult<Json<serde_json::Value>> {
    // Guard before touching track_status so FK violations become clean 404s.
    if state.crater.get_track(id).await?.is_none() {
        return Err(AppError::NotFound);
    }
    match req.status {
        None => {
            state.crater.clear_status(id).await?;
        }
        Some(s) => {
            let status = TrackStatus::from_str(&s)
                .map_err(|_| AppError::BadRequest(format!("unknown status '{s}'")))?;
            match req.note {
                Some(note) => state.crater.set_status_with_note(id, status, &note).await?,
                None       => state.crater.set_status(id, status).await?,
            }
        }
    }
    let track = state.crater.get_track(id).await?.ok_or(AppError::NotFound)?;
    Ok(Json(serde_json::to_value(track).unwrap()))
}
