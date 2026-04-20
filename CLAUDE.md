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

## Status as of session 1

Design + scaffold complete. One crate is real code; the rest is
design docs.

- `crates/sc_client/` — **implemented**, not yet compiled on a real
  machine. The unofficial SoundCloud v2 API client. Scrapes
  `client_id`, searches tracks with server-side filters (genre, BPM)
  and client-side filters (play-count ceiling, likes floor), streams
  paginated results through a callback. See its module docs.
- `crates/core/` — **designed only**, see `docs/01-core-crate.md`.
  SQLite schema, session/dedup logic, digest config, runner.
- `crates/server/` — **designed only**, see `docs/02-server-api.md`.
  axum HTTP API, WebSocket streaming, scheduler, audio proxy.
- Web UI — **designed only**, see `docs/03-ui-design.md`. HTMX +
  Askama, three-column dig view, keyboard-first.
- OAuth capture flow — **documented**, see `docs/04-oauth-capture.md`.
  User-facing instructions for grabbing the SoundCloud token from
  DevTools.

## First session priorities

In order:

1. **Verify `sc_client` compiles.** Run `cargo check -p sc_client`.
   Fix any typos/imports. The code was written without a compiler
   available, so 0-2 small fixes are plausible.
2. **Run the smoke test.** `cargo run -p sc_client --example
   search_demo`. Should print ~20 low-play drum & bass tracks.
   - If `client_id` extraction fails, SoundCloud has likely shipped
     a new bundle format; the regex in `src/client_id.rs` needs
     updating. Fetch `https://soundcloud.com` manually, find the
     current asset URL pattern, adjust.
   - If the search URL shape is wrong (e.g. `filter.genre_or_tag`
     param name has changed), inspect a real v2 request in browser
     DevTools and align.
3. **Run the unit tests.** `cargo test -p sc_client`.
4. **Run the live integration tests** (optional, hits real
   SoundCloud). `cargo test -p sc_client --features live-tests --
   --nocapture`.
5. **Start on `core`.** Add `core` to the workspace members in the
   root `Cargo.toml`. Implement per `docs/01-core-crate.md`, in this
   order:
   a. `db.rs` + migrations + `Crater::new`
   b. `tracks.rs` — upsert, set_status, get_track
   c. `session.rs` — the streaming search + dedup primitive
   d. `digests.rs` + `digest_runner.rs`
   Integration tests against in-memory SQLite as you go.

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
- Don't implement playlist CRUD in `sc_client` before we have an
  OAuth token captured. It's the last thing to build; the user
  needs to do the DevTools capture flow first (see
  `docs/04-oauth-capture.md`).
- Don't add track acquisition logic in v1. Schema makes space for
  it; implementation is deferred.
- Don't suggest moving to Cowork or other tools. Claude Code is
  the right environment for this.
- Don't reimplement design decisions already in `docs/`. If you
  disagree with one, raise it explicitly, don't silently diverge.
- Don't add dependencies without a reason. The stack was chosen to
  stay slim.

## Open questions flagged in the design docs

Each doc has an "Open questions" section at the bottom listing
choices that went with a default but could go either way. When you
hit one during implementation, re-read it and either (a) go with
the documented default or (b) surface the tradeoff for discussion.
Don't silently pick the other option.

Notable ones:
- Cron timezone storage (docs/01) — current plan: separate
  `timezone` column, default `America/Los_Angeles`.
- Session state in-memory vs DB (docs/02) — current plan:
  in-memory hashmap with 5min TTL.
- Rate-limit retry policy (docs/02) — current plan: one retry with
  30s backoff, then surface.
- Waveform rendering approach (docs/03) — v1: SoundCloud's PNG.
  v2: Web Audio decoded.
- Continuous autoplay (docs/03) — v1: stop at end of track.

## Running the project

```sh
# Verify sc_client
cargo check -p sc_client
cargo test -p sc_client

# Run the search demo (requires network)
cargo run -p sc_client --example search_demo

# Live integration tests (hits real SoundCloud)
cargo test -p sc_client --features live-tests -- --nocapture

# Once core exists:
cargo check -p core
cargo test -p core

# Once server exists:
cargo run -p server
```

Env vars (will matter once `server` exists):

```
CRATER_BIND=0.0.0.0:8080
CRATER_DATA_DIR=/data
CRATER_SC_OAUTH_TOKEN=<from DevTools capture, see docs/04>
CRATER_NTFY_URL=http://unraid.local:8090  # optional
CRATER_NTFY_TOPIC=crater                  # optional
CRATER_TIMEZONE=America/Los_Angeles
CRATER_LOG=server=info,core=info,sc_client=info
```

## Deployment target

Docker container on Unraid. Single image, multi-stage build
(Rust builder → debian slim runtime). Volumes: `/data` (SQLite
+ config), `/music` (acquired audio, future). Port 8080.

Not a priority for the first few sessions — get the app working
locally first, then package.
