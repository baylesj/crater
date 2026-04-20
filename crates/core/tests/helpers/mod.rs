//! Shared test helpers for crater-core integration tests.

use sqlx::SqlitePool;

/// Open a fresh in-memory database with all migrations applied.
pub async fn open() -> SqlitePool {
    crater_core::db::open_in_memory().await.expect("in-memory db failed")
}

#[allow(dead_code)]
/// Build a minimal `sc_client::Track` for test inserts.
///
/// Only `id`, `title`, `playback_count`, and `likes_count` are set — the
/// rest default to `None`, which is the realistic case for partial API
/// responses and what the upsert path must handle.
pub fn make_track(id: u64, title: &str, plays: u64, likes: u64) -> sc_client::Track {
    sc_client::Track {
        id,
        title:          Some(title.to_owned()),
        permalink_url:  Some(format!("https://soundcloud.com/test/track-{id}")),
        playback_count: Some(plays),
        likes_count:    Some(likes),
        duration:       Some(180_000), // 3 min
        bpm:            None,
        genre:          None,
        tag_list:       None,
        reposts_count:  None,
        comment_count:  None,
        created_at:     None,
        user:           None,
        access:         None,
    }
}
