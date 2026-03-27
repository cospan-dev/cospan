use std::sync::Arc;

use axum::Json;
use axum::extract::{Query, State};
use serde::Deserialize;

use crate::error::NodeError;
use crate::state::NodeState;

#[derive(Deserialize)]
pub struct GetRepoInfoParams {
    pub did: String,
    pub repo: String,
}

pub async fn get_repo_info(
    State(state): State<Arc<NodeState>>,
    Query(params): Query<GetRepoInfoParams>,
) -> Result<Json<serde_json::Value>, NodeError> {
    let store = state.store.lock().await;

    if !store.exists(&params.did, &params.repo) {
        return Err(NodeError::RepoNotFound {
            did: params.did.clone(),
            name: params.repo.clone(),
        });
    }

    let head = store.get_head(&params.did, &params.repo).ok();
    let refs = store
        .list_refs(&params.did, &params.repo)
        .unwrap_or_default();

    let head_json = head.map(|h| match h {
        panproto_core::vcs::HeadState::Branch(name) => {
            serde_json::json!({ "type": "branch", "ref": name })
        }
        panproto_core::vcs::HeadState::Detached(id) => {
            serde_json::json!({ "type": "detached", "target": id.to_string() })
        }
    });

    Ok(Json(serde_json::json!({
        "did": params.did,
        "repo": params.repo,
        "head": head_json,
        "refCount": refs.len(),
        "nodeDid": state.config.did,
    })))
}
