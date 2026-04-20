# UI design (HTMX + Askama)

Browser-based UI served by the `server` crate. HTMX for partial page
updates, Askama for server-side templates, vanilla JS for two things:
audio playback and WebSocket handling. Zero build step.

## Design principles

- **Keyboard-first.** You're a DJ going through tracks fast. Mouse is
  optional. Vim-ish bindings: `j/k` for track navigation, space to
  play/pause, `y` to queue, `n` to reject, `h` to heart, `/` for search.
- **Density over whitespace.** Show many tracks at once. DJ-tool
  aesthetic, not a marketing site.
- **Dark by default.** Because of course.
- **Mobile-tolerant, not mobile-first.** Primary use is laptop on the
  couch. Phone support for "check digest result on Monday morning" is
  nice-to-have, not core.
- **Progressive enhancement.** If JS fails, search + queue + reject
  still work via form submits. Audio preview and WS streaming degrade.

## Page map

```
/                         → redirect to /dig
/dig                      main search/browse page
/queue                    current export queue, large cards, drag-reorder
/digests                  digest list + create/edit
/digests/:id              digest detail: history, next run, edit
/history                  all tracks ever seen, filterable
/settings                 tokens, ntfy config, timezone
```

## `/dig` — the main view

This is where 95% of the time gets spent. Three-column desktop layout:

```
┌──────────────────────────────────────────────────────────────────────────────┐
│  crater                              [dig] [queue: 7] [digests] [history] [⚙] │
├─────────────────┬────────────────────────────────────────┬───────────────────┤
│ FILTERS         │ RESULTS                                │ NOW PLAYING       │
│                 │                                        │                   │
│ query           │ ▶ phantom loop — deep resolve     ★   │  phantom loop     │
│ [drum and bass] │   412 ▶ · 38 ♥ · ratio 0.092 · 174bpm │  deep resolve     │
│                 │   4:32 · dnb · 2w ago                  │                   │
│ genre           │                                        │ [waveform canvas] │
│ [drum & bass ▾] │   neuroclast — untitled sketch 04    │                   │
│                 │   156 ▶ · 22 ♥ · ratio 0.141 · 172bpm │ ├──●──────────┤  │
│ bpm             │   5:18 · dnb · 4d ago                  │  1:23 / 4:32      │
│ [170]─[178]     │                                        │                   │
│                 │ • bassweight — halftime experiment     │ [─][▶/⏸][+][−]   │
│ max plays       │   891 ▶ · 67 ♥ · ratio 0.075 · 87bpm  │                   │
│ [1000]          │   6:44 · halftime · 3w ago             │ queue [y]         │
│                 │                                        │ reject [n]        │
│ min likes       │   ... (scroll)                         │ heart [h]         │
│ [3]             │                                        │ open on SC [o]    │
│                 │ scanned 847 · accepted 24 · 3 pages    │                   │
│ [search] [reset]│ [load more]                            │ [playing on LAN]  │
│                 │                                        │                   │
│ saved queries   │                                        │                   │
│ • weekly dnb    │                                        │                   │
│ • halftime      │                                        │                   │
│ • late-night    │                                        │                   │
└─────────────────┴────────────────────────────────────────┴───────────────────┘
```

### Result card anatomy

Each track is a single row, ~3 lines tall. Left edge: a narrow indicator
column for status:
- ▶ currently playing
- ★ hearted (gold)
- ✓ queued (green)
- blank otherwise

Compact metadata row: plays · likes · ratio · bpm · duration · genre · age.
`ratio` is likes/plays, formatted to 3 decimals. `age` is humanized
("2w ago", "4d ago").

Click title → play. Click artist → open their SoundCloud profile in new
tab. Hover → keyboard-shortcut tooltip on actions.

Keybindings when focus is on results:
- `j` / `↓` : next track
- `k` / `↑` : previous track
- `space` : play/pause the selected track
- `enter` : play selected
- `y` : queue selected
- `n` : reject selected
- `h` : heart selected
- `o` : open on SoundCloud
- `/` : focus query input
- `?` : show all shortcuts

### Filter sidebar

Live: changes to filters fire a debounced 400ms HTMX request that
re-runs the search. The "search" button is for explicit triggering (and
keyboard accessibility).

Saved queries: clicking loads the filter values, doesn't auto-search.
"Save current filters as..." lives at the bottom.

### Now-playing pane

Only visible when something's playing. Collapses to a bottom bar on
narrow screens.

Waveform: rendered to a `<canvas>` using the Web Audio API if we can
get the decoded audio (via `fetch` + `decodeAudioData`), else falls
back to SoundCloud's pre-rendered waveform URL from the track metadata.
For v1, do the fallback only — the `AudioContext` route opens up
features (scrubbable waveform, eventual crossfading) that we should
build when we build them, not on day one.

### Streaming search UX

When a search runs: `POST /api/search` gets a session_id, opens the WS
if not already open, subscribes to the session. As `search.track`
messages arrive, append HTML rows via `outerHTML` swap targeted at a
`#results` container. Bottom status bar updates `pages_scanned /
total_scanned / accepted` in real time.

"Load more" button: calls the session's next_batch until exhausted or
user stops. Button disables + shows spinner during fetch.

## `/queue` — the export queue

Full-width vertical list, big cards (2x dig view size), drag handle on
each row for reorder. Above the list:

```
┌──────────────────────────────────────────────────────────────────────┐
│  QUEUE — 24 tracks · 1h 43m total                                    │
│                                                                      │
│  Export as: [crater dig — 2026-04-19]     visibility: (•) private    │
│             [ export to SoundCloud ]                   ( ) public    │
└──────────────────────────────────────────────────────────────────────┘
```

Drag-to-reorder updates local state; a "save order" button appears if
order has changed (or we auto-persist on each drop — lean toward
auto-persist via HTMX POST to `/ui/queue/reorder`).

## `/digests`

List view: one card per digest. Name, filters summary, cron expression
humanized ("every Sunday at 6:00 AM"), last run status, next run time.
Enable/disable toggle on the card. Click through for detail.

`/digests/:id` shows the filters in an editable form (same widgets as
the dig page sidebar), cron with a visual builder ("every [Sunday ▾]
at [06:00]"), and a runs table at the bottom. Each run row links to the
generated SoundCloud playlist.

"Create digest" button uses current dig page filters as a starting
point — the common flow is "I have a search that works, make it
recurring."

## `/history`

All tracks ever seen, with filter chips for status. Default view:
"rejected" so you can undo regrets. Search by artist/title. Useful
mostly as an escape hatch.

## Audio implementation details

Playback flow:
1. User triggers play on track X.
2. JS calls `/api/stream/:X` — server returns HLS manifest (content-type
   `application/vnd.apple.mpegurl`).
3. For browsers with native HLS (Safari): `audio.src = "/api/stream/X"`,
   done.
4. For Chrome/Firefox: use `hls.js` (tiny library, ~50KB gzipped,
   embedded via `rust-embed`). It parses the manifest and feeds
   segments into a Media Source Extensions buffer.

Why not MP3: SoundCloud serves streams as HLS-wrapped Opus in most cases,
not progressive MP3. The unofficial "progressive MP3" URL exists but is
increasingly unreliable. HLS is the stable path.

Latency from click to first audio: ~500ms on LAN. Acceptable.

## Styling

Single `style.css`, no framework. Hand-rolled CSS variables for a
dark palette:

```css
:root {
  --bg-0: #0f1013;
  --bg-1: #171920;
  --bg-2: #1f222b;
  --fg-0: #e8ebf0;
  --fg-1: #a0a6b0;
  --fg-2: #5c6370;
  --accent: #c9ad7f;     /* warm gold — hearted tracks */
  --queued: #7fb383;     /* muted green */
  --danger: #c06970;     /* muted red */
  --bpm: #7fa3b3;        /* for BPM badges */
}
```

Monospace for numeric metadata (plays, bpm, ratios) so columns align.
Sans for everything else. One typeface family total — Inter + a
monospace like Berkeley Mono or JetBrains Mono.

## JS architecture

Three small modules, no framework:

```
web/static/js/
├── ws.js          WebSocket client, subscription management
├── audio.js       HLS playback, waveform, keyboard handlers
└── keys.js        global keyboard shortcut dispatcher
```

HTMX handles everything else. Total JS should be <20KB of hand-written
code + hls.js (~50KB gzipped).

## Accessibility

- All interactive elements keyboard-reachable.
- ARIA live region announces "X tracks added" when search streams in,
  so screen readers hear progress.
- Focus ring visible. No `:focus { outline: none; }` nonsense.
- Color not the sole signal — status indicators also use distinct
  glyphs (★ ✓ ▶).

## Open questions

- **Waveform: decoded audio or SC's pre-rendered PNG.** Decoded is
  prettier and unlocks future features; PNG is simpler and already
  aligned with the track. v1: PNG. v2 (when we do Web Audio work):
  decoded.
- **Continuous autoplay.** If you're going through 30 tracks, should
  `space` on the current track advance to the next after it ends, or
  stop? Lean toward stop (DJ use case = decide, don't just listen),
  but make it a setting.
- **Mobile layout collapse.** Three columns fold to single column on
  <900px, now-playing becomes a sticky bottom bar. Filters collapse
  into a drawer. Standard responsive pattern.
