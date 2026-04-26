//! One-time PDS backfill for record types that the Tap may have missed.
//!
//! Fetches records directly from each user's PDS via `com.atproto.repo.listRecords`.
//! Used for state-tracking records (pull.status, issue.state) that arrive as separate
//! records and may have been emitted before the appview was connected to the Tap.

use std::sync::Arc;

use crate::state::AppState;

use super::consumer;

/// Collections to backfill from each user's PDS.
const BACKFILL_COLLECTIONS: &[&str] =
    &["sh.tangled.repo.pull.status", "sh.tangled.repo.issue.state"];

/// Run the PDS backfill. Fetches state records for all known DIDs.
pub async fn run(state: Arc<AppState>) {
    tracing::info!("starting PDS backfill for state records");

    // Get all unique DIDs: PR authors, issue authors, AND repo owners
    let dids = match sqlx::query_scalar::<_, String>(
        "SELECT DISTINCT did FROM pulls \
         UNION \
         SELECT DISTINCT did FROM issues \
         UNION \
         SELECT DISTINCT repo_did FROM pulls WHERE repo_did <> '' \
         UNION \
         SELECT DISTINCT repo_did FROM issues WHERE repo_did <> '' \
         UNION \
         SELECT DISTINCT did FROM repos WHERE source = 'tangled'",
    )
    .fetch_all(&state.db)
    .await
    {
        Ok(dids) => dids,
        Err(e) => {
            tracing::error!(error = %e, "failed to query DIDs for backfill");
            return;
        }
    };

    tracing::info!(did_count = dids.len(), "backfilling state records from PDS");

    let mut total_processed = 0u64;
    let mut total_errors = 0u64;

    for did in &dids {
        // Resolve DID to PDS URL
        let pds_url = match state.did_resolver.resolve_pds(did).await {
            Some(url) => url,
            None => {
                tracing::debug!(did, "could not resolve PDS, skipping");
                total_errors += 1;
                continue;
            }
        };

        for collection in BACKFILL_COLLECTIONS {
            match fetch_and_process(&state, &pds_url, did, collection).await {
                Ok(count) => total_processed += count,
                Err(e) => {
                    tracing::debug!(
                        did, collection, error = %e,
                        "PDS backfill error for collection"
                    );
                    total_errors += 1;
                }
            }
        }
    }

    tracing::info!(total_processed, total_errors, "PDS backfill complete");
}

/// Fetch all records of a collection from a user's PDS and process them.
async fn fetch_and_process(
    state: &Arc<AppState>,
    pds_url: &str,
    did: &str,
    collection: &str,
) -> anyhow::Result<u64> {
    let mut cursor: Option<String> = None;
    let mut count = 0u64;

    loop {
        let mut url = format!(
            "{}/xrpc/com.atproto.repo.listRecords?repo={}&collection={}&limit=100",
            pds_url.trim_end_matches('/'),
            did,
            collection,
        );
        if let Some(ref c) = cursor {
            url.push_str(&format!("&cursor={}", c));
        }

        let resp = state.http_client.get(&url).send().await?;

        if !resp.status().is_success() {
            return Ok(count);
        }

        let body: serde_json::Value = resp.json().await?;

        let records = body
            .get("records")
            .and_then(|v| v.as_array())
            .cloned()
            .unwrap_or_default();

        if records.is_empty() {
            break;
        }

        for record in &records {
            let uri = record.get("uri").and_then(|v| v.as_str()).unwrap_or("");
            let value = record.get("value");

            // Parse AT-URI: at://did/collection/rkey
            let parts: Vec<&str> = uri.trim_start_matches("at://").splitn(3, '/').collect();
            if parts.len() < 3 {
                continue;
            }
            let rkey = parts[2];

            if let Some(val) = value {
                let compat_event = serde_json::json!({
                    "did": did,
                    "commit": {
                        "collection": collection,
                        "operation": "create",
                        "rkey": rkey,
                        "record": val,
                    }
                });

                if let Err(e) = consumer::process_event(state, &compat_event).await {
                    tracing::debug!(
                        error = %e, did, collection, rkey,
                        "backfill record processing error"
                    );
                }
                count += 1;
            }
        }

        // Check for next page
        cursor = body
            .get("cursor")
            .and_then(|v| v.as_str())
            .map(|s| s.to_string());

        if cursor.is_none() {
            break;
        }
    }

    Ok(count)
}
