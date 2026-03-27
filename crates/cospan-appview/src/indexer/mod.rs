mod consumer;
mod jetstream;

use std::sync::Arc;

use crate::state::AppState;

/// Run the firehose indexer. Connects to Jetstream and processes events.
pub async fn run(state: Arc<AppState>) -> anyhow::Result<()> {
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
