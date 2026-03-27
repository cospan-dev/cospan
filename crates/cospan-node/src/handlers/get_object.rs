use std::sync::Arc;

use axum::extract::{Query, State};
use axum::http::header;
use axum::response::IntoResponse;
use serde::Deserialize;

use crate::error::NodeError;
use crate::state::NodeState;

#[derive(Deserialize)]
pub struct GetObjectParams {
    pub did: String,
    pub repo: String,
    pub id: String,
}

pub async fn get_object(
    State(state): State<Arc<NodeState>>,
    Query(params): Query<GetObjectParams>,
) -> Result<impl IntoResponse, NodeError> {
    let id: panproto_core::vcs::ObjectId = params
        .id
        .parse()
        .map_err(|_| NodeError::InvalidRequest(format!("invalid object ID: {}", params.id)))?;

    let store = state.store.lock().await;
    let object = store
        .get_object(&params.did, &params.repo, &id)
        .map_err(|_| NodeError::ObjectNotFound(params.id.clone()))?;

    // Serialize object to msgpack for wire transfer
    let bytes = rmp_serde::to_vec(&object)
        .map_err(|e| NodeError::Internal(format!("serialization error: {e}")))?;

    Ok((
        [(
            header::CONTENT_TYPE,
            "application/vnd.panproto.object+msgpack",
        )],
        bytes,
    ))
}
