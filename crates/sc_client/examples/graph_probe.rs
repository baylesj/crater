//! Probe the social-graph endpoints for one track.
//!
//! Run with:
//!   cargo run -p sc_client --example graph_probe -- <track_id>
//!
//! Prints the track's likers and reposters, then the most recent likes and
//! reposts of the first liker. Useful for checking that SoundCloud hasn't
//! changed the shape of these endpoints.

use sc_client::Client;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "sc_client=info".into()),
        )
        .with_target(false)
        .init();

    let track_id: u64 = std::env::args()
        .nth(1)
        .ok_or_else(|| anyhow::anyhow!("usage: graph_probe <track_id>"))?
        .parse()?;
    let client = Client::new()?;

    let likers = client.track_likers(track_id, 100).await?;
    let reposters = client.track_reposters(track_id, 100).await?;
    println!("{} likers, {} reposters", likers.len(), reposters.len());
    for u in likers.iter().take(8) {
        println!(
            "  liker {:>12}  {:<24} likes={:?} followers={:?} tracks={:?}",
            u.id,
            u.username.as_deref().unwrap_or("?"),
            u.likes_count,
            u.followers_count,
            u.track_count,
        );
    }
    for u in reposters.iter().take(4) {
        println!("  reposter {:>9}  {}", u.id, u.username.as_deref().unwrap_or("?"));
    }

    let Some(first) = likers.first() else { return Ok(()) };
    let likes = client.user_track_likes(first.id, 20).await?;
    let reposts = client.user_track_reposts(first.id, 20).await?;
    println!("\nuser {}: {} recent likes, {} recent reposts", first.id, likes.len(), reposts.len());
    for t in likes.iter().take(8) {
        println!(
            "  {:>8} plays  {}",
            t.playback_count.unwrap_or(0),
            t.title.as_deref().unwrap_or("?"),
        );
    }
    Ok(())
}
