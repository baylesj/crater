PRAGMA journal_mode = WAL;
PRAGMA foreign_keys = ON;

CREATE TABLE tracks (
    id               INTEGER PRIMARY KEY,
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
    created_at_sc    TEXT,
    first_seen       DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP,
    last_seen        DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP,
    raw_json         TEXT
);

CREATE INDEX idx_tracks_genre     ON tracks(genre);
CREATE INDEX idx_tracks_playback  ON tracks(playback_count);
CREATE INDEX idx_tracks_last_seen ON tracks(last_seen);

CREATE TABLE track_status (
    track_id    INTEGER PRIMARY KEY REFERENCES tracks(id) ON DELETE CASCADE,
    status      TEXT NOT NULL CHECK (status IN ('queued','rejected','hearted','exported')),
    updated_at  DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP,
    note        TEXT
);

CREATE INDEX idx_status_status ON track_status(status);

CREATE TABLE digests (
    id                   INTEGER PRIMARY KEY AUTOINCREMENT,
    name                 TEXT NOT NULL UNIQUE,
    query_json           TEXT NOT NULL,
    ranking              TEXT NOT NULL DEFAULT 'engagement_ratio',
    cron_expr            TEXT NOT NULL,
    timezone             TEXT NOT NULL DEFAULT 'America/Los_Angeles',
    target_size          INTEGER NOT NULL DEFAULT 25,
    max_pages            INTEGER NOT NULL DEFAULT 20,
    playlist_visibility  TEXT NOT NULL DEFAULT 'private'
                         CHECK (playlist_visibility IN ('private','public')),
    playlist_title_tmpl  TEXT NOT NULL,
    enabled              INTEGER NOT NULL DEFAULT 1,
    last_run_at          DATETIME,
    next_run_at          DATETIME,
    created_at           DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP
);

CREATE INDEX idx_digests_next_run ON digests(next_run_at) WHERE enabled = 1;

CREATE TABLE digest_runs (
    id             INTEGER PRIMARY KEY AUTOINCREMENT,
    digest_id      INTEGER REFERENCES digests(id) ON DELETE SET NULL,
    digest_name    TEXT NOT NULL,
    ran_at         DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP,
    finished_at    DATETIME,
    status         TEXT NOT NULL CHECK (status IN ('running','success','failed')),
    playlist_sc_id INTEGER,
    playlist_url   TEXT,
    track_count    INTEGER,
    pages_scanned  INTEGER,
    error          TEXT,
    tracks_json    TEXT
);

CREATE INDEX idx_runs_digest ON digest_runs(digest_id, ran_at DESC);

CREATE TABLE digest_run_tracks (
    run_id    INTEGER NOT NULL REFERENCES digest_runs(id) ON DELETE CASCADE,
    track_id  INTEGER NOT NULL REFERENCES tracks(id)      ON DELETE CASCADE,
    rank      INTEGER NOT NULL,
    PRIMARY KEY (run_id, track_id)
);

CREATE TABLE acquisitions (
    track_id     INTEGER PRIMARY KEY REFERENCES tracks(id) ON DELETE CASCADE,
    status       TEXT NOT NULL CHECK (status IN ('pending','in_progress','acquired','failed')),
    file_path    TEXT,
    attempts     INTEGER NOT NULL DEFAULT 0,
    last_attempt DATETIME,
    error        TEXT,
    created_at   DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP
);

CREATE TABLE kv (
    k          TEXT PRIMARY KEY,
    v          TEXT NOT NULL,
    updated_at DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP
);
