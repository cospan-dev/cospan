pub(crate) mod consumer;
mod dispatch;
mod jetstream;
mod tap;

use std::sync::Arc;

use crate::state::AppState;

/// Run the indexer. Spawns both Jetstream (live) and Tap (backfill + live) consumers.
pub async fn run(state: Arc<AppState>) -> anyhow::Result<()> {
    // Spawn Tap consumer if TAP_URL is configured
    let tap_state = state.clone();
    tokio::spawn(async move {
        loop {
            match tap::subscribe(&tap_state).await {
                Ok(()) => {
                    tracing::info!("tap connection closed, reconnecting in 5s");
                }
                Err(e) => {
                    tracing::error!(error = %e, "tap connection error, reconnecting in 10s");
                    tokio::time::sleep(std::time::Duration::from_secs(10)).await;
                }
            }
            tokio::time::sleep(std::time::Duration::from_secs(5)).await;
        }
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
