//! Live integration tests — hit real SoundCloud.
//!
//! These require network access and a reachable SoundCloud API.
//! Not run by default; gate them behind the `live-tests` feature so CI
//! doesn't break without network.
//!
//! Run with:
//!   cargo test -p crater --features live-tests --test live -- --nocapture

mod common;
use common::TestServer;

use futures_util::{SinkExt, StreamExt};
use serde_json::{json, Value};
use std::time::Duration;
use tokio_tungstenite::{connect_async, tungstenite::Message};

const TIMEOUT: Duration = Duration::from_secs(60);

// ── Helpers ───────────────────────────────────────────────────────────────────

/// Start a search, subscribe over WS, collect all track events until
/// `search.complete`. Returns the track list and final complete payload.
async fn run_search(
    srv: &TestServer,
    filters: Value,
    target: usize,
    max_pages: u32,
) -> (Vec<Value>, Value) {
    let resp = srv.client
        .post(srv.url("/api/search"))
        .json(&json!({
            "filters":     filters,
            "target_size": target,
            "max_pages":   max_pages,
        }))
        .send().await.unwrap();
    assert_eq!(resp.status(), 200, "search start failed");
    let body: Value = resp.json().await.unwrap();
    let session_id = body["session_id"].as_str().expect("session_id").to_owned();

    let (mut ws, _) = connect_async(srv.ws_url("/ws")).await.expect("WS connect");
    ws.send(Message::Text(serde_json::to_string(&json!({
        "type":       "subscribe",
        "channel":    "search",
        "session_id": session_id,
    })).unwrap().into())).await.unwrap();

    let mut tracks  = Vec::new();
    let mut complete = json!(null);

    tokio::time::timeout(TIMEOUT, async {
        while let Some(Ok(msg)) = ws.next().await {
            let text = msg.into_text().unwrap_or_default();
            if text.is_empty() { continue; }
            let ev: Value = serde_json::from_str(&text).unwrap_or_default();
            match ev["type"].as_str() {
                Some("search.track")    => tracks.push(ev["track"].clone()),
                Some("search.complete") => { complete = ev; break; }
                Some("search.error")    => panic!("search.error: {ev}"),
                _                       => {}
            }
        }
    })
    .await
    .expect("timed out before search.complete");

    (tracks, complete)
}

// ── Tests ─────────────────────────────────────────────────────────────────────

/// SC responds and returns at least one obscure DnB track.
#[tokio::test]
async fn live_search_drum_and_bass() {
    let srv = TestServer::start().await;
    let (tracks, complete) = run_search(
        &srv,
        json!({"genre_or_tag": "drum & bass", "max_plays": 2000, "min_likes": 1}),
        10,
        5,
    ).await;

    println!(
        "live_search_drum_and_bass: {} tracks, {} scanned, {} pages",
        tracks.len(),
        complete["total_scanned"],
        complete["pages_scanned"],
    );
    assert!(
        !tracks.is_empty(),
        "expected >0 tracks — SC may be unreachable or client_id scrape failed"
    );
}

/// Filters that clearly return nothing (absurdly low play cap) produce
/// a well-formed complete event with 0 tracks and >0 pages scanned.
#[tokio::test]
async fn live_search_no_results_complete_event() {
    let srv = TestServer::start().await;
    let (tracks, complete) = run_search(
        &srv,
        // max_plays=1 will pass almost nothing; we scan at least one page.
        json!({"genre_or_tag": "drum & bass", "max_plays": 1}),
        5,
        2,
    ).await;

    println!("live_search_no_results: tracks={}, scanned={}, pages={}",
        tracks.len(), complete["total_scanned"], complete["pages_scanned"]);

    // Whether or not tracks come back, the complete event must be well-formed.
    assert!(complete["pages_scanned"].as_u64().unwrap_or(0) > 0,
        "pages_scanned should be >0 even with no accepted tracks");
}

/// Find a track via search, queue it, confirm it appears in the queued list,
/// then clear it.
#[tokio::test]
async fn live_track_status_roundtrip() {
    let srv = TestServer::start().await;
    let (tracks, _) = run_search(
        &srv,
        json!({"genre_or_tag": "drum & bass", "max_plays": 2000}),
        1,
        3,
    ).await;

    let track_id = tracks
        .first()
        .and_then(|t| t["id"].as_i64())
        .expect("search returned no tracks");
    println!("live_track_status_roundtrip: using track id={track_id}");

    // Queue it
    let resp = srv.client
        .post(srv.url(&format!("/api/tracks/{track_id}/status")))
        .json(&json!({"status": "queued"}))
        .send().await.unwrap();
    assert_eq!(resp.status(), 200, "set status → queued failed");

    // Must appear in the queued list
    let list: Value = srv.client
        .get(srv.url("/api/tracks?status=queued"))
        .send().await.unwrap()
        .json().await.unwrap();
    assert!(
        list.as_array().unwrap().iter().any(|t| t["id"] == track_id),
        "track {track_id} not found in queued list"
    );

    // Clear it
    let resp = srv.client
        .post(srv.url(&format!("/api/tracks/{track_id}/status")))
        .json(&json!({"status": null}))
        .send().await.unwrap();
    assert_eq!(resp.status(), 200, "clear status failed");

    // Must be gone from queued list
    let list: Value = srv.client
        .get(srv.url("/api/tracks?status=queued"))
        .send().await.unwrap()
        .json().await.unwrap();
    assert!(
        !list.as_array().unwrap().iter().any(|t| t["id"] == track_id),
        "track {track_id} still in queued list after clearing"
    );
}
