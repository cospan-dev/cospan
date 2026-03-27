use std::sync::Arc;

use axum::Json;
use axum::extract::{Query, State};
use serde::Deserialize;

use crate::error::NodeError;
use crate::state::NodeState;

#[derive(Deserialize)]
pub struct GetHeadParams {
    pub did: String,
    pub repo: String,
}

pub async fn get_head(
    State(state): State<Arc<NodeState>>,
    Query(params): Query<GetHeadParams>,
) -> Result<Json<serde_json::Value>, NodeError> {
    let store = state.store.lock().await;
    let head = store
        .get_head(&params.did, &params.repo)
        .map_err(|_| NodeError::RepoNotFound {
            did: params.did.clone(),
            name: params.repo.clone(),
        })?;

    let head_json = match head {
        panproto_core::vcs::HeadState::Branch(name) => serde_json::json!({
            "type": "branch",
            "ref": name,
        }),
        panproto_core::vcs::HeadState::Detached(id) => serde_json::json!({
            "type": "detached",
            "target": id.to_string(),
        }),
    };

    Ok(Json(serde_json::json!({ "head": head_json })))
}
