//! SSE endpoint for streaming index events to connected clients.
//!
//! `GET /xrpc/dev.cospan.sync.subscribeEvents`: streams events from the
//! indexer to connected clients using Server-Sent Events.

use std::convert::Infallible;
use std::sync::Arc;

use axum::extract::State;
use axum::response::sse::{Event, KeepAlive, Sse};
use futures_util::stream::Stream;
use tokio_stream::StreamExt;
use tokio_stream::wrappers::BroadcastStream;

use crate::state::AppState;

/// Events published by the indexer after successful record processing.
#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
#[serde(tag = "type", rename_all = "camelCase")]
pub enum IndexEvent {
    /// A ref was updated (push).
    RefUpdate {
        repo_did: String,
        repo_name: String,
        ref_name: String,
        new_target: String,
        committer_did: String,
        breaking_change_count: i32,
    },
    /// An issue was created.
    IssueCreated {
        repo_did: String,
        repo_name: String,
        issue_rkey: String,
        title: String,
        author_did: String,
    },
    /// An issue's state changed (open -> closed, etc).
    IssueStateChanged {
        repo_did: String,
        repo_name: String,
        issue_rkey: String,
        old_state: String,
        new_state: String,
    },
    /// A pull request was created.
    PullCreated {
        repo_did: String,
        repo_name: String,
        pull_rkey: String,
        title: String,
        author_did: String,
    },
    /// A pull request's state changed.
    PullStateChanged {
        repo_did: String,
        repo_name: String,
        pull_rkey: String,
        old_state: String,
        new_state: String,
    },
    /// A star was added.
    StarCreated { did: String, subject: String },
    /// A star was removed.
    StarDeleted { did: String, subject: String },
}

/// SSE handler that subscribes to the broadcast channel and streams events.
pub async fn subscribe_events(
    State(state): State<Arc<AppState>>,
) -> Sse<impl Stream<Item = Result<Event, Infallible>>> {
    let rx = state.event_tx.subscribe();
    let stream = BroadcastStream::new(rx).filter_map(|result| match result {
        Ok(event) => {
            let json = serde_json::to_string(&event).unwrap_or_default();
            let event_type = match &event {
                IndexEvent::RefUpdate { .. } => "refUpdate",
                IndexEvent::IssueCreated { .. } => "issueCreated",
                IndexEvent::IssueStateChanged { .. } => "issueStateChanged",
                IndexEvent::PullCreated { .. } => "pullCreated",
                IndexEvent::PullStateChanged { .. } => "pullStateChanged",
                IndexEvent::StarCreated { .. } => "starCreated",
                IndexEvent::StarDeleted { .. } => "starDeleted",
            };
            Some(Ok(Event::default().event(event_type).data(json)))
        }
        Err(_) => {
            // Lagged: receiver fell behind, skip the lost messages.
            None
        }
    });

    Sse::new(stream).keep_alive(KeepAlive::default())
}
