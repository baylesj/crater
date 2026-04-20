//! Track upsert, status transitions, and queries.
//!
//! `StoredTrack` is a flattened join of `tracks` + `track_status`.
//! The `status` and `status_note` fields are `None` when no status row exists.

use sqlx::SqlitePool;

use crate::error::{CoreError, Result};

// ── Types ────────────────────────────────────────────────────────────────────

#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TrackStatus {
    Queued,
    Rejected,
    Hearted,
    Exported,
}

impl TrackStatus {
    pub fn as_str(&self) -> &'static str {
        match self {
            TrackStatus::Queued   => "queued",
            TrackStatus::Rejected => "rejected",
            TrackStatus::Hearted  => "hearted",
            TrackStatus::Exported => "exported",
        }
    }
}

impl std::str::FromStr for TrackStatus {
    type Err = CoreError;
    fn from_str(s: &str) -> Result<Self> {
        match s {
            "queued"   => Ok(Self::Queued),
            "rejected" => Ok(Self::Rejected),
            "hearted"  => Ok(Self::Hearted),
            "exported" => Ok(Self::Exported),
            other      => Err(CoreError::InvalidStatus(other.to_owned())),
        }
    }
}

/// A track row joined with its optional status. Returned by all query functions.
#[derive(Debug, Clone, sqlx::FromRow)]
pub struct StoredTrack {
    pub id:            i64,
    pub title:         Option<String>,
    pub artist:        Option<String>,
    pub artist_sc_id:  Option<i64>,
    pub permalink_url: String,
    pub duration_ms:   Option<i64>,
    pub bpm:           Option<f64>,
    pub genre:         Option<String>,
    pub tag_list:      Option<String>,
    pub playback_count: Option<i64>,
    pub likes_count:   Option<i64>,
    pub reposts_count: Option<i64>,
    pub comment_count: Option<i64>,
    pub created_at_sc: Option<String>,
    pub first_seen:    chrono::DateTime<chrono::Utc>,
    pub last_seen:     chrono::DateTime<chrono::Utc>,
    pub raw_json:      Option<String>,
    // From LEFT JOIN track_status:
    pub status:        Option<String>,
    pub status_note:   Option<String>,
}

impl StoredTrack {
    pub fn track_status(&self) -> Option<TrackStatus> {
        self.status.as_deref().and_then(|s| s.parse().ok())
    }

    /// True for statuses that mean "don't show in new digs".
    pub fn is_suppressed(&self) -> bool {
        matches!(
            self.track_status(),
            Some(TrackStatus::Rejected) | Some(TrackStatus::Exported)
        )
    }

    pub fn engagement_ratio(&self) -> Option<f64> {
        match (self.likes_count, self.playback_count) {
            (Some(l), Some(p)) if p > 0 => Some(l as f64 / p as f64),
            _ => None,
        }
    }

    /// log(likes+1) / sqrt(plays+1) — balances engagement and obscurity.
    pub fn score(&self) -> Option<f64> {
        match (self.likes_count, self.playback_count) {
            (Some(l), Some(p)) => {
                Some((l as f64 + 1.0).ln() / (p as f64 + 1.0).sqrt())
            }
            _ => None,
        }
    }
}

// ── Queries ──────────────────────────────────────────────────────────────────

pub async fn upsert_track(pool: &SqlitePool, track: &sc_client::Track) -> Result<()> {
    let raw_json      = serde_json::to_string(track)?;
    let artist        = track.user.as_ref().and_then(|u| u.username.as_deref());
    let artist_sc_id  = track.user.as_ref().map(|u| u.id as i64);

    sqlx::query(r#"
        INSERT INTO tracks (
            id, title, artist, artist_sc_id, permalink_url,
            duration_ms, bpm, genre, tag_list,
            playback_count, likes_count, reposts_count, comment_count,
            created_at_sc, raw_json, last_seen
        ) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, CURRENT_TIMESTAMP)
        ON CONFLICT(id) DO UPDATE SET
            title          = excluded.title,
            artist         = excluded.artist,
            artist_sc_id   = excluded.artist_sc_id,
            permalink_url  = excluded.permalink_url,
            duration_ms    = excluded.duration_ms,
            bpm            = excluded.bpm,
            genre          = excluded.genre,
            tag_list       = excluded.tag_list,
            playback_count = excluded.playback_count,
            likes_count    = excluded.likes_count,
            reposts_count  = excluded.reposts_count,
            comment_count  = excluded.comment_count,
            created_at_sc  = excluded.created_at_sc,
            raw_json       = excluded.raw_json,
            last_seen      = CURRENT_TIMESTAMP
    "#)
    .bind(track.id as i64)
    .bind(track.title.as_deref())
    .bind(artist)
    .bind(artist_sc_id)
    .bind(track.permalink_url.as_deref().unwrap_or(""))
    .bind(track.duration.map(|d| d as i64))
    .bind(track.bpm)
    .bind(track.genre.as_deref())
    .bind(track.tag_list.as_deref())
    .bind(track.playback_count.map(|c| c as i64))
    .bind(track.likes_count.map(|c| c as i64))
    .bind(track.reposts_count.map(|c| c as i64))
    .bind(track.comment_count.map(|c| c as i64))
    .bind(track.created_at.as_deref())
    .bind(&raw_json)
    .execute(pool)
    .await?;

    Ok(())
}

pub async fn set_status(
    pool:     &SqlitePool,
    track_id: i64,
    status:   &TrackStatus,
    note:     Option<&str>,
) -> Result<()> {
    sqlx::query(r#"
        INSERT INTO track_status (track_id, status, note, updated_at)
        VALUES (?, ?, ?, CURRENT_TIMESTAMP)
        ON CONFLICT(track_id) DO UPDATE SET
            status     = excluded.status,
            note       = excluded.note,
            updated_at = CURRENT_TIMESTAMP
    "#)
    .bind(track_id)
    .bind(status.as_str())
    .bind(note)
    .execute(pool)
    .await?;
    Ok(())
}

pub async fn clear_status(pool: &SqlitePool, track_id: i64) -> Result<()> {
    sqlx::query("DELETE FROM track_status WHERE track_id = ?")
        .bind(track_id)
        .execute(pool)
        .await?;
    Ok(())
}

pub async fn get_track(pool: &SqlitePool, track_id: i64) -> Result<Option<StoredTrack>> {
    Ok(sqlx::query_as::<_, StoredTrack>(r#"
        SELECT t.id, t.title, t.artist, t.artist_sc_id, t.permalink_url,
               t.duration_ms, t.bpm, t.genre, t.tag_list,
               t.playback_count, t.likes_count, t.reposts_count, t.comment_count,
               t.created_at_sc, t.first_seen, t.last_seen, t.raw_json,
               ts.status, ts.note AS status_note
        FROM tracks t
        LEFT JOIN track_status ts ON ts.track_id = t.id
        WHERE t.id = ?
    "#)
    .bind(track_id)
    .fetch_optional(pool)
    .await?)
}

pub async fn tracks_with_status(
    pool:   &SqlitePool,
    status: &TrackStatus,
) -> Result<Vec<StoredTrack>> {
    Ok(sqlx::query_as::<_, StoredTrack>(r#"
        SELECT t.id, t.title, t.artist, t.artist_sc_id, t.permalink_url,
               t.duration_ms, t.bpm, t.genre, t.tag_list,
               t.playback_count, t.likes_count, t.reposts_count, t.comment_count,
               t.created_at_sc, t.first_seen, t.last_seen, t.raw_json,
               ts.status, ts.note AS status_note
        FROM tracks t
        JOIN track_status ts ON ts.track_id = t.id
        WHERE ts.status = ?
        ORDER BY ts.updated_at DESC
    "#)
    .bind(status.as_str())
    .fetch_all(pool)
    .await?)
}
