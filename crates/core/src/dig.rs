//! Graph crawl: find tracks through the people who liked your tracks.
//!
//! SoundCloud search can only return what its keyword index ranks, which
//! skews popular. The people who liked or reposted a track you hearted are
//! a better index: their other likes are, by construction, things someone
//! with overlapping taste chose. This module does that walk at personal
//! scale, a small item-based collaborative filter over the local graph.
//!
//! A dig runs in four phases:
//!
//! 1. **Seeds.** Positive tracks (hearted, queued, exported). For each seed
//!    whose liker/reposter lists are stale, fetch them into `endorsements`.
//! 2. **Curators.** Score every user who endorsed a seed:
//!    `affinity^1.5 × selectivity`. Affinity counts the seeds they endorsed
//!    (reposts weigh more) minus a penalty for tracks you rejected that they
//!    endorsed; the exponent rewards people who bridge several seeds, which
//!    is the strongest taste signal there is. Selectivity discounts people
//!    who like everything (13,000 likes) and also tiny accounts, which in
//!    practice are mostly the artist's friends.
//! 3. **Expand.** Fetch recent likes and reposts of the top curators.
//! 4. **Score.** Each candidate's score is the sum of its endorsing curators'
//!    scores, times an obscurity factor (fewer plays is better) and a tag
//!    bonus against the seeds' tag profile. Tracks you've already judged are
//!    dropped, and the list is reranked so no single curator or artist
//!    fills it.
//!
//! Every fetched list is cached with a timestamp (`crawl_log`), so repeated
//! digs are cheap and the graph grows over time. Scoring always runs over the
//! whole cached graph, not only what this run fetched.
//!
//! Rate limiting: a 429 stops fetching but not the dig; scoring runs over
//! whatever is cached and the result says it was cut short.

use std::collections::{HashMap, HashSet};
use std::time::Duration;

use sqlx::SqlitePool;

use crate::error::Result;
use crate::tracks::{upsert_track, StoredTrack};

/// Delay between SoundCloud requests. A dig can make a couple of hundred,
/// so this matters more here than for search.
const REQUEST_DELAY: Duration = Duration::from_millis(250);

/// Used for selectivity when SoundCloud doesn't report a user's like count.
const UNKNOWN_LIKES: f64 = 1000.0;

const REPOST_WEIGHT: f64 = 1.5;
const LIKE_WEIGHT: f64 = 1.0;
const REJECT_PENALTY: f64 = 0.5;

/// Each further result whose strongest curator was already used is scaled
/// by this, so one prolific curator can't fill the list.
const CURATOR_REUSE_DECAY: f64 = 0.7;

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
#[serde(default)]
pub struct DigParams {
    /// Seed tracks. Empty means every hearted, queued and exported track,
    /// newest first, capped at `max_seeds`.
    pub seeds:               Vec<i64>,
    pub max_seeds:           usize,
    pub likers_per_seed:     usize,
    pub reposters_per_seed:  usize,
    /// How many of the best-scoring curators to expand this run.
    pub curators:            usize,
    pub likes_per_curator:   usize,
    pub reposts_per_curator: usize,
    /// Cached lists younger than this are reused rather than re-fetched.
    pub max_age_hours:       u32,
    /// Hard play-count ceiling on results.
    pub max_plays:           Option<i64>,
    /// Drops DJ mixes and long-form uploads.
    pub max_duration_ms:     Option<i64>,
    pub max_per_artist:      usize,
    pub limit:               usize,
}

impl Default for DigParams {
    fn default() -> Self {
        Self {
            seeds:               Vec::new(),
            max_seeds:           30,
            likers_per_seed:     200,
            reposters_per_seed:  50,
            curators:            40,
            likes_per_curator:   100,
            reposts_per_curator: 50,
            max_age_hours:       72,
            max_plays:           Some(10_000),
            max_duration_ms:     Some(12 * 60 * 1000),
            max_per_artist:      2,
            limit:               100,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize)]
#[serde(rename_all = "snake_case")]
pub enum DigPhase {
    Seeds,
    Curators,
    Scoring,
}

#[derive(Debug, Clone, serde::Serialize)]
pub struct DigProgress {
    pub phase:    DigPhase,
    pub done:     usize,
    pub total:    usize,
    /// SoundCloud requests made so far this run.
    pub requests: usize,
}

/// A curator who led to a result, for showing why a track was picked.
#[derive(Debug, Clone, serde::Serialize)]
pub struct Via {
    pub user_id:       i64,
    pub username:      Option<String>,
    pub permalink_url: Option<String>,
    pub kind:          &'static str,
}

#[derive(Debug, Clone, serde::Serialize)]
pub struct DigResult {
    pub track: StoredTrack,
    pub score: f64,
    /// Strongest curators first, at most a handful.
    pub via:   Vec<Via>,
    /// Total curators who endorsed this track.
    pub support: usize,
    /// Titles of the seeds this track was reached from, most-connected
    /// first: "because you hearted …".
    pub because: Vec<String>,
}

#[derive(Debug, Clone, serde::Serialize)]
pub struct DigOutcome {
    pub results:      Vec<DigResult>,
    pub seeds:        usize,
    pub curators:     usize,
    pub candidates:   usize,
    pub requests:     usize,
    /// True when SoundCloud rate-limited the crawl and scoring ran on
    /// partial data.
    pub rate_limited: bool,
}

pub async fn dig<F>(
    db:     &SqlitePool,
    sc:     &sc_client::Client,
    params: &DigParams,
    mut on_progress: F,
) -> Result<DigOutcome>
where
    F: FnMut(DigProgress),
{
    let mut fetcher = Fetcher { db, sc, max_age_hours: params.max_age_hours, requests: 0, rate_limited: false };

    // ── 1. Seeds ────────────────────────────────────────────────────────────
    let seeds = if params.seeds.is_empty() {
        default_seeds(db, params.max_seeds).await?
    } else {
        params.seeds.clone()
    };
    for (i, &seed) in seeds.iter().enumerate() {
        on_progress(DigProgress { phase: DigPhase::Seeds, done: i, total: seeds.len(), requests: fetcher.requests });
        fetcher.track_endorsers(seed, params.likers_per_seed, params.reposters_per_seed).await?;
    }

    // ── 2. Curators ─────────────────────────────────────────────────────────
    let graph = Graph::load(db).await?;
    let seed_set: HashSet<i64> = seeds.iter().copied().collect();
    let curator_scores = graph.curator_scores(&seed_set);
    let mut ranked: Vec<(i64, f64)> = curator_scores.iter().map(|(&u, &s)| (u, s)).collect();
    ranked.sort_by(|a, b| b.1.total_cmp(&a.1));
    let to_expand: Vec<i64> = ranked.iter().take(params.curators).map(|&(u, _)| u).collect();

    for (i, &user) in to_expand.iter().enumerate() {
        on_progress(DigProgress { phase: DigPhase::Curators, done: i, total: to_expand.len(), requests: fetcher.requests });
        fetcher.user_endorsements(user, params.likes_per_curator, params.reposts_per_curator).await?;
    }

    // ── 3–4. Score over the whole cached graph ──────────────────────────────
    on_progress(DigProgress { phase: DigPhase::Scoring, done: 0, total: 1, requests: fetcher.requests });
    let graph = Graph::load(db).await?;
    let curator_scores = graph.curator_scores(&seed_set);
    let seed_tracks = graph.tracks_by_id(db, &seeds).await?;
    let profile = TagProfile::from_tracks(&seed_tracks);
    let seed_titles: HashMap<i64, String> = seed_tracks
        .iter()
        .map(|t| (t.id, t.title.clone().unwrap_or_else(|| format!("track {}", t.id))))
        .collect();
    let curator_seeds = graph.curator_seeds(&seed_set);

    // Strongest edge per (curator, track): a like plus a repost counts once.
    let mut best: HashMap<(i64, i64), (f64, &'static str)> = HashMap::new();
    for e in &graph.edges {
        if seed_set.contains(&e.track_id) {
            continue;
        }
        let Some(&cs) = curator_scores.get(&e.user_id) else { continue };
        let w = cs * e.weight();
        let slot = best.entry((e.user_id, e.track_id)).or_insert((0.0, "like"));
        if w > slot.0 {
            *slot = (w, e.kind_str());
        }
    }
    let mut raw: HashMap<i64, (f64, Vec<(f64, i64, &'static str)>)> = HashMap::new();
    for ((user, track), (w, kind)) in best {
        let entry = raw.entry(track).or_default();
        entry.0 += w;
        entry.1.push((w, user, kind));
    }
    let candidates = raw.len();

    let mut ids: Vec<i64> = raw.keys().copied().collect();
    ids.sort_unstable();
    let tracks = graph.tracks_by_id(db, &ids).await?;

    let mut results: Vec<DigResult> = Vec::new();
    for t in tracks {
        if t.status.is_some() {
            continue;
        }
        if let (Some(max), Some(p)) = (params.max_plays, t.playback_count) {
            if p > max {
                continue;
            }
        }
        if let (Some(max), Some(d)) = (params.max_duration_ms, t.duration_ms) {
            if d > max {
                continue;
            }
        }
        let (base, mut via) = raw.remove(&t.id).unwrap_or_default();
        let score = base * obscurity(t.playback_count) * profile.bonus(&t);
        via.sort_by(|a, b| b.0.total_cmp(&a.0));
        let support = via.len();
        let mut seed_hits: HashMap<i64, f64> = HashMap::new();
        for &(w, user, _) in &via {
            for seed in curator_seeds.get(&user).into_iter().flatten() {
                *seed_hits.entry(*seed).or_default() += w;
            }
        }
        let mut because: Vec<(i64, f64)> = seed_hits.into_iter().collect();
        because.sort_by(|a, b| b.1.total_cmp(&a.1));
        let because = because
            .into_iter()
            .take(2)
            .filter_map(|(id, _)| seed_titles.get(&id).cloned())
            .collect();
        let via = via
            .into_iter()
            .take(4)
            .map(|(_, user_id, kind)| {
                let (username, permalink_url) = graph.user_names(user_id);
                Via { user_id, username, permalink_url, kind }
            })
            .collect();
        results.push(DigResult { track: t, score, via, support, because });
    }
    let results = diversify(results, params.max_per_artist, params.limit);

    on_progress(DigProgress { phase: DigPhase::Scoring, done: 1, total: 1, requests: fetcher.requests });
    Ok(DigOutcome {
        results,
        seeds: seeds.len(),
        curators: curator_scores.len(),
        candidates,
        requests: fetcher.requests,
        rate_limited: fetcher.rate_limited,
    })
}

/// Greedy rerank: repeatedly take the best remaining result, after scaling
/// each by how often its strongest curator has already been used. Artists
/// over `max_per_artist` are dropped outright. Quadratic, but over a few
/// thousand candidates at most.
fn diversify(mut pool: Vec<DigResult>, max_per_artist: usize, limit: usize) -> Vec<DigResult> {
    let mut curator_uses: HashMap<i64, i32> = HashMap::new();
    let mut artist_uses: HashMap<String, usize> = HashMap::new();
    let mut out = Vec::with_capacity(limit);
    let artist_key = |r: &DigResult| {
        r.track.artist_sc_id.map(|id| id.to_string())
            .or_else(|| r.track.artist.clone())
            .unwrap_or_default()
    };
    let adjusted = |r: &DigResult, uses: &HashMap<i64, i32>| {
        let n = r.via.first().and_then(|v| uses.get(&v.user_id)).copied().unwrap_or(0);
        r.score * CURATOR_REUSE_DECAY.powi(n)
    };
    while out.len() < limit && !pool.is_empty() {
        let (i, _) = pool
            .iter()
            .enumerate()
            .map(|(i, r)| (i, adjusted(r, &curator_uses)))
            .max_by(|a, b| a.1.total_cmp(&b.1))
            .expect("pool is non-empty");
        let r = pool.swap_remove(i);
        let n = artist_uses.entry(artist_key(&r)).or_default();
        if *n >= max_per_artist {
            continue;
        }
        *n += 1;
        if let Some(v) = r.via.first() {
            *curator_uses.entry(v.user_id).or_default() += 1;
        }
        out.push(r);
    }
    out
}

async fn default_seeds(db: &SqlitePool, max: usize) -> Result<Vec<i64>> {
    let rows: Vec<(i64,)> = sqlx::query_as(
        "SELECT track_id FROM track_status
         WHERE status IN ('hearted', 'queued', 'exported')
         ORDER BY CASE status WHEN 'hearted' THEN 0 ELSE 1 END, updated_at DESC
         LIMIT ?",
    )
    .bind(max as i64)
    .fetch_all(db)
    .await?;
    Ok(rows.into_iter().map(|(id,)| id).collect())
}

/// Penalises plays gently: 0 plays → 1.0, ~100 → 0.5, ~1k → 0.33, ~10k → 0.25.
fn obscurity(plays: Option<i64>) -> f64 {
    1.0 / (10.0 + plays.unwrap_or(0).max(0) as f64).log10()
}

/// How much a like from this user is worth. Full weight from 200 to 1,000
/// likes; linear below (tiny accounts are mostly the artist's friends), and
/// falling with the square root above (4,000 → 0.5, 16,000 → 0.25).
fn selectivity(likes_count: Option<i64>) -> f64 {
    let likes = likes_count.map(|l| l as f64).unwrap_or(UNKNOWN_LIKES).max(1.0);
    (likes / 200.0).min(1.0) * (1000.0 / likes).sqrt().min(1.0)
}

// ── Fetching ─────────────────────────────────────────────────────────────────

struct Fetcher<'a> {
    db:            &'a SqlitePool,
    sc:            &'a sc_client::Client,
    max_age_hours: u32,
    requests:      usize,
    rate_limited:  bool,
}

impl Fetcher<'_> {
    async fn track_endorsers(&mut self, track_id: i64, likers: usize, reposters: usize) -> Result<()> {
        let id = track_id as u64;
        let subject = format!("track:{track_id}:likers");
        if self.should_fetch(&subject).await? {
            let users = self.sc.track_likers(id, likers).await;
            self.requests += likers.div_ceil(50);
            if let Some(users) = self.settle(&subject, users)? {
                store_users(self.db, &users, track_id, "like", &subject).await?;
            }
        }
        let subject = format!("track:{track_id}:reposters");
        if self.should_fetch(&subject).await? {
            let users = self.sc.track_reposters(id, reposters).await;
            self.requests += reposters.div_ceil(50);
            if let Some(users) = self.settle(&subject, users)? {
                store_users(self.db, &users, track_id, "repost", &subject).await?;
            }
        }
        Ok(())
    }

    async fn user_endorsements(&mut self, user_id: i64, likes: usize, reposts: usize) -> Result<()> {
        let id = user_id as u64;
        let subject = format!("user:{user_id}:likes");
        if self.should_fetch(&subject).await? {
            let tracks = self.sc.user_track_likes(id, likes).await;
            self.requests += likes.div_ceil(50);
            if let Some(tracks) = self.settle(&subject, tracks)? {
                store_tracks(self.db, &tracks, user_id, "like", &subject).await?;
            }
        }
        let subject = format!("user:{user_id}:reposts");
        if self.should_fetch(&subject).await? {
            let tracks = self.sc.user_track_reposts(id, reposts).await;
            self.requests += reposts.div_ceil(50);
            if let Some(tracks) = self.settle(&subject, tracks)? {
                store_tracks(self.db, &tracks, user_id, "repost", &subject).await?;
            }
        }
        Ok(())
    }

    async fn should_fetch(&self, subject: &str) -> Result<bool> {
        if self.rate_limited {
            return Ok(false);
        }
        let fresh: Option<(i64,)> = sqlx::query_as(
            "SELECT 1 FROM crawl_log WHERE subject = ?
             AND fetched_at > datetime('now', '-' || ? || ' hours')",
        )
        .bind(subject)
        .bind(self.max_age_hours)
        .fetch_optional(self.db)
        .await?;
        if fresh.is_none() {
            tokio::time::sleep(REQUEST_DELAY).await;
        }
        Ok(fresh.is_none())
    }

    /// Turns a fetch result into data to store, or `None` for failures that
    /// shouldn't stop the dig. Deleted or private resources are logged as
    /// empty so they aren't retried every run.
    fn settle<T>(&mut self, subject: &str, r: sc_client::Result<Vec<T>>) -> Result<Option<Vec<T>>> {
        match r {
            Ok(v) => Ok(Some(v)),
            Err(sc_client::ScError::NotFound) => Ok(Some(Vec::new())),
            Err(sc_client::ScError::RateLimited) => {
                tracing::warn!(subject, "rate limited; scoring with cached graph");
                self.rate_limited = true;
                Ok(None)
            }
            Err(e @ (sc_client::ScError::Deserialize(_) | sc_client::ScError::Unexpected { .. })) => {
                tracing::warn!(subject, error = %e, "skipping list");
                Ok(None)
            }
            Err(e) => Err(e.into()),
        }
    }
}

async fn store_users(
    db:       &SqlitePool,
    users:    &[sc_client::User],
    track_id: i64,
    kind:     &str,
    subject:  &str,
) -> Result<()> {
    let mut tx = db.begin().await?;
    for u in users {
        upsert_user(&mut *tx, u).await?;
        insert_endorsement(&mut *tx, u.id as i64, track_id, kind).await?;
    }
    log_fetch(&mut *tx, subject, users.len()).await?;
    tx.commit().await?;
    Ok(())
}

async fn store_tracks(
    db:      &SqlitePool,
    tracks:  &[sc_client::Track],
    user_id: i64,
    kind:    &str,
    subject: &str,
) -> Result<()> {
    let mut tx = db.begin().await?;
    for t in tracks {
        upsert_track(&mut *tx, t).await?;
        insert_endorsement(&mut *tx, user_id, t.id as i64, kind).await?;
    }
    log_fetch(&mut *tx, subject, tracks.len()).await?;
    tx.commit().await?;
    Ok(())
}

async fn upsert_user<'c, E>(db: E, u: &sc_client::User) -> Result<()>
where
    E: sqlx::Executor<'c, Database = sqlx::Sqlite>,
{
    sqlx::query(
        "INSERT INTO sc_users (id, username, permalink_url, likes_count, followers_count, track_count, last_seen)
         VALUES (?, ?, ?, ?, ?, ?, CURRENT_TIMESTAMP)
         ON CONFLICT(id) DO UPDATE SET
             username        = excluded.username,
             permalink_url   = excluded.permalink_url,
             likes_count     = COALESCE(excluded.likes_count, likes_count),
             followers_count = COALESCE(excluded.followers_count, followers_count),
             track_count     = COALESCE(excluded.track_count, track_count),
             last_seen       = CURRENT_TIMESTAMP",
    )
    .bind(u.id as i64)
    .bind(u.username.as_deref())
    .bind(u.permalink_url.as_deref())
    .bind(u.likes_count.map(|c| c as i64))
    .bind(u.followers_count.map(|c| c as i64))
    .bind(u.track_count.map(|c| c as i64))
    .execute(db)
    .await?;
    Ok(())
}

async fn insert_endorsement<'c, E>(db: E, user_id: i64, track_id: i64, kind: &str) -> Result<()>
where
    E: sqlx::Executor<'c, Database = sqlx::Sqlite>,
{
    sqlx::query("INSERT OR IGNORE INTO endorsements (user_id, track_id, kind) VALUES (?, ?, ?)")
        .bind(user_id)
        .bind(track_id)
        .bind(kind)
        .execute(db)
        .await?;
    Ok(())
}

async fn log_fetch<'c, E>(db: E, subject: &str, n: usize) -> Result<()>
where
    E: sqlx::Executor<'c, Database = sqlx::Sqlite>,
{
    sqlx::query(
        "INSERT INTO crawl_log (subject, fetched_at, item_count) VALUES (?, CURRENT_TIMESTAMP, ?)
         ON CONFLICT(subject) DO UPDATE SET fetched_at = CURRENT_TIMESTAMP, item_count = excluded.item_count",
    )
    .bind(subject)
    .bind(n as i64)
    .execute(db)
    .await?;
    Ok(())
}

// ── Scoring ──────────────────────────────────────────────────────────────────

#[derive(Debug, Clone, sqlx::FromRow)]
struct Edge {
    user_id:  i64,
    track_id: i64,
    kind:     String,
}

impl Edge {
    fn weight(&self) -> f64 {
        if self.kind == "repost" { REPOST_WEIGHT } else { LIKE_WEIGHT }
    }

    fn kind_str(&self) -> &'static str {
        if self.kind == "repost" { "repost" } else { "like" }
    }
}

/// The cached graph, loaded whole. At personal scale (tens of thousands of
/// edges) this is cheaper and simpler than expressing the scorer in SQL.
struct Graph {
    edges:    Vec<Edge>,
    /// id → (username, permalink, likes_count)
    users:    HashMap<i64, (Option<String>, Option<String>, Option<i64>)>,
    rejected: HashSet<i64>,
}

impl Graph {
    async fn load(db: &SqlitePool) -> Result<Self> {
        let edges: Vec<Edge> = sqlx::query_as("SELECT user_id, track_id, kind FROM endorsements")
            .fetch_all(db)
            .await?;
        let users: Vec<(i64, Option<String>, Option<String>, Option<i64>)> =
            sqlx::query_as("SELECT id, username, permalink_url, likes_count FROM sc_users")
                .fetch_all(db)
                .await?;
        let rejected: Vec<(i64,)> =
            sqlx::query_as("SELECT track_id FROM track_status WHERE status = 'rejected'")
                .fetch_all(db)
                .await?;
        Ok(Self {
            edges,
            users: users.into_iter().map(|(id, name, url, likes)| (id, (name, url, likes))).collect(),
            rejected: rejected.into_iter().map(|(id,)| id).collect(),
        })
    }

    fn user_names(&self, id: i64) -> (Option<String>, Option<String>) {
        self.users.get(&id).map(|u| (u.0.clone(), u.1.clone())).unwrap_or_default()
    }

    /// Users with a positive score: endorsed at least one seed, and not
    /// outweighed by endorsing tracks you rejected.
    fn curator_scores(&self, seeds: &HashSet<i64>) -> HashMap<i64, f64> {
        // Best edge per (user, seed), so a like plus a repost of the same
        // seed counts once, as the repost.
        let mut per_seed: HashMap<(i64, i64), f64> = HashMap::new();
        let mut penalty: HashMap<i64, f64> = HashMap::new();
        for e in &self.edges {
            if seeds.contains(&e.track_id) {
                let w = per_seed.entry((e.user_id, e.track_id)).or_default();
                *w = w.max(e.weight());
            } else if self.rejected.contains(&e.track_id) {
                *penalty.entry(e.user_id).or_default() += REJECT_PENALTY;
            }
        }
        let mut affinity: HashMap<i64, f64> = HashMap::new();
        for ((user, _), w) in per_seed {
            *affinity.entry(user).or_default() += w;
        }
        affinity
            .into_iter()
            .filter_map(|(user, a)| {
                let a = a - penalty.get(&user).copied().unwrap_or(0.0);
                (a > 0.0).then(|| {
                    let likes = self.users.get(&user).and_then(|u| u.2);
                    (user, a.powf(1.5) * selectivity(likes))
                })
            })
            .collect()
    }

    fn curator_seeds(&self, seeds: &HashSet<i64>) -> HashMap<i64, Vec<i64>> {
        let mut out: HashMap<i64, Vec<i64>> = HashMap::new();
        for e in &self.edges {
            if seeds.contains(&e.track_id) {
                let v = out.entry(e.user_id).or_default();
                if !v.contains(&e.track_id) {
                    v.push(e.track_id);
                }
            }
        }
        out
    }

    async fn tracks_by_id(&self, db: &SqlitePool, ids: &[i64]) -> Result<Vec<StoredTrack>> {
        let mut out = Vec::with_capacity(ids.len());
        // SQLite's default bound-parameter limit is 32766; stay well under.
        for chunk in ids.chunks(500) {
            let placeholders = vec!["?"; chunk.len()].join(",");
            let sql = format!(
                "SELECT t.id, t.title, t.artist, t.artist_sc_id, t.permalink_url,
                        t.duration_ms, t.bpm, t.genre, t.tag_list,
                        t.playback_count, t.likes_count, t.reposts_count, t.comment_count,
                        t.artwork_url, t.created_at_sc, t.first_seen, t.last_seen, NULL AS raw_json,
                        ts.status, ts.note AS status_note
                 FROM tracks t
                 LEFT JOIN track_status ts ON ts.track_id = t.id
                 WHERE t.id IN ({placeholders})"
            );
            let mut q = sqlx::query_as::<_, StoredTrack>(&sql);
            for id in chunk {
                q = q.bind(id);
            }
            out.extend(q.fetch_all(db).await?);
        }
        Ok(out)
    }
}

/// Normalised tag frequencies across the seeds.
struct TagProfile {
    weights: HashMap<String, f64>,
}

impl TagProfile {
    fn from_tracks(tracks: &[StoredTrack]) -> Self {
        let mut counts: HashMap<String, f64> = HashMap::new();
        for t in tracks {
            for tag in track_tags(t) {
                *counts.entry(tag).or_default() += 1.0;
            }
        }
        let n = tracks.len().max(1) as f64;
        Self { weights: counts.into_iter().map(|(k, c)| (k, c / n)).collect() }
    }

    /// 1.0 for no overlap, up to 1.5 for a track whose tags match tags most
    /// seeds share.
    fn bonus(&self, t: &StoredTrack) -> f64 {
        let overlap: f64 = track_tags(t).iter().filter_map(|tag| self.weights.get(tag)).sum();
        1.0 + 0.5 * overlap.min(1.0)
    }
}

/// Genre plus `tag_list`, normalised so `"Drum & Bass"`, `drum-and-bass`
/// and `drumandbass` compare equal.
fn track_tags(t: &StoredTrack) -> HashSet<String> {
    let mut out = HashSet::new();
    if let Some(g) = &t.genre {
        out.insert(normalize_tag(g));
    }
    if let Some(list) = &t.tag_list {
        out.extend(parse_tag_list(list).iter().map(|s| normalize_tag(s)));
    }
    out.remove("");
    out
}

/// Splits SoundCloud's tag list: space-separated, multi-word tags quoted.
fn parse_tag_list(list: &str) -> Vec<String> {
    let mut out = Vec::new();
    let mut cur = String::new();
    let mut quoted = false;
    for c in list.chars() {
        match c {
            '"' => {
                if quoted && !cur.is_empty() {
                    out.push(std::mem::take(&mut cur));
                }
                quoted = !quoted;
            }
            c if c.is_whitespace() && !quoted => {
                if !cur.is_empty() {
                    out.push(std::mem::take(&mut cur));
                }
            }
            c => cur.push(c),
        }
    }
    if !cur.is_empty() {
        out.push(cur);
    }
    out
}

fn normalize_tag(s: &str) -> String {
    s.to_lowercase()
        .replace('&', "and")
        .chars()
        .filter(|c| c.is_alphanumeric())
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_quoted_tags() {
        let tags = parse_tag_list(r#""Future Garage" Post-dubstep "UK Garage" 2-step"#);
        assert_eq!(tags, vec!["Future Garage", "Post-dubstep", "UK Garage", "2-step"]);
    }

    #[test]
    fn normalizes_tag_spellings() {
        assert_eq!(normalize_tag("Drum & Bass"), normalize_tag("drum-and-bass"));
        assert_eq!(normalize_tag("UK Garage"), "ukgarage");
    }

    #[test]
    fn selectivity_peaks_for_mid_size_likers() {
        assert_eq!(selectivity(Some(500)), 1.0);
        assert!((selectivity(Some(50)) - 0.25).abs() < 1e-9);
        assert!((selectivity(Some(4000)) - 0.5).abs() < 1e-9);
        assert!(selectivity(Some(16_000)) < 0.26);
    }

    #[test]
    fn obscurity_prefers_fewer_plays() {
        assert!(obscurity(Some(0)) > obscurity(Some(100)));
        assert!(obscurity(Some(100)) > obscurity(Some(10_000)));
        assert!((obscurity(Some(0)) - 1.0).abs() < 1e-9);
    }
}
