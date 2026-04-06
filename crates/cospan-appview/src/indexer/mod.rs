pub(crate) mod consumer;
mod dispatch;
mod jetstream;
mod knot_consumer;
mod pds_backfill;
mod tap;

use std::sync::Arc;

use crate::state::AppState;

/// Run the indexer. Spawns both Jetstream (live) and Tap (backfill + live) consumers.
pub async fn run(state: Arc<AppState>) -> anyhow::Result<()> {
    // Spawn Tap consumers for each URL in TAP_URL (comma-separated)
    if let Ok(tap_urls) = std::env::var("TAP_URL") {
        for url in tap_urls.split(',').map(|s| s.trim().to_string()) {
            if url.is_empty() { continue; }
            let tap_state = state.clone();
            tokio::spawn(async move {
                loop {
                    match tap::subscribe_to(&tap_state, &url).await {
                        Ok(()) => {
                            tracing::info!(url = %url, "tap connection closed, reconnecting in 5s");
                        }
                        Err(e) => {
                            tracing::error!(url = %url, error = %e, "tap connection error, reconnecting in 10s");
                            tokio::time::sleep(std::time::Duration::from_secs(10)).await;
                        }
                    }
                    tokio::time::sleep(std::time::Duration::from_secs(5)).await;
                }
            });
        }
    }

    // Spawn knot event consumer (discovers knots, connects to each /events WebSocket)
    let knot_state = state.clone();
    tokio::spawn(async move {
        knot_consumer::run(knot_state).await;
    });

    // Run PDS backfill after 60s delay (let Tap deliver what it can first)
    let backfill_state = state.clone();
    tokio::spawn(async move {
        tokio::time::sleep(std::time::Duration::from_secs(60)).await;
        pds_backfill::run(backfill_state).await;
    });

    // Run Jetstream consumer (primary live stream with cursor persistence)
    loop {
        tracing::info!("connecting to jetstream");
        match jetstream::subscribe(&state).await {
            Ok(()) => {
                tracing::info!("jetstream connection closed cleanly");
            }
            Err(e) => {
                tracing::error!(error = %e, "jetstream connection error, reconnecting in 5s");
                tokio::time::sleep(std::time::Duration::from_secs(5)).await;
            }
        }
    }
}
