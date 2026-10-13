-- Social graph cache for the crawler (core::dig).
--
-- The graph is append-mostly and grows with every dig: likers of a seed
-- become known users, their likes become known tracks, and the next dig
-- starts with all of it instead of re-fetching. Nothing here is user
-- judgment, so all of it is safe to drop and rebuild.

CREATE TABLE sc_users (
    id               INTEGER PRIMARY KEY,  -- SoundCloud user id
    username         TEXT,
    permalink_url    TEXT,
    likes_count      INTEGER,
    followers_count  INTEGER,
    track_count      INTEGER,
    last_seen        DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP
);

-- A user liked or reposted a track. Reposts are kept separately because
-- they are a public endorsement, which the scorer weights higher.
CREATE TABLE endorsements (
    user_id     INTEGER NOT NULL REFERENCES sc_users(id) ON DELETE CASCADE,
    track_id    INTEGER NOT NULL REFERENCES tracks(id) ON DELETE CASCADE,
    kind        TEXT NOT NULL CHECK (kind IN ('like', 'repost')),
    first_seen  DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP,
    PRIMARY KEY (user_id, track_id, kind)
);

CREATE INDEX idx_endorsements_track ON endorsements(track_id);

-- When each edge list was last fetched, keyed like `track:123:likers` or
-- `user:456:likes`, so a dig only re-fetches lists older than its max age.
CREATE TABLE crawl_log (
    subject     TEXT PRIMARY KEY,
    fetched_at  DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP,
    item_count  INTEGER NOT NULL DEFAULT 0
);
