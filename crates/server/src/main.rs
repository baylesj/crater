//! crater server entry point.
//!
//! Parses config, opens the database, starts the scheduler, binds the HTTP
//! server. On SIGTERM/SIGINT: drains in-flight requests and exits cleanly.

use clap::Parser;
use tower_http::{cors::CorsLayer, trace::TraceLayer};

mod config;
mod error;
mod routes;
mod scheduler;
mod state;

use config::Config;
use state::AppState;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let cfg = Config::parse();

    // Logging
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| cfg.log.parse().unwrap_or_else(|_| "info".parse().unwrap())),
        )
        .init();

    tracing::info!(bind = %cfg.bind, data_dir = %cfg.data_dir.display(), "starting crater");

    // ── Core facade ───────────────────────────────────────────────────────────
    let crater = crater_core::Crater::new(crater_core::Config {
        data_dir:         cfg.data_dir.clone(),
        sc_oauth_token:   cfg.sc_oauth_token.clone(),
        cached_client_id: None,
    })
    .await?;

    let state = AppState::new(crater);

    // ── Scheduler ─────────────────────────────────────────────────────────────
    let _sched = scheduler::start(state.clone()).await?;

    // ── HTTP router ───────────────────────────────────────────────────────────
    let app = routes::router(state)
        .layer(TraceLayer::new_for_http())
        .layer(CorsLayer::permissive()); // LAN-only; tighten for public deploy

    // ── Serve ─────────────────────────────────────────────────────────────────
    let listener = tokio::net::TcpListener::bind(cfg.bind).await?;
    tracing::info!("listening on http://{}", cfg.bind);

    axum::serve(listener, app)
        .with_graceful_shutdown(shutdown_signal())
        .await?;

    tracing::info!("server stopped");
    Ok(())
}

async fn shutdown_signal() {
    use tokio::signal;

    let ctrl_c = async {
        signal::ctrl_c().await.expect("failed to install Ctrl+C handler");
    };

    #[cfg(unix)]
    let terminate = async {
        signal::unix::signal(signal::unix::SignalKind::terminate())
            .expect("failed to install SIGTERM handler")
            .recv()
            .await;
    };

    #[cfg(not(unix))]
    let terminate = std::future::pending::<()>();

    tokio::select! {
        _ = ctrl_c    => tracing::info!("received Ctrl+C"),
        _ = terminate => tracing::info!("received SIGTERM"),
    }
}
