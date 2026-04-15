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

    // Return an empty-repo default (Branch("main")) when the vcs store
    // doesn't exist yet. git-remote-cospan calls getHead before the first
    // push to determine the default branch; returning 404 would prevent
    // any initial push from ever succeeding.
    let head = store
        .get_head(&params.did, &params.repo)
        .unwrap_or_else(|_| panproto_core::vcs::HeadState::Branch("main".to_string()));

    // Flat format matches panproto-xrpc NodeClient expectations.
    let head_json = match head {
        panproto_core::vcs::HeadState::Branch(name) => serde_json::json!({
            "branch": name,
        }),
        panproto_core::vcs::HeadState::Detached(id) => serde_json::json!({
            "detached": id.to_string(),
        }),
    };

    Ok(Json(head_json))
}
