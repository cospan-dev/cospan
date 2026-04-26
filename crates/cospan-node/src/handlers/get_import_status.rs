//! `GET /xrpc/dev.panproto.node.getImportStatus`
//!
//! Reports whether the panproto-vcs import has completed for a repo.
//! The frontend uses this to show a "schema analysis in progress"
//! indicator while the background import runs after a push.

use std::sync::Arc;

use axum::Json;
use axum::extract::{Query, State};
use serde::Deserialize;
use serde_json::{Value, json};

use crate::error::NodeError;
use crate::state::NodeState;

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Params {
    pub did: String,
    pub repo: String,
}

pub async fn get_import_status(
    State(state): State<Arc<NodeState>>,
    Query(params): Query<Params>,
) -> Result<Json<Value>, NodeError> {
    let store = state.store.lock().await;

    let has_mirror = store.has_git_mirror(&params.did, &params.repo);
    if !has_mirror {
        return Ok(Json(json!({
            "status": "no_repo",
            "message": "Repository not found on this node.",
            "ready": false,
        })));
    }

    let has_marks = !store
        .load_import_marks(&params.did, &params.repo)
        .is_empty();

    drop(store);

    // Schema data is available via two paths:
    // 1. Full import via git-remote-cospan (marks file exists)
    // 2. On-demand parsing (getProjectSchema parses HEAD files on each request)
    //
    // If marks exist, the full commit history is available.
    // If no marks but mirror exists, on-demand parsing provides HEAD data.
    // The "importing" state only applies when git-remote-cospan is actively
    // pushing schema objects (detected by marks file growing).
    if has_marks {
        Ok(Json(json!({
            "status": "ready",
            "message": "Full schema history available.",
            "ready": true,
        })))
    } else if has_mirror {
        // Mirror exists but no marks: on-demand parsing provides HEAD data.
        // Full commit history requires pushing via git-remote-cospan.
        Ok(Json(json!({
            "status": "ready",
            "message": "Schema data available for HEAD. Push via git-remote-cospan for full commit history.",
            "ready": true,
        })))
    } else {
        Ok(Json(json!({
            "status": "no_repo",
            "message": "Repository not found on this node.",
            "ready": false,
        })))
    }
}
