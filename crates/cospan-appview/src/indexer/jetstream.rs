use std::sync::Arc;

use futures_util::StreamExt;
use tokio_tungstenite::tungstenite::Message;

use crate::db;
use crate::state::AppState;

use super::consumer;

/// Wanted collections for Jetstream subscription.
const WANTED_COLLECTIONS: &[&str] = &[
    "dev.cospan.node",
    "dev.cospan.actor.profile",
    "dev.cospan.repo",
    "dev.cospan.vcs.refUpdate",
    "dev.cospan.repo.issue",
    "dev.cospan.repo.issue.comment",
    "dev.cospan.repo.issue.state",
    "dev.cospan.repo.pull",
    "dev.cospan.repo.pull.comment",
    "dev.cospan.repo.pull.state",
    "dev.cospan.repo.dependency",
    "dev.cospan.repo.collaborator",
    "dev.cospan.feed.star",
    "dev.cospan.feed.reaction",
    "dev.cospan.graph.follow",
    "dev.cospan.label.definition",
    "dev.cospan.org",
    "dev.cospan.org.member",
    "dev.cospan.pipeline",
    // Tangled interop
    "sh.tangled.repo.issue",
    "sh.tangled.feed.star",
    "sh.tangled.graph.follow",
    "sh.tangled.repo.pull",
];

/// Classify an error as retryable or permanent.
///
/// Retryable errors are transient infrastructure issues (database connection
/// failures, timeouts) that will likely succeed on retry. Permanent errors
/// are data-level problems (bad JSON, constraint violations, unknown schemas)
/// that will never succeed and should go to the DLQ.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum ErrorClass {
    /// Transient error — retry with backoff, will reprocess on reconnect.
    Retryable,
    /// Permanent error — insert into DLQ, do not retry.
    Permanent,
}

fn classify_error(err: &anyhow::Error) -> ErrorClass {
    let msg = format!("{err:#}");
    let lower = msg.to_lowercase();

    // Database connection/pool errors are retryable
    if lower.contains("connection refused")
        || lower.contains("connection reset")
        || lower.contains("pool timed out")
        || lower.contains("broken pipe")
        || lower.contains("timed out")
        || lower.contains("too many connections")
        || lower.contains("connection closed")
    {
        return ErrorClass::Retryable;
    }

    // Check for sqlx-specific retryable errors
    if let Some(sqlx_err) = err.downcast_ref::<sqlx::Error>() {
        match sqlx_err {
            sqlx::Error::PoolTimedOut => return ErrorClass::Retryable,
            sqlx::Error::PoolClosed => return ErrorClass::Retryable,
            sqlx::Error::Io(_) => return ErrorClass::Retryable,
            sqlx::Error::Database(db_err) => {
                // PostgreSQL error codes:
                // 23505 = unique_violation (constraint violation — permanent)
                // 40001 = serialization_failure (retryable)
                // 40P01 = deadlock_detected (retryable)
                // 08xxx = connection exceptions (retryable)
                if let Some(code) = db_err.code() {
                    let code_str = code.as_ref();
                    if code_str == "23505" || code_str == "23503" || code_str == "23502" {
                        return ErrorClass::Permanent;
                    }
                    if code_str.starts_with("08")
                        || code_str == "40001"
                        || code_str == "40P01"
                        || code_str == "57P01"
                    {
                        return ErrorClass::Retryable;
                    }
                }
                // Other database errors are permanent by default
                return ErrorClass::Permanent;
            }
            _ => {}
        }
    }

    // Deserialization / parse errors are permanent
    if lower.contains("deserialize")
        || lower.contains("invalid type")
        || lower.contains("missing field")
        || lower.contains("unknown variant")
        || lower.contains("parse error")
        || lower.contains("invalid value")
        || lower.contains("expected")
    {
        return ErrorClass::Permanent;
    }

    // Default: treat unknown errors as permanent to avoid infinite retries.
    // Retryable errors should be explicitly listed above.
    ErrorClass::Permanent
}

/// Insert a failed event into the dead letter queue.
#[allow(clippy::too_many_arguments)]
async fn insert_dlq(
    pool: &sqlx::PgPool,
    collection: &str,
    rkey: &str,
    did: &str,
    operation: &str,
    error: &anyhow::Error,
    raw_event: &serde_json::Value,
    cursor_us: i64,
) -> Result<(), sqlx::Error> {
    let dlq_id = uuid::Uuid::new_v4().to_string();
    let error_detail = serde_json::json!({
        "error": format!("{error:#}"),
        "error_chain": format!("{error:?}"),
    });

    sqlx::query(
        "INSERT INTO dlq_entries (id, collection, rkey, did, operation, error_detail, raw_record, cursor_us) \
         VALUES ($1, $2, $3, $4, $5, $6, $7, $8) \
         ON CONFLICT (id) DO NOTHING",
    )
    .bind(&dlq_id)
    .bind(collection)
    .bind(rkey)
    .bind(did)
    .bind(operation)
    .bind(&error_detail)
    .bind(raw_event)
    .bind(cursor_us)
    .execute(pool)
    .await?;

    tracing::warn!(
        dlq_id = %dlq_id,
        collection = collection,
        did = did,
        rkey = rkey,
        error = %error,
        "event inserted into dead letter queue"
    );

    Ok(())
}

/// Extract event metadata fields for DLQ insertion.
fn extract_event_metadata(event: &serde_json::Value) -> (&str, &str, &str, &str) {
    let commit = event.get("commit");
    let collection = commit
        .and_then(|c| c.get("collection"))
        .and_then(|v| v.as_str())
        .unwrap_or("");
    let operation = commit
        .and_then(|c| c.get("operation"))
        .and_then(|v| v.as_str())
        .unwrap_or("");
    let did = event.get("did").and_then(|v| v.as_str()).unwrap_or("");
    let rkey = commit
        .and_then(|c| c.get("rkey"))
        .and_then(|v| v.as_str())
        .unwrap_or("");
    (collection, rkey, did, operation)
}

pub async fn subscribe(state: &Arc<AppState>) -> anyhow::Result<()> {
    // Build URL with wanted collections and cursor
    let cursor = db::cursor::get_cursor(&state.db).await?;
    let mut url = state.config.jetstream_url.clone();

    let params: Vec<String> = WANTED_COLLECTIONS
        .iter()
        .map(|c| format!("wantedCollections={c}"))
        .collect();
    url = format!("{url}?{}", params.join("&"));

    if let Some(cursor_us) = cursor {
        url = format!("{url}&cursor={cursor_us}");
        tracing::info!(cursor_us, "resuming from cursor");
    }

    let (ws, _) = tokio_tungstenite::connect_async(&url).await?;
    let (_, mut read) = ws.split();

    let mut event_count: u64 = 0;
    let mut last_cursor_save = std::time::Instant::now();
    let mut consecutive_retryable_errors: u32 = 0;

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
                tracing::warn!(error = %e, "failed to parse jetstream event JSON");
                // Raw JSON parse failure is permanent — the event data is malformed.
                // We cannot extract collection/rkey/did, so we use sentinel values.
                let raw = serde_json::Value::String(String::from_utf8_lossy(&data).into_owned());
                if let Err(dlq_err) = insert_dlq(
                    &state.db,
                    "<unparseable>",
                    "",
                    "",
                    "",
                    &anyhow::anyhow!("JSON parse error: {e}"),
                    &raw,
                    0,
                )
                .await
                {
                    tracing::error!(error = %dlq_err, "failed to insert DLQ entry for unparseable event");
                }
                continue;
            }
        };

        let time_us = event.get("time_us").and_then(|v| v.as_i64()).unwrap_or(0);

        if let Err(e) = consumer::process_event(state, &event).await {
            let error_class = classify_error(&e);
            let (collection, rkey, did, operation) = extract_event_metadata(&event);

            match error_class {
                ErrorClass::Retryable => {
                    consecutive_retryable_errors += 1;
                    tracing::warn!(
                        error = %e,
                        collection = collection,
                        did = did,
                        retryable_count = consecutive_retryable_errors,
                        "retryable error processing event (will reprocess on reconnect)"
                    );

                    // If we're getting many consecutive retryable errors, back off
                    // to avoid hammering a struggling database.
                    if consecutive_retryable_errors >= 10 {
                        let backoff = std::time::Duration::from_millis(
                            100 * u64::from(consecutive_retryable_errors.min(100)),
                        );
                        tracing::warn!(
                            backoff_ms = backoff.as_millis() as u64,
                            "backing off due to consecutive retryable errors"
                        );
                        tokio::time::sleep(backoff).await;
                    }

                    // For retryable errors, don't advance the cursor so the event
                    // will be reprocessed on reconnect. If too many pile up, break
                    // to trigger reconnect with fresh connection.
                    if consecutive_retryable_errors >= 50 {
                        tracing::error!(
                            "too many consecutive retryable errors, breaking to reconnect"
                        );
                        break;
                    }

                    continue;
                }
                ErrorClass::Permanent => {
                    tracing::error!(
                        error = %e,
                        collection = collection,
                        did = did,
                        rkey = rkey,
                        "permanent error processing event, inserting into DLQ"
                    );

                    if let Err(dlq_err) = insert_dlq(
                        &state.db, collection, rkey, did, operation, &e, &event, time_us,
                    )
                    .await
                    {
                        tracing::error!(
                            error = %dlq_err,
                            "failed to insert DLQ entry, logging original error"
                        );
                    }
                }
            }
        } else {
            // Reset retryable error counter on success
            consecutive_retryable_errors = 0;
        }

        event_count += 1;

        // Persist cursor every 500 events or 5 seconds
        if (event_count.is_multiple_of(500)
            || last_cursor_save.elapsed() > std::time::Duration::from_secs(5))
            && time_us > 0
        {
            db::cursor::save_cursor(&state.db, time_us).await?;
            last_cursor_save = std::time::Instant::now();
        }
    }

    Ok(())
}
