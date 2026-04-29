//! HTML UI pages.
//!
//! Self-contained HTML documents with inline CSS and JS — no build step, no
//! external dependencies except HTMX (CDN). Static enough that Askama would
//! add complexity without benefit until we need server-side dynamic content.
//!
//! `/dig` is the main page. `/queue` and `/digests` load their data via the
//! JSON API on page load.

use axum::response::Html;

pub async fn index() -> axum::response::Redirect {
    axum::response::Redirect::to("/dig")
}

pub async fn dig()               -> Html<String> { Html(DIG.replace("/* CSS_PLACEHOLDER */", CSS)) }
pub async fn hearted_page()      -> Html<String> { Html(HEARTED.replace("/* CSS_PLACEHOLDER */", CSS)) }
pub async fn queue_page()        -> Html<String> { Html(QUEUE.replace("/* CSS_PLACEHOLDER */", CSS)) }
pub async fn digests_page()      -> Html<String> { Html(DIGESTS.replace("/* CSS_PLACEHOLDER */", CSS)) }
pub async fn digest_detail_page(
    axum::extract::Path(id): axum::extract::Path<i64>,
) -> Html<String> {
    Html(DIGEST_DETAIL.replace("/* CSS_PLACEHOLDER */", CSS).replace("DIGEST_ID_PLACEHOLDER", &id.to_string()))
}
pub async fn history_page()      -> Html<String> { Html(HISTORY.replace("/* CSS_PLACEHOLDER */", CSS)) }
pub async fn settings_page()    -> Html<String> { Html(SETTINGS.replace("/* CSS_PLACEHOLDER */", CSS)) }

// ── Shared CSS ────────────────────────────────────────────────────────────────
//
// Inlined into each page (small, no extra round-trips).

const CSS: &str = r##"
:root {
  --bg-0: #0f1013; --bg-1: #171920; --bg-2: #1f222b; --bg-3: #2a2e38;
  --fg-0: #e8ebf0; --fg-1: #a0a6b0; --fg-2: #5c6370;
  --accent: #c9ad7f;
  --queued: #7fb383;
  --green:  #7fb383;
  --danger: #c06970;
  --bpm:    #7fa3b3;
  --sel:    #252830;
}
*, *::before, *::after { box-sizing: border-box; margin: 0; padding: 0; }
html, body { height: 100%; }
body {
  background: var(--bg-0);
  color: var(--fg-0);
  font: 13px/1.5 system-ui, -apple-system, sans-serif;
  overflow: hidden;
}
a { color: inherit; text-decoration: none; }
a:hover { color: var(--fg-0); }
button {
  cursor: pointer;
  border: none;
  background: none;
  color: inherit;
  font: inherit;
}
button:focus-visible, a:focus-visible, input:focus-visible, select:focus-visible {
  outline: 1px solid var(--accent);
  outline-offset: 1px;
}
input, select {
  background: var(--bg-2);
  border: 1px solid var(--bg-2);
  color: var(--fg-0);
  padding: 5px 8px;
  border-radius: 3px;
  font: 13px/1.4 system-ui, sans-serif;
  width: 100%;
}
input:focus, select:focus { outline: 1px solid var(--accent); border-color: var(--accent); }
input::placeholder { color: var(--fg-2); }

/* ── header ──────────────────────────────────────────────────────────────── */
header {
  height: 38px;
  background: var(--bg-1);
  border-bottom: 1px solid var(--bg-2);
  display: flex;
  align-items: center;
  padding: 0 16px;
  gap: 20px;
  flex-shrink: 0;
}
.logo { font-size: 14px; font-weight: 700; letter-spacing: .06em; color: var(--accent); }
nav { display: flex; gap: 14px; }
nav a { font-size: 12px; color: var(--fg-1); padding: 3px 0; border-bottom: 2px solid transparent; }
nav a:hover, nav a.active { color: var(--fg-0); border-bottom-color: var(--accent); }
.nav-badge { font-size: 10px; background: var(--bg-2); border-radius: 10px; padding: 1px 5px;
             color: var(--fg-2); margin-left: 3px; font-variant-numeric: tabular-nums; }
.nav-cog { margin-left: auto; font-size: 20px; color: var(--fg-2);
           padding: 2px 4px; line-height: 1; border-bottom: 2px solid transparent; }
.nav-cog:hover, .nav-cog.active { color: var(--fg-0); border-bottom-color: var(--accent); }

/* ── dig layout ──────────────────────────────────────────────────────────── */
.dig-layout {
  display: grid;
  grid-template-columns: 220px 1fr;
  height: calc(100vh - 38px);
  overflow: hidden;
}
.dig-layout.has-playing { grid-template-columns: 220px 1fr 280px; }

/* ── filter sidebar ──────────────────────────────────────────────────────── */
#filters {
  overflow-y: auto;
  padding: 14px 12px;
  border-right: 1px solid var(--bg-2);
  display: flex;
  flex-direction: column;
  gap: 10px;
}
.filter-label {
  font-size: 10px;
  font-weight: 600;
  text-transform: uppercase;
  letter-spacing: .08em;
  color: var(--fg-2);
  margin-bottom: 4px;
}
.filter-group { display: flex; flex-direction: column; }
.filter-pair { display: grid; grid-template-columns: 1fr 1fr; gap: 5px; }
.filter-actions { display: flex; gap: 6px; margin-top: 4px; }
.btn-primary {
  background: var(--accent); color: var(--bg-0); border-radius: 3px;
  padding: 6px 12px; font-size: 12px; font-weight: 600; flex: 1;
}
.btn-primary:hover { opacity: .88; }
.btn-secondary {
  background: var(--bg-2); color: var(--fg-1); border-radius: 3px;
  padding: 6px 10px; font-size: 12px;
}
.btn-secondary:hover { color: var(--fg-0); }
hr.rule { border: none; border-top: 1px solid var(--bg-2); }

/* ── results column ──────────────────────────────────────────────────────── */
#results {
  display: flex;
  flex-direction: column;
  overflow: hidden;
}
#status-bar {
  flex-shrink: 0;
  padding: 6px 12px;
  font-size: 11px;
  color: var(--fg-2);
  border-bottom: 1px solid var(--bg-2);
  min-height: 28px;
  display: flex;
  align-items: center;
  justify-content: space-between;
}
#track-list {
  flex: 1;
  overflow-y: auto;
  list-style: none;
}

/* ── track card ──────────────────────────────────────────────────────────── */
.track {
  display: grid;
  grid-template-columns: 18px 40px 1fr auto;
  gap: 0 8px;
  padding: 7px 12px 7px 8px;
  border-bottom: 1px solid var(--bg-2);
  cursor: pointer;
  align-items: start;
}
.track:hover    { background: var(--bg-1); }
.track.selected { background: var(--sel); }
.track.playing  { background: var(--sel); }
.track.status-queued  { background: rgba(127,179,131,.07); }
.track.status-hearted { background: rgba(201,173,127,.07); }
.track.status-rejected { opacity: .45; }
.track.status-rejected .track-title { text-decoration: line-through; }

.ind {
  font-size: 11px;
  padding-top: 2px;
  text-align: center;
  flex-shrink: 0;
  width: 14px;
}
.ind-playing  { color: var(--fg-0); }
.ind-hearted  { color: var(--accent); }
.ind-queued   { color: var(--queued); }
.ind-rejected { color: var(--danger); }

.track-body { min-width: 0; }
.track-line1 {
  display: flex;
  align-items: baseline;
  gap: 0;
  overflow: hidden;
  white-space: nowrap;
  text-overflow: ellipsis;
  font-size: 13px;
}
.track-title {
  font-weight: 500;
  color: var(--fg-0);
  padding: 0;
  cursor: pointer;
  white-space: nowrap;
  overflow: hidden;
  text-overflow: ellipsis;
}
.track-title:hover { color: var(--accent); }
.track-sep  { color: var(--fg-2); padding: 0 3px; flex-shrink: 0; }
.track-artist { font-size: 12px; color: var(--fg-1); flex-shrink: 0; }
.track-artist:hover { color: var(--fg-0); text-decoration: underline; }

.track-meta, .track-meta2 {
  font-size: 11px;
  color: var(--fg-2);
  font-variant-numeric: tabular-nums;
  font-family: 'JetBrains Mono', 'Berkeley Mono', 'Cascadia Code', Consolas, monospace;
  margin-top: 2px;
  white-space: nowrap;
  overflow: hidden;
  text-overflow: ellipsis;
}

.track-actions {
  display: flex;
  align-items: center;
  gap: 2px;
  opacity: 0;
  flex-shrink: 0;
}
.track:hover .track-actions,
.track.selected .track-actions { opacity: 1; }
.track-actions button, .track-actions a {
  font-size: 12px;
  color: var(--fg-2);
  padding: 2px 5px;
  border-radius: 2px;
}
.track-actions button:hover, .track-actions a:hover { color: var(--fg-0); background: var(--bg-2); }
.track-actions .act-queue:hover  { color: var(--queued); background: var(--bg-2); }
.track-actions .act-heart:hover  { color: var(--accent); background: var(--bg-2); }
.track-actions .act-reject:hover { color: var(--danger); background: var(--bg-2); }

/* ── track artwork thumbnail ────────────────────────────────────────────── */
.track-art {
  width: 36px; height: 36px;
  border-radius: 2px;
  overflow: hidden;
  background: var(--bg-2);
  flex-shrink: 0;
  align-self: center;
}
.track-art img { width: 100%; height: 100%; object-fit: cover; display: block; }
.track-art-placeholder {
  width: 100%; height: 100%;
  display: flex; align-items: center; justify-content: center;
  font-size: 14px; color: var(--bg-3);
  user-select: none;
}

/* ── now-playing pane ────────────────────────────────────────────────────── */
#now-playing {
  display: none;
  flex-direction: column;
  padding: 16px;
  gap: 10px;
  border-left: 1px solid var(--bg-2);
  overflow-y: auto;
}
.has-playing #now-playing { display: flex; }

.np-title { font-size: 14px; font-weight: 600; color: var(--fg-0); line-height: 1.3; }
.np-artist { font-size: 12px; color: var(--fg-1); }

.np-art {
  width: 100%;
  aspect-ratio: 1;
  border-radius: 3px;
  overflow: hidden;
  background: var(--bg-2);
}
.np-art img { width: 100%; height: 100%; object-fit: cover; display: block; }
.np-art:empty { display: none; }

.np-waveform {
  height: 56px;
  background: var(--bg-2);
  border-radius: 3px;
  overflow: hidden;
  position: relative;
}
/* Placeholder bars to suggest a waveform until we have real data */
.np-waveform::after {
  content: '';
  position: absolute;
  inset: 0;
  background: repeating-linear-gradient(
    90deg,
    transparent 0px, transparent 2px,
    var(--bg-0) 2px, var(--bg-0) 3px
  );
  opacity: .3;
}

.np-scrubber-row {
  display: flex;
  align-items: center;
  gap: 6px;
  font-size: 11px;
  font-family: monospace;
  color: var(--fg-2);
}
.np-scrubber-row input[type=range] {
  flex: 1;
  padding: 0;
  height: 4px;
  accent-color: var(--accent);
  background: var(--bg-2);
  border: none;
  border-radius: 2px;
}

.np-controls {
  display: flex;
  gap: 6px;
  justify-content: center;
}
.np-controls button {
  font-size: 16px;
  color: var(--fg-1);
  padding: 4px 8px;
  border-radius: 3px;
}
.np-controls button:hover { color: var(--fg-0); background: var(--bg-2); }

.np-btn-group {
  display: flex;
  flex-direction: column;
  gap: 4px;
  margin-top: auto;
}
.np-btn {
  font-size: 12px;
  color: var(--fg-1);
  padding: 5px 8px;
  border-radius: 3px;
  text-align: left;
  border: 1px solid var(--bg-2);
}
.np-btn:hover { color: var(--fg-0); background: var(--bg-2); }
.np-btn .kbd { float: right; color: var(--fg-2); font-family: monospace; }
.np-btn.np-active-queue  { color: var(--queued); border-color: var(--queued); }
.np-btn.np-active-heart  { color: var(--accent); border-color: var(--accent); }
.np-btn.np-active-reject { color: var(--danger); border-color: var(--danger); }

.np-sc-link { font-size: 11px; color: var(--fg-2); text-align: center; margin-top: 4px; }
.np-sc-link:hover { color: var(--fg-0); }

/* ── keyboard help overlay ───────────────────────────────────────────────── */
#help-overlay {
  position: fixed;
  inset: 0;
  background: rgba(15,16,19,.87);
  display: flex;
  align-items: center;
  justify-content: center;
  z-index: 100;
}
#help-overlay.hidden { display: none; }
.help-box {
  background: var(--bg-1);
  border: 1px solid var(--bg-2);
  border-radius: 6px;
  padding: 20px 24px;
  width: 360px;
}
.help-box h3 {
  font-size: 11px; font-weight: 600; text-transform: uppercase;
  letter-spacing: .1em; color: var(--fg-2); margin-bottom: 12px;
}
.help-row {
  display: flex;
  justify-content: space-between;
  padding: 4px 0;
  font-size: 12px;
  border-bottom: 1px solid var(--bg-2);
}
.help-row:last-child { border: none; }
.help-key { font-family: monospace; color: var(--accent); min-width: 80px; }
.help-close { margin-top: 14px; text-align: right; }
.help-close button { font-size: 12px; color: var(--fg-1); }
.help-close button:hover { color: var(--fg-0); }

/* ── generic page layout (queue, digests) ────────────────────────────────── */
.page-layout {
  height: calc(100vh - 38px);
  overflow-y: auto;
  padding: 20px 24px;
  max-width: 900px;
  margin: 0 auto;
}
.page-header {
  display: flex;
  align-items: baseline;
  justify-content: space-between;
  margin-bottom: 16px;
}
.page-title {
  font-size: 11px; font-weight: 600; text-transform: uppercase;
  letter-spacing: .1em; color: var(--fg-2);
}
.card {
  background: var(--bg-1);
  border: 1px solid var(--bg-2);
  border-radius: 4px;
  padding: 12px 16px;
  margin-bottom: 8px;
}
.card-title { font-size: 13px; font-weight: 500; }
.card-meta  { font-size: 11px; color: var(--fg-2); margin-top: 4px;
              font-family: monospace; font-variant-numeric: tabular-nums; }
.card-actions { margin-top: 10px; display: flex; gap: 8px; }
.empty-state { color: var(--fg-2); font-size: 13px; padding: 24px 0; text-align: center; }

/* ── recent queries (dig sidebar) ───────────────────────────────────────────── */
.recent-query-item {
  display: block; width: 100%; text-align: left;
  font-size: 11px; color: var(--fg-2);
  padding: 3px 6px; border-radius: 3px; cursor: pointer;
  overflow: hidden; white-space: nowrap; text-overflow: ellipsis;
}
.recent-query-item:hover { color: var(--fg-0); background: var(--bg-2); }
"##;

// ── /dig ──────────────────────────────────────────────────────────────────────

const DIG: &str = r##"<!DOCTYPE html>
<html lang="en">
<head>
<meta charset="utf-8">
<meta name="viewport" content="width=device-width, initial-scale=1">
<title>crater / dig</title>
<style>/* CSS_PLACEHOLDER */</style>
</head>
<body>
<header>
  <span class="logo">crater</span>
  <nav>
    <a href="/dig" class="active">dig</a>
    <a href="/hearted">hearted <span class="nav-badge" id="hearted-count">0</span></a>
    <a href="/queue">queue <span class="nav-badge" id="queue-count">0</span></a>
    <a href="/digests">digests</a>
  </nav>
  <a href="/settings" class="nav-cog" title="settings">⚙</a>
</header>

<div class="dig-layout" id="layout">

  <!-- FILTER SIDEBAR -->
  <aside id="filters">
    <div class="filter-group">
      <div class="filter-label">query</div>
      <input id="f-query" type="text" placeholder="liquid, neurofunk, …"
             onkeydown="if(event.key==='Enter') startSearch()">
    </div>
    <div class="filter-group">
      <div class="filter-label">genre / tag</div>
      <input id="f-genre" type="text" placeholder="drum &amp; bass" list="genre-suggestions">
    </div>
    <datalist id="genre-suggestions">
      <!-- drum & bass family -->
      <option value="drum &amp; bass">
      <option value="liquid drum &amp; bass">
      <option value="neurofunk">
      <option value="darkstep">
      <option value="techstep">
      <option value="jump up">
      <option value="jungle">
      <!-- bass / experimental / left field -->
      <option value="bass">
      <option value="bass music">
      <option value="experimental bass">
      <option value="left field">
      <option value="halftime">
      <!-- dubstep / future bass -->
      <option value="dubstep">
      <option value="brostep">
      <option value="future bass">
      <option value="wave">
      <!-- house -->
      <option value="house">
      <option value="deep house">
      <option value="tech house">
      <option value="progressive house">
      <!-- techno / trance -->
      <option value="techno">
      <option value="trance">
      <option value="progressive trance">
      <!-- broad electronic -->
      <option value="electronic">
      <option value="electronica">
      <option value="dance &amp; EDM">
      <!-- uk dance -->
      <option value="uk garage">
      <option value="garage">
      <option value="grime">
      <option value="uk dance">
      <!-- breakbeat / footwork -->
      <option value="breaks">
      <option value="breakbeat">
      <option value="footwork">
      <option value="juke">
      <option value="jersey club">
      <!-- hardcore -->
      <option value="hardcore">
      <option value="hardstyle">
      <!-- synth / wave -->
      <option value="synthwave">
      <option value="vaporwave">
      <option value="darkwave">
      <!-- experimental / IDM -->
      <option value="IDM">
      <option value="glitch">
      <option value="glitch hop">
      <option value="experimental">
      <option value="ambient">
      <option value="drone">
      <!-- downtempo -->
      <option value="downtempo">
      <option value="lo-fi">
      <option value="trip hop">
      <!-- hip-hop / trap -->
      <option value="hip hop &amp; rap">
      <option value="hip hop">
      <option value="trap">
      <option value="drill">
      <option value="phonk">
      <!-- global / afrobeats -->
      <option value="afrobeats">
      <option value="dancehall">
      <option value="reggaeton">
      <option value="reggae">
      <option value="dub">
      <!-- soul / jazz -->
      <option value="r&amp;b &amp; soul">
      <option value="jazz &amp; blues">
      <option value="disco">
      <!-- rock / indie -->
      <option value="indie">
      <option value="alternative rock">
      <option value="folk &amp; singer-songwriter">
      <option value="rock">
      <option value="metal">
      <!-- pop / mainstream -->
      <option value="pop">
      <!-- classical / cinematic -->
      <option value="classical">
      <option value="piano">
      <option value="soundtrack">
      <!-- country / world -->
      <option value="country">
      <option value="latin">
      <option value="world">
    </datalist>
    <div class="filter-group">
      <div class="filter-label">BPM</div>
      <div class="filter-pair">
        <input id="f-bpm-from" type="number" placeholder="170" min="0" max="300">
        <input id="f-bpm-to"   type="number" placeholder="178" min="0" max="300">
      </div>
    </div>
    <div class="filter-group">
      <div class="filter-label">max plays</div>
      <input id="f-max-plays" type="number" placeholder="1000" min="0">
    </div>
    <div class="filter-group">
      <div class="filter-label">min likes</div>
      <input id="f-min-likes" type="number" placeholder="3" min="0">
    </div>
    <div class="filter-group">
      <div class="filter-label">max duration (min)</div>
      <input id="f-max-dur" type="number" placeholder="10" min="0">
    </div>
    <div class="filter-group">
      <div class="filter-label">sort</div>
      <select id="f-sort">
        <option value="relevance">relevance</option>
        <option value="created_at">newest first</option>
      </select>
    </div>
    <hr class="rule">
    <div class="filter-group">
      <div class="filter-label">target tracks</div>
      <input id="f-target" type="number" value="30" min="1" max="200">
    </div>
    <div class="filter-group">
      <div class="filter-label">max pages</div>
      <input id="f-pages" type="number" value="50" min="1" max="500">
    </div>
    <div class="filter-actions">
      <button class="btn-primary"    onclick="startSearch()">search</button>
      <button class="btn-secondary"  onclick="resetFilters()">reset</button>
    </div>
    <div id="recent-section" style="display:none">
      <div class="filter-label">recent</div>
      <div id="recent-queries"></div>
    </div>
    <div id="rejected-bar" style="display:none;margin-top:auto;padding-top:8px;border-top:1px solid var(--bg-2)">
      <a href="/history" id="rejected-link"
         style="font-size:11px;color:var(--fg-2)"></a>
    </div>
  </aside>

  <!-- RESULTS -->
  <section id="results">
    <div id="status-bar">
      <span id="status-text">enter filters and press search  ·  <kbd>?</kbd> for shortcuts</span>
      <span id="status-counts"></span>
    </div>
    <ul id="track-list"></ul>
  </section>

  <!-- NOW PLAYING -->
  <aside id="now-playing">
    <div class="np-title"  id="np-title">—</div>
    <div class="np-artist" id="np-artist"></div>
    <div class="np-art" id="np-art"></div>
    <div class="np-waveform"></div>
    <div class="np-scrubber-row">
      <span id="np-current">0:00</span>
      <input type="range" id="np-scrubber" min="0" max="100" value="0"
             oninput="scrub(this.value)">
      <span id="np-duration">—</span>
    </div>
    <div class="np-controls">
      <button onclick="prevTrack()" title="previous">⏮</button>
      <button onclick="togglePlay()" title="play / pause  [space]" id="np-playpause">▶</button>
      <button onclick="nextTrack()" title="next">⏭</button>
    </div>
    <div class="np-btn-group">
      <button class="np-btn" id="np-btn-queue"  onclick="actSelected('queued')"   title="queue [y]">✓ queue<span class="kbd">y</span></button>
      <button class="np-btn" id="np-btn-reject" onclick="actSelected('rejected')" title="reject [n]">✕ reject<span class="kbd">n</span></button>
      <button class="np-btn" id="np-btn-heart"  onclick="actSelected('hearted')"  title="heart [h]">♥ heart<span class="kbd">h</span></button>
    </div>
    <a class="np-sc-link" id="np-sc-link" href="#" target="_blank" rel="noopener">open on SoundCloud ↗</a>
  </aside>

</div><!-- .dig-layout -->

<!-- KEYBOARD HELP OVERLAY -->
<div id="help-overlay" class="hidden" onclick="hideHelp()">
  <div class="help-box" onclick="event.stopPropagation()">
    <h3>Keyboard shortcuts</h3>
    <div class="help-row"><span class="help-key">j / ↓</span><span>next track</span></div>
    <div class="help-row"><span class="help-key">k / ↑</span><span>previous track</span></div>
    <div class="help-row"><span class="help-key">space</span><span>play / pause</span></div>
    <div class="help-row"><span class="help-key">enter</span><span>play selected</span></div>
    <div class="help-row"><span class="help-key">y</span><span>queue selected</span></div>
    <div class="help-row"><span class="help-key">n</span><span>reject selected</span></div>
    <div class="help-row"><span class="help-key">h</span><span>heart selected</span></div>
    <div class="help-row"><span class="help-key">o</span><span>open on SoundCloud</span></div>
    <div class="help-row"><span class="help-key">/</span><span>focus search input</span></div>
    <div class="help-row"><span class="help-key">Esc</span><span>blur input / close help</span></div>
    <div class="help-close"><button onclick="hideHelp()">close</button></div>
  </div>
</div>

<audio id="audio" preload="none"></audio>

<script>
'use strict';

// ── State ─────────────────────────────────────────────────────────────────────
let ws            = null;
let sessionId     = null;
let tracks        = [];     // StoredTrack[]
let selectedIdx   = -1;
let nowPlayingIdx = -1;     // index in tracks[]
let pagesScanned  = 0;
let totalScanned  = 0;
let searching     = false;
let rejectedCount = 0;

// ── URL ↔ filter sync ─────────────────────────────────────────────────────────
function syncToUrl(f, target, pages) {
  const sp = new URLSearchParams();
  if (f.query)          sp.set('q',  f.query);
  if (f.genre_or_tag)   sp.set('g',  f.genre_or_tag);
  if (f.bpm_from)       sp.set('bf', f.bpm_from);
  if (f.bpm_to)         sp.set('bt', f.bpm_to);
  if (f.max_plays)      sp.set('mp', f.max_plays);
  if (f.min_likes)      sp.set('ml', f.min_likes);
  if (f.duration_to_ms) sp.set('md', f.duration_to_ms / 60_000);
  if (f.sort_by && f.sort_by !== 'relevance') sp.set('s', f.sort_by);
  if (target && target !== 30) sp.set('t', target);
  if (pages  && pages  !== 50) sp.set('p', pages);
  history.replaceState(null, '', sp.toString() ? `?${sp}` : location.pathname);
}

function loadFromUrl() {
  const sp = new URLSearchParams(location.search);
  const keys = ['q','g','bf','bt','mp','ml','md','s','t','p'];
  if (!keys.some(k => sp.has(k))) return false;
  if (sp.has('q'))  document.getElementById('f-query').value     = sp.get('q');
  if (sp.has('g'))  document.getElementById('f-genre').value     = sp.get('g');
  if (sp.has('bf')) document.getElementById('f-bpm-from').value  = sp.get('bf');
  if (sp.has('bt')) document.getElementById('f-bpm-to').value    = sp.get('bt');
  if (sp.has('mp')) document.getElementById('f-max-plays').value = sp.get('mp');
  if (sp.has('ml')) document.getElementById('f-min-likes').value = sp.get('ml');
  if (sp.has('md')) document.getElementById('f-max-dur').value   = sp.get('md');
  if (sp.has('s'))  document.getElementById('f-sort').value      = sp.get('s');
  if (sp.has('t'))  document.getElementById('f-target').value    = sp.get('t');
  if (sp.has('p'))  document.getElementById('f-pages').value     = sp.get('p');
  return true;
}

// ── Recent queries ─────────────────────────────────────────────────────────────
const RECENT_KEY = 'crater_recent_queries';
const RECENT_MAX = 10;

function queryLabel(f) {
  const parts = [
    f.query          ? `"${f.query}"` : null,
    f.genre_or_tag   || null,
    (f.bpm_from || f.bpm_to) ? `${f.bpm_from||'?'}–${f.bpm_to||'?'}bpm` : null,
    f.max_plays      ? `≤${f.max_plays} plays` : null,
    f.min_likes      ? `≥${f.min_likes} likes` : null,
    f.duration_to_ms ? `≤${Math.round(f.duration_to_ms/60000)}min` : null,
    f.sort_by === 'created_at' ? 'newest' : null,
  ].filter(Boolean);
  return parts.length ? parts.join(' · ') : 'no filters';
}

function saveRecentQuery(f, target, pages) {
  const label = queryLabel(f);
  let recent = getRecentQueries();
  recent = recent.filter(r => r.label !== label);
  recent.unshift({label, f, target, pages});
  if (recent.length > RECENT_MAX) recent.length = RECENT_MAX;
  try { localStorage.setItem(RECENT_KEY, JSON.stringify(recent)); } catch {}
  renderRecentQueries(recent);
}

function getRecentQueries() {
  try { return JSON.parse(localStorage.getItem(RECENT_KEY) || '[]'); } catch { return []; }
}

function renderRecentQueries(recent) {
  recent = recent || getRecentQueries();
  const section = document.getElementById('recent-section');
  const list    = document.getElementById('recent-queries');
  if (!recent.length) { section.style.display = 'none'; return; }
  section.style.display = '';
  list.innerHTML = recent.map((r, i) =>
    `<button class="recent-query-item" onclick="applyQuery(${i})" title="${esc(r.label)}">${esc(r.label)}</button>`
  ).join('');
}

function applyQuery(idx) {
  const recent = getRecentQueries();
  if (idx >= recent.length) return;
  const {f, target, pages} = recent[idx];
  document.getElementById('f-query').value     = f.query         || '';
  document.getElementById('f-genre').value     = f.genre_or_tag  || '';
  document.getElementById('f-bpm-from').value  = f.bpm_from      || '';
  document.getElementById('f-bpm-to').value    = f.bpm_to        || '';
  document.getElementById('f-max-plays').value = f.max_plays      || '';
  document.getElementById('f-min-likes').value = f.min_likes     || '';
  document.getElementById('f-max-dur').value   = f.duration_to_ms ? f.duration_to_ms / 60_000 : '';
  document.getElementById('f-sort').value      = f.sort_by       || 'relevance';
  document.getElementById('f-target').value    = target || 30;
  document.getElementById('f-pages').value     = pages  || 50;
  startSearch();
}

// ── WebSocket ──────────────────────────────────────────────────────────────────
function ensureWS() {
  if (ws && ws.readyState < 2) return;
  ws = new WebSocket(`ws://${location.host}/ws`);
  ws.onopen    = () => console.debug('WS open');
  ws.onclose   = () => console.debug('WS closed');
  ws.onerror   = e  => console.warn('WS error', e);
  ws.onmessage = ({data}) => {
    let msg;
    try { msg = JSON.parse(data); } catch { return; }
    if (msg.session_id && msg.session_id !== sessionId) return;
    if (msg.type === 'search.track')    onTrack(msg);
    if (msg.type === 'search.complete') onComplete(msg);
    if (msg.type === 'search.error')    onSearchError(msg);
  };
}

// ── Search ────────────────────────────────────────────────────────────────────
async function startSearch() {
  ensureWS();
  tracks = [];
  selectedIdx = -1;
  nowPlayingIdx = -1;
  pagesScanned = 0;
  totalScanned = 0;
  searching = true;

  document.getElementById('track-list').innerHTML = '';
  document.getElementById('status-text').textContent   = 'searching…';
  document.getElementById('status-counts').textContent = '';
  document.getElementById('layout').classList.remove('has-playing');
  resetAudio();

  const f = {};
  const q  = val('f-query');
  const g  = val('f-genre');
  const bf = num('f-bpm-from');
  const bt = num('f-bpm-to');
  const mp = num('f-max-plays');
  const ml = num('f-min-likes');
  const md = num('f-max-dur');
  const so = document.getElementById('f-sort').value;
  if (q)  f.query          = q;
  if (g)  f.genre_or_tag   = g;
  if (bf) f.bpm_from       = bf;
  if (bt) f.bpm_to         = bt;
  if (mp) f.max_plays      = mp;
  if (ml) f.min_likes      = ml;
  if (md) f.duration_to_ms = md * 60_000;
  if (so && so !== 'relevance') f.sort_by = so;

  const target = num('f-target') || 30;
  const pages  = num('f-pages')  || 50;

  syncToUrl(f, target, pages);
  saveRecentQuery(f, target, pages);

  let resp;
  try {
    resp = await fetch('/api/search', {
      method: 'POST',
      headers: {'Content-Type': 'application/json'},
      body: JSON.stringify({filters: f, target_size: target, max_pages: pages}),
    });
  } catch(e) {
    setStatus('network error: ' + e.message);
    searching = false;
    return;
  }
  if (!resp.ok) { setStatus('server error ' + resp.status); searching = false; return; }

  const { session_id } = await resp.json();
  sessionId = session_id;

  const sub = () => ws.send(JSON.stringify({type:'subscribe', channel:'search', session_id}));
  if (ws.readyState === 1) sub();
  else ws.addEventListener('open', sub, {once: true});
}

function onTrack({track, total_scanned, pages_scanned}) {
  pagesScanned = pages_scanned;
  totalScanned = total_scanned;
  // Skip tracks already processed in a previous session
  if (track.status && ['hearted','queued','rejected','exported'].includes(track.status)) return;
  const idx = tracks.length;
  tracks.push(track);
  document.getElementById('track-list').insertAdjacentHTML('beforeend', cardHtml(track, idx));
  document.getElementById('status-counts').textContent =
    `${tracks.length} shown · ${totalScanned} scanned · ${pagesScanned} pages`;
}

function onComplete({exhausted, total_accepted, total_scanned, pages_scanned}) {
  searching = false;
  // Update final counts — only reach the client via Track events when tracks
  // are emitted; pick up the definitive numbers from Complete instead.
  if (total_scanned != null) totalScanned = total_scanned;
  if (pages_scanned != null) pagesScanned = pages_scanned;

  if (total_accepted === 0) {
    if (pagesScanned === 0) {
      setStatus('0 tracks — no pages fetched (check genre / tag spelling)');
    } else if (totalScanned === 0) {
      setStatus(`0 tracks · ${pagesScanned} pages scanned · nothing passed filters — try loosening max plays or BPM range`);
    } else {
      setStatus(`0 tracks shown · ${totalScanned} filtered · ${pagesScanned} pages`);
    }
  } else {
    const more = exhausted ? 'exhausted' : 'more available';
    setStatus(`${total_accepted} tracks · ${more}`);
  }
  refreshQueueCount();
}

function onSearchError({message}) {
  searching = false;
  setStatus('search error: ' + message);
}

// ── Track card rendering ──────────────────────────────────────────────────────
function cardHtml(t, idx) {
  const playing = idx === nowPlayingIdx;
  const ind = playing                   ? '▶'
            : t.status === 'hearted'    ? '★'
            : t.status === 'queued'     ? '✓'
            : t.status === 'rejected'   ? '✕'
            : '';
  const indCls = playing                  ? 'ind ind-playing'
               : t.status === 'hearted'   ? 'ind ind-hearted'
               : t.status === 'queued'    ? 'ind ind-queued'
               : t.status === 'rejected'  ? 'ind ind-rejected'
               : 'ind';
  const statusCls = t.status ? ` status-${t.status}` : '';

  const ratio = (t.likes_count > 0 && t.playback_count > 0)
    ? (t.likes_count / t.playback_count).toFixed(3) : null;
  const bpm = t.bpm ? `${Math.round(t.bpm)}bpm` : null;
  const dur = fmtDur(t.duration_ms);
  const meta1 = [
    t.playback_count != null ? `${t.playback_count} ▶` : null,
    t.likes_count    != null ? `${t.likes_count} ♥`    : null,
    ratio ? `ratio ${ratio}` : null,
    bpm   ? `<span style="color:var(--bpm)">${bpm}</span>` : null,
  ].filter(Boolean).join(' · ');
  const meta2 = [
    dur,
    t.genre,
    humanAge(t.first_seen),
  ].filter(Boolean).join(' · ');

  const aUrl = artistUrl(t.permalink_url);

  const artHtml = t.artwork_url
    ? `<img src="${esc(t.artwork_url)}" alt="" loading="lazy">`
    : `<div class="track-art-placeholder">♪</div>`;

  return `<li class="track${playing ? ' playing' : ''}${statusCls}" data-id="${t.id}" data-idx="${idx}"
     onclick="clickTrack(event, ${idx})">
  <span class="${indCls}" id="ind-${t.id}">${ind}</span>
  <div class="track-art">${artHtml}</div>
  <div class="track-body">
    <div class="track-line1">
      <button class="track-title" onclick="event.stopPropagation();playIdx(${idx})">${esc(t.title||'untitled')}</button>
      <span class="track-sep"> — </span>
      <a class="track-artist" href="${esc(aUrl)}" target="_blank" rel="noopener"
         onclick="event.stopPropagation()">${esc(t.artist||'unknown')}</a>
    </div>
    <div class="track-meta">${meta1}</div>
    <div class="track-meta2">${meta2}</div>
  </div>
  <div class="track-actions" onclick="event.stopPropagation()">
    <button class="act-queue"  onclick="act(${t.id},'queued')"   title="queue [y]">✓</button>
    <button class="act-heart"  onclick="act(${t.id},'hearted')"  title="heart [h]">♥</button>
    <button class="act-reject" onclick="act(${t.id},'rejected')" title="reject [n]">✕</button>
    <a href="${esc(t.permalink_url||'#')}" target="_blank" rel="noopener" title="open on SoundCloud [o]">↗</a>
  </div>
</li>`;
}

function refreshCard(idx) {
  if (idx < 0 || idx >= tracks.length) return;
  const el = document.querySelector(`[data-idx="${idx}"]`);
  if (!el) return;
  el.outerHTML = cardHtml(tracks[idx], idx);
  if (idx === selectedIdx) {
    const fresh = document.querySelector(`[data-idx="${idx}"]`);
    if (fresh) fresh.classList.add('selected');
  }
}

// ── Selection and playback ────────────────────────────────────────────────────
function clickTrack(event, idx) {
  setSelected(idx);
}

function setSelected(idx) {
  if (idx < 0) idx = 0;
  if (idx >= tracks.length) idx = tracks.length - 1;
  if (idx < 0) return;
  document.querySelectorAll('.track.selected').forEach(el => el.classList.remove('selected'));
  selectedIdx = idx;
  const el = document.querySelector(`[data-idx="${idx}"]`);
  if (el) {
    el.classList.add('selected');
    el.scrollIntoView({block: 'nearest'});
  }
}

function playIdx(idx) {
  if (idx < 0 || idx >= tracks.length) return;
  const prev = nowPlayingIdx;
  nowPlayingIdx = idx;
  if (prev >= 0) refreshCard(prev);
  refreshCard(idx);
  setSelected(idx);
  const t = tracks[idx];
  updateNowPlayingPanel(t);
  document.getElementById('layout').classList.add('has-playing');

  const audio = document.getElementById('audio');
  audio.src = `/api/stream/${t.id}`;
  audio.play().catch(e => console.warn('audio stub (stream not yet implemented):', e));
  document.getElementById('np-playpause').textContent = '⏸';
}

function prevTrack() { if (nowPlayingIdx > 0) playIdx(nowPlayingIdx - 1); }
function nextTrack() { if (nowPlayingIdx < tracks.length - 1) playIdx(nowPlayingIdx + 1); }

function togglePlay() {
  if (nowPlayingIdx < 0) {
    if (selectedIdx >= 0) playIdx(selectedIdx);
    return;
  }
  const audio = document.getElementById('audio');
  const btn   = document.getElementById('np-playpause');
  if (audio.paused) { audio.play().catch(() => {}); btn.textContent = '⏸'; }
  else              { audio.pause();                 btn.textContent = '▶'; }
}

function resetAudio() {
  const audio = document.getElementById('audio');
  audio.pause();
  audio.src = '';
  document.getElementById('np-playpause').textContent = '▶';
  document.getElementById('np-current').textContent   = '0:00';
  document.getElementById('np-scrubber').value        = 0;
}

function scrub(pct) {
  const audio = document.getElementById('audio');
  if (audio.duration) audio.currentTime = audio.duration * pct / 100;
}

function updateNowPlayingPanel(t) {
  document.getElementById('np-title').textContent  = t.title  || 'untitled';
  document.getElementById('np-artist').textContent = t.artist || 'unknown';
  document.getElementById('np-duration').textContent = fmtDur(t.duration_ms);
  const link = document.getElementById('np-sc-link');
  link.href = t.permalink_url || '#';
  const artEl = document.getElementById('np-art');
  if (t.artwork_url) {
    const large = t.artwork_url.replace(/-large\./, '-t500x500.');
    artEl.innerHTML = `<img src="${esc(large)}" alt="">`;
  } else {
    artEl.innerHTML = '';
  }
  updateNowPlayingButtons(t.status);
}

function updateNowPlayingButtons(status) {
  const btns = {
    queued:   document.getElementById('np-btn-queue'),
    rejected: document.getElementById('np-btn-reject'),
    hearted:  document.getElementById('np-btn-heart'),
  };
  const cls = {queued: 'np-active-queue', rejected: 'np-active-reject', hearted: 'np-active-heart'};
  for (const [s, el] of Object.entries(btns)) {
    el.classList.toggle(cls[s], s === status);
  }
}

// ── Status actions ────────────────────────────────────────────────────────────
async function act(id, status) {
  await fetch(`/api/tracks/${id}/status`, {
    method: 'POST',
    headers: {'Content-Type': 'application/json'},
    body: JSON.stringify({status}),
  });
  const idx = tracks.findIndex(t => t.id === id);
  if (idx >= 0) {
    tracks[idx] = {...tracks[idx], status};
    refreshCard(idx);
    if (idx === nowPlayingIdx) updateNowPlayingButtons(status);
    if (status === 'rejected') {
      rejectedCount++;
      updateRejectedBar();
      if (idx === selectedIdx && idx + 1 < tracks.length) setSelected(idx + 1);
    }
  }
  refreshQueueCount();
  refreshHeartedCount();
}

function actSelected(status) {
  const idx = nowPlayingIdx >= 0 ? nowPlayingIdx : selectedIdx;
  if (idx >= 0) act(tracks[idx].id, status);
}

// ── Nav badge counts ──────────────────────────────────────────────────────────
async function refreshQueueCount() {
  try {
    const r = await fetch('/api/tracks?status=queued');
    const arr = await r.json();
    document.getElementById('queue-count').textContent = Array.isArray(arr) ? arr.length : 0;
  } catch {}
}

async function refreshHeartedCount() {
  try {
    const r = await fetch('/api/tracks?status=hearted');
    const arr = await r.json();
    document.getElementById('hearted-count').textContent = Array.isArray(arr) ? arr.length : 0;
  } catch {}
}

async function refreshRejectedCount() {
  try {
    const r = await fetch('/api/tracks?status=rejected');
    const arr = await r.json();
    rejectedCount = Array.isArray(arr) ? arr.length : 0;
    updateRejectedBar();
  } catch {}
}

function updateRejectedBar() {
  const bar  = document.getElementById('rejected-bar');
  const link = document.getElementById('rejected-link');
  if (rejectedCount === 0) { bar.style.display = 'none'; return; }
  bar.style.display = '';
  link.textContent = `[x] ${rejectedCount} rejected — view`;
}

// ── Keyboard shortcuts ────────────────────────────────────────────────────────
document.addEventListener('keydown', e => {
  const tag = e.target.tagName;
  if (['INPUT','TEXTAREA','SELECT'].includes(tag)) {
    if (e.key === 'Escape') { e.target.blur(); e.preventDefault(); }
    return;
  }
  if (!document.getElementById('help-overlay').classList.contains('hidden')) {
    if (e.key === 'Escape' || e.key === '?') { hideHelp(); e.preventDefault(); }
    return;
  }
  switch (e.key) {
    case 'j': case 'ArrowDown':  e.preventDefault(); setSelected(selectedIdx + 1); break;
    case 'k': case 'ArrowUp':    e.preventDefault(); setSelected(selectedIdx - 1); break;
    case ' ':                    e.preventDefault(); togglePlay(); break;
    case 'Enter':                if (selectedIdx >= 0) playIdx(selectedIdx); break;
    case 'y': actSelected('queued');   break;
    case 'n': actSelected('rejected'); break;
    case 'h': actSelected('hearted');  break;
    case 'o':
      const idx = nowPlayingIdx >= 0 ? nowPlayingIdx : selectedIdx;
      if (idx >= 0 && tracks[idx]?.permalink_url)
        window.open(tracks[idx].permalink_url, '_blank', 'noopener');
      break;
    case '/': e.preventDefault(); document.getElementById('f-query').focus(); break;
    case '?': showHelp(); break;
    case 'Escape': hideHelp(); break;
  }
});

function showHelp() { document.getElementById('help-overlay').classList.remove('hidden'); }
function hideHelp() { document.getElementById('help-overlay').classList.add('hidden');    }

// ── Audio events ──────────────────────────────────────────────────────────────
(function() {
  const audio = document.getElementById('audio');
  audio.addEventListener('timeupdate', () => {
    if (!audio.duration) return;
    document.getElementById('np-scrubber').value =
      (audio.currentTime / audio.duration * 100).toFixed(1);
    document.getElementById('np-current').textContent = fmtDur(audio.currentTime * 1000);
  });
  audio.addEventListener('ended', () => {
    document.getElementById('np-playpause').textContent = '▶';
    // Stop at end (v1 — per design doc "decide, don't just listen")
  });
})();

// ── Utilities ─────────────────────────────────────────────────────────────────
function val(id) { return document.getElementById(id)?.value.trim() || ''; }
function num(id) { const v = parseFloat(document.getElementById(id)?.value); return isNaN(v) ? 0 : v; }

function esc(s) {
  const d = document.createElement('div');
  d.textContent = typeof s === 'string' ? s : String(s ?? '');
  return d.innerHTML;
}

function fmtDur(ms) {
  if (!ms && ms !== 0) return '';
  const s = Math.floor(ms / 1000);
  return `${Math.floor(s / 60)}:${String(s % 60).padStart(2, '0')}`;
}

function humanAge(dateStr) {
  if (!dateStr) return '';
  const diff = Date.now() - new Date(dateStr).getTime();
  const days = Math.floor(diff / 86_400_000);
  if (days < 1)   return 'today';
  if (days < 7)   return `${days}d ago`;
  if (days < 30)  return `${Math.floor(days / 7)}w ago`;
  if (days < 365) return `${Math.floor(days / 30)}mo ago`;
  return `${Math.floor(days / 365)}y ago`;
}

function artistUrl(permalinkUrl) {
  if (!permalinkUrl) return '#';
  try {
    const u = new URL(permalinkUrl);
    const parts = u.pathname.split('/').filter(Boolean);
    return parts.length >= 1 ? `${u.origin}/${parts[0]}` : '#';
  } catch { return '#'; }
}

function setStatus(msg) {
  document.getElementById('status-text').textContent = msg;
}

function resetFilters() {
  ['f-query','f-genre','f-bpm-from','f-bpm-to','f-max-plays','f-min-likes','f-max-dur'].forEach(
    id => { document.getElementById(id).value = ''; }
  );
  document.getElementById('f-sort').value   = 'relevance';
  document.getElementById('f-target').value = 30;
  document.getElementById('f-pages').value  = 50;
  history.replaceState(null, '', location.pathname);
}

// ── Init ──────────────────────────────────────────────────────────────────────
refreshQueueCount();
refreshHeartedCount();
refreshRejectedCount();
renderRecentQueries();
if (loadFromUrl()) startSearch();
</script>
</body>
</html>"##;

// ── /queue ────────────────────────────────────────────────────────────────────

const QUEUE: &str = r##"<!DOCTYPE html>
<html lang="en">
<head>
<meta charset="utf-8">
<meta name="viewport" content="width=device-width, initial-scale=1">
<title>crater / queue</title>
<style>/* CSS_PLACEHOLDER */</style>
</head>
<body>
<header>
  <span class="logo">crater</span>
  <nav>
    <a href="/dig">dig</a>
    <a href="/hearted">hearted</a>
    <a href="/queue" class="active">queue <span class="nav-badge" id="queue-count">0</span></a>
    <a href="/digests">digests</a>
  </nav>
  <a href="/settings" class="nav-cog" title="settings">⚙</a>
</header>

<div class="page-layout">
  <div class="page-header">
    <span class="page-title" id="queue-header">QUEUE</span>
    <div style="display:flex;gap:8px;align-items:center">
      <span style="font-size:12px;color:var(--fg-2)" id="queue-duration"></span>
    </div>
  </div>

  <div style="background:var(--bg-1);border:1px solid var(--bg-2);border-radius:4px;padding:14px 16px;margin-bottom:16px">
    <div style="font-size:11px;font-weight:600;text-transform:uppercase;letter-spacing:.08em;color:var(--fg-2);margin-bottom:10px">Export to SoundCloud</div>
    <div style="display:grid;grid-template-columns:1fr auto;gap:8px;align-items:center">
      <input id="export-name" type="text" placeholder="crater dig — 2026-04-19">
      <label style="font-size:12px;color:var(--fg-1);display:flex;gap:6px;align-items:center;cursor:pointer">
        <input type="checkbox" id="export-public" style="width:auto">public
      </label>
    </div>
    <div style="margin-top:8px">
      <button class="btn-primary" style="width:auto;padding:7px 16px" onclick="exportQueue()">export to SoundCloud</button>
      <span id="export-status" style="font-size:12px;color:var(--fg-2);margin-left:10px"></span>
    </div>
  </div>

  <div id="queue-list"></div>
</div>

<script>
'use strict';

let queuedTracks = [];

async function loadQueue() {
  const r = await fetch('/api/tracks?status=queued');
  queuedTracks = await r.json();
  renderQueue();
}

function renderQueue() {
  const list = document.getElementById('queue-list');
  document.getElementById('queue-count').textContent = queuedTracks.length;

  if (queuedTracks.length === 0) {
    list.innerHTML = '<div class="empty-state">No tracks queued. Add tracks from the dig page.</div>';
    document.getElementById('queue-header').textContent = 'QUEUE — empty';
    document.getElementById('queue-duration').textContent = '';
    document.getElementById('export-name').value = defaultPlaylistName();
    return;
  }

  const totalMs = queuedTracks.reduce((s, t) => s + (t.duration_ms || 0), 0);
  const h = Math.floor(totalMs / 3_600_000);
  const m = Math.floor((totalMs % 3_600_000) / 60_000);
  document.getElementById('queue-header').textContent =
    `QUEUE — ${queuedTracks.length} track${queuedTracks.length === 1 ? '' : 's'}`;
  document.getElementById('queue-duration').textContent =
    h > 0 ? `${h}h ${m}m total` : `${m}m total`;
  document.getElementById('export-name').value = defaultPlaylistName();

  list.innerHTML = queuedTracks.map((t, i) => `
    <div class="card" data-id="${t.id}">
      <div style="display:grid;grid-template-columns:1fr auto;gap:8px;align-items:start">
        <div>
          <div class="card-title">${esc(t.title||'untitled')} <span style="color:var(--fg-2);font-weight:400"> — ${esc(t.artist||'unknown')}</span></div>
          <div class="card-meta">${fmtDur(t.duration_ms)} · ${t.playback_count??'?'} plays · ${t.likes_count??'?'} likes${t.bpm ? ' · ' + Math.round(t.bpm) + 'bpm' : ''}</div>
        </div>
        <div style="display:flex;gap:6px;align-items:center">
          <a href="${esc(t.permalink_url||'#')}" target="_blank" rel="noopener"
             style="font-size:12px;color:var(--fg-2)">SC ↗</a>
          <button onclick="removeFromQueue(${t.id})"
                  style="font-size:12px;color:var(--danger);padding:3px 7px;border:1px solid var(--bg-2);border-radius:3px">remove</button>
        </div>
      </div>
    </div>`).join('');
}

async function removeFromQueue(id) {
  await fetch(`/api/tracks/${id}/status`, {
    method: 'POST',
    headers: {'Content-Type': 'application/json'},
    body: JSON.stringify({status: null}),
  });
  queuedTracks = queuedTracks.filter(t => t.id !== id);
  renderQueue();
}

async function exportQueue() {
  const name       = document.getElementById('export-name').value.trim() || defaultPlaylistName();
  const visibility = document.getElementById('export-public').checked ? 'public' : 'private';
  const status     = document.getElementById('export-status');
  status.textContent = 'exporting…';
  try {
    const r = await fetch('/api/playlists/export', {
      method: 'POST',
      headers: {'Content-Type': 'application/json'},
      body: JSON.stringify({name, visibility, track_ids: queuedTracks.map(t => t.id)}),
    });
    if (r.ok) {
      const { playlist_url, track_count } = await r.json();
      status.textContent = `exported ${track_count} tracks`;
      status.style.color = 'var(--queued)';
      if (playlist_url) {
        status.innerHTML += ` · <a href="${esc(playlist_url)}" target="_blank" style="color:var(--accent)">view ↗</a>`;
      }
      loadQueue();
    } else {
      const err = await r.json().catch(() => ({}));
      status.textContent = 'error: ' + (err.message || r.status);
      status.style.color = 'var(--danger)';
    }
  } catch(e) {
    status.textContent = 'network error: ' + e.message;
    status.style.color = 'var(--danger)';
  }
}

function defaultPlaylistName() {
  const d = new Date();
  return `crater dig — ${d.getFullYear()}-${String(d.getMonth()+1).padStart(2,'0')}-${String(d.getDate()).padStart(2,'0')}`;
}

function fmtDur(ms) {
  if (!ms && ms !== 0) return '';
  const s = Math.floor(ms / 1000);
  return `${Math.floor(s / 60)}:${String(s % 60).padStart(2,'0')}`;
}

function esc(s) {
  const d = document.createElement('div');
  d.textContent = typeof s === 'string' ? s : String(s ?? '');
  return d.innerHTML;
}

loadQueue();
</script>
</body>
</html>"##;

// ── /hearted ──────────────────────────────────────────────────────────────────

const HEARTED: &str = r##"<!DOCTYPE html>
<html lang="en">
<head>
<meta charset="utf-8">
<meta name="viewport" content="width=device-width, initial-scale=1">
<title>crater / hearted</title>
<style>/* CSS_PLACEHOLDER */</style>
</head>
<body>
<header>
  <span class="logo">crater</span>
  <nav>
    <a href="/dig">dig</a>
    <a href="/hearted" class="active">hearted <span class="nav-badge" id="hearted-count">0</span></a>
    <a href="/queue">queue</a>
    <a href="/digests">digests</a>
  </nav>
  <a href="/settings" class="nav-cog" title="settings">⚙</a>
</header>

<div class="page-layout">
  <div class="page-header">
    <span class="page-title" id="hearted-header">HEARTED</span>
    <span style="font-size:12px;color:var(--fg-2)" id="hearted-duration"></span>
  </div>

  <div id="hearted-list"></div>
</div>

<script>
'use strict';

let heartedTracks = [];

async function load() {
  const r = await fetch('/api/tracks?status=hearted');
  heartedTracks = await r.json();
  if (!Array.isArray(heartedTracks)) heartedTracks = [];
  render();
}

function render() {
  const list = document.getElementById('hearted-list');
  document.getElementById('hearted-count').textContent = heartedTracks.length;

  if (heartedTracks.length === 0) {
    list.innerHTML = '<div class="empty-state">No hearted tracks. Heart tracks from the dig page using [h] or the ♥ button.</div>';
    document.getElementById('hearted-header').textContent = 'HEARTED — empty';
    document.getElementById('hearted-duration').textContent = '';
    return;
  }

  const totalMs = heartedTracks.reduce((s, t) => s + (t.duration_ms || 0), 0);
  const h = Math.floor(totalMs / 3_600_000);
  const m = Math.floor((totalMs % 3_600_000) / 60_000);
  document.getElementById('hearted-header').textContent =
    `HEARTED — ${heartedTracks.length} track${heartedTracks.length === 1 ? '' : 's'}`;
  document.getElementById('hearted-duration').textContent =
    h > 0 ? `${h}h ${m}m total` : `${m}m total`;

  list.innerHTML = heartedTracks.map(t => {
    const dur = fmtDur(t.duration_ms);
    const meta = [
      t.playback_count != null ? `${t.playback_count} plays` : null,
      t.likes_count    != null ? `${t.likes_count} likes`    : null,
      t.bpm ? `${Math.round(t.bpm)}bpm` : null,
      dur,
      t.genre,
    ].filter(Boolean).join(' · ');

    return `<div class="card" data-id="${t.id}">
      <div style="display:grid;grid-template-columns:1fr auto;gap:8px;align-items:start">
        <div>
          <div class="card-title">
            <a href="${esc(t.permalink_url||'#')}" target="_blank" rel="noopener"
               style="color:inherit">${esc(t.title||'untitled')}</a>
            <span style="color:var(--fg-2);font-weight:400"> — ${esc(t.artist||'unknown')}</span>
          </div>
          <div class="card-meta">${meta}</div>
        </div>
        <div style="display:flex;gap:6px;align-items:center">
          <a href="${esc(t.permalink_url||'#')}" target="_blank" rel="noopener"
             style="font-size:12px;color:var(--fg-2)">SC ↗</a>
          <button onclick="promoteToQueue(${t.id})"
                  style="font-size:12px;color:var(--queued);padding:3px 7px;border:1px solid var(--bg-2);border-radius:3px">
            queue it
          </button>
          <button onclick="unheart(${t.id})"
                  style="font-size:12px;color:var(--fg-2);padding:3px 7px;border:1px solid var(--bg-2);border-radius:3px">
            un-heart
          </button>
        </div>
      </div>
    </div>`;
  }).join('');
}

async function promoteToQueue(id) {
  await fetch(`/api/tracks/${id}/status`, {
    method: 'POST',
    headers: {'Content-Type': 'application/json'},
    body: JSON.stringify({status: 'queued'}),
  });
  heartedTracks = heartedTracks.filter(t => t.id !== id);
  render();
}

async function unheart(id) {
  await fetch(`/api/tracks/${id}/status`, {
    method: 'POST',
    headers: {'Content-Type': 'application/json'},
    body: JSON.stringify({status: null}),
  });
  heartedTracks = heartedTracks.filter(t => t.id !== id);
  render();
}

function fmtDur(ms) {
  if (!ms && ms !== 0) return '';
  const s = Math.floor(ms / 1000);
  return `${Math.floor(s / 60)}:${String(s % 60).padStart(2,'0')}`;
}

function esc(s) {
  const d = document.createElement('div');
  d.textContent = typeof s === 'string' ? s : String(s ?? '');
  return d.innerHTML;
}

load();
</script>
</body>
</html>"##;

// ── /digests ──────────────────────────────────────────────────────────────────

const DIGESTS: &str = r##"<!DOCTYPE html>
<html lang="en">
<head>
<meta charset="utf-8">
<meta name="viewport" content="width=device-width, initial-scale=1">
<title>crater / digests</title>
<style>/* CSS_PLACEHOLDER */</style>
</head>
<body>
<header>
  <span class="logo">crater</span>
  <nav>
    <a href="/dig">dig</a>
    <a href="/hearted">hearted</a>
    <a href="/queue">queue</a>
    <a href="/digests" class="active">digests</a>
  </nav>
  <a href="/settings" class="nav-cog" title="settings">⚙</a>
</header>

<div class="page-layout">
  <div class="page-header">
    <span class="page-title">DIGESTS</span>
    <a href="/dig" style="font-size:12px;color:var(--accent)">+ create from current search</a>
  </div>

  <div id="digest-list"></div>
</div>

<script>
'use strict';

async function loadDigests() {
  const r = await fetch('/api/digests');
  const digests = await r.json();
  renderDigests(Array.isArray(digests) ? digests : (digests.digests || []));
}

function renderDigests(digests) {
  const list = document.getElementById('digest-list');
  if (digests.length === 0) {
    list.innerHTML = '<div class="empty-state">No digests. Create one from the dig page after finding a search that works.</div>';
    return;
  }
  list.innerHTML = digests.map(d => {
    const spec    = d.spec || {};
    const filters = spec.filters || {};
    const parts   = [
      filters.query          ? `"${esc(filters.query)}"` : null,
      filters.genre_or_tag   ? esc(filters.genre_or_tag) : null,
      filters.bpm_from || filters.bpm_to
        ? `${filters.bpm_from||'?'}–${filters.bpm_to||'?'}bpm` : null,
      filters.max_plays ? `≤${filters.max_plays} plays` : null,
    ].filter(Boolean).join(' · ');
    const nextRun = d.next_run_at ? new Date(d.next_run_at).toLocaleString() : '—';
    const lastRun = d.last_run_at ? new Date(d.last_run_at).toLocaleString() : 'never';
    const enabled = d.enabled !== false;

    return `<div class="card" data-id="${d.id}">
      <div style="display:grid;grid-template-columns:1fr auto;gap:8px;align-items:start">
        <div>
          <div class="card-title">${esc(spec.name||'unnamed')}</div>
          <div class="card-meta" style="margin-top:5px">${parts || 'no filters'}</div>
          <div class="card-meta">${esc(spec.cron_expr||'')} · target ${spec.target_size||25} tracks</div>
          <div class="card-meta">last run: ${lastRun} · next: ${nextRun}</div>
        </div>
        <div style="display:flex;gap:8px;align-items:center">
          <label style="font-size:12px;color:var(--fg-1);display:flex;gap:5px;align-items:center;cursor:pointer">
            <input type="checkbox" ${enabled ? 'checked' : ''} onchange="toggleDigest(${d.id}, this.checked)"
                   style="width:auto">enabled
          </label>
          <button onclick="runDigest(${d.id})"
                  style="font-size:12px;color:var(--queued);padding:3px 8px;border:1px solid var(--bg-2);border-radius:3px">
            run now
          </button>
          <button onclick="deleteDigest(${d.id})"
                  style="font-size:12px;color:var(--danger);padding:3px 8px;border:1px solid var(--bg-2);border-radius:3px">
            delete
          </button>
        </div>
      </div>
    </div>`;
  }).join('');
}

async function toggleDigest(id, enabled) {
  await fetch(`/api/digests/${id}`, {
    method: 'PATCH',
    headers: {'Content-Type': 'application/json'},
    body: JSON.stringify({enabled}),
  });
}

async function runDigest(id) {
  const r = await fetch(`/api/digests/${id}/run`, {method: 'POST'});
  if (r.ok) alert('Digest run started. Check back later for results.');
  else alert('Failed to start digest run.');
}

async function deleteDigest(id) {
  if (!confirm('Delete this digest?')) return;
  await fetch(`/api/digests/${id}`, {method: 'DELETE'});
  loadDigests();
}

function esc(s) {
  const d = document.createElement('div');
  d.textContent = typeof s === 'string' ? s : String(s ?? '');
  return d.innerHTML;
}

loadDigests();
</script>
</body>
</html>"##;

// ── /digests/:id ──────────────────────────────────────────────────────────────

const DIGEST_DETAIL: &str = r##"<!DOCTYPE html>
<html lang="en">
<head>
<meta charset="utf-8">
<meta name="viewport" content="width=device-width, initial-scale=1">
<title>crater / digest</title>
<style>/* CSS_PLACEHOLDER */</style>
</head>
<body>
<header>
  <span class="logo">crater</span>
  <nav>
    <a href="/dig">dig</a>
    <a href="/hearted">hearted</a>
    <a href="/queue">queue</a>
    <a href="/digests" class="active">digests</a>
  </nav>
  <a href="/settings" class="nav-cog" title="settings">⚙</a>
</header>

<div class="page-layout">
  <div class="page-header">
    <span class="page-title" id="digest-name">loading…</span>
    <div style="display:flex;gap:8px">
      <button class="btn-secondary" onclick="runNow()" style="padding:5px 12px;font-size:12px">run now</button>
      <button class="btn-primary"   onclick="saveSpec()" style="padding:5px 12px;font-size:12px">save</button>
    </div>
  </div>

  <div id="digest-detail"></div>

  <div style="margin-top:24px">
    <div class="page-title" style="margin-bottom:12px">RUN HISTORY</div>
    <div id="run-history"></div>
  </div>
</div>

<datalist id="genre-suggestions">
  <!-- drum & bass family -->
  <option value="drum &amp; bass">
  <option value="liquid drum &amp; bass">
  <option value="neurofunk">
  <option value="darkstep">
  <option value="techstep">
  <option value="jump up">
  <option value="jungle">
  <!-- bass / experimental / left field -->
  <option value="bass">
  <option value="bass music">
  <option value="experimental bass">
  <option value="left field">
  <option value="halftime">
  <!-- dubstep / future bass -->
  <option value="dubstep">
  <option value="brostep">
  <option value="future bass">
  <option value="wave">
  <!-- house -->
  <option value="house">
  <option value="deep house">
  <option value="tech house">
  <option value="progressive house">
  <!-- techno / trance -->
  <option value="techno">
  <option value="trance">
  <option value="progressive trance">
  <!-- broad electronic -->
  <option value="electronic">
  <option value="electronica">
  <option value="dance &amp; EDM">
  <!-- uk dance -->
  <option value="uk garage">
  <option value="garage">
  <option value="grime">
  <option value="uk dance">
  <!-- breakbeat / footwork -->
  <option value="breaks">
  <option value="breakbeat">
  <option value="footwork">
  <option value="juke">
  <option value="jersey club">
  <!-- hardcore -->
  <option value="hardcore">
  <option value="hardstyle">
  <!-- synth / wave -->
  <option value="synthwave">
  <option value="vaporwave">
  <option value="darkwave">
  <!-- experimental / IDM -->
  <option value="IDM">
  <option value="glitch">
  <option value="glitch hop">
  <option value="experimental">
  <option value="ambient">
  <option value="drone">
  <!-- downtempo -->
  <option value="downtempo">
  <option value="lo-fi">
  <option value="trip hop">
  <!-- hip-hop / trap -->
  <option value="hip hop &amp; rap">
  <option value="hip hop">
  <option value="trap">
  <option value="drill">
  <option value="phonk">
  <!-- global / afrobeats -->
  <option value="afrobeats">
  <option value="dancehall">
  <option value="reggaeton">
  <option value="reggae">
  <option value="dub">
  <!-- soul / jazz -->
  <option value="r&amp;b &amp; soul">
  <option value="jazz &amp; blues">
  <option value="disco">
  <!-- rock / indie -->
  <option value="indie">
  <option value="alternative rock">
  <option value="folk &amp; singer-songwriter">
  <option value="rock">
  <option value="metal">
  <!-- pop / mainstream -->
  <option value="pop">
  <!-- classical / cinematic -->
  <option value="classical">
  <option value="piano">
  <option value="soundtrack">
  <!-- country / world -->
  <option value="country">
  <option value="latin">
  <option value="world">
</datalist>

<script>
'use strict';
const DIGEST_ID = DIGEST_ID_PLACEHOLDER;
let digest = null;

async function load() {
  const [dr, rr] = await Promise.all([
    fetch(`/api/digests/${DIGEST_ID}`),
    fetch(`/api/digests/${DIGEST_ID}/runs?limit=20`),
  ]);
  digest = await dr.json();
  const runs = await rr.json();
  render(digest, runs);
}

function render(d, runs) {
  const spec = d.spec || {};
  const f    = spec.filters || {};
  document.getElementById('digest-name').textContent = spec.name || 'unnamed';

  document.getElementById('digest-detail').innerHTML = `
    <div class="card">
      <div style="display:grid;grid-template-columns:1fr 1fr;gap:16px">
        <div>
          <div class="filter-label" style="margin-bottom:8px">Search filters</div>
          <div class="filter-group" style="margin-bottom:8px">
            <div class="filter-label">query</div>
            <input id="d-query" type="text" value="${esc(f.query||'')}">
          </div>
          <div class="filter-group" style="margin-bottom:8px">
            <div class="filter-label">genre / tag</div>
            <input id="d-genre" type="text" value="${esc(f.genre_or_tag||'')}" list="genre-suggestions">
          </div>
          <div class="filter-group" style="margin-bottom:8px">
            <div class="filter-label">BPM</div>
            <div class="filter-pair">
              <input id="d-bpm-from" type="number" value="${f.bpm_from||''}" placeholder="from">
              <input id="d-bpm-to"   type="number" value="${f.bpm_to||''}"   placeholder="to">
            </div>
          </div>
          <div class="filter-group" style="margin-bottom:8px">
            <div class="filter-label">max plays</div>
            <input id="d-max-plays" type="number" value="${f.max_plays||''}">
          </div>
          <div class="filter-group" style="margin-bottom:8px">
            <div class="filter-label">min likes</div>
            <input id="d-min-likes" type="number" value="${f.min_likes||''}">
          </div>
          <div class="filter-group">
            <div class="filter-label">max duration (min)</div>
            <input id="d-max-dur" type="number" value="${f.duration_to_ms ? f.duration_to_ms/60000 : ''}">
          </div>
        </div>
        <div>
          <div class="filter-label" style="margin-bottom:8px">Schedule</div>
          <div class="filter-group" style="margin-bottom:8px">
            <div class="filter-label">name</div>
            <input id="d-name" type="text" value="${esc(spec.name||'')}">
          </div>
          <div class="filter-group" style="margin-bottom:8px">
            <div class="filter-label">cron expression</div>
            <input id="d-cron" type="text" value="${esc(spec.cron_expr||'')}" placeholder="0 0 6 * * SUN">
          </div>
          <div class="filter-group" style="margin-bottom:8px">
            <div class="filter-label">timezone</div>
            <input id="d-tz" type="text" value="${esc(spec.timezone||'America/Los_Angeles')}">
          </div>
          <div class="filter-group" style="margin-bottom:8px">
            <div class="filter-label">target tracks</div>
            <input id="d-target" type="number" value="${spec.target_size||25}">
          </div>
          <div class="filter-group" style="margin-bottom:8px">
            <div class="filter-label">playlist title template</div>
            <input id="d-title-tmpl" type="text" value="${esc(spec.playlist_title_tmpl||'')}">
          </div>
          <div class="filter-group">
            <label style="font-size:12px;color:var(--fg-1);display:flex;gap:6px;align-items:center;cursor:pointer">
              <input type="checkbox" id="d-public" style="width:auto" ${spec.playlist_visibility==='public'?'checked':''}>
              public playlist
            </label>
          </div>
        </div>
      </div>
      <div style="margin-top:12px;font-size:12px;color:var(--fg-2)">
        Next run: ${d.next_run_at ? new Date(d.next_run_at).toLocaleString() : '—'} ·
        Last run: ${d.last_run_at ? new Date(d.last_run_at).toLocaleString() : 'never'}
      </div>
    </div>`;

  const runList = document.getElementById('run-history');
  if (!Array.isArray(runs) || runs.length === 0) {
    runList.innerHTML = '<div class="empty-state">No runs yet.</div>';
    return;
  }
  runList.innerHTML = runs.map(r => {
    const statusColor = r.status === 'success' ? 'var(--queued)'
                      : r.status === 'failed'  ? 'var(--danger)'
                      : 'var(--fg-2)';
    const ranAt = new Date(r.ran_at).toLocaleString();
    return `<div class="card">
      <div style="display:flex;justify-content:space-between;align-items:start">
        <div>
          <div class="card-meta">${ranAt}</div>
          <div class="card-meta">${r.track_count ?? 0} tracks · ${r.pages_scanned ?? 0} pages scanned</div>
          ${r.error ? `<div style="font-size:11px;color:var(--danger);margin-top:4px">${esc(r.error)}</div>` : ''}
        </div>
        <div style="display:flex;gap:10px;align-items:center">
          <span style="font-size:12px;color:${statusColor}">${r.status}</span>
          ${r.playlist_url ? `<a href="${esc(r.playlist_url)}" target="_blank" style="font-size:12px;color:var(--accent)">playlist ↗</a>` : ''}
        </div>
      </div>
    </div>`;
  }).join('');
}

async function saveSpec() {
  if (!digest) return;
  const spec = digest.spec;
  const f = spec.filters || {};
  const md = parseFloat(document.getElementById('d-max-dur').value);
  const updated = {
    ...spec,
    name:                document.getElementById('d-name').value.trim(),
    cron_expr:           document.getElementById('d-cron').value.trim(),
    timezone:            document.getElementById('d-tz').value.trim(),
    target_size:         parseInt(document.getElementById('d-target').value) || 25,
    playlist_title_tmpl: document.getElementById('d-title-tmpl').value.trim(),
    playlist_visibility: document.getElementById('d-public').checked ? 'public' : 'private',
    filters: {
      ...f,
      query:          document.getElementById('d-query').value.trim() || null,
      genre_or_tag:   document.getElementById('d-genre').value.trim() || null,
      bpm_from:       parseInt(document.getElementById('d-bpm-from').value) || null,
      bpm_to:         parseInt(document.getElementById('d-bpm-to').value)   || null,
      max_plays:      parseInt(document.getElementById('d-max-plays').value) || null,
      min_likes:      parseInt(document.getElementById('d-min-likes').value) || null,
      duration_to_ms: md ? md * 60_000 : null,
    },
  };
  const r = await fetch(`/api/digests/${DIGEST_ID}`, {
    method: 'PATCH',
    headers: {'Content-Type': 'application/json'},
    body: JSON.stringify(updated),
  });
  if (r.ok) { digest = await r.json(); alert('Saved.'); }
  else { const e = await r.json().catch(() => ({})); alert('Error: ' + (e.message || r.status)); }
}

async function runNow() {
  const r = await fetch(`/api/digests/${DIGEST_ID}/run`, {method: 'POST'});
  if (r.ok) alert('Run started.');
  else alert('Failed to start run.');
}

function esc(s) {
  const d = document.createElement('div');
  d.textContent = typeof s === 'string' ? s : String(s ?? '');
  return d.innerHTML;
}

load();
</script>
</body>
</html>"##;

// ── /history ──────────────────────────────────────────────────────────────────

const HISTORY: &str = r##"<!DOCTYPE html>
<html lang="en">
<head>
<meta charset="utf-8">
<meta name="viewport" content="width=device-width, initial-scale=1">
<title>crater / history</title>
<style>/* CSS_PLACEHOLDER */</style>
</head>
<body>
<header>
  <span class="logo">crater</span>
  <nav>
    <a href="/dig">dig</a>
    <a href="/hearted">hearted</a>
    <a href="/queue">queue</a>
    <a href="/digests">digests</a>
    <a href="/history" class="active">history</a>
  </nav>
  <a href="/settings" class="nav-cog" title="settings">⚙</a>
</header>

<div class="page-layout">
  <div class="page-header">
    <span class="page-title">HISTORY</span>
    <div style="display:flex;gap:8px">
      <select id="h-status" onchange="load()" style="width:auto;font-size:12px;padding:4px 8px">
        <option value="rejected">rejected</option>
        <option value="hearted">hearted</option>
        <option value="queued">queued</option>
        <option value="exported">exported</option>
      </select>
    </div>
  </div>
  <div id="h-count" style="font-size:12px;color:var(--fg-2);margin-bottom:12px"></div>
  <div id="history-list"></div>
</div>

<script>
'use strict';

async function load() {
  const status = document.getElementById('h-status').value;
  const r = await fetch(`/api/tracks?status=${status}`);
  const tracks = await r.json();
  render(Array.isArray(tracks) ? tracks : []);
}

function render(tracks) {
  document.getElementById('h-count').textContent = `${tracks.length} track${tracks.length === 1 ? '' : 's'}`;
  const list = document.getElementById('history-list');
  if (tracks.length === 0) {
    list.innerHTML = '<div class="empty-state">No tracks with this status.</div>';
    return;
  }
  list.innerHTML = tracks.map(t => {
    const dur   = fmtDur(t.duration_ms);
    const ratio = (t.likes_count > 0 && t.playback_count > 0)
      ? (t.likes_count / t.playback_count).toFixed(3) : null;
    const meta  = [
      t.playback_count != null ? `${t.playback_count} plays` : null,
      t.likes_count    != null ? `${t.likes_count} likes`    : null,
      ratio ? `ratio ${ratio}` : null,
      t.bpm ? `${Math.round(t.bpm)}bpm` : null,
      dur,
      t.genre,
    ].filter(Boolean).join(' · ');

    return `<div class="card" style="display:grid;grid-template-columns:1fr auto;gap:8px;align-items:start">
      <div>
        <div class="card-title">
          <a href="${esc(t.permalink_url||'#')}" target="_blank" rel="noopener">${esc(t.title||'untitled')}</a>
          <span style="color:var(--fg-2);font-weight:400"> — ${esc(t.artist||'unknown')}</span>
        </div>
        <div class="card-meta">${meta}</div>
      </div>
      <button onclick="clearStatus(${t.id})"
              style="font-size:12px;color:var(--fg-2);padding:3px 8px;border:1px solid var(--bg-2);border-radius:3px;flex-shrink:0">
        undo
      </button>
    </div>`;
  }).join('');
}

async function clearStatus(id) {
  await fetch(`/api/tracks/${id}/status`, {
    method: 'POST',
    headers: {'Content-Type': 'application/json'},
    body: JSON.stringify({status: null}),
  });
  load();
}

function fmtDur(ms) {
  if (!ms && ms !== 0) return '';
  const s = Math.floor(ms / 1000);
  return `${Math.floor(s / 60)}:${String(s % 60).padStart(2, '0')}`;
}

function esc(s) {
  const d = document.createElement('div');
  d.textContent = typeof s === 'string' ? s : String(s ?? '');
  return d.innerHTML;
}

load();
</script>
</body>
</html>"##;

// ── Settings page ─────────────────────────────────────────────────────────────

pub const SETTINGS: &str = r##"<!DOCTYPE html>
<html lang="en">
<head>
<meta charset="utf-8">
<meta name="viewport" content="width=device-width, initial-scale=1">
<title>crater — settings</title>
<style>/* CSS_PLACEHOLDER */
.settings-section {
  margin-bottom: 36px;
}
.settings-section h2 {
  font-size: 11px; font-weight: 600; text-transform: uppercase;
  letter-spacing: .08em; color: var(--fg-2);
  margin-bottom: 16px; padding-bottom: 6px;
  border-bottom: 1px solid var(--bg-2);
}
.settings-row {
  display: flex; align-items: center; gap: 8px; margin-bottom: 10px;
}
.settings-row label {
  font-size: 11px; font-weight: 600; text-transform: uppercase;
  letter-spacing: .06em; color: var(--fg-2);
  width: 90px; flex-shrink: 0;
}
.settings-row input { flex: 1; }
.settings-msg { font-size: 12px; margin-top: 4px; min-height: 18px; }
.stat-grid {
  display: grid; grid-template-columns: repeat(5, 1fr); gap: 8px;
  margin-bottom: 14px;
}
.stat-cell {
  background: var(--bg-1); border: 1px solid var(--bg-2); border-radius: 4px;
  padding: 10px 12px; text-align: center;
}
.stat-cell .stat-num {
  font-size: 22px; font-weight: 600; font-variant-numeric: tabular-nums;
  color: var(--fg-0); display: block;
}
.stat-cell .stat-label {
  font-size: 10px; text-transform: uppercase; letter-spacing: .06em;
  color: var(--fg-2); margin-top: 2px; display: block;
}
.status-badge {
  display: inline-flex; align-items: center; gap: 6px;
  font-size: 12px; padding: 4px 10px; border-radius: 3px;
  background: var(--bg-2); margin-bottom: 12px;
}
.btn-row { display: flex; gap: 8px; flex-wrap: wrap; align-items: center; }
.btn { padding: 6px 14px; border: 1px solid var(--bg-3); border-radius: 3px;
       font-size: 12px; cursor: pointer; background: var(--bg-2); color: var(--fg-0); }
.btn:hover { color: var(--fg-0); background: var(--bg-3); }
.btn-primary { background: var(--accent); color: var(--bg-0); border-color: var(--accent); font-weight: 600; }
.btn-primary:hover { opacity: .88; }
.btn-danger  { color: var(--danger); border-color: var(--bg-3); }
.btn-danger:hover { background: var(--bg-3); }
</style>
</head>
<body>
<header>
  <span class="logo">crater</span>
  <nav>
    <a href="/dig">dig</a>
    <a href="/hearted">hearted</a>
    <a href="/queue">queue</a>
    <a href="/digests">digests</a>
    <a href="/history">history</a>
  </nav>
  <a href="/settings" class="nav-cog active" title="settings">⚙</a>
</header>
<div class="page-layout">

<!-- SoundCloud connection ------------------------------------------------- -->
<section class="settings-section">
  <h2>SoundCloud connection</h2>
  <div id="sc-status-badge" class="status-badge" style="color:var(--fg-2)">checking…</div>

  <div id="sc-oauth-row" style="display:none;margin-bottom:16px">
    <a href="/auth/soundcloud" class="btn btn-primary">connect via OAuth</a>
    <span style="font-size:12px;color:var(--fg-2);margin-left:8px">opens SoundCloud consent page</span>
  </div>

  <div style="font-size:11px;font-weight:600;text-transform:uppercase;letter-spacing:.06em;color:var(--fg-2);margin-bottom:6px">manual token</div>
  <div style="font-size:12px;color:var(--fg-2);margin-bottom:8px">
    Paste the <code style="font-family:monospace;background:var(--bg-2);padding:1px 4px;border-radius:2px">Authorization</code>
    header value from DevTools — with or without the "OAuth " prefix.
  </div>
  <div class="btn-row">
    <input id="token-input" type="password" placeholder="OAuth 2-…"
           style="flex:1;min-width:0;font-family:monospace;font-size:12px">
    <button class="btn btn-primary" onclick="saveToken()">save</button>
    <button class="btn" onclick="testToken()">test</button>
  </div>
  <div id="token-msg" class="settings-msg"></div>
</section>

<!-- Notifications --------------------------------------------------------- -->
<section class="settings-section">
  <h2>Notifications</h2>
  <div style="font-size:12px;color:var(--fg-2);margin-bottom:12px">
    crater can send a push notification via <a href="https://ntfy.sh" target="_blank" style="color:var(--accent)">ntfy</a>
    when a digest run completes. Leave blank to disable.
  </div>
  <div class="settings-row">
    <label>ntfy URL</label>
    <input id="ntfy-url" type="text" placeholder="http://unraid.local:8090">
  </div>
  <div class="settings-row">
    <label>topic</label>
    <input id="ntfy-topic" type="text" placeholder="crater">
  </div>
  <div class="btn-row" style="margin-top:4px">
    <button class="btn btn-primary" onclick="saveNtfy()">save</button>
  </div>
  <div id="ntfy-msg" class="settings-msg"></div>
</section>

<!-- Library --------------------------------------------------------------- -->
<section class="settings-section">
  <h2>Library</h2>
  <div class="stat-grid" id="stat-grid">
    <div class="stat-cell"><span class="stat-num" id="stat-total">—</span><span class="stat-label">total</span></div>
    <div class="stat-cell"><span class="stat-num" id="stat-queued" style="color:var(--queued)">—</span><span class="stat-label">queued</span></div>
    <div class="stat-cell"><span class="stat-num" id="stat-hearted" style="color:var(--accent)">—</span><span class="stat-label">hearted</span></div>
    <div class="stat-cell"><span class="stat-num" id="stat-rejected" style="color:var(--danger)">—</span><span class="stat-label">rejected</span></div>
    <div class="stat-cell"><span class="stat-num" id="stat-exported" style="color:var(--fg-2)">—</span><span class="stat-label">exported</span></div>
  </div>
  <div class="btn-row">
    <button class="btn btn-danger" onclick="clearByStatus('rejected')">clear all rejected</button>
    <button class="btn btn-danger" onclick="clearByStatus('exported')">clear all exported</button>
  </div>
  <div id="library-msg" class="settings-msg"></div>
</section>

<!-- Session --------------------------------------------------------------- -->
<section class="settings-section">
  <h2>Session</h2>
  <form method="post" action="/logout">
    <button type="submit" class="btn btn-danger">log out</button>
  </form>
</section>

</div>
<script>
'use strict';

function esc(s) {
  const d = document.createElement('div');
  d.textContent = typeof s === 'string' ? s : String(s ?? '');
  return d.innerHTML;
}

function setMsg(id, text, ok) {
  const el = document.getElementById(id);
  el.textContent = text;
  el.style.color = ok ? 'var(--green)' : ok === false ? 'var(--danger)' : 'var(--fg-2)';
}

// ── Load settings on page load ─────────────────────────────────────────────
async function loadSettings() {
  const [sr, tr] = await Promise.all([
    fetch('/api/settings'),
    fetch('/api/stats'),
  ]);

  // Settings
  const s = await sr.json();
  document.getElementById('ntfy-url').value   = s.ntfy_url   || '';
  document.getElementById('ntfy-topic').value = s.ntfy_topic || '';

  // OAuth button
  if (s.pkce_configured) {
    document.getElementById('sc-oauth-row').style.display = '';
  }

  // SC status badge
  const badge = document.getElementById('sc-status-badge');
  if (s.sc_token_stored) {
    badge.textContent = 'token stored — test to verify';
    badge.style.color = 'var(--fg-1)';
  } else {
    badge.textContent = 'no token stored';
    badge.style.color = 'var(--fg-2)';
  }

  // Stats
  if (tr.ok) {
    const t = await tr.json();
    document.getElementById('stat-total').textContent    = t.total;
    document.getElementById('stat-queued').textContent   = t.queued;
    document.getElementById('stat-hearted').textContent  = t.hearted;
    document.getElementById('stat-rejected').textContent = t.rejected;
    document.getElementById('stat-exported').textContent = t.exported;
  }
}

// ── SC token ──────────────────────────────────────────────────────────────
async function saveToken() {
  const token = document.getElementById('token-input').value.trim();
  if (!token) { setMsg('token-msg', 'enter a token first', false); return; }
  const r = await fetch('/api/settings/sc-token', {
    method: 'POST',
    headers: {'Content-Type': 'application/json'},
    body: JSON.stringify({token}),
  });
  const j = await r.json();
  if (r.ok) {
    setMsg('token-msg', 'saved', true);
    document.getElementById('sc-status-badge').textContent = 'token stored — test to verify';
    document.getElementById('sc-status-badge').style.color = 'var(--fg-1)';
    document.getElementById('token-input').value = '';
  } else {
    setMsg('token-msg', j.message || 'save failed', false);
  }
}

async function testToken() {
  setMsg('token-msg', 'testing…', null);
  const r = await fetch('/api/settings/sc-token/test');
  const j = await r.json();
  if (r.ok && j.status === 'ok') {
    setMsg('token-msg', `connected as ${esc(j.username)}`, true);
    const badge = document.getElementById('sc-status-badge');
    badge.innerHTML = `connected as <strong style="color:var(--fg-0)">${esc(j.username)}</strong>`;
    badge.style.color = 'var(--green)';
  } else {
    setMsg('token-msg', j.message || 'test failed — token may be expired', false);
  }
}

// ── ntfy ──────────────────────────────────────────────────────────────────
async function saveNtfy() {
  const ntfy_url   = document.getElementById('ntfy-url').value.trim();
  const ntfy_topic = document.getElementById('ntfy-topic').value.trim();
  const r = await fetch('/api/settings/ntfy', {
    method: 'POST',
    headers: {'Content-Type': 'application/json'},
    body: JSON.stringify({ntfy_url, ntfy_topic}),
  });
  const j = await r.json();
  setMsg('ntfy-msg', r.ok ? 'saved' : (j.message || 'save failed'), r.ok);
}

// ── Library ───────────────────────────────────────────────────────────────
async function clearByStatus(status) {
  const label = status === 'rejected' ? 'rejected' : 'exported';
  if (!confirm(`Clear all ${label} tracks? This cannot be undone.`)) return;
  setMsg('library-msg', `clearing ${label} tracks…`, null);
  try {
    const r    = await fetch(`/api/tracks?status=${status}`);
    const list = await r.json();
    for (const t of list) {
      await fetch(`/api/tracks/${t.id}/status`, {
        method: 'POST',
        headers: {'Content-Type': 'application/json'},
        body: JSON.stringify({status: null}),
      });
    }
    setMsg('library-msg', `cleared ${list.length} ${label} tracks`, true);
    // Refresh stats
    const sr = await fetch('/api/stats');
    if (sr.ok) {
      const t = await sr.json();
      document.getElementById('stat-rejected').textContent = t.rejected;
      document.getElementById('stat-exported').textContent = t.exported;
    }
  } catch(e) {
    setMsg('library-msg', 'error: ' + e.message, false);
  }
}

loadSettings();
</script>
</body>
</html>"##;
