# CLAUDE.md

Context for Claude Code working on this project. Read the rest of the
repo (especially `docs/`) after this, but this file is the fast
orientation.

## What this is

`crater` — a self-hosted SoundCloud crate-digging tool for finding
obscure tracks (under a play-count ceiling) for DJ set construction.
Rust backend, HTMX web UI, SQLite, deployed as a single Docker
container on the user's Unraid server.

Named as a pun on the user's producer name (Chasma Sound). Also a
verb: "to crater" = to dig deep.

## Current status

All three crates are implemented and the server runs end-to-end.

- `crates/sc_client/` — **implemented**. Unofficial SoundCloud v2
  API client. Scrapes `client_id`, searches tracks with server-side
  filters (genre, BPM) and client-side filters (play-count ceiling,
  likes floor). Also has OAuth PKCE support and playlist CRUD.
- `crates/core/` — **implemented**. SQLite schema via sqlx migrations,
  `Crater` facade, track upsert/status, session streaming, digest
  CRUD, digest runner, HLS stream URL resolution, kv store.
- `crates/server/` — **implemented**. axum HTTP API, HTMX + Askama
  UI, WebSocket live updates, audio proxy, scheduler, settings page,
  optional app password auth, SoundCloud PKCE OAuth flow.

The server can be run today with `just dev` or `cargo run -p crater`.
Playlist export works once a SoundCloud OAuth token is in place (either
via the built-in PKCE flow or by pasting a token in Settings).

## Architecture cheatsheet

```
User's browser (LAN)
  │
  ├── HTMX fragment requests ─────┐
  ├── JSON /api/* ────────────────┤
  ├── WebSocket /ws ──────────────┤
  └── Audio /api/stream/:id ──────┤
                                  ▼
                        crater server (axum)
                              │
                              ├── core::Crater (facade)
                              │     │
                              │     ├── SQLite (sqlx)
                              │     └── sc_client::Client
                              │
                              └── scheduler (tokio-cron-scheduler)
                                    │
                                    └── runs digests on schedule
```

## Dependencies / stack decisions (already made, don't relitigate)

- **Rust 2021**, workspace layout, three crates (`sc_client`, `core`,
  `server`).
- **axum 0.8** with ws + macros features.
- **sqlx 0.8** (not diesel, not sea-orm) — async, compile-time
  queries, simple migrations.
- **HTMX + Askama** for UI (not Leptos, not React, not Yew). Zero
  build step. Server owns state.
- **hls.js** for audio playback (embedded via `rust-embed`).
- **SQLite** (not Postgres) — single user, ~100k tracks max, fits
  on any laptop.
- **tokio-cron-scheduler** for digest schedules.
- **Unofficial SoundCloud v2 API** to start. The user will apply for
  official API in parallel; `sc_client` is the swap boundary when
  approved.

## Code conventions

- **Error handling:** `anyhow::Result` in binaries and examples,
  typed `thiserror` errors in libraries. Every lib crate has its
  own error enum.
- **Async:** `tokio` everywhere.
- **Tracing:** `tracing` not `log`. Structured fields over string
  interpolation. Target-level filters (`sc_client=debug,core=info`).
- **Tests:** co-located `#[cfg(test)] mod tests` for units;
  `tests/` dir for integration; live tests gated behind a
  `live-tests` feature.
- **Schema tolerance:** serde structs use `#[serde(default)]` on
  everything non-essential so SoundCloud's API drift doesn't break
  deserialization. Store `raw_json` for future field recovery.
- **Comments:** explain *why*, not *what*. Module-level doc
  comments summarize purpose and scope. Avoid redundant inline
  comments that restate the code.
- **No emoji in code or commit messages.** The user doesn't use
  them and doesn't want them in output.
- **Module-level docs** go at the top of each file as `//!`
  comments. These were written with care; don't casually rewrite
  them when refactoring.

## User preferences (Jordan)

- Senior software engineer at Google Chrome, C++ and Rust. Assume
  fluency — no need to explain Rust idioms, ownership, async, etc.
- Strong preference for markdown (`.md`) deliverables and
  Obsidian-compatible formatting. When producing docs, keep them
  as `.md` in `docs/`.
- Prefers concise, direct communication. No filler. No
  "Certainly!" or "Great question!" openings.
- Vim keybindings, dark themes, dense UIs. Reflect in any UI work.
- Comfortable reading diffs; prefers precise patches over "here's
  the whole file again."
- Cares about correctness and good error handling; willing to
  trade verbosity for a robust system.
- Running on macOS (MacBook) for dev, Unraid for deployment.

## What NOT to do

- Don't switch the stack. HTMX, sqlx, axum are decided. If a
  better option comes up, flag it but don't pre-emptively rewrite.
- Don't add track acquisition logic in v1. Schema makes space for
  it; implementation is deferred.
- Don't suggest moving to Cowork or other tools. Claude Code is
  the right environment for this.
- Don't reimplement design decisions already in `docs/`. If you
  disagree with one, raise it explicitly, don't silently diverge.
- Don't add dependencies without a reason. The stack was chosen to
  stay slim.

## Open questions / deferred decisions

- Waveform rendering (docs/03) — v1 uses SoundCloud's PNG. v2 would
  be Web Audio decoded. Not implemented yet.
- Continuous autoplay (docs/03) — v1 stops at end of track. Not
  implemented yet.
- Rate-limit retry policy (docs/02) — current plan: one retry with
  30s backoff, then surface. Check `digest_runner.rs` for current
  behavior.
- Docker packaging — the server works locally; multi-stage Dockerfile
  for Unraid has not been written yet.

## Running the project

```sh
# Start the server with local dev defaults (data in ./data, debug logging)
just dev
# or manually:
cargo run -p crater

# Type-check the whole workspace
just check

# All non-live tests
just test

# Server smoke tests (real server, no SC network)
just test-smoke

# Live integration tests (hits real SoundCloud)
just test-live
just test-client-live

# sc_client search demo
cargo run -p sc_client --example search_demo
```

Env vars:

```
CRATER_BIND=0.0.0.0:8080           # default
CRATER_DATA_DIR=/data              # where crater.db lives
CRATER_PASSWORD=<secret>           # optional: gate the whole UI with a password

# Official SoundCloud API (get from https://soundcloud.com/you/apps)
# If absent, falls back to scraping a client_id from SC's homepage.
CRATER_SC_CLIENT_ID=<id>
CRATER_SC_CLIENT_SECRET=<secret>
CRATER_REDIRECT_URI=http://localhost:8080/auth/soundcloud/callback

# Fallback: manually captured OAuth token (Settings page or DevTools)
CRATER_SC_OAUTH_TOKEN=<token>

CRATER_NTFY_URL=http://unraid.local:8090   # optional push notifications
CRATER_NTFY_TOPIC=crater                   # optional
CRATER_TIMEZONE=America/Los_Angeles        # default
CRATER_LOG=crater=info,crater_core=info,sc_client=info
```

## Deployment target

Docker container on Unraid. Single image, multi-stage build
(Rust builder → debian slim runtime). Volumes: `/data` (SQLite
+ config), `/music` (acquired audio, future). Port 8080.

Not a priority for the first few sessions — get the app working
locally first, then package.
