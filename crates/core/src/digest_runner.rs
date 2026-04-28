//! Executes a single digest: search → rank → (export TODO) → mark exported → log.
//!
//! Playlist creation (step 5 in the design doc) is stubbed — it requires an
//! OAuth token and `sc_client::playlist` module which are not yet implemented.
//! Everything else runs: search, rank, mark tracks as exported, write run log.

use sqlx::SqlitePool;

use crate::digests::{get_digest, next_run_utc};
use crate::error::{CoreError, Result};
use crate::filters::rank_tracks;
use crate::session::Session;
use crate::tracks::{set_status, StoredTrack, TrackStatus};

// ── Public types ─────────────────────────────────────────────────────────────

#[derive(Debug, Clone, serde::Serialize)]
pub struct DigestRun {
    pub id:            i64,
    pub digest_id:     Option<i64>,
    pub digest_name:   String,
    pub status:        RunStatus,
    pub playlist_url:  Option<String>,
    pub track_count:   Option<i64>,
    pub pages_scanned: Option<i64>,
    pub error:         Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize)]
#[serde(rename_all = "snake_case")]
pub enum RunStatus {
    Running,
    Success,
    Failed,
}

impl RunStatus {
    pub fn as_str(&self) -> &'static str {
        match self {
            RunStatus::Running => "running",
            RunStatus::Success => "success",
            RunStatus::Failed  => "failed",
        }
    }
}

// ── Entry point ───────────────────────────────────────────────────────────────

pub async fn run_digest(
    pool:        &SqlitePool,
    sc:          &sc_client::Client,
    digest_id:   i64,
    oauth_token: Option<&str>,
) -> Result<DigestRun> {
    let digest = get_digest(pool, digest_id).await?.ok_or(CoreError::NotFound)?;

    // Insert the run record immediately so the UI can see it's in progress.
    let run_id = sqlx::query(
        "INSERT INTO digest_runs (digest_id, digest_name, status) VALUES (?, ?, 'running')"
    )
    .bind(digest_id)
    .bind(&digest.spec.name)
    .execute(pool)
    .await?
    .last_insert_rowid();

    match run_inner(pool, sc, &digest, run_id, oauth_token).await {
        Ok(run) => Ok(run),
        Err(e) => {
            let msg = e.to_string();
            // Best-effort: persist the failure. Don't mask the original error.
            sqlx::query(
                "UPDATE digest_runs SET status='failed', finished_at=CURRENT_TIMESTAMP, error=? WHERE id=?"
            )
            .bind(&msg)
            .bind(run_id)
            .execute(pool)
            .await
            .ok();
            Err(e)
        }
    }
}

// ── Title template ────────────────────────────────────────────────────────────

/// Expand a playlist title template with the current UTC date.
///
/// Supported tokens: `{name}` (digest name), `{date}` (YYYY-MM-DD),
/// `{year}`, `{month}` (zero-padded), `{week}` (ISO week, zero-padded).
fn render_title(tmpl: &str, digest_name: &str) -> String {
    use chrono::Utc;
    let now = Utc::now();
    tmpl
        .replace("{name}",  digest_name)
        .replace("{date}",  &now.format("%Y-%m-%d").to_string())
        .replace("{year}",  &now.format("%Y").to_string())
        .replace("{month}", &now.format("%m").to_string())
        .replace("{week}",  &now.format("%V").to_string())
}

// ── Inner implementation ──────────────────────────────────────────────────────

async fn run_inner(
    pool:        &SqlitePool,
    sc:          &sc_client::Client,
    digest:      &crate::digests::Digest,
    run_id:      i64,
    oauth_token: Option<&str>,
) -> Result<DigestRun> {
    let spec = &digest.spec;

    // Step 3: search via Session
    let mut session = Session::new(spec.filters.clone(), pool.clone(), sc.clone());
    let batch = session
        .next_batch(spec.target_size as usize, spec.max_pages)
        .await?;

    // Step 4: rank and truncate
    let selected: Vec<StoredTrack> = rank_tracks(batch.tracks, &spec.ranking)
        .into_iter()
        .take(spec.target_size as usize)
        .collect();

    // Step 5: create the SoundCloud playlist.
    // Fail the run clearly if no OAuth token is configured — silent skips
    // would leave users wondering why digests produce no output.
    let title = render_title(&spec.playlist_title_tmpl, &spec.name);
    let token = oauth_token.ok_or_else(|| crate::error::CoreError::Other(
        anyhow::anyhow!(
            "SoundCloud OAuth token not configured — save a token in Settings \
             or set CRATER_SC_OAUTH_TOKEN"
        )
    ))?;
    let sc_ids: Vec<u64> = selected.iter().map(|t| t.id as u64).collect();
    let playlist = sc
        .create_playlist(token, &title, spec.playlist_visibility.as_str(), &sc_ids)
        .await
        .map_err(crate::error::CoreError::Sc)?;

    let playlist_url   = playlist.permalink_url.clone();
    let playlist_sc_id = Some(playlist.id as i64);
    tracing::info!(
        digest  = %spec.name,
        tracks  = selected.len(),
        url     = ?playlist_url,
        "playlist created"
    );

    // Step 6: mark tracks as exported
    for track in &selected {
        set_status(pool, track.id, &TrackStatus::Exported, None).await?;
    }

    // Step 7: record which tracks went into this run
    for (rank, track) in selected.iter().enumerate() {
        sqlx::query(
            "INSERT INTO digest_run_tracks (run_id, track_id, rank) VALUES (?, ?, ?)"
        )
        .bind(run_id)
        .bind(track.id)
        .bind(rank as i64)
        .execute(pool)
        .await?;
    }

    // Step 8: update the run record
    let tracks_json = serde_json::to_string(&selected.iter().map(|t| t.id).collect::<Vec<_>>())?;
    sqlx::query(r#"
        UPDATE digest_runs SET
            status         = 'success',
            finished_at    = CURRENT_TIMESTAMP,
            playlist_sc_id = ?,
            playlist_url   = ?,
            track_count    = ?,
            pages_scanned  = ?,
            tracks_json    = ?
        WHERE id = ?
    "#)
    .bind(playlist_sc_id)
    .bind(playlist_url.as_deref())
    .bind(selected.len() as i64)
    .bind(batch.pages_scanned as i64)
    .bind(&tracks_json)
    .bind(run_id)
    .execute(pool)
    .await?;

    // Step 9: advance the digest schedule
    let next_run = next_run_utc(&spec.cron_expr).ok();
    sqlx::query(
        "UPDATE digests SET last_run_at=CURRENT_TIMESTAMP, next_run_at=? WHERE id=?"
    )
    .bind(next_run)
    .bind(digest.id)
    .execute(pool)
    .await?;

    // Step 10: ntfy notification — TODO when server config is available

    Ok(DigestRun {
        id:            run_id,
        digest_id:     Some(digest.id),
        digest_name:   spec.name.clone(),
        status:        RunStatus::Success,
        playlist_url,
        track_count:   Some(selected.len() as i64),
        pages_scanned: Some(batch.pages_scanned as i64),
        error:         None,
    })
}
