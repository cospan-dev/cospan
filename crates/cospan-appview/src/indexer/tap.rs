//! Tap WebSocket consumer for full historical backfill + live events.
//!
//! Connects to a Tap instance (github.com/bluesky-social/indigo/cmd/tap)
//! which handles repo discovery, backfill from PDS, and live firehose events.
//! Events arrive as JSON with a `live` boolean indicating backfill vs live.

use std::sync::Arc;

use futures_util::StreamExt;
use tokio_tungstenite::connect_async;
use tokio_tungstenite::tungstenite::Message;

use super::consumer;
use crate::state::AppState;

/// Subscribe to a Tap instance at the given URL.
pub async fn subscribe_to(state: &Arc<AppState>, tap_url: &str) -> anyhow::Result<()> {
    tracing::info!(url = %tap_url, "connecting to Tap");

    let (ws, _) = connect_async(tap_url).await?;
    let (_, mut read) = ws.split();

    let mut event_count: u64 = 0;
    let mut backfill_count: u64 = 0;
    let mut live_count: u64 = 0;

    while let Some(msg) = read.next().await {
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
            Err(e) => {
                tracing::warn!(error = %e, "failed to parse Tap event");
                continue;
            }
        };

        // Tap event format: { "id": N, "type": "record"|"identity", "record": { ... } }
        let event_type = event.get("type").and_then(|v| v.as_str()).unwrap_or("");
        if event_type != "record" {
            continue; // Skip identity events for now
        }

        let record_event = match event.get("record") {
            Some(r) => r,
            None => continue,
        };

        let is_live = record_event
            .get("live")
            .and_then(|v| v.as_bool())
            .unwrap_or(true);
        let action = record_event
            .get("action")
            .and_then(|v| v.as_str())
            .unwrap_or("");
        let collection = record_event
            .get("collection")
            .and_then(|v| v.as_str())
            .unwrap_or("");
        let did = record_event
            .get("did")
            .and_then(|v| v.as_str())
            .unwrap_or("");
        let rkey = record_event
            .get("rkey")
            .and_then(|v| v.as_str())
            .unwrap_or("");
        let record_data = record_event.get("record");

        // Map Tap action to Jetstream-compatible operation
        let operation = match action {
            "create" | "update" => action,
            "delete" => "delete",
            _ => continue,
        };

        // Build a Jetstream-compatible event structure so the consumer can handle it
        let compat_event = serde_json::json!({
            "did": did,
            "commit": {
                "collection": collection,
                "operation": operation,
                "rkey": rkey,
                "record": record_data,
            }
        });

        // Process through the same consumer pipeline
        if let Err(e) = consumer::process_event(state, &compat_event).await {
            tracing::warn!(
                error = %e,
                collection,
                did,
                rkey,
                live = is_live,
                "tap event processing error"
            );
        }

        event_count += 1;
        if is_live {
            live_count += 1;
        } else {
            backfill_count += 1;
        }

        if event_count.is_multiple_of(1000) {
            tracing::info!(
                total = event_count,
                backfill = backfill_count,
                live = live_count,
                "tap progress"
            );
        }
    }

    tracing::info!(
        total = event_count,
        backfill = backfill_count,
        live = live_count,
        "tap connection closed"
    );

    Ok(())
}
