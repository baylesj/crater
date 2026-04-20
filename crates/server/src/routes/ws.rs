//! WebSocket endpoint — single multiplexed connection.
//!
//! Client sends subscribe/unsubscribe/ping JSON messages.
//! Server pushes search.track / search.complete / digest.* events.
//!
//! Protocol details: docs/02-server-api.md § WebSocket protocol.

use axum::{
    extract::{
        ws::{Message, WebSocket, WebSocketUpgrade},
        State,
    },
    response::IntoResponse,
};
use tokio::sync::broadcast;
use uuid::Uuid;

use crate::state::{SearchEvent, SharedState};

// ── Entry point ───────────────────────────────────────────────────────────────

pub async fn handler(
    ws:    WebSocketUpgrade,
    State(state): State<SharedState>,
) -> impl IntoResponse {
    ws.on_upgrade(|socket| handle(socket, state))
}

// ── Message shapes ────────────────────────────────────────────────────────────

#[derive(Debug, serde::Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
enum ClientMsg {
    Subscribe   { channel: String, session_id: Option<Uuid> },
    Unsubscribe { channel: String, session_id: Option<Uuid> },
    Ping,
}

// ── Socket handler ────────────────────────────────────────────────────────────

async fn handle(mut socket: WebSocket, state: SharedState) {
    let mut search_rx: Option<broadcast::Receiver<SearchEvent>> = None;
    let mut digest_rx = state.digest_events.subscribe();

    loop {
        tokio::select! {
            // ── Inbound from client ───────────────────────────────────────────
            msg = socket.recv() => {
                match msg {
                    Some(Ok(Message::Text(text))) => {
                        match serde_json::from_str::<ClientMsg>(&text) {
                            Ok(ClientMsg::Ping) => {
                                let _ = socket.send(Message::Text(r#"{"type":"pong"}"#.into())).await;
                            }
                            Ok(ClientMsg::Subscribe { channel, session_id }) => {
                                if channel == "search" {
                                    if let Some(id) = session_id {
                                        search_rx = crate::state::subscribe_session(&state.sessions, id);
                                        if search_rx.is_none() {
                                            let _ = socket.send(Message::Text(
                                                serde_json::json!({
                                                    "type": "error",
                                                    "error": "session_not_found",
                                                    "session_id": id,
                                                }).to_string().into()
                                            )).await;
                                        }
                                    }
                                }
                                // "digest_runs" subscription is implicit — everyone gets digest events
                            }
                            Ok(ClientMsg::Unsubscribe { channel, .. }) => {
                                if channel == "search" {
                                    search_rx = None;
                                }
                            }
                            Err(_) => {
                                // Ignore malformed messages
                            }
                        }
                    }
                    Some(Ok(Message::Close(_))) | None => break,
                    _ => {} // binary, ping frames, etc.
                }
            }

            // ── Outbound: search events ───────────────────────────────────────
            event = recv_search(&mut search_rx) => {
                match event {
                    Some(ev) => {
                        if let Ok(json) = serde_json::to_string(&ev) {
                            if socket.send(Message::Text(json.into())).await.is_err() {
                                break;
                            }
                        }
                        // Unsubscribe automatically when search ends
                        if matches!(ev, SearchEvent::Complete { .. } | SearchEvent::Error { .. }) {
                            search_rx = None;
                        }
                    }
                    None => {
                        search_rx = None; // channel closed / lagged
                    }
                }
            }

            // ── Outbound: digest events ───────────────────────────────────────
            Ok(ev) = digest_rx.recv() => {
                if let Ok(json) = serde_json::to_string(&ev) {
                    if socket.send(Message::Text(json.into())).await.is_err() {
                        break;
                    }
                }
            }
        }
    }
}

/// Polls the search receiver if subscribed, otherwise blocks forever (disabling
/// the select arm cleanly without spinning).
async fn recv_search(rx: &mut Option<broadcast::Receiver<SearchEvent>>) -> Option<SearchEvent> {
    match rx {
        Some(r) => match r.recv().await {
            Ok(ev)                                      => Some(ev),
            Err(broadcast::error::RecvError::Lagged(_)) => None, // dropped messages
            Err(broadcast::error::RecvError::Closed)    => None,
        },
        None => std::future::pending().await,
    }
}
