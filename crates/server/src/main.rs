//! crater server entry point.
//!
//! Parses config, opens the database, starts the scheduler, binds the HTTP
//! server. On SIGTERM/SIGINT: drains in-flight requests and exits cleanly.

use clap::Parser;
use crater_server::{build_router, config::Config, scheduler, state::AppState};
use sc_client::OAuthConfig;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let cfg = Config::parse();

    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| cfg.log.parse().unwrap_or_else(|_| "info".parse().unwrap())),
        )
        .init();

    tracing::info!(bind = %cfg.bind, data_dir = %cfg.data_dir.display(), "starting crater");

    if cfg.sc_client_id.is_some() {
        tracing::info!("using official SoundCloud API credentials");
    } else {
        tracing::warn!("CRATER_SC_CLIENT_ID not set — falling back to client_id scrape");
    }

    if cfg.password.is_some() {
        tracing::info!("crater password auth enabled");
    }

    let sc_oauth_cfg = match (&cfg.sc_client_id, &cfg.sc_client_secret, &cfg.sc_redirect_uri) {
        (Some(id), Some(secret), Some(redirect)) => Some(OAuthConfig {
            client_id:     id.clone(),
            client_secret: secret.clone(),
            redirect_uri:  redirect.clone(),
        }),
        _ => None,
    };

    let crater = crater_core::Crater::new(crater_core::Config {
        data_dir:         cfg.data_dir.clone(),
        sc_oauth_cfg,
        sc_oauth_token:   cfg.sc_oauth_token.clone(),
        cached_client_id: None,
    })
    .await?;

    let state = AppState::new(crater, cfg.clone());

    let _sched = scheduler::start(state.clone()).await?;

    let app = build_router(state);

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
