# `core` crate design

Business logic layer between `sc_client` (network) and `server` (HTTP).
Owns the SQLite database, the search session model, digest definitions,
and the future acquisition queue. Pure library — no HTTP, no TUI, no
async runtime assumptions beyond `tokio`.

## Dependencies

```toml
[dependencies]
sc_client = { path = "../sc_client" }
anyhow = { workspace = true }
thiserror = { workspace = true }
serde = { workspace = true }
serde_json = { workspace = true }
tokio = { workspace = true }
tracing = { workspace = true }
sqlx = { version = "0.8", features = ["runtime-tokio-rustls", "sqlite", "chrono", "macros", "migrate"] }
chrono = { version = "0.4", features = ["serde"] }
uuid = { version = "1", features = ["v4", "serde"] }
cron = "0.13"  # parse cron expressions for digest schedules
```

## Module layout

```
crates/core/src/
├── lib.rs             public re-exports, Crater struct (the facade)
├── db.rs              sqlx pool, migrations runner, connection helpers
├── tracks.rs          Track upsert, status transitions, queries
├── session.rs         Search session abstraction (dedup, pagination, progress)
├── digests.rs         Digest CRUD, cron parsing, next_run calculation
├── digest_runner.rs   Executes a single digest: search → pick → export
├── filters.rs         SearchFilters ↔ DB persistence, ranking strategies
└── error.rs           CoreError (wraps ScError + sqlx errors)
```

## Database

SQLite via `sqlx`, file at `$CRATER_DATA_DIR/crater.db`. Migrations in
`crates/core/migrations/` (sqlx convention), run automatically at boot.
WAL mode for concurrent reads during digest execution.

### Schema

```sql
-- migrations/0001_init.sql

PRAGMA journal_mode = WAL;
PRAGMA foreign_keys = ON;

-- Every track we've ever seen, regardless of status. Upserted on every
-- search hit so play counts stay fresh (tracks get more popular over
-- time, which we want to know about for re-ranking).
CREATE TABLE tracks (
    id               INTEGER PRIMARY KEY,  -- SoundCloud track id
    title            TEXT,
    artist           TEXT,
    artist_sc_id     INTEGER,
    permalink_url    TEXT NOT NULL,
    duration_ms      INTEGER,
    bpm              REAL,
    genre            TEXT,
    tag_list         TEXT,
    playback_count   INTEGER,
    likes_count      INTEGER,
    reposts_count    INTEGER,
    comment_count    INTEGER,
    created_at_sc    TEXT,       -- ISO 8601 from SoundCloud
    first_seen       DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP,
    last_seen        DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP,
    raw_json         TEXT        -- full v2 response, for future fields
);

CREATE INDEX idx_tracks_genre ON tracks(genre);
CREATE INDEX idx_tracks_playback ON tracks(playback_count);
CREATE INDEX idx_tracks_last_seen ON tracks(last_seen);

-- One row per track, tracks user disposition. Split from `tracks` so
-- "forget everything I've rated but keep metadata" is a single DELETE.
CREATE TABLE track_status (
    track_id    INTEGER PRIMARY KEY REFERENCES tracks(id) ON DELETE CASCADE,
    status      TEXT NOT NULL CHECK (status IN (
        'queued',       -- in current export queue
        'rejected',     -- user said no, don't show again
        'hearted',      -- loved it, will be acquired later
        'exported'      -- already on a playlist, don't re-surface
    )),
    updated_at  DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP,
    note        TEXT    -- optional user-entered note for the track
);

CREATE INDEX idx_status_status ON track_status(status);

-- Saved search configurations that get run on a schedule.
CREATE TABLE digests (
    id                   INTEGER PRIMARY KEY AUTOINCREMENT,
    name                 TEXT NOT NULL UNIQUE,
    query_json           TEXT NOT NULL,  -- serialized SearchFilters
    ranking              TEXT NOT NULL DEFAULT 'engagement_ratio',
    cron_expr            TEXT NOT NULL,  -- 6-field cron (with seconds)
    target_size          INTEGER NOT NULL DEFAULT 25,
    max_pages            INTEGER NOT NULL DEFAULT 20,
    playlist_visibility  TEXT NOT NULL DEFAULT 'private'
                         CHECK (playlist_visibility IN ('private', 'public')),
    playlist_title_tmpl  TEXT NOT NULL,  -- e.g. "crater — dnb — {year}-W{week}"
    enabled              INTEGER NOT NULL DEFAULT 1,
    last_run_at          DATETIME,
    next_run_at          DATETIME,       -- computed from cron_expr
    created_at           DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP
);

CREATE INDEX idx_digests_next_run ON digests(next_run_at) WHERE enabled = 1;

-- Audit log of digest executions. Keeps history even when digests are
-- deleted (FK on delete set null).
CREATE TABLE digest_runs (
    id                INTEGER PRIMARY KEY AUTOINCREMENT,
    digest_id         INTEGER REFERENCES digests(id) ON DELETE SET NULL,
    digest_name       TEXT NOT NULL,  -- denormalized snapshot
    ran_at            DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP,
    finished_at       DATETIME,
    status            TEXT NOT NULL CHECK (status IN ('running','success','failed')),
    playlist_sc_id    INTEGER,
    playlist_url      TEXT,
    track_count       INTEGER,
    pages_scanned     INTEGER,
    error             TEXT,
    tracks_json       TEXT  -- snapshot of track IDs + metadata at run time
);

CREATE INDEX idx_runs_digest ON digest_runs(digest_id, ran_at DESC);

-- Which tracks went into which digest run — for "don't re-export this
-- track in digest X" logic and for the activity feed UI.
CREATE TABLE digest_run_tracks (
    run_id      INTEGER NOT NULL REFERENCES digest_runs(id) ON DELETE CASCADE,
    track_id    INTEGER NOT NULL REFERENCES tracks(id) ON DELETE CASCADE,
    rank        INTEGER NOT NULL,
    PRIMARY KEY (run_id, track_id)
);

-- Future: track acquisition queue. Schema in place so v2 doesn't need migration.
CREATE TABLE acquisitions (
    track_id     INTEGER PRIMARY KEY REFERENCES tracks(id) ON DELETE CASCADE,
    status       TEXT NOT NULL CHECK (status IN (
        'pending', 'in_progress', 'acquired', 'failed'
    )),
    file_path    TEXT,
    attempts     INTEGER NOT NULL DEFAULT 0,
    last_attempt DATETIME,
    error        TEXT,
    created_at   DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP
);

-- Generic key-value for small config that doesn't deserve its own table:
-- cached client_id, oauth_token fingerprint, schema version, etc.
CREATE TABLE kv (
    k TEXT PRIMARY KEY,
    v TEXT NOT NULL,
    updated_at DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP
);
```

### Notes on schema decisions

**Why split `tracks` and `track_status`.** Tracks are cheap to re-fetch
from SoundCloud, but user judgments (rejected/hearted) are the irreplaceable
state. Splitting means we can aggressively refresh tracks without risk of
nuking user data, and `DELETE FROM tracks WHERE last_seen < ...` is a
safe cleanup.

**Why store `raw_json`.** Cheap insurance. If we add a feature later that
needs a field we didn't extract (e.g. waveform URL, download URL), we
don't need to re-scrape SoundCloud for the whole cache.

**Why `digest_run_tracks` as a separate table.** Need to support "show me
what was in digest run #47" and "has this track been exported in any
digest in the last 30 days" — both are trivial SQL with the join table
and awkward without it.

**Why `tracks_json` in `digest_runs` despite the join table.** Belt and
suspenders: the join table gives us exact track IDs but if those tracks
get deleted from `tracks` (cleanup job), we lose the historical snapshot.
The `tracks_json` blob preserves "what this playlist was, in this
moment" forever, which is useful for "regenerate playlist X" later.

## Public API

```rust
// crates/core/src/lib.rs

pub use sc_client;  // re-export so server crate doesn't depend on sc_client directly

pub struct Crater {
    db: SqlitePool,
    sc: sc_client::Client,
}

impl Crater {
    pub async fn new(config: Config) -> Result<Self, CoreError>;

    // Track-level operations
    pub async fn upsert_track(&self, track: &sc_client::Track) -> Result<()>;
    pub async fn set_status(&self, track_id: u64, status: TrackStatus) -> Result<()>;
    pub async fn clear_status(&self, track_id: u64) -> Result<()>;
    pub async fn get_track(&self, track_id: u64) -> Result<Option<StoredTrack>>;
    pub async fn tracks_with_status(&self, status: TrackStatus) -> Result<Vec<StoredTrack>>;

    // Session — the core "give me fresh stuff I haven't seen" primitive
    pub fn new_session(&self, filters: sc_client::SearchFilters) -> Session;

    // Digest CRUD
    pub async fn create_digest(&self, spec: DigestSpec) -> Result<Digest>;
    pub async fn list_digests(&self) -> Result<Vec<Digest>>;
    pub async fn update_digest(&self, id: i64, spec: DigestSpec) -> Result<Digest>;
    pub async fn delete_digest(&self, id: i64) -> Result<()>;

    // Digest execution (called by scheduler, also exposed for manual runs)
    pub async fn run_digest(&self, id: i64) -> Result<DigestRun>;
}

pub struct Config {
    pub data_dir: PathBuf,           // where crater.db lives
    pub sc_oauth_token: Option<String>,  // needed for digest export
    pub cached_client_id: Option<String>,
}
```

### The `Session` abstraction

The UX problem: searches involve pagination behind the scenes (SoundCloud
returns 50 at a time, we filter client-side, often <10% pass the ceiling,
so 5+ network requests to fill a page of results). The UI needs to stream
progress. This is the abstraction that makes that clean.

```rust
pub struct Session {
    id: Uuid,
    filters: sc_client::SearchFilters,
    // internal: shared ref to Crater's pool + sc client
}

pub struct SessionBatch {
    pub tracks: Vec<StoredTrack>,    // only NEW tracks (not previously seen)
    pub pages_scanned: u32,
    pub total_scanned: u32,          // all tracks fetched, pre-filter
    pub exhausted: bool,             // true if no more pages available
}

impl Session {
    /// Fetch the next batch. Blocks until at least `min` fresh tracks found
    /// OR `max_pages` pages scanned OR search exhausted. Whichever first.
    pub async fn next_batch(&mut self, min: usize, max_pages: u32) -> Result<SessionBatch>;

    /// Stream-oriented variant: yields each new track as it's accepted,
    /// for WebSocket "results trickling in" UX. Returns a stream.
    pub fn stream(&mut self) -> impl Stream<Item = Result<StoredTrack>>;
}
```

**Dedup logic** inside `next_batch`:

1. Fetch a page via `sc_client::search_tracks_filtered`.
2. For each result, upsert into `tracks` (refreshes play counts).
3. Left-join `track_status`: drop any with status in `('rejected','exported')`
   — those are "don't show me again" by definition.
4. Hearted tracks: include them, but mark them so the UI renders a "❤" badge
   and they sort below never-seen-before tracks.
5. Queued tracks: include with a "in queue" badge.

This means a fresh search never shows tracks you've rejected, shows
hearted tracks as a gentle "hey, this matches your new query too", and
lets you see what's already in your queue.

### Ranking strategies

```rust
pub enum Ranking {
    /// Sort by likes/plays ratio descending. Best "hidden gem" signal.
    EngagementRatio,
    /// Sort by created_at descending. Prioritize new uploads.
    Recency,
    /// Random shuffle, for serendipity.
    Shuffle,
    /// Composite: log(likes+1) / sqrt(plays+1) — balances engagement and obscurity
    /// better than raw ratio for very-low-play tracks where ratio is noisy.
    Score,
}
```

`EngagementRatio` defaults to filtering out tracks with <5 plays (ratio
is meaningless on n=1 samples). `Score` is the one I'd pick for digests
— it's less fooled by a track with 2 plays and 2 likes (ratio 1.0).

### Digest types

```rust
pub struct DigestSpec {
    pub name: String,
    pub filters: sc_client::SearchFilters,
    pub ranking: Ranking,
    pub cron_expr: String,       // "0 0 6 * * SUN" = Sun 6am
    pub target_size: u32,
    pub max_pages: u32,
    pub playlist_visibility: PlaylistVisibility,
    pub playlist_title_tmpl: String,  // supports {year}, {week}, {month}, {date}
}

pub enum PlaylistVisibility { Private, Public }

pub struct Digest {
    pub id: i64,
    pub spec: DigestSpec,
    pub enabled: bool,
    pub last_run_at: Option<DateTime<Utc>>,
    pub next_run_at: Option<DateTime<Utc>>,
    pub created_at: DateTime<Utc>,
}
```

Cron is parsed by the `cron` crate, validated at create/update time.
`next_run_at` is computed and stored so the scheduler can do a cheap
`SELECT * FROM digests WHERE enabled AND next_run_at <= now()` lookup
every minute.

### Digest runner flow

```
run_digest(id):
    1. Load digest spec
    2. INSERT INTO digest_runs (status='running')
    3. Run search via Session with filters, accumulate up to target_size
       tracks that pass the ranking threshold AND aren't in
       ('exported','rejected')
    4. Rank by chosen strategy, truncate to target_size
    5. Create SoundCloud playlist via sc_client::Playlist::create
       (requires oauth_token; fail loudly if absent)
    6. Mark tracks as status='exported' in track_status
    7. INSERT INTO digest_run_tracks for each track with rank
    8. UPDATE digest_runs SET status='success', playlist_url=...
    9. UPDATE digests SET last_run_at, next_run_at
   10. Fire ntfy notification if configured
```

Failures at any step: write the error to `digest_runs.error`, set
`status='failed'`, don't mark tracks as exported. Next scheduled run
will try again from scratch (idempotent-ish — same tracks might surface,
which is fine since we only mark exported on success).

## Testing strategy

Unit tests (no network, no DB): filter logic, ranking math, cron parsing,
template expansion.

Integration tests (in-memory SQLite, mocked sc_client): session dedup,
digest runner with a fake sc_client that returns fixed tracks, status
transitions.

Live tests (behind `live-tests` feature, like sc_client): full
end-to-end digest run against real SoundCloud, creates a real private
playlist, cleans it up after. Opt-in.

## Open questions

- **Track refresh cadence.** Tracks get more popular over time. Do we
  refresh `playback_count` on every search hit (current plan) or have a
  dedicated refresh job? Current plan is simpler but means a track hit
  in 5 different searches in one day gets 5 writes. Probably fine for
  SQLite but worth measuring.
- **Soft vs hard delete for rejected tracks.** Current plan: hard-keep
  the row in `track_status` forever so we never resurface. Risk: if you
  reject something today and change your mind in a year, there's no
  path back except direct DB edit. Might want a "clear rejections older
  than N days" option.
- **Cron timezone.** Store cron as UTC or local? Unraid runs UTC by
  default, user probably thinks in Pacific. Plan: store a
  `timezone TEXT` column next to `cron_expr`, default to
  `America/Los_Angeles`, evaluate the cron in that zone.
