//! Digest CRUD and cron schedule helpers.
//!
//! A `Digest` is a saved search configuration that runs on a schedule and
//! exports its results to a SoundCloud playlist. This module owns the DB
//! operations; `digest_runner` handles execution.

use std::str::FromStr;

use chrono::{DateTime, Utc};
use sqlx::SqlitePool;

use crate::error::{CoreError, Result};
use crate::filters::Ranking;

// ── Public types ─────────────────────────────────────────────────────────────

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct DigestSpec {
    pub name:                 String,
    pub filters:              sc_client::SearchFilters,
    pub ranking:              Ranking,
    pub cron_expr:            String,
    /// IANA timezone name, e.g. `"America/Los_Angeles"`. Currently stored
    /// but not applied to cron evaluation (UTC used). TODO: chrono-tz.
    pub timezone:             String,
    pub target_size:          u32,
    pub max_pages:            u32,
    pub playlist_visibility:  PlaylistVisibility,
    pub playlist_title_tmpl:  String,
}

#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PlaylistVisibility {
    Private,
    Public,
}

impl PlaylistVisibility {
    pub fn as_str(&self) -> &'static str {
        match self {
            PlaylistVisibility::Private => "private",
            PlaylistVisibility::Public  => "public",
        }
    }
}

impl FromStr for PlaylistVisibility {
    type Err = CoreError;
    fn from_str(s: &str) -> Result<Self> {
        match s {
            "private" => Ok(Self::Private),
            "public"  => Ok(Self::Public),
            other     => Err(CoreError::InvalidRanking(format!("invalid visibility: {other}"))),
        }
    }
}

#[derive(Debug, Clone)]
pub struct Digest {
    pub id:          i64,
    pub spec:        DigestSpec,
    pub enabled:     bool,
    pub last_run_at: Option<DateTime<Utc>>,
    pub next_run_at: Option<DateTime<Utc>>,
    pub created_at:  DateTime<Utc>,
}

// ── Cron helpers ─────────────────────────────────────────────────────────────

/// Parse `cron_expr` and return the next run time in UTC.
///
/// Timezone is currently ignored — evaluation is always in UTC. When
/// `chrono-tz` is added, pass `digest.spec.timezone` here instead.
pub fn next_run_utc(cron_expr: &str) -> Result<DateTime<Utc>> {
    use cron::Schedule;
    let schedule = Schedule::from_str(cron_expr).map_err(|e| CoreError::InvalidCron {
        expr:   cron_expr.to_owned(),
        detail: e.to_string(),
    })?;
    schedule.upcoming(Utc).next().ok_or_else(|| CoreError::InvalidCron {
        expr:   cron_expr.to_owned(),
        detail: "no upcoming occurrences".to_owned(),
    })
}

// ── CRUD ─────────────────────────────────────────────────────────────────────

pub async fn create_digest(pool: &SqlitePool, spec: &DigestSpec) -> Result<Digest> {
    let next_run    = next_run_utc(&spec.cron_expr)?;
    let query_json  = serde_json::to_string(&spec.filters)?;

    let id = sqlx::query(r#"
        INSERT INTO digests (
            name, query_json, ranking, cron_expr, timezone,
            target_size, max_pages, playlist_visibility, playlist_title_tmpl,
            next_run_at
        ) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?)
    "#)
    .bind(&spec.name)
    .bind(&query_json)
    .bind(spec.ranking.as_str())
    .bind(&spec.cron_expr)
    .bind(&spec.timezone)
    .bind(spec.target_size  as i64)
    .bind(spec.max_pages    as i64)
    .bind(spec.playlist_visibility.as_str())
    .bind(&spec.playlist_title_tmpl)
    .bind(next_run)
    .execute(pool)
    .await?
    .last_insert_rowid();

    get_digest(pool, id).await?.ok_or(CoreError::NotFound)
}

pub async fn list_digests(pool: &SqlitePool) -> Result<Vec<Digest>> {
    sqlx::query_as::<_, DigestRow>("SELECT * FROM digests ORDER BY created_at DESC")
        .fetch_all(pool)
        .await?
        .into_iter()
        .map(digest_from_row)
        .collect()
}

pub async fn get_digest(pool: &SqlitePool, id: i64) -> Result<Option<Digest>> {
    sqlx::query_as::<_, DigestRow>("SELECT * FROM digests WHERE id = ?")
        .bind(id)
        .fetch_optional(pool)
        .await?
        .map(digest_from_row)
        .transpose()
}

pub async fn update_digest(pool: &SqlitePool, id: i64, spec: &DigestSpec) -> Result<Digest> {
    let next_run   = next_run_utc(&spec.cron_expr)?;
    let query_json = serde_json::to_string(&spec.filters)?;

    sqlx::query(r#"
        UPDATE digests SET
            name = ?, query_json = ?, ranking = ?, cron_expr = ?, timezone = ?,
            target_size = ?, max_pages = ?, playlist_visibility = ?,
            playlist_title_tmpl = ?, next_run_at = ?
        WHERE id = ?
    "#)
    .bind(&spec.name)
    .bind(&query_json)
    .bind(spec.ranking.as_str())
    .bind(&spec.cron_expr)
    .bind(&spec.timezone)
    .bind(spec.target_size as i64)
    .bind(spec.max_pages   as i64)
    .bind(spec.playlist_visibility.as_str())
    .bind(&spec.playlist_title_tmpl)
    .bind(next_run)
    .bind(id)
    .execute(pool)
    .await?;

    get_digest(pool, id).await?.ok_or(CoreError::NotFound)
}

pub async fn delete_digest(pool: &SqlitePool, id: i64) -> Result<()> {
    sqlx::query("DELETE FROM digests WHERE id = ?")
        .bind(id)
        .execute(pool)
        .await?;
    Ok(())
}

// ── Internal row mapping ─────────────────────────────────────────────────────

#[derive(sqlx::FromRow)]
struct DigestRow {
    id:                  i64,
    name:                String,
    query_json:          String,
    ranking:             String,
    cron_expr:           String,
    timezone:            String,
    target_size:         i64,
    max_pages:           i64,
    playlist_visibility: String,
    playlist_title_tmpl: String,
    enabled:             i64,
    last_run_at:         Option<DateTime<Utc>>,
    next_run_at:         Option<DateTime<Utc>>,
    created_at:          DateTime<Utc>,
}

fn digest_from_row(row: DigestRow) -> Result<Digest> {
    Ok(Digest {
        id: row.id,
        spec: DigestSpec {
            name:                row.name,
            filters:             serde_json::from_str(&row.query_json)?,
            ranking:             row.ranking.parse()?,
            cron_expr:           row.cron_expr,
            timezone:            row.timezone,
            target_size:         row.target_size as u32,
            max_pages:           row.max_pages   as u32,
            playlist_visibility: row.playlist_visibility.parse()?,
            playlist_title_tmpl: row.playlist_title_tmpl,
        },
        enabled:     row.enabled != 0,
        last_run_at: row.last_run_at,
        next_run_at: row.next_run_at,
        created_at:  row.created_at,
    })
}

// ── Tests ────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn next_run_utc_parses_valid_expr() {
        // "at 06:00 every Sunday"
        let dt = next_run_utc("0 0 6 * * SUN").expect("valid cron");
        assert!(dt > Utc::now());
    }

    #[test]
    fn next_run_utc_rejects_garbage() {
        assert!(next_run_utc("not a cron expression").is_err());
    }
}
