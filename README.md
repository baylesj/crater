# crater

A self-hosted SoundCloud crate-digging tool for DJs who want to find
tracks that aren't already on every curated playlist.

> Named for [Chasma Sound](https://soundcloud.com/) — a chasma is a
> deep, steep-sided depression; a crater is what you make when you
> dig hard enough. Also a verb: to crater is to go deep.

## Status

**Alpha.** All three crates are implemented and the server runs end-to-end.
Search, track triage, digest scheduling, audio playback, and playlist
export all work.

## Architecture

```
crates/
├── sc_client/   SoundCloud v2 API client + playlist CRUD
├── core/        SQLite data layer, session streaming, digest runner
└── server/      axum HTTP API, HTMX + Askama UI, WebSocket, scheduler
```

## Running locally

```sh
# Install just (brew install just), then:
just dev
```

Open `http://localhost:8080` in a browser. The server creates `./data/crater.db`
on first run and scrapes a SoundCloud `client_id` automatically if no official
API credentials are configured.

For playlist export, either paste an OAuth token in **Settings** or configure
the official API credentials (see env vars below) to use the built-in PKCE flow.

## Environment variables

```
CRATER_BIND=0.0.0.0:8080           # default
CRATER_DATA_DIR=/data

# Optional: password-protect the UI (recommended if exposed beyond LAN)
CRATER_PASSWORD=<secret>

# Official SoundCloud API (enables PKCE OAuth flow; falls back to scraping if absent)
CRATER_SC_CLIENT_ID=<id>
CRATER_SC_CLIENT_SECRET=<secret>
CRATER_REDIRECT_URI=http://localhost:8080/auth/soundcloud/callback

# Fallback: manually captured OAuth token (paste in Settings page)
CRATER_SC_OAUTH_TOKEN=<token>

CRATER_NTFY_URL=http://unraid.local:8090   # optional push notifications
CRATER_NTFY_TOPIC=crater
CRATER_TIMEZONE=America/Los_Angeles
CRATER_LOG=crater=info,crater_core=info,sc_client=info
```

## Tests

```sh
just test            # all non-live tests
just test-smoke      # server smoke tests (no network)
just test-live       # hits real SoundCloud
```

## Documentation

- [docs/](docs/) — design specs for all crates
- [docs/04-oauth-capture.md](docs/04-oauth-capture.md) — OAuth token setup

## Roadmap

- [x] `sc_client`: client_id scraping, search, client-side filters
- [x] `sc_client`: playlist CRUD, OAuth PKCE support
- [x] `core`: SQLite cache, seen/rejected/hearted/exported tracking
- [x] `core`: digest definitions + scheduler
- [x] `server`: axum HTTP API
- [x] `server`: HTMX + Askama UI (search, queue, digests, settings)
- [x] `server`: audio streaming (HLS proxy), WebSocket live updates
- [x] `server`: app password auth + SoundCloud PKCE OAuth flow
- [ ] Docker + Unraid deploy
- [ ] Track acquisition (yt-dlp subprocess)
- [ ] Web Audio API features (waveform, A/B preview)
- [ ] Continuous autoplay

## Ethical scope

This tool only reads public metadata. It does not download audio,
circumvent paywalls, or scrape at scale. It exists to let one person
find obscure tracks more efficiently than the SoundCloud web UI
allows.

## License

MIT (for now; reconsider before distributing)
