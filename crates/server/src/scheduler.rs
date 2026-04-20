//! Digest scheduler — wraps `tokio-cron-scheduler`.
//!
//! At boot, one cron job is registered per enabled digest. For v1, digest
//! changes require a server restart to take effect (simple and correct for
//! a single-user tool). v2 will add dynamic add/remove/replace.

use std::sync::Arc;

use tokio_cron_scheduler::{Job, JobScheduler};

use crate::state::{AppState, DigestEvent};

pub async fn start(state: Arc<AppState>) -> anyhow::Result<JobScheduler> {
    let sched = JobScheduler::new().await?;

    let digests = state.crater.list_digests().await?;
    let enabled: Vec<_> = digests.into_iter().filter(|d| d.enabled).collect();

    tracing::info!(count = enabled.len(), "registering digest cron jobs");

    for digest in enabled {
        let cron = digest.spec.cron_expr.clone();
        let digest_id = digest.id;
        let state = state.clone();

        let job = Job::new_async(cron.as_str(), move |_, _| {
            let state = state.clone();
            Box::pin(async move {
                tracing::info!(digest_id, "cron: starting scheduled digest run");

                let run_id_hint = {
                    // Fire a "started" event optimistically; real run_id filled in below
                    let _ = state.digest_events.send(DigestEvent::RunStarted {
                        digest_id,
                        run_id: 0,
                    });
                };
                let _ = run_id_hint;

                match state.crater.run_digest(digest_id).await {
                    Ok(run) => {
                        tracing::info!(
                            digest_id,
                            run_id = run.id,
                            track_count = ?run.track_count,
                            "digest run completed"
                        );
                        let _ = state.digest_events.send(DigestEvent::RunCompleted {
                            digest_id,
                            run_id:       run.id,
                            playlist_url: run.playlist_url,
                            track_count:  run.track_count,
                        });
                    }
                    Err(e) => {
                        tracing::error!(digest_id, error = %e, "digest run failed");
                        let _ = state.digest_events.send(DigestEvent::RunFailed {
                            digest_id,
                            run_id: 0,
                            error:  e.to_string(),
                        });
                    }
                }
            })
        })?;

        sched.add(job).await?;
    }

    sched.start().await?;
    Ok(sched)
}
