//! Search session API.
//!
//! `POST /api/search` starts a session: spawns a background task that runs
//! the search and broadcasts each result into a channel. The response includes
//! a `session_id` the client uses to open a WS subscription or poll.
//!
//! `GET /api/search/:id` is the non-WS polling fallback.

use axum::{
    extract::{Path, State},
    Json,
};
use uuid::Uuid;

use crater_core::SearchFilters;

use crate::{
    error::{ApiResult, AppError},
    state::{
        create_session, get_snapshot, with_session_tx, SearchEvent, SearchStatus,
        SharedState,
    },
};

// ── Request / response types ──────────────────────────────────────────────────

#[derive(Debug, serde::Deserialize)]
pub struct SearchRequest {
    pub filters:     SearchFilters,
    #[serde(default = "default_target")]
    pub target_size: usize,
    #[serde(default = "default_pages")]
    pub max_pages:   u32,
}

fn default_target() -> usize { 30 }
fn default_pages()  -> u32   { 20 }

#[derive(serde::Serialize)]
pub struct SearchStarted {
    pub session_id: Uuid,
    /// WS endpoint; client subscribes with `{ type: "subscribe", channel: "search", session_id }`.
    pub ws_url: &'static str,
}

// ── Handlers ──────────────────────────────────────────────────────────────────

pub async fn start(
    State(state): State<SharedState>,
    Json(req): Json<SearchRequest>,
) -> ApiResult<Json<SearchStarted>> {
    let session_id = Uuid::new_v4();
    let (_rx, snapshot) = create_session(&state.sessions, session_id);

    // Clone what the background task needs
    let crater  = state.crater.clone();
    let sessions = state.sessions.clone();
    let target  = req.target_size;
    let pages   = req.max_pages;
    let filters = req.filters;

    tokio::spawn(async move {
        let mut session = crater.new_session(filters);

        match session.next_batch(target, pages).await {
            Ok(batch) => {
                let total_accepted = batch.tracks.len();

                // Update snapshot and broadcast each track in order
                for track in batch.tracks {
                    {
                        let mut snap = snapshot.write().unwrap();
                        snap.pages_scanned = batch.pages_scanned;
                        snap.total_scanned = batch.total_scanned;
                        snap.tracks.push(track.clone());
                    }
                    with_session_tx(&sessions, session_id, |tx, _| {
                        let _ = tx.send(SearchEvent::Track {
                            session_id,
                            track,
                            total_scanned: batch.total_scanned,
                            pages_scanned: batch.pages_scanned,
                        });
                    });
                }

                {
                    let mut snap = snapshot.write().unwrap();
                    snap.status    = SearchStatus::Complete;
                    snap.exhausted = batch.exhausted;
                }
                with_session_tx(&sessions, session_id, |tx, _| {
                    let _ = tx.send(SearchEvent::Complete {
                        session_id,
                        exhausted:      batch.exhausted,
                        total_accepted,
                        total_scanned:  batch.total_scanned,
                        pages_scanned:  batch.pages_scanned,
                    });
                });
            }

            Err(e) => {
                let msg = e.to_string();
                {
                    let mut snap = snapshot.write().unwrap();
                    snap.status = SearchStatus::Failed;
                    snap.error  = Some(msg.clone());
                }
                with_session_tx(&sessions, session_id, |tx, _| {
                    let _ = tx.send(SearchEvent::Error {
                        session_id,
                        error:   "search_failed".into(),
                        message: msg,
                    });
                });
            }
        }
    });

    Ok(Json(SearchStarted { session_id, ws_url: "/ws" }))
}

pub async fn poll(
    State(state): State<SharedState>,
    Path(id): Path<Uuid>,
) -> ApiResult<Json<serde_json::Value>> {
    let snap = get_snapshot(&state.sessions, id).ok_or(AppError::NotFound)?;
    Ok(Json(serde_json::to_value(snap).unwrap()))
}
