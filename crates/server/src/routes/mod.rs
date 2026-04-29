//! Route registration. Called from `main.rs` to assemble the axum Router.

pub mod digests;
pub mod playlists;
pub mod search;
pub mod settings;
pub mod stream;
pub mod tracks;
pub mod ui;
pub mod ws;

use axum::{
    middleware,
    routing::{get, post},
    Router,
};
use tower_sessions::{MemoryStore, SessionManagerLayer};

use crate::{auth, state::SharedState};

pub fn router(state: SharedState) -> Router {
    // Session store — in-memory, single-user, fine for LAN tool.
    // Sessions survive requests but not server restarts.
    let session_layer = SessionManagerLayer::new(MemoryStore::default());

    Router::new()
        // ── Auth ──────────────────────────────────────────────────────────────
        .route("/login",                       get(auth::login_page).post(auth::login_submit))
        .route("/logout",                      post(auth::logout))
        .route("/auth/soundcloud",             get(auth::sc_authorize))
        .route("/auth/soundcloud/callback",    get(auth::sc_callback))
        // ── JSON API ──────────────────────────────────────────────────────────
        .route("/api/health",                  get(health))
        .route("/api/search",                  post(search::start))
        .route("/api/search/{id}",             get(search::poll))
        .route("/api/tracks",                  get(tracks::list))
        .route("/api/tracks/{id}",             get(tracks::get))
        .route("/api/tracks/{id}/status",      post(tracks::set_status))
        .route("/api/digests",                 get(digests::list).post(digests::create))
        .route("/api/digests/{id}",            get(digests::get).patch(digests::update).delete(digests::delete_one))
        .route("/api/digests/{id}/run",        post(digests::trigger_run))
        .route("/api/digests/{id}/runs",       get(digests::list_runs))
        .route("/api/playlists/export",        post(playlists::export))
        .route("/api/stream/{id}",             get(stream::stream))
        .route("/api/stream/{id}/seg",         get(stream::segment))
        .route("/api/settings/sc-token",       post(settings::save_sc_token))
        .route("/api/settings/sc-token/test",  get(settings::test_sc_token))
        // ── WebSocket ────────────────────────────────────────────────────────
        .route("/ws",                          get(ws::handler))
        // ── UI ───────────────────────────────────────────────────────────────
        .route("/",                            get(ui::index))
        .route("/dig",                         get(ui::dig))
        .route("/queue",                       get(ui::queue_page))
        .route("/digests",                     get(ui::digests_page))
        .route("/digests/{id}",                get(ui::digest_detail_page))
        .route("/hearted",                     get(ui::hearted_page))
        .route("/history",                     get(ui::history_page))
        .route("/settings",                    get(ui::settings_page))
        .route_layer(middleware::from_fn_with_state(state.clone(), auth::require_auth))
        .layer(session_layer)
        .with_state(state)
}

async fn health() -> axum::Json<serde_json::Value> {
    axum::Json(serde_json::json!({
        "status":  "ok",
        "version": env!("CARGO_PKG_VERSION"),
    }))
}
