//! Shared application state and session management.
//!
//! `AppState` is cheaply cloneable (everything behind Arc). It holds the
//! `Crater` facade, the in-memory search session map, and a broadcast channel
//! for digest lifecycle events (used by the WebSocket layer).
//!
//! Sessions are lazily cleaned up on access — if a session is older than
//! `SESSION_TTL`, it's treated as expired. For a single-user tool this is
//! simpler than a background reaper.

use std::collections::HashMap;
use std::sync::{Arc, RwLock};
use std::time::{Duration, Instant};

use tokio::sync::broadcast;
use uuid::Uuid;

use crater_core::{Crater, StoredTrack};

const SESSION_TTL: Duration = Duration::from_secs(5 * 60);
const EVENT_BUF:   usize    = 256;
const DIGEST_BUF:  usize    = 16;

pub type SharedState = Arc<AppState>;

pub struct AppState {
    pub crater:        Arc<Crater>,
    pub sessions:      Arc<RwLock<HashMap<Uuid, SessionEntry>>>,
    /// Broadcast channel for digest run lifecycle events.
    pub digest_events: broadcast::Sender<DigestEvent>,
}

impl AppState {
    pub fn new(crater: Crater) -> SharedState {
        let (digest_events, _) = broadcast::channel(DIGEST_BUF);
        Arc::new(Self {
            crater:        Arc::new(crater),
            sessions:      Arc::new(RwLock::new(HashMap::new())),
            digest_events,
        })
    }
}

// ── Session types ─────────────────────────────────────────────────────────────

pub struct SessionEntry {
    /// Broadcast sender — background task pushes SearchEvent here;
    /// WS connections subscribe via `.subscribe()`.
    pub events:     broadcast::Sender<SearchEvent>,
    /// Accumulated snapshot for the polling-fallback GET endpoint.
    pub snapshot:   Arc<RwLock<SessionSnapshot>>,
    pub created_at: Instant,
}

impl SessionEntry {
    pub fn new() -> (Self, broadcast::Receiver<SearchEvent>) {
        let (tx, rx) = broadcast::channel(EVENT_BUF);
        let entry = Self {
            events:     tx,
            snapshot:   Arc::new(RwLock::new(SessionSnapshot::default())),
            created_at: Instant::now(),
        };
        (entry, rx)
    }

    pub fn is_expired(&self) -> bool {
        self.created_at.elapsed() > SESSION_TTL
    }
}

#[derive(Debug, Clone, Default, serde::Serialize)]
pub struct SessionSnapshot {
    pub status:        SearchStatus,
    pub tracks:        Vec<StoredTrack>,
    pub pages_scanned: u32,
    pub total_scanned: u32,
    pub exhausted:     bool,
    pub error:         Option<String>,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, serde::Serialize)]
#[serde(rename_all = "snake_case")]
pub enum SearchStatus {
    #[default]
    Running,
    Complete,
    Failed,
}

// ── Event types (WS messages server → client) ─────────────────────────────────

#[derive(Debug, Clone, serde::Serialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum SearchEvent {
    /// One track yielded from the search.
    #[serde(rename = "search.track")]
    Track {
        session_id:    Uuid,
        track:         StoredTrack,
        total_scanned: u32,
        pages_scanned: u32,
    },
    /// Search finished normally.
    #[serde(rename = "search.complete")]
    Complete {
        session_id:     Uuid,
        exhausted:      bool,
        total_accepted: usize,
    },
    /// Search failed.
    #[serde(rename = "search.error")]
    Error {
        session_id: Uuid,
        error:      String,
        message:    String,
    },
}

#[derive(Debug, Clone, serde::Serialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum DigestEvent {
    #[serde(rename = "digest.run_started")]
    RunStarted {
        digest_id: i64,
        run_id:    i64,
    },
    #[serde(rename = "digest.run_completed")]
    RunCompleted {
        digest_id:    i64,
        run_id:       i64,
        playlist_url: Option<String>,
        track_count:  Option<i64>,
    },
    #[serde(rename = "digest.run_failed")]
    RunFailed {
        digest_id: i64,
        run_id:    i64,
        error:     String,
    },
}

// ── Session helpers ───────────────────────────────────────────────────────────

/// Insert a new session and return the broadcast receiver + snapshot Arc.
pub fn create_session(
    sessions: &RwLock<HashMap<Uuid, SessionEntry>>,
    id:       Uuid,
) -> (broadcast::Receiver<SearchEvent>, Arc<RwLock<SessionSnapshot>>) {
    let (entry, rx) = SessionEntry::new();
    let snapshot = entry.snapshot.clone();
    sessions.write().unwrap().insert(id, entry);
    (rx, snapshot)
}

/// Borrow a session's broadcast sender to push events, if not expired.
pub fn with_session_tx<F>(
    sessions: &RwLock<HashMap<Uuid, SessionEntry>>,
    id:       Uuid,
    f:        F,
) where
    F: FnOnce(&broadcast::Sender<SearchEvent>, &Arc<RwLock<SessionSnapshot>>),
{
    let guard = sessions.read().unwrap();
    if let Some(entry) = guard.get(&id) {
        if !entry.is_expired() {
            f(&entry.events, &entry.snapshot);
        }
    }
}

/// Get a snapshot clone for the polling endpoint.
pub fn get_snapshot(
    sessions: &RwLock<HashMap<Uuid, SessionEntry>>,
    id:       Uuid,
) -> Option<SessionSnapshot> {
    let guard = sessions.read().unwrap();
    let entry = guard.get(&id)?;
    if entry.is_expired() {
        return None;
    }
    let snap = entry.snapshot.read().unwrap().clone();
    Some(snap)
}

/// Subscribe to an existing session's event stream.
pub fn subscribe_session(
    sessions: &RwLock<HashMap<Uuid, SessionEntry>>,
    id:       Uuid,
) -> Option<broadcast::Receiver<SearchEvent>> {
    let guard = sessions.read().unwrap();
    let entry = guard.get(&id)?;
    if entry.is_expired() {
        return None;
    }
    Some(entry.events.subscribe())
}
