use std::sync::Arc;

use axum::Json;
use axum::extract::{Query, State};
use serde::Deserialize;

use crate::error::NodeError;
use crate::state::NodeState;

#[derive(Deserialize)]
pub struct GetRefParams {
    pub did: String,
    pub repo: String,
    #[serde(rename = "ref")]
    pub ref_name: String,
}

pub async fn get_ref(
    State(state): State<Arc<NodeState>>,
    Query(params): Query<GetRefParams>,
) -> Result<Json<serde_json::Value>, NodeError> {
    let store = state.store.lock().await;
    let target = store
        .get_ref(&params.did, &params.repo, &params.ref_name)
        .map_err(|e| NodeError::Internal(format!("store error: {e}")))?
        .ok_or_else(|| NodeError::RefNotFound(params.ref_name.clone()))?;

    Ok(Json(serde_json::json!({
        "ref": params.ref_name,
        "target": target.to_string(),
    })))
}
