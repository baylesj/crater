//! Graph crawl API.
//!
//! `POST /api/dig` starts a crawl in the background and returns a session id,
//! exactly like `POST /api/search`, so the dig page reuses the same WebSocket
//! subscription and polling fallback. The difference is the event stream:
//! `dig.progress` while crawling, then every result at once as `search.track`
//! events (with a `dig` field explaining the pick), then `search.complete`.
//! Results only exist once scoring finishes, so there is nothing to stream
//! earlier.

use axum::{extract::State, Json};
use uuid::Uuid;

use crater_core::DigParams;

use crate::{
    error::{ApiResult, AppError},
    routes::search::SearchStarted,
    state::{create_session, with_session_tx, DigMeta, SearchEvent, SearchStatus, SharedState},
};

pub async fn start(
    State(state): State<SharedState>,
    params: Option<Json<DigParams>>,
) -> ApiResult<Json<SearchStarted>> {
    let params = params.map(|Json(p)| p).unwrap_or_default();
    let guard = state
        .dig_lock
        .clone()
        .try_lock_owned()
        .map_err(|_| AppError::Conflict("a dig is already running".into()))?;

    let session_id = Uuid::new_v4();
    let (_rx, snapshot) = create_session(&state.sessions, session_id);
    let crater = state.crater.clone();
    let sessions = state.sessions.clone();

    tokio::spawn(async move {
        let _guard = guard;
        let result = crater
            .dig(&params, |progress| {
                with_session_tx(&sessions, session_id, |tx, _| {
                    let _ = tx.send(SearchEvent::DigProgress { session_id, progress });
                });
            })
            .await;

        match result {
            Ok(outcome) => {
                let n = outcome.results.len();
                let candidates = outcome.candidates as u32;
                for r in outcome.results {
                    snapshot.write().unwrap().tracks.push(r.track.clone());
                    with_session_tx(&sessions, session_id, |tx, _| {
                        let _ = tx.send(SearchEvent::Track {
                            session_id,
                            track:         r.track,
                            total_scanned: candidates,
                            pages_scanned: 0,
                            dig: Some(DigMeta {
                                score:   r.score,
                                support: r.support,
                                via:     r.via,
                                because: r.because,
                            }),
                        });
                    });
                }
                {
                    let mut snap = snapshot.write().unwrap();
                    snap.status        = SearchStatus::Complete;
                    snap.exhausted     = true;
                    snap.total_scanned = candidates;
                }
                with_session_tx(&sessions, session_id, |tx, _| {
                    let _ = tx.send(SearchEvent::Complete {
                        session_id,
                        exhausted:      !outcome.rate_limited,
                        total_accepted: n,
                        total_scanned:  candidates,
                        pages_scanned:  0,
                    });
                });
            }
            Err(e) => {
                let msg = e.to_string();
                tracing::warn!(error = %msg, "dig failed");
                {
                    let mut snap = snapshot.write().unwrap();
                    snap.status = SearchStatus::Failed;
                    snap.error  = Some(msg.clone());
                }
                with_session_tx(&sessions, session_id, |tx, _| {
                    let _ = tx.send(SearchEvent::Error {
                        session_id,
                        error:   "dig_failed".into(),
                        message: msg,
                    });
                });
            }
        }
    });

    Ok(Json(SearchStarted { session_id, ws_url: "/ws" }))
}
