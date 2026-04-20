//! End-to-end smoke test: search for low-play drum & bass tracks.
//!
//! Run with:
//!   cargo run -p sc_client --example search_demo
//!
//! Tweak the filters below to try other genres/ranges. This hits the real
//! SoundCloud v2 API, so expect the first run to be slower (scraping
//! `client_id`) and subsequent runs to be fast.

use sc_client::{Client, SearchFilters};

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // Pretty logs. Override with RUST_LOG=debug or RUST_LOG=trace.
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "sc_client=info,search_demo=info".into()),
        )
        .with_target(false)
        .init();

    let client = Client::new()?;

    let filters = SearchFilters {
        query:          Some("liquid".into()),
        genre_or_tag:   Some("drum & bass".into()),
        bpm_from:       Some(170),
        bpm_to:         Some(178),
        max_plays:      Some(1000),        // the magic "undiscovered" filter
        min_likes:      Some(3),           // exclude pure noise
        duration_to_ms: Some(10 * 60 * 1000), // 10 min cap — no hour-long mixes
        limit:          Some(50),
        ..Default::default()
    };

    tracing::info!(?filters, "searching SoundCloud");

    let mut pages_scanned = 0;
    let tracks = client
        .search_tracks_filtered(&filters, 20, 10, |page, total| {
            pages_scanned += 1;
            tracing::info!(
                page = pages_scanned,
                new_on_page = page.len(),
                total_so_far = total,
                "progress"
            );
            true // keep going
        })
        .await?;

    println!();
    println!(
        "Found {} tracks under {} plays (scanned {} pages):",
        tracks.len(),
        filters.max_plays.unwrap(),
        pages_scanned
    );
    println!("{}", "-".repeat(80));

    for (i, t) in tracks.iter().enumerate() {
        let title = t.title.as_deref().unwrap_or("(untitled)");
        let artist = t
            .user
            .as_ref()
            .and_then(|u| u.username.as_deref())
            .unwrap_or("(unknown)");
        let plays    = t.playback_count.unwrap_or(0);
        let likes    = t.likes_count.unwrap_or(0);
        let duration = t.duration.map(|ms| format!("{}:{:02}", ms / 60_000, (ms % 60_000) / 1000))
                        .unwrap_or_else(|| "?:??".into());
        let url      = t.permalink_url.as_deref().unwrap_or("");

        println!(
            "{:>2}. {} — {}\n     {} · {} plays · {} likes · ratio {:.3}\n     {}",
            i + 1,
            artist,
            title,
            duration,
            plays,
            likes,
            t.engagement_ratio().unwrap_or(0.0),
            url,
        );
    }

    Ok(())
}
