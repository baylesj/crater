//! Shared helpers for crater server integration tests.

use std::net::SocketAddr;

use crater_server::{build_router, config::Config, state::AppState};

/// A crater server bound to a random port with a fresh in-memory SQLite DB
/// and no SoundCloud credentials. Dropped when the test ends; the tokio
/// runtime cancels the background task automatically.
pub struct TestServer {
    pub addr:   SocketAddr,
    pub client: reqwest::Client,
    _tempdir:   tempfile::TempDir,
    _task:      tokio::task::JoinHandle<()>,
}

impl TestServer {
    pub async fn start() -> Self {
        let tempdir = tempfile::TempDir::new().expect("tempdir");

        let crater = crater_core::Crater::new(crater_core::Config {
            data_dir:         tempdir.path().to_path_buf(),
            sc_oauth_cfg:     None,
            sc_oauth_token:   None,
            cached_client_id: None,
        })
        .await
        .expect("crater init");

        let config = Config {
            bind:             "127.0.0.1:0".parse().unwrap(),
            data_dir:         tempdir.path().to_path_buf(),
            music_dir:        None,
            sc_client_id:     None,
            sc_client_secret: None,
            sc_redirect_uri:  None,
            sc_oauth_token:   None,
            password:         None,
            ntfy_url:         None,
            ntfy_topic:       None,
            timezone:         "America/Los_Angeles".to_owned(),
            log:              "info".to_owned(),
        };

        let state = AppState::new(crater, config);
        let router = build_router(state);

        let listener = tokio::net::TcpListener::bind("127.0.0.1:0")
            .await
            .expect("bind");
        let addr = listener.local_addr().unwrap();

        let task = tokio::spawn(async move {
            axum::serve(listener, router).await.unwrap();
        });

        // reqwest follows redirects by default — fine for UI page checks.
        let client = reqwest::Client::builder()
            .redirect(reqwest::redirect::Policy::limited(5))
            .build()
            .unwrap();

        TestServer { addr, client, _tempdir: tempdir, _task: task }
    }

    pub fn url(&self, path: &str) -> String {
        format!("http://{}{}", self.addr, path)
    }

    pub fn ws_url(&self, path: &str) -> String {
        format!("ws://{}{}", self.addr, path)
    }
}
