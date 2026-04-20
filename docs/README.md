# crater docs

Design specs written before implementation. Read in order:

1. **[01-core-crate.md](01-core-crate.md)** — The `core` crate.
   SQLite schema, session/dedup model, digest config types. The data
   layer between `sc_client` (network) and `server` (HTTP).

2. **[02-server-api.md](02-server-api.md)** — The `server` crate.
   axum routes, JSON request/response shapes, WebSocket protocol,
   scheduler integration. The HTTP layer.

3. **[03-ui-design.md](03-ui-design.md)** — The web UI. HTMX + Askama
   page layouts, keybindings, audio playback, styling. The user-facing
   layer.

4. **[04-oauth-capture.md](04-oauth-capture.md)** — One-time manual
   step to get a SoundCloud OAuth token for playlist export.
   User-facing doc, not implementation spec.

These are specs, not implementation. The code in `crates/sc_client` is
real; everything else is paper. When picking up `core` or `server`, the
relevant doc here should be enough context to start writing code
without re-deriving decisions.

## Decision status legend

Throughout the specs, "Open questions" sections list things that could
go either way. Defaults are chosen; flag for discussion if any of them
feel wrong at implementation time.
