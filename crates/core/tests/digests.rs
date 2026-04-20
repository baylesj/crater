//! Integration tests for digest CRUD and schedule helpers.

use crater_core::{
    digests::{create_digest, delete_digest, get_digest, list_digests, update_digest},
    filters::Ranking,
    DigestSpec, PlaylistVisibility,
};

mod helpers;
use helpers::open;

fn spec(name: &str) -> DigestSpec {
    DigestSpec {
        name:                name.to_owned(),
        filters:             sc_client::SearchFilters::default(),
        ranking:             Ranking::Score,
        cron_expr:           "0 0 6 * * SUN".to_owned(),
        timezone:            "America/Los_Angeles".to_owned(),
        target_size:         25,
        max_pages:           20,
        playlist_visibility: PlaylistVisibility::Private,
        playlist_title_tmpl: "crater — {year}-W{week}".to_owned(),
    }
}

// ── create / get ──────────────────────────────────────────────────────────────

#[tokio::test]
async fn create_and_get_roundtrip() {
    let pool = open().await;
    let digest = create_digest(&pool, &spec("Weekly DnB")).await.unwrap();

    assert!(digest.id > 0);
    assert_eq!(digest.spec.name, "Weekly DnB");
    assert_eq!(digest.spec.ranking, Ranking::Score);
    assert!(digest.enabled);
    assert!(digest.next_run_at.is_some());
    assert!(digest.last_run_at.is_none());

    let fetched = get_digest(&pool, digest.id).await.unwrap().expect("should exist");
    assert_eq!(fetched.id, digest.id);
    assert_eq!(fetched.spec.name, "Weekly DnB");
}

#[tokio::test]
async fn get_digest_returns_none_for_unknown_id() {
    let pool = open().await;
    assert!(get_digest(&pool, 9999).await.unwrap().is_none());
}

// ── list ──────────────────────────────────────────────────────────────────────

#[tokio::test]
async fn list_digests_returns_all() {
    let pool = open().await;
    create_digest(&pool, &spec("Digest A")).await.unwrap();
    create_digest(&pool, &spec("Digest B")).await.unwrap();

    let list = list_digests(&pool).await.unwrap();
    assert_eq!(list.len(), 2);
}

#[tokio::test]
async fn list_digests_empty_when_none() {
    let pool = open().await;
    let list = list_digests(&pool).await.unwrap();
    assert!(list.is_empty());
}

// ── update ────────────────────────────────────────────────────────────────────

#[tokio::test]
async fn update_digest_changes_spec() {
    let pool = open().await;
    let digest = create_digest(&pool, &spec("Original Name")).await.unwrap();

    let mut updated_spec = spec("Updated Name");
    updated_spec.target_size = 10;
    updated_spec.ranking = Ranking::Recency;

    let updated = update_digest(&pool, digest.id, &updated_spec).await.unwrap();
    assert_eq!(updated.spec.name, "Updated Name");
    assert_eq!(updated.spec.target_size, 10);
    assert_eq!(updated.spec.ranking, Ranking::Recency);
}

#[tokio::test]
async fn update_digest_recomputes_next_run() {
    let pool = open().await;
    let digest = create_digest(&pool, &spec("A")).await.unwrap();
    let original_next = digest.next_run_at;

    // Switch to a different schedule
    let mut new_spec = spec("A");
    new_spec.cron_expr = "0 0 8 * * MON".to_owned(); // Monday 8am
    let updated = update_digest(&pool, digest.id, &new_spec).await.unwrap();

    // next_run_at should still be set and in the future
    assert!(updated.next_run_at.is_some());
    // It should have changed (different cron → different next run)
    assert_ne!(updated.next_run_at, original_next);
}

// ── delete ────────────────────────────────────────────────────────────────────

#[tokio::test]
async fn delete_digest_removes_it() {
    let pool = open().await;
    let digest = create_digest(&pool, &spec("To Delete")).await.unwrap();
    delete_digest(&pool, digest.id).await.unwrap();

    assert!(get_digest(&pool, digest.id).await.unwrap().is_none());
}

#[tokio::test]
async fn delete_nonexistent_is_ok() {
    let pool = open().await;
    // Should not error — DELETE WHERE id=X with no matching row is a no-op.
    delete_digest(&pool, 9999).await.unwrap();
}

// ── filters roundtrip ─────────────────────────────────────────────────────────

#[tokio::test]
async fn filters_survive_serialization_roundtrip() {
    let pool = open().await;
    let mut s = spec("Filter Test");
    s.filters = sc_client::SearchFilters {
        query:          Some("drum and bass".to_owned()),
        bpm_from:       Some(170),
        bpm_to:         Some(178),
        max_plays:      Some(1000),
        min_likes:      Some(3),
        duration_to_ms: Some(10 * 60 * 1000),
        ..Default::default()
    };

    let digest = create_digest(&pool, &s).await.unwrap();
    let fetched = get_digest(&pool, digest.id).await.unwrap().unwrap();

    assert_eq!(fetched.spec.filters.query.as_deref(), Some("drum and bass"));
    assert_eq!(fetched.spec.filters.bpm_from, Some(170));
    assert_eq!(fetched.spec.filters.max_plays, Some(1000));
    assert_eq!(fetched.spec.filters.duration_to_ms, Some(10 * 60 * 1000));
}

// ── unique name constraint ────────────────────────────────────────────────────

#[tokio::test]
async fn duplicate_name_is_rejected() {
    let pool = open().await;
    create_digest(&pool, &spec("Same Name")).await.unwrap();
    let result = create_digest(&pool, &spec("Same Name")).await;
    assert!(result.is_err());
}
