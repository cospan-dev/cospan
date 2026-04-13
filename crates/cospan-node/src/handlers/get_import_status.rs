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

    let has_marks = store
        .load_import_marks(&params.did, &params.repo)
        .len()
        > 0;

    let has_vcs = store.exists(&params.did, &params.repo);

    drop(store);

    if has_marks {
        Ok(Json(json!({
            "status": "ready",
            "message": "Schema analysis complete.",
            "ready": true,
        })))
    } else if has_vcs {
        Ok(Json(json!({
            "status": "importing",
            "message": "Schema analysis in progress. Structural data will appear shortly.",
            "ready": false,
        })))
    } else {
        Ok(Json(json!({
            "status": "pending",
            "message": "Schema analysis has not started yet.",
            "ready": false,
        })))
    }
}
