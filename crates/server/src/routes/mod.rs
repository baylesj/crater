//! Route registration. Called from `main.rs` to assemble the axum Router.

pub mod digests;
pub mod playlists;
pub mod search;
pub mod tracks;
pub mod ui;
pub mod ws;

use axum::{
    routing::{get, post},
    Router,
};

use crate::state::SharedState;

pub fn router(state: SharedState) -> Router {
    Router::new()
        // ── JSON API ──────────────────────────────────────────────────────────
        .route("/api/health",              get(health))
        // Search sessions
        .route("/api/search",              post(search::start))
        .route("/api/search/:id",          get(search::poll))
        // Track status
        .route("/api/tracks",              get(tracks::list))
        .route("/api/tracks/:id",          get(tracks::get))
        .route("/api/tracks/:id/status",   post(tracks::set_status))
        // Digests
        .route("/api/digests",             get(digests::list).post(digests::create))
        .route("/api/digests/:id",         get(digests::get).patch(digests::update).delete(digests::delete_one))
        .route("/api/digests/:id/run",     post(digests::trigger_run))
        .route("/api/digests/:id/runs",    get(digests::list_runs))
        // Playlist export
        .route("/api/playlists/export",    post(playlists::export))
        // Audio stream proxy (not yet implemented)
        .route("/api/stream/:id",          get(stream_stub))
        // ── WebSocket ────────────────────────────────────────────────────────
        .route("/ws",                      get(ws::handler))
        // ── UI ───────────────────────────────────────────────────────────────
        .route("/",                        get(ui::index))
        .route("/dig",                     get(ui::dig))
        .route("/queue",                   get(ui::queue_page))
        .route("/digests",                 get(ui::digests_page))
        .route("/digests/:id",             get(ui::digest_detail_page))
        .route("/history",                 get(ui::history_page))
        .with_state(state)
}

async fn health() -> axum::Json<serde_json::Value> {
    axum::Json(serde_json::json!({
        "status":  "ok",
        "version": env!("CARGO_PKG_VERSION"),
    }))
}

async fn stream_stub() -> impl axum::response::IntoResponse {
    (
        axum::http::StatusCode::NOT_IMPLEMENTED,
        axum::Json(serde_json::json!({
            "error":   "not_implemented",
            "message": "audio stream proxy not yet implemented",
        })),
    )
}
