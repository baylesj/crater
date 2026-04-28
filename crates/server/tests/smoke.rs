//! Smoke tests — spin up a real crater server on a random port and exercise
//! every endpoint category.
//!
//! These run with no SoundCloud credentials; they verify the HTTP/WS layer is
//! wired correctly and the DB schema is healthy. SC-dependent behaviour lives
//! in tests/live.rs.
//!
//! Run with:
//!   cargo test -p crater --test smoke

mod common;
use common::TestServer;

use serde_json::{json, Value};

// ── Health ────────────────────────────────────────────────────────────────────

#[tokio::test]
async fn health_check() {
    let srv = TestServer::start().await;
    let resp = srv.client.get(srv.url("/api/health")).send().await.unwrap();
    assert_eq!(resp.status(), 200);
    let body: Value = resp.json().await.unwrap();
    assert_eq!(body["status"], "ok");
}

// ── UI pages ──────────────────────────────────────────────────────────────────

#[tokio::test]
async fn ui_pages_return_200() {
    let srv = TestServer::start().await;
    // "/" redirects to "/dig"; reqwest follows it.
    for path in ["/", "/dig", "/queue", "/digests", "/history", "/settings"] {
        let resp = srv.client.get(srv.url(path)).send().await.unwrap();
        let status = resp.status();
        assert!(
            status.is_success(),
            "GET {path} returned {status} — expected 200"
        );
        let ct = resp.headers()
            .get("content-type")
            .and_then(|v| v.to_str().ok())
            .unwrap_or("");
        assert!(ct.contains("text/html"), "GET {path} content-type was {ct:?}");
    }
}

// ── Tracks ────────────────────────────────────────────────────────────────────

#[tokio::test]
async fn tracks_empty_on_fresh_db() {
    let srv = TestServer::start().await;
    for status in ["queued", "hearted", "rejected", "exported"] {
        let resp = srv.client
            .get(srv.url(&format!("/api/tracks?status={status}")))
            .send().await.unwrap();
        assert_eq!(resp.status(), 200, "status={status}");
        let body: Value = resp.json().await.unwrap();
        assert_eq!(body, json!([]), "expected empty list for status={status}");
    }
}

#[tokio::test]
async fn track_get_unknown_is_404() {
    let srv = TestServer::start().await;
    let resp = srv.client.get(srv.url("/api/tracks/999999")).send().await.unwrap();
    assert_eq!(resp.status(), 404);
}

#[tokio::test]
async fn track_set_status_unknown_is_404() {
    let srv = TestServer::start().await;
    let resp = srv.client
        .post(srv.url("/api/tracks/999999/status"))
        .json(&json!({"status": "queued"}))
        .send().await.unwrap();
    assert_eq!(resp.status(), 404);
}

// ── Digests ───────────────────────────────────────────────────────────────────

#[tokio::test]
async fn digests_empty_on_fresh_db() {
    let srv = TestServer::start().await;
    let resp = srv.client.get(srv.url("/api/digests")).send().await.unwrap();
    assert_eq!(resp.status(), 200);
    let body: Value = resp.json().await.unwrap();
    assert_eq!(body, json!([]));
}

#[tokio::test]
async fn digest_crud_lifecycle() {
    let srv = TestServer::start().await;

    let spec = json!({
        "name": "smoke-test digest",
        "filters": {"genre_or_tag": "drum & bass", "max_plays": 500},
        "ranking": "score",
        "cron_expr": "0 0 6 * * SUN",
        "timezone": "America/Los_Angeles",
        "target_size": 25,
        "max_pages": 20,
        "playlist_visibility": "private",
        "playlist_title_tmpl": "crater {date}"
    });

    // Create
    let resp = srv.client
        .post(srv.url("/api/digests"))
        .json(&spec)
        .send().await.unwrap();
    assert_eq!(resp.status(), 200, "create failed");
    let created: Value = resp.json().await.unwrap();
    let id = created["id"].as_i64().expect("id missing");
    assert_eq!(created["spec"]["name"], "smoke-test digest");
    assert_eq!(created["enabled"], true);

    // Get by id
    let resp = srv.client.get(srv.url(&format!("/api/digests/{id}"))).send().await.unwrap();
    assert_eq!(resp.status(), 200);
    let fetched: Value = resp.json().await.unwrap();
    assert_eq!(fetched["id"], id);

    // Appears in list
    let list: Value = srv.client
        .get(srv.url("/api/digests"))
        .send().await.unwrap()
        .json().await.unwrap();
    assert!(
        list.as_array().unwrap().iter().any(|d| d["id"] == id),
        "created digest not in list"
    );

    // Patch — toggle disabled
    let resp = srv.client
        .patch(srv.url(&format!("/api/digests/{id}")))
        .json(&json!({"enabled": false}))
        .send().await.unwrap();
    assert_eq!(resp.status(), 200);
    let patched: Value = resp.json().await.unwrap();
    assert_eq!(patched["enabled"], false);

    // Get runs (empty)
    let resp = srv.client
        .get(srv.url(&format!("/api/digests/{id}/runs")))
        .send().await.unwrap();
    assert_eq!(resp.status(), 200);
    let runs: Value = resp.json().await.unwrap();
    assert_eq!(runs, json!([]));

    // Delete (204 No Content)
    let resp = srv.client
        .delete(srv.url(&format!("/api/digests/{id}")))
        .send().await.unwrap();
    assert!(resp.status().is_success(), "delete returned {}", resp.status());

    // Confirm gone
    let resp = srv.client.get(srv.url(&format!("/api/digests/{id}"))).send().await.unwrap();
    assert_eq!(resp.status(), 404);
}

#[tokio::test]
async fn digest_invalid_cron_is_400() {
    let srv = TestServer::start().await;
    let resp = srv.client
        .post(srv.url("/api/digests"))
        .json(&json!({
            "name": "bad cron",
            "filters": {},
            "ranking": "score",
            "cron_expr": "not a valid cron expression",
            "timezone": "America/Los_Angeles",
            "target_size": 25,
            "max_pages": 20,
            "playlist_visibility": "private",
            "playlist_title_tmpl": ""
        }))
        .send().await.unwrap();
    assert_eq!(resp.status(), 400);
    let body: Value = resp.json().await.unwrap();
    assert_eq!(body["error"], "invalid_cron");
}

// ── Search ────────────────────────────────────────────────────────────────────

#[tokio::test]
async fn search_start_returns_session_id() {
    let srv = TestServer::start().await;
    let resp = srv.client
        .post(srv.url("/api/search"))
        .json(&json!({
            "filters": {"genre_or_tag": "drum & bass", "max_plays": 100},
            "target_size": 5,
            "max_pages": 2
        }))
        .send().await.unwrap();
    assert_eq!(resp.status(), 200);
    let body: Value = resp.json().await.unwrap();
    assert!(
        body["session_id"].is_string(),
        "expected session_id, got: {body}"
    );
}

#[tokio::test]
async fn search_poll_unknown_session_is_404() {
    let srv = TestServer::start().await;
    let resp = srv.client
        .get(srv.url("/api/search/00000000-0000-0000-0000-000000000000"))
        .send().await.unwrap();
    assert_eq!(resp.status(), 404);
}

// ── WebSocket ─────────────────────────────────────────────────────────────────

#[tokio::test]
async fn websocket_ping_pong() {
    use futures_util::{SinkExt, StreamExt};
    use tokio_tungstenite::{connect_async, tungstenite::Message};

    let srv = TestServer::start().await;
    let (mut ws, _) = connect_async(srv.ws_url("/ws"))
        .await
        .expect("WS handshake failed");

    ws.send(Message::Text(r#"{"type":"ping"}"#.into())).await.unwrap();

    let reply = tokio::time::timeout(
        std::time::Duration::from_secs(3),
        ws.next(),
    )
    .await
    .expect("timeout waiting for pong")
    .expect("stream closed early")
    .expect("WS error");

    let val: Value = serde_json::from_str(&reply.into_text().unwrap()).unwrap();
    assert_eq!(val["type"], "pong");
}

#[tokio::test]
async fn websocket_unknown_session_returns_error() {
    use futures_util::{SinkExt, StreamExt};
    use tokio_tungstenite::{connect_async, tungstenite::Message};

    let srv = TestServer::start().await;
    let (mut ws, _) = connect_async(srv.ws_url("/ws")).await.unwrap();

    // Subscribe to a session that doesn't exist.
    ws.send(Message::Text(serde_json::to_string(&json!({
        "type": "subscribe",
        "channel": "search",
        "session_id": "00000000-0000-0000-0000-000000000000"
    })).unwrap().into())).await.unwrap();

    let reply = tokio::time::timeout(
        std::time::Duration::from_secs(3),
        ws.next(),
    )
    .await
    .expect("timeout")
    .expect("stream closed")
    .expect("WS error");

    let val: Value = serde_json::from_str(&reply.into_text().unwrap()).unwrap();
    assert_eq!(val["error"], "session_not_found");
}
