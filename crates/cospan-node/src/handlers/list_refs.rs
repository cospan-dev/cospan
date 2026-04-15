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

    // Try panproto-vcs store first (has refs from git-remote-cospan push).
    if let Ok(refs) = store.list_refs(&params.did, &params.repo) {
        if !refs.is_empty() {
            let refs_json: Vec<serde_json::Value> = refs
                .into_iter()
                .map(|(name, id)| {
                    serde_json::json!({ "name": name, "target": id.to_string() })
                })
                .collect();
            return Ok(Json(serde_json::json!({ "refs": refs_json })));
        }
    }

    // Fall back to git mirror refs (from raw git push).
    if store.has_git_mirror(&params.did, &params.repo) {
        let mirror = store
            .open_or_init_git_mirror(&params.did, &params.repo)
            .map_err(|e| NodeError::Internal(format!("open mirror: {e}")))?;
        drop(store);

        let mut refs_json: Vec<serde_json::Value> = Vec::new();
        let references = mirror
            .references()
            .map_err(|e| NodeError::Internal(format!("list refs: {e}")))?;

        for r in references.flatten() {
            let Some(name) = r.name() else { continue };
            let Some(oid) = r.target() else { continue };
            refs_json.push(serde_json::json!({
                "name": name,
                "target": oid.to_string(),
            }));
        }

        return Ok(Json(serde_json::json!({ "refs": refs_json })));
    }

    drop(store);
    Ok(Json(serde_json::json!({ "refs": [] })))
}
