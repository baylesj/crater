# crater

A self-hosted SoundCloud crate-digging tool for DJs who want to find
tracks that aren't already on every curated playlist.

> Named for [Chasma Sound](https://soundcloud.com/) — a chasma is a
> deep, steep-sided depression; a crater is what you make when you
> dig hard enough. Also a verb: to crater is to go deep.

## Status

**Pre-alpha.** Session 1: `sc_client` crate. Search works, filtering
works, nothing else yet. See `docs/` for design specs of what comes next.

## Architecture

```
crates/
├── sc_client/   SoundCloud v2 API client  (implemented)
├── core/        Business logic + SQLite   (designed — see docs/01)
└── server/      axum + HTMX UI            (designed — see docs/02, 03)
```

## Documentation

- [docs/](docs/) — full design specs for all crates
- [docs/04-oauth-capture.md](docs/04-oauth-capture.md) — one-time setup
  to enable playlist export

## Running the smoke test

```sh
cd /path/to/crater
cargo run -p sc_client --example search_demo
```

First run scrapes SoundCloud for a `client_id` (~2s), subsequent runs in
the same process are instant. You should see ~20 drum & bass tracks
with under 1000 plays printed to stdout with their URLs.

To tweak the query, edit the `SearchFilters` in
`crates/sc_client/examples/search_demo.rs`.

## Running unit tests

```sh
cargo test -p sc_client
```

## Running live integration tests

These hit the real SoundCloud API and are gated behind a feature flag:

```sh
cargo test -p sc_client --features live-tests -- --nocapture
```

## Roadmap

- [x] `sc_client`: client_id scraping, search, client-side filters
- [ ] `sc_client`: playlist CRUD (needs OAuth token)
- [ ] `core`: SQLite cache, seen/rejected tracking
- [ ] `core`: digest definitions + scheduler
- [ ] `server`: axum HTTP API
- [ ] `server`: HTMX + Askama UI
- [ ] Docker + Unraid deploy
- [ ] Track acquisition (yt-dlp subprocess)
- [ ] Web Audio API features (waveform, A/B preview)

## Ethical scope

This tool only reads public metadata. It does not download audio,
circumvent paywalls, or scrape at scale. It exists to let one person
find obscure tracks more efficiently than the SoundCloud web UI
allows.

## License

MIT (for now; reconsider before distributing)
