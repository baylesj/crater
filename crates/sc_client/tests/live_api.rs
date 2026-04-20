//! Integration tests that hit the real SoundCloud v2 API.
//!
//! These are gated behind the `live-tests` feature so they don't run in
//! normal CI. Run locally with:
//!
//!   cargo test -p sc_client --features live-tests -- --nocapture
//!
//! Expect occasional flakes from rate limits or client_id rotation; if
//! `test_extract_client_id` starts failing, SoundCloud has likely shipped
//! a new web bundle structure and `client_id.rs` needs an update.

#![cfg(feature = "live-tests")]

use sc_client::{Client, SearchFilters};

fn init_tracing() {
    let _ = tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "sc_client=debug".into()),
        )
        .with_test_writer()
        .try_init();
}

#[tokio::test]
async fn test_extract_client_id() {
    init_tracing();
    let http = reqwest::Client::new();
    let id = sc_client::extract_client_id(&http)
        .await
        .expect("should scrape client_id from soundcloud.com");

    // 32 chars is the current length but we allow some slack.
    assert!(
        id.len() >= 20 && id.len() <= 64,
        "client_id length {} outside expected range",
        id.len()
    );
    assert!(
        id.chars().all(|c| c.is_ascii_alphanumeric()),
        "client_id should be alphanumeric, got {id}"
    );
}

#[tokio::test]
async fn test_search_returns_results() {
    init_tracing();
    let client = Client::new().unwrap();
    let resp = client
        .search_tracks(&SearchFilters {
            query: Some("techno".into()),
            limit: Some(10),
            ..Default::default()
        })
        .await
        .expect("search should succeed");

    assert!(!resp.collection.is_empty(), "expected at least some results");
    assert!(resp.collection.len() <= 10, "respect limit");

    // Sanity-check the first track has fields we rely on.
    let t = &resp.collection[0];
    assert!(t.id > 0);
    assert!(
        t.permalink_url.is_some(),
        "track should have a permalink_url"
    );
}

#[tokio::test]
async fn test_play_count_ceiling_filters_popular_tracks() {
    init_tracing();
    let client = Client::new().unwrap();

    let filters = SearchFilters {
        query: Some("ambient".into()),
        max_plays: Some(500),
        min_likes: Some(1),
        limit: Some(50),
        ..Default::default()
    };

    let tracks = client
        .search_tracks_filtered(&filters, 10, 5, |_, _| true)
        .await
        .expect("filtered search should succeed");

    // All returned tracks must respect the ceiling.
    for t in &tracks {
        let plays = t
            .playback_count
            .expect("filtered tracks should have known play counts");
        assert!(
            plays <= 500,
            "track {} has {} plays, exceeds ceiling of 500",
            t.id,
            plays
        );
    }

    tracing::info!(
        count = tracks.len(),
        "live search returned {} tracks under ceiling",
        tracks.len()
    );
}
