# `server` crate: HTTP API design

axum-based HTTP server that exposes the `core` crate over the network.
Runs as a single Docker container on Unraid, accessible from the LAN.

## Scope

- REST-ish JSON API for all CRUD (tracks, status, digests)
- WebSocket for streaming search progress (so the UI can render results
  as they arrive rather than blocking on a multi-page scan)
- HTMX-friendly HTML fragment endpoints for the UI layer (covered in
  `03-ui-design.md`)
- Static file serving for the web UI (embedded in the binary via
  `rust-embed` so deploys are a single artifact)

## Dependencies

```toml
[dependencies]
core = { path = "../core" }
axum = { version = "0.8", features = ["ws", "macros"] }
tokio = { workspace = true }
tokio-cron-scheduler = "0.13"
tower = "0.5"
tower-http = { version = "0.6", features = ["trace", "compression-gzip", "cors"] }
serde = { workspace = true }
serde_json = { workspace = true }
tracing = { workspace = true }
tracing-subscriber = { workspace = true }
anyhow = { workspace = true }
thiserror = { workspace = true }
clap = { version = "4", features = ["derive", "env"] }
askama = "0.12"
askama_axum = "0.4"
rust-embed = "8"
futures-util = "0.3"  # for WS stream handling
```

## Module layout

```
crates/server/src/
├── main.rs             entry, config, axum router assembly
├── config.rs           Config struct, env var parsing via clap
├── state.rs            AppState (Arc<Crater>, Arc<Scheduler>, etc.)
├── error.rs            AppError -> HTTP response impl
├── routes/
│   ├── mod.rs
│   ├── search.rs       POST /api/search, GET /api/search/:session_id
│   ├── tracks.rs       POST /api/tracks/:id/{queue,reject,heart,clear}
│   ├── digests.rs      CRUD for digests
│   ├── playlists.rs    POST /api/playlists/export
│   ├── stream.rs       GET /api/stream/:id (audio proxy)
│   ├── ws.rs           GET /ws (WebSocket entry)
│   └── ui/             HTMX fragment endpoints (separate file)
├── ws.rs               WebSocket message types + dispatch
└── scheduler.rs        tokio-cron-scheduler wiring for digests
```

## Configuration

All via env vars (12-factor, Docker-friendly):

```rust
#[derive(Parser, Debug)]
pub struct Config {
    #[arg(long, env = "CRATER_BIND", default_value = "0.0.0.0:8080")]
    pub bind: SocketAddr,

    #[arg(long, env = "CRATER_DATA_DIR", default_value = "/data")]
    pub data_dir: PathBuf,

    #[arg(long, env = "CRATER_MUSIC_DIR")]
    pub music_dir: Option<PathBuf>,  // optional until acquisition ships

    /// User OAuth token for SoundCloud playlist writes. Without this,
    /// read operations work but export fails.
    #[arg(long, env = "CRATER_SC_OAUTH_TOKEN", hide_env_values = true)]
    pub sc_oauth_token: Option<String>,

    #[arg(long, env = "CRATER_NTFY_URL")]
    pub ntfy_url: Option<String>,

    #[arg(long, env = "CRATER_NTFY_TOPIC")]
    pub ntfy_topic: Option<String>,

    /// Default timezone for cron evaluation.
    #[arg(long, env = "CRATER_TIMEZONE", default_value = "America/Los_Angeles")]
    pub timezone: String,

    #[arg(long, env = "CRATER_LOG", default_value = "server=info,core=info,sc_client=info")]
    pub log: String,
}
```

## Auth / security posture

LAN-only. No user auth because this is single-user on your home network
behind your reverse proxy. If you ever put it on the open internet:

- Wrap in authelia / tailscale / CF Access for auth.
- Add a `CRATER_API_TOKEN` env var and a middleware that requires
  `Authorization: Bearer $TOKEN` on `/api/*` routes. Scaffold the
  middleware now, gate behind env var presence, so it's easy to
  enable later.

## REST API

All routes prefixed with `/api`. Request/response bodies JSON unless
noted. Errors use standard problem-details shape:

```json
{ "error": "code", "message": "human-readable", "details": {...} }
```

### Search

**`POST /api/search`** — start a search session.

Request:
```json
{
  "filters": {
    "query": "drum and bass",
    "genre_or_tag": "drum & bass",
    "bpm_from": 170,
    "bpm_to": 178,
    "duration_from_ms": null,
    "duration_to_ms": null,
    "max_plays": 1000,
    "min_likes": 3,
    "limit": 50
  },
  "target_size": 30,
  "max_pages": 20
}
```

Response:
```json
{
  "session_id": "b0e5c9f1-...",
  "ws_url": "/ws/search/b0e5c9f1-..."
}
```

Server spawns a background task that runs the `Session` and pushes
results to a channel keyed by `session_id`. Client opens a WebSocket to
pull from that channel (see below). Session expires after 5 minutes of
no WS connection.

**`GET /api/search/:session_id`** — poll snapshot of session state
(non-streaming fallback for clients that can't WS).

```json
{
  "status": "running" | "complete" | "failed",
  "tracks": [ StoredTrack, ... ],
  "pages_scanned": 7,
  "total_scanned": 340,
  "exhausted": false,
  "error": null
}
```

### Tracks

**`GET /api/tracks/:id`** — fetch single track with status.

**`POST /api/tracks/:id/status`** — set status.
```json
{ "status": "queued" | "rejected" | "hearted" | "exported" | null,
  "note": "optional text" }
```
`null` clears status. Returns updated `StoredTrack`.

**`GET /api/tracks?status=queued`** — list tracks by status. Supports
`?limit=`, `?offset=`, `?order=added_desc|added_asc`.

### Playlists

**`POST /api/playlists/export`** — one-shot export of queued tracks.
```json
{
  "name": "crater dig — 2026-04-19",
  "visibility": "private",
  "track_ids": [1234, 5678, ...]  // optional, defaults to all queued
}
```
Returns:
```json
{
  "playlist_sc_id": 987654321,
  "playlist_url": "https://soundcloud.com/jordan/sets/...",
  "track_count": 24
}
```
On success, moves those tracks from `queued` → `exported`. Fails with
400 if `sc_oauth_token` not configured.

### Digests

**`GET /api/digests`** — list.

**`POST /api/digests`** — create.
```json
{
  "name": "Weekly DnB dig",
  "filters": { ...SearchFilters... },
  "ranking": "score",
  "cron_expr": "0 0 6 * * SUN",
  "target_size": 25,
  "max_pages": 30,
  "playlist_visibility": "private",
  "playlist_title_tmpl": "crater — dnb — {year}-W{week}"
}
```

**`PATCH /api/digests/:id`** — partial update.

**`DELETE /api/digests/:id`**.

**`POST /api/digests/:id/run`** — trigger immediately. Returns run id;
the actual work happens in the background (poll `/api/digests/:id/runs`
or subscribe via WS).

**`GET /api/digests/:id/runs?limit=20`** — run history.

### Audio stream proxy

**`GET /api/stream/:track_id`** — server proxies the SoundCloud HLS
manifest and segments. Rationale:

1. SoundCloud stream URLs require the `client_id` we've scraped — the
   browser doesn't have it.
2. CORS — SoundCloud's CDN doesn't send permissive headers for LAN
   origins.
3. Keeps the `client_id` server-side where it's easier to rotate.

Implementation: server calls `sc_client::get_stream_url(track_id)` (to
be added), gets back an HLS manifest URL, fetches it, rewrites the
segment URLs to point at `/api/stream/:track_id/segment/:idx`, serves
the rewritten manifest. Segments are proxied 1:1.

Audio only, never downloaded to disk. Adds a few hundred ms of latency
vs. direct play, imperceptible in practice.

## WebSocket protocol

**`GET /ws`** — single endpoint, multiplexed by message type.

Messages are JSON objects with a `type` discriminator:

Client → server:
```json
{ "type": "subscribe", "channel": "search", "session_id": "..." }
{ "type": "subscribe", "channel": "digest_runs" }
{ "type": "unsubscribe", "channel": "search", "session_id": "..." }
{ "type": "ping" }
```

Server → client:
```json
{ "type": "pong" }

// Search progress
{ "type": "search.track",
  "session_id": "...",
  "track": StoredTrack,
  "total_scanned": 127,
  "pages_scanned": 3 }

{ "type": "search.complete",
  "session_id": "...",
  "exhausted": false,
  "total_accepted": 24 }

{ "type": "search.error",
  "session_id": "...",
  "error": "rate_limited",
  "message": "..." }

// Digest events
{ "type": "digest.run_started", "digest_id": 3, "run_id": 47 }
{ "type": "digest.run_completed", "digest_id": 3, "run_id": 47,
  "playlist_url": "...", "track_count": 25 }
{ "type": "digest.run_failed", "digest_id": 3, "run_id": 47,
  "error": "..." }
```

Heartbeat every 30s via ping/pong. Server closes the socket on missed
pongs.

## HTMX fragment endpoints

These return HTML fragments instead of JSON, for direct HTMX swaps.
Separate from the JSON API above.

```
GET  /ui/tracks/:id/card           render single track card
POST /ui/tracks/:id/queue          toggle queue, returns updated card
POST /ui/tracks/:id/reject         reject, returns empty (hx-swap=outerHTML)
POST /ui/tracks/:id/heart          heart, returns updated card
GET  /ui/queue                     render queue sidebar
GET  /ui/digests                   render digest list page section
```

Details in `03-ui-design.md`.

## Scheduler

`tokio-cron-scheduler` spawned at boot. One job registered per enabled
digest:

```rust
async fn register_digest_jobs(sched: &JobScheduler, crater: Arc<Crater>) -> Result<()> {
    let digests = crater.list_digests().await?;
    for d in digests.into_iter().filter(|d| d.enabled) {
        let cron = d.spec.cron_expr.clone();
        let crater = crater.clone();
        let id = d.id;
        sched.add(
            Job::new_async(&cron, move |_, _| {
                let crater = crater.clone();
                Box::pin(async move {
                    if let Err(e) = crater.run_digest(id).await {
                        tracing::error!(digest_id = id, error = ?e, "digest run failed");
                    }
                })
            })?
        ).await?;
    }
    Ok(())
}
```

Re-registration on digest CRUD: for v1, just restart the server (simple
and correct). v2: track job handles by digest id and
add/remove/replace on the fly.

## Graceful shutdown

On SIGTERM (Docker stop):
1. Stop accepting new HTTP connections.
2. Signal scheduler to drain.
3. Wait for in-flight digest runs to finish (with 60s timeout).
4. Close DB pool.
5. Exit.

## Observability

- `tracing` spans around every request (via `tower-http::TraceLayer`).
- `/api/health` returns `{ "status": "ok", "version": "...", "uptime_s": ... }`.
- `/api/metrics` — Prometheus format, optional, behind a feature flag.
  Tracks: request counts/latencies, digest run counts/durations,
  sc_client rate-limit hits, WS connection counts.

## Open questions

- **Single WS endpoint vs per-channel.** Current design multiplexes on
  one socket. Simpler. Alternative: `/ws/search/:id` and `/ws/digests`
  as separate endpoints. Less code server-side. Going with multiplex
  because the client-side JS gets simpler (one connection to manage).
- **Session state in memory vs DB.** Current plan: in-memory hashmap
  keyed by `session_id`, 5min TTL. Lost on restart, which is fine.
  Only downside: can't resume a session after a client reconnect. For
  a personal tool, acceptable.
- **Rate-limit handling surface.** When sc_client returns
  `ScError::RateLimited`, the search just halts. Should the server
  auto-retry with backoff, or surface the error to the client and let
  them retry? Current plan: one retry with 30s backoff, then surface.
