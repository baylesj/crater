//! Integration tests for track upsert, status transitions, and queries.
//!
//! All tests use an in-memory SQLite database so they're fast and isolated.

use crater_core::TrackStatus;

mod helpers;
use helpers::{make_track, open};

// ── upsert_track ─────────────────────────────────────────────────────────────

#[tokio::test]
async fn upsert_inserts_new_track() {
    let pool = open().await;
    upsert(&pool, 1, "Ghost in the Shell", 500, 50).await;

    let track = crater_core::get_track(&pool, 1).await.unwrap().expect("track should exist");
    assert_eq!(track.id, 1);
    assert_eq!(track.title.as_deref(), Some("Ghost in the Shell"));
    assert!(track.status.is_none());
}

#[tokio::test]
async fn upsert_refreshes_play_count() {
    let pool = open().await;
    upsert(&pool, 1, "Track A", 100, 10).await;
    upsert(&pool, 1, "Track A", 250, 10).await; // play count goes up

    let track = crater_core::get_track(&pool, 1).await.unwrap().unwrap();
    assert_eq!(track.playback_count, Some(250));
}

#[tokio::test]
async fn get_track_returns_none_for_unknown_id() {
    let pool = open().await;
    let result = crater_core::get_track(&pool, 9999).await.unwrap();
    assert!(result.is_none());
}

// ── set_status / clear_status ─────────────────────────────────────────────────

#[tokio::test]
async fn set_status_stores_queued() {
    let pool = open().await;
    upsert(&pool, 1, "Queued Track", 10, 2).await;
    crater_core::set_status(&pool, 1, &TrackStatus::Queued, None).await.unwrap();

    let track = crater_core::get_track(&pool, 1).await.unwrap().unwrap();
    assert_eq!(track.track_status(), Some(TrackStatus::Queued));
    assert!(!track.is_suppressed());
}

#[tokio::test]
async fn set_status_updates_existing_status() {
    let pool = open().await;
    upsert(&pool, 1, "Track", 10, 2).await;
    crater_core::set_status(&pool, 1, &TrackStatus::Queued, None).await.unwrap();
    crater_core::set_status(&pool, 1, &TrackStatus::Hearted, Some("love it")).await.unwrap();

    let track = crater_core::get_track(&pool, 1).await.unwrap().unwrap();
    assert_eq!(track.track_status(), Some(TrackStatus::Hearted));
    assert_eq!(track.status_note.as_deref(), Some("love it"));
}

#[tokio::test]
async fn clear_status_removes_status_row() {
    let pool = open().await;
    upsert(&pool, 1, "Track", 10, 2).await;
    crater_core::set_status(&pool, 1, &TrackStatus::Rejected, None).await.unwrap();
    crater_core::clear_status(&pool, 1).await.unwrap();

    let track = crater_core::get_track(&pool, 1).await.unwrap().unwrap();
    assert!(track.status.is_none());
    assert!(!track.is_suppressed());
}

#[tokio::test]
async fn clear_status_on_track_with_no_status_is_ok() {
    let pool = open().await;
    upsert(&pool, 1, "Track", 10, 2).await;
    // Should not error even though there's nothing to clear.
    crater_core::clear_status(&pool, 1).await.unwrap();
}

// ── is_suppressed ─────────────────────────────────────────────────────────────

#[tokio::test]
async fn rejected_is_suppressed() {
    let pool = open().await;
    upsert(&pool, 1, "Track", 10, 2).await;
    crater_core::set_status(&pool, 1, &TrackStatus::Rejected, None).await.unwrap();

    let track = crater_core::get_track(&pool, 1).await.unwrap().unwrap();
    assert!(track.is_suppressed());
}

#[tokio::test]
async fn exported_is_suppressed() {
    let pool = open().await;
    upsert(&pool, 1, "Track", 10, 2).await;
    crater_core::set_status(&pool, 1, &TrackStatus::Exported, None).await.unwrap();

    let track = crater_core::get_track(&pool, 1).await.unwrap().unwrap();
    assert!(track.is_suppressed());
}

#[tokio::test]
async fn hearted_is_not_suppressed() {
    let pool = open().await;
    upsert(&pool, 1, "Track", 10, 2).await;
    crater_core::set_status(&pool, 1, &TrackStatus::Hearted, None).await.unwrap();

    let track = crater_core::get_track(&pool, 1).await.unwrap().unwrap();
    assert!(!track.is_suppressed());
}

#[tokio::test]
async fn queued_is_not_suppressed() {
    let pool = open().await;
    upsert(&pool, 1, "Track", 10, 2).await;
    crater_core::set_status(&pool, 1, &TrackStatus::Queued, None).await.unwrap();

    let track = crater_core::get_track(&pool, 1).await.unwrap().unwrap();
    assert!(!track.is_suppressed());
}

// ── tracks_with_status ────────────────────────────────────────────────────────

#[tokio::test]
async fn tracks_with_status_returns_only_matching() {
    let pool = open().await;
    upsert(&pool, 1, "Queued A",   10, 1).await;
    upsert(&pool, 2, "Queued B",   20, 2).await;
    upsert(&pool, 3, "Rejected C", 30, 3).await;
    crater_core::set_status(&pool, 1, &TrackStatus::Queued,   None).await.unwrap();
    crater_core::set_status(&pool, 2, &TrackStatus::Queued,   None).await.unwrap();
    crater_core::set_status(&pool, 3, &TrackStatus::Rejected, None).await.unwrap();

    let queued = crater_core::tracks_with_status(&pool, &TrackStatus::Queued).await.unwrap();
    assert_eq!(queued.len(), 2);
    assert!(queued.iter().all(|t| t.track_status() == Some(TrackStatus::Queued)));

    let rejected = crater_core::tracks_with_status(&pool, &TrackStatus::Rejected).await.unwrap();
    assert_eq!(rejected.len(), 1);
    assert_eq!(rejected[0].id, 3);
}

#[tokio::test]
async fn tracks_with_status_empty_when_none_match() {
    let pool = open().await;
    upsert(&pool, 1, "Track", 10, 2).await;
    // No status set at all.
    let exported = crater_core::tracks_with_status(&pool, &TrackStatus::Exported).await.unwrap();
    assert!(exported.is_empty());
}

// ── engagement_ratio / score ──────────────────────────────────────────────────

#[tokio::test]
async fn engagement_ratio_computed_correctly() {
    let pool = open().await;
    upsert(&pool, 1, "Track", 200, 20).await;

    let track = crater_core::get_track(&pool, 1).await.unwrap().unwrap();
    let ratio = track.engagement_ratio().expect("should have ratio");
    assert!((ratio - 0.1).abs() < 1e-9);
}

#[tokio::test]
async fn score_nonzero_for_valid_track() {
    let pool = open().await;
    upsert(&pool, 1, "Track", 100, 10).await;

    let track = crater_core::get_track(&pool, 1).await.unwrap().unwrap();
    assert!(track.score().expect("should have score") > 0.0);
}

// ── helpers ───────────────────────────────────────────────────────────────────

async fn upsert(pool: &sqlx::SqlitePool, id: u64, title: &str, plays: u64, likes: u64) {
    crater_core::upsert_track(pool, &make_track(id, title, plays, likes))
        .await
        .expect("upsert failed");
}
