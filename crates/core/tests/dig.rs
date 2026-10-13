//! Integration tests for graph-crawl scoring.
//!
//! Every edge list is pre-marked as freshly fetched in `crawl_log`, so the
//! crawl never touches the network and these test scoring over a hand-built
//! graph.

use crater_core::{dig::dig, DigParams, TrackStatus};
use sqlx::SqlitePool;

mod helpers;
use helpers::{make_track, open};

async fn track(pool: &SqlitePool, id: u64, plays: u64) {
    let mut t = make_track(id, &format!("track {id}"), plays, 10);
    t.user = Some(sc_client::User {
        id:               1000 + id,
        username:         Some(format!("artist {id}")),
        permalink_url:    None,
        followers_count:  None,
        avatar_url:       None,
        likes_count:      None,
        followings_count: None,
        track_count:      None,
        reposts_count:    None,
    });
    crater_core::upsert_track(pool, &t).await.unwrap();
}

async fn user(pool: &SqlitePool, id: i64, likes: i64) {
    sqlx::query("INSERT INTO sc_users (id, username, likes_count) VALUES (?, ?, ?)")
        .bind(id)
        .bind(format!("user {id}"))
        .bind(likes)
        .execute(pool)
        .await
        .unwrap();
    for list in ["likes", "reposts"] {
        fresh(pool, &format!("user:{id}:{list}")).await;
    }
}

async fn like(pool: &SqlitePool, user_id: i64, track_id: i64) {
    sqlx::query("INSERT INTO endorsements (user_id, track_id, kind) VALUES (?, ?, 'like')")
        .bind(user_id)
        .bind(track_id)
        .execute(pool)
        .await
        .unwrap();
}

async fn fresh(pool: &SqlitePool, subject: &str) {
    sqlx::query("INSERT INTO crawl_log (subject) VALUES (?)")
        .bind(subject)
        .execute(pool)
        .await
        .unwrap();
}

async fn status(pool: &SqlitePool, id: i64, s: TrackStatus) {
    crater_core::set_status(pool, id, &s, None).await.unwrap();
}

/// Seeds 1 and 2 (hearted); 103 rejected. Curators:
/// - 10: 500 likes, liked both seeds → bridges them, strongest.
/// - 11: 500 likes, liked seed 1, but also liked rejected track 103.
/// - 12: 20,000 likes, liked seed 1 → likes everything, weak.
/// - 13: liked no seed → not a curator.
async fn graph(pool: &SqlitePool) {
    for id in [1, 2, 100, 101, 102, 103, 104] {
        track(pool, id, 50).await;
    }
    status(pool, 1, TrackStatus::Hearted).await;
    status(pool, 2, TrackStatus::Hearted).await;
    status(pool, 103, TrackStatus::Rejected).await;
    for seed in [1, 2] {
        fresh(pool, &format!("track:{seed}:likers")).await;
        fresh(pool, &format!("track:{seed}:reposters")).await;
    }
    for (id, likes) in [(10, 500), (11, 500), (12, 20_000), (13, 500)] {
        user(pool, id, likes).await;
    }
    for (u, t) in [(10, 1), (10, 2), (10, 100), (11, 1), (11, 101), (11, 103), (12, 1), (12, 102), (13, 104)] {
        like(pool, u, t).await;
    }
}

async fn run(pool: &SqlitePool) -> crater_core::DigOutcome {
    let sc = sc_client::Client::with_client_id("offline".into()).unwrap();
    dig(pool, &sc, &DigParams::default(), |_| {}).await.unwrap()
}

#[tokio::test]
async fn ranks_bridging_curator_first_and_selective_over_prolific() {
    let pool = open().await;
    graph(&pool).await;
    let out = run(&pool).await;

    let ids: Vec<i64> = out.results.iter().map(|r| r.track.id).collect();
    assert_eq!(ids, vec![100, 101, 102]);
    assert_eq!(out.seeds, 2);
    assert_eq!(out.requests, 0);
}

#[tokio::test]
async fn excludes_judged_tracks_and_non_curators() {
    let pool = open().await;
    graph(&pool).await;
    let ids: Vec<i64> = run(&pool).await.results.iter().map(|r| r.track.id).collect();

    assert!(!ids.contains(&103), "rejected track surfaced");
    assert!(!ids.contains(&104), "track from a non-curator surfaced");
    assert!(!ids.contains(&1) && !ids.contains(&2), "seed surfaced");
}

#[tokio::test]
async fn explains_picks() {
    let pool = open().await;
    graph(&pool).await;
    let out = run(&pool).await;
    let top = &out.results[0];

    assert_eq!(top.support, 1);
    assert_eq!(top.via[0].user_id, 10);
    let mut because = top.because.clone();
    because.sort();
    assert_eq!(because, vec!["track 1", "track 2"]);
}

#[tokio::test]
async fn play_ceiling_drops_popular_tracks() {
    let pool = open().await;
    graph(&pool).await;
    track(&pool, 100, 50_000).await; // re-upsert with more plays
    let ids: Vec<i64> = run(&pool).await.results.iter().map(|r| r.track.id).collect();

    assert!(!ids.contains(&100));
}
