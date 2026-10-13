//! Run a graph crawl from the command line and print the top results.
//!
//! Run with:
//!   cargo run -p crater-core --example dig -- <data_dir> [limit]
//!
//! Seeds are every hearted, queued and exported track in `<data_dir>/crater.db`.
//! The crawl writes the graph it fetches into that database, so point it at
//! a copy if you don't want that.

use crater_core::{Config, Crater, DigParams};

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "crater_core=info,sc_client=info".into()),
        )
        .with_target(false)
        .init();

    let mut args = std::env::args().skip(1);
    let data_dir = args.next().ok_or_else(|| anyhow::anyhow!("usage: dig <data_dir> [limit]"))?;
    let limit: usize = args.next().map(|s| s.parse()).transpose()?.unwrap_or(50);

    let crater = Crater::new(Config {
        data_dir:         data_dir.into(),
        sc_oauth_cfg:     None,
        sc_oauth_token:   None,
        cached_client_id: None,
    })
    .await?;

    let params = DigParams { limit, ..Default::default() };
    let started = std::time::Instant::now();
    let outcome = crater
        .dig(&params, |p| {
            eprint!("\r{:?} {}/{} ({} requests)   ", p.phase, p.done, p.total, p.requests);
        })
        .await?;
    eprintln!();

    println!(
        "{} seeds, {} curators, {} candidates, ~{} requests, {:.0?}{}",
        outcome.seeds,
        outcome.curators,
        outcome.candidates,
        outcome.requests,
        started.elapsed(),
        if outcome.rate_limited { " (rate limited)" } else { "" },
    );
    for (i, r) in outcome.results.iter().enumerate() {
        let t = &r.track;
        let via: Vec<_> = r.via.iter().map(|v| v.username.as_deref().unwrap_or("?")).collect();
        println!(
            "{:>3}. {:.3}  {} — {}\n      {} plays · {} · via {} ({}) · because {}\n      {}",
            i + 1,
            r.score,
            t.artist.as_deref().unwrap_or("?"),
            t.title.as_deref().unwrap_or("?"),
            t.playback_count.unwrap_or(0),
            t.genre.as_deref().unwrap_or("-"),
            r.support,
            via.join(", "),
            r.because.join(" / "),
            t.permalink_url,
        );
    }
    Ok(())
}
