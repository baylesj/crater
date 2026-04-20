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
pub async fn queue_page()        -> Html<String> { Html(QUEUE.replace("/* CSS_PLACEHOLDER */", CSS)) }
pub async fn digests_page()      -> Html<String> { Html(DIGESTS.replace("/* CSS_PLACEHOLDER */", CSS)) }
pub async fn digest_detail_page(
    axum::extract::Path(id): axum::extract::Path<i64>,
) -> Html<String> {
    Html(DIGEST_DETAIL.replace("/* CSS_PLACEHOLDER */", CSS).replace("DIGEST_ID_PLACEHOLDER", &id.to_string()))
}
pub async fn history_page()      -> Html<String> { Html(HISTORY.replace("/* CSS_PLACEHOLDER */", CSS)) }

// ── Shared CSS ────────────────────────────────────────────────────────────────
//
// Inlined into each page (small, no extra round-trips).

const CSS: &str = r##"
:root {
  --bg-0: #0f1013; --bg-1: #171920; --bg-2: #1f222b;
  --fg-0: #e8ebf0; --fg-1: #a0a6b0; --fg-2: #5c6370;
  --accent: #c9ad7f;
  --queued: #7fb383;
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
  grid-template-columns: 18px 1fr auto;
  gap: 0 8px;
  padding: 7px 12px 7px 8px;
  border-bottom: 1px solid var(--bg-2);
  cursor: pointer;
  align-items: start;
}
.track:hover   { background: var(--bg-1); }
.track.selected { background: var(--sel); }
.track.playing  { background: var(--sel); }

.ind {
  font-size: 11px;
  padding-top: 2px;
  text-align: center;
  flex-shrink: 0;
  width: 14px;
}
.ind-playing { color: var(--fg-0); }
.ind-hearted { color: var(--accent); }
.ind-queued  { color: var(--queued); }

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
    <a href="/queue">queue <span class="nav-badge" id="queue-count">0</span></a>
    <a href="/digests">digests</a>
  </nav>
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
      <input id="f-genre" type="text" placeholder="drum &amp; bass">
    </div>
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
    <hr class="rule">
    <div class="filter-group">
      <div class="filter-label">target tracks</div>
      <input id="f-target" type="number" value="30" min="1" max="200">
    </div>
    <div class="filter-group">
      <div class="filter-label">max pages</div>
      <input id="f-pages" type="number" value="20" min="1" max="100">
    </div>
    <div class="filter-actions">
      <button class="btn-primary"    onclick="startSearch()">search</button>
      <button class="btn-secondary"  onclick="resetFilters()">reset</button>
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
      <button class="np-btn" onclick="actSelected('queued')"   title="queue [y]">queue<span class="kbd">y</span></button>
      <button class="np-btn" onclick="actSelected('rejected')" title="reject [n]">reject<span class="kbd">n</span></button>
      <button class="np-btn" onclick="actSelected('hearted')"  title="heart [h]">heart<span class="kbd">h</span></button>
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
  if (q)  f.query          = q;
  if (g)  f.genre_or_tag   = g;
  if (bf) f.bpm_from       = bf;
  if (bt) f.bpm_to         = bt;
  if (mp) f.max_plays      = mp;
  if (ml) f.min_likes      = ml;
  if (md) f.duration_to_ms = md * 60_000;

  let resp;
  try {
    resp = await fetch('/api/search', {
      method: 'POST',
      headers: {'Content-Type': 'application/json'},
      body: JSON.stringify({
        filters:     f,
        target_size: num('f-target') || 30,
        max_pages:   num('f-pages')  || 20,
      }),
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
  const idx = tracks.length;
  tracks.push(track);
  document.getElementById('track-list').insertAdjacentHTML('beforeend', cardHtml(track, idx));
  document.getElementById('status-counts').textContent =
    `${tracks.length} accepted · ${totalScanned} scanned · ${pagesScanned} pages`;
}

function onComplete({exhausted, total_accepted}) {
  searching = false;
  const more = exhausted ? 'exhausted' : 'more available';
  setStatus(`${total_accepted} tracks · ${more}`);
  refreshQueueCount();
}

function onSearchError({message}) {
  searching = false;
  setStatus('search error: ' + message);
}

// ── Track card rendering ──────────────────────────────────────────────────────
function cardHtml(t, idx) {
  const playing = idx === nowPlayingIdx;
  const ind = playing           ? '▶'
            : t.status === 'hearted' ? '★'
            : t.status === 'queued'  ? '✓'
            : '';
  const indCls = playing                ? 'ind ind-playing'
               : t.status === 'hearted' ? 'ind ind-hearted'
               : t.status === 'queued'  ? 'ind ind-queued'
               : 'ind';

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

  return `<li class="track${playing ? ' playing' : ''}" data-id="${t.id}" data-idx="${idx}"
     onclick="clickTrack(event, ${idx})">
  <span class="${indCls}" id="ind-${t.id}">${ind}</span>
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
    <button onclick="act(${t.id},'queued')"   title="queue [y]">✓</button>
    <button onclick="act(${t.id},'hearted')"  title="heart [h]">♥</button>
    <button onclick="act(${t.id},'rejected')" title="reject [n]">✕</button>
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
    if (status === 'rejected') {
      // Advance selection past rejected track
      if (idx === selectedIdx && idx + 1 < tracks.length) setSelected(idx + 1);
    }
  }
  refreshQueueCount();
}

function actSelected(status) {
  const idx = nowPlayingIdx >= 0 ? nowPlayingIdx : selectedIdx;
  if (idx >= 0) act(tracks[idx].id, status);
}

// ── Queue count in nav ────────────────────────────────────────────────────────
async function refreshQueueCount() {
  try {
    const r = await fetch('/api/tracks?status=queued');
    const arr = await r.json();
    document.getElementById('queue-count').textContent = Array.isArray(arr) ? arr.length : 0;
  } catch {}
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

// ── Init ──────────────────────────────────────────────────────────────────────
refreshQueueCount();
document.getElementById('nav-dig').classList.add('active');
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
    <a href="/queue" class="active">queue <span class="nav-badge" id="queue-count">0</span></a>
    <a href="/digests">digests</a>
  </nav>
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
    <a href="/queue">queue <span class="nav-badge" id="queue-count">0</span></a>
    <a href="/digests" class="active">digests</a>
  </nav>
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
    <a href="/queue">queue</a>
    <a href="/digests" class="active">digests</a>
  </nav>
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
            <input id="d-genre" type="text" value="${esc(f.genre_or_tag||'')}">
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
    <a href="/queue">queue</a>
    <a href="/digests">digests</a>
    <a href="/history" class="active">history</a>
  </nav>
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
