//! crater server — library target.
//!
//! Declaring modules here (rather than in main.rs) lets integration tests
//! import the router, state, and config types directly from the `crater_server`
//! crate without needing to compile a full binary or duplicate the setup.

pub mod auth;
pub mod config;
pub mod error;
pub mod routes;
pub mod scheduler;
pub mod state;

use tower_http::{cors::CorsLayer, trace::TraceLayer};

/// Assemble the axum Router with all middleware layers.
///
/// Factored out of main so tests can call it with a test-specific `SharedState`
/// (temp SQLite, no SC credentials, no password) and bind to a random port.
pub fn build_router(state: state::SharedState) -> axum::Router {
    routes::router(state)
        .layer(TraceLayer::new_for_http())
        .layer(CorsLayer::permissive())
}
