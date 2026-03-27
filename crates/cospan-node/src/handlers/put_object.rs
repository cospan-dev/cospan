use std::sync::Arc;

use axum::Json;
use axum::body::Bytes;
use axum::extract::{Query, State};
use serde::Deserialize;

use crate::auth::DidAuth;
use crate::error::NodeError;
use crate::state::NodeState;

#[derive(Deserialize)]
pub struct PutObjectParams {
    pub did: String,
    pub repo: String,
}

pub async fn put_object(
    State(state): State<Arc<NodeState>>,
    auth: DidAuth,
    Query(params): Query<PutObjectParams>,
    body: Bytes,
) -> Result<Json<serde_json::Value>, NodeError> {
    state
        .authz
        .check_push(&auth.did, &params.did, &params.repo)?;

    // Deserialize the object from msgpack
    let object: panproto_core::vcs::Object = rmp_serde::from_slice(&body)
        .map_err(|e| NodeError::InvalidRequest(format!("invalid msgpack object: {e}")))?;

    // Store returns the content-addressed ID
    let store = state.store.lock().await;
    let id = store
        .put_object(&params.did, &params.repo, &object)
        .map_err(|e| NodeError::Internal(format!("store error: {e}")))?;

    Ok(Json(serde_json::json!({
        "id": id.to_string(),
        "stored": true,
    })))
}
