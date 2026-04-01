//! Knot event consumer — connects to Tangled knot servers' WebSocket event
//! streams to ingest refUpdates (commits) and other knot-authored records.
//!
//! Tangled knots serve events at wss://{host}/events (not via ATProto relay).
//! This consumer discovers knots from the nodes table and subscribes to each.

use std::sync::Arc;

use futures_util::StreamExt;
use tokio_tungstenite::connect_async;
use tokio_tungstenite::tungstenite::Message;

use super::consumer;
use crate::db;
use crate::state::AppState;

/// Discover all knot URLs and spawn a consumer for each.
pub async fn run(state: Arc<AppState>) {
    // Wait for initial data to be ingested before discovering knots
    tokio::time::sleep(std::time::Duration::from_secs(30)).await;

    loop {
        match discover_and_consume(&state).await {
            Ok(()) => {
                tracing::info!("knot consumer cycle complete, re-discovering in 5m");
            }
            Err(e) => {
                tracing::error!(error = %e, "knot consumer error, retrying in 1m");
            }
        }
        // Re-discover knots periodically
        tokio::time::sleep(std::time::Duration::from_secs(300)).await;
    }
}

async fn discover_and_consume(state: &Arc<AppState>) -> anyhow::Result<()> {
    // Discover knot URLs from repos.node_url (more reliable than nodes.public_endpoint)
    let repos = db::repo::list_recent(&state.db, 5000, None).await?;
    let mut seen = std::collections::HashSet::new();
    let knot_urls: Vec<String> = repos
        .iter()
        .filter(|r| !r.node_url.is_empty())
        .filter(|r| !r.node_url.contains("localhost") && !r.node_url.contains("192.168."))
        .filter(|r| seen.insert(r.node_url.clone()))
        .map(|r| {
            let ws_url = r.node_url.replace("https://", "wss://").replace("http://", "ws://");
            format!("{ws_url}/events")
        })
        .collect();

    if knot_urls.is_empty() {
        tracing::info!("no knots discovered, waiting for nodes to be ingested");
        return Ok(());
    }

    tracing::info!(count = knot_urls.len(), "discovered knots, connecting to event streams");

    // Spawn a consumer for each knot (with concurrency limit)
    let semaphore = Arc::new(tokio::sync::Semaphore::new(10)); // max 10 concurrent connections
    let mut handles = Vec::new();

    for url in knot_urls {
        let state = state.clone();
        let sem = semaphore.clone();
        handles.push(tokio::spawn(async move {
            let _permit = sem.acquire().await;
            if let Err(e) = consume_knot(&state, &url).await {
                tracing::debug!(url = %url, error = %e, "knot connection failed");
            }
        }));
    }

    // Wait for all to complete (they'll timeout or disconnect)
    for handle in handles {
        let _ = handle.await;
    }

    Ok(())
}

/// Connect to a single knot's event stream and process events.
async fn consume_knot(state: &Arc<AppState>, url: &str) -> anyhow::Result<()> {
    // Pass cursor=0 to get ALL historical events from the knot
    let url_with_cursor = if url.contains('?') {
        format!("{url}&cursor=0")
    } else {
        format!("{url}?cursor=0")
    };
    let (ws, _) = tokio::time::timeout(
        std::time::Duration::from_secs(10),
        connect_async(&url_with_cursor),
    )
    .await
    .map_err(|_| anyhow::anyhow!("connection timeout"))??;

    let (_, mut read) = ws.split();
    let mut count = 0u64;

    // Read events with a timeout per message (2 min for historical replay)
    while let Ok(Some(msg)) = tokio::time::timeout(
        std::time::Duration::from_secs(120),
        read.next(),
    )
    .await
    {
        let msg = msg?;
        let data = match msg {
            Message::Text(text) => text.as_bytes().to_vec(),
            Message::Binary(bin) => bin.to_vec(),
            Message::Ping(_) | Message::Pong(_) => continue,
            Message::Close(_) => break,
            _ => continue,
        };

        let event: serde_json::Value = match serde_json::from_slice(&data) {
            Ok(v) => v,
            Err(_) => continue,
        };

        // Knot event format: { "rkey": "...", "nsid": "...", "event": { ... } }
        let nsid = event.get("nsid").and_then(|v| v.as_str()).unwrap_or("");
        let rkey = event.get("rkey").and_then(|v| v.as_str()).unwrap_or("");
        let record = event.get("event");

        if nsid.is_empty() || rkey.is_empty() || record.is_none() {
            continue;
        }

        // Extract the DID from the record (knot events don't have a top-level DID)
        let did = record
            .and_then(|r| r.get("repoDid").or(r.get("committerDid")))
            .and_then(|v| v.as_str())
            .unwrap_or("");

        // Build a Jetstream-compatible event for the consumer pipeline
        let compat_event = serde_json::json!({
            "did": did,
            "commit": {
                "collection": nsid,
                "operation": "create",
                "rkey": rkey,
                "record": record,
            }
        });

        if let Err(e) = consumer::process_event(state, &compat_event).await {
            tracing::debug!(
                error = %e,
                nsid,
                rkey,
                "knot event processing error"
            );
        }

        count += 1;
    }

    if count > 0 {
        tracing::info!(url = %url, events = count, "knot stream processed");
    }

    Ok(())
}
