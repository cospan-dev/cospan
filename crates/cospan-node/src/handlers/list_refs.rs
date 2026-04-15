use std::sync::Arc;

use axum::Json;
use axum::extract::{Query, State};
use serde::Deserialize;

use crate::error::NodeError;
use crate::state::NodeState;

#[derive(Deserialize)]
pub struct ListRefsParams {
    pub did: String,
    pub repo: String,
}

pub async fn list_refs(
    State(state): State<Arc<NodeState>>,
    Query(params): Query<ListRefsParams>,
) -> Result<Json<serde_json::Value>, NodeError> {
    let store = state.store.lock().await;

    // Return refs from the panproto-vcs store. We do NOT fall back to
    // git mirror refs: those are git OIDs (20 bytes) while panproto-xrpc
    // clients expect panproto ObjectIds (32 bytes). Empty response is the
    // correct behavior for repos without vcs data.
    let refs = store
        .list_refs(&params.did, &params.repo)
        .unwrap_or_default();
    let refs_json: Vec<serde_json::Value> = refs
        .into_iter()
        .map(|(name, id)| {
            serde_json::json!({ "name": name, "target": id.to_string() })
        })
        .collect();
    Ok(Json(serde_json::json!({ "refs": refs_json })))
}
