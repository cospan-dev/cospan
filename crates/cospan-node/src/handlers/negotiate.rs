use std::sync::Arc;

use axum::Json;
use axum::extract::State;
use serde::Deserialize;

use panproto_core::vcs::Store;

use crate::auth::DidAuth;
use crate::error::NodeError;
use crate::state::NodeState;

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct NegotiateInput {
    pub did: String,
    pub repo: String,
    /// Object IDs the client already has (for push: the full local object set).
    pub have: Vec<String>,
    /// Refs the client wants (for push: the ref names it intends to update).
    #[serde(default)]
    pub want: Vec<String>,
}

pub async fn negotiate(
    State(state): State<Arc<NodeState>>,
    _auth: DidAuth,
    Json(input): Json<NegotiateInput>,
) -> Result<Json<serde_json::Value>, NodeError> {
    let _ = input.want; // ref names are informational; client follows up with setRef

    let store = state.store.lock().await;
    // Auto-init on first push: an authenticated client negotiating against
    // a not-yet-existing repo means "I'm about to push you everything".
    let fs_store = store
        .open_or_init(&input.did, &input.repo)
        .map_err(|e| NodeError::Internal(format!("store error: {e}")))?;

    // For push: the server replies with the subset of `have` it doesn't
    // yet have. The client then putObjects each one before calling setRef.
    let mut need = Vec::with_capacity(input.have.len());
    for id_str in input.have {
        let id = match id_str.parse::<panproto_core::vcs::ObjectId>() {
            Ok(id) => id,
            Err(_) => continue,
        };
        let already_have = fs_store.has(&id);
        if !already_have {
            need.push(id_str);
        }
    }

    // Wire format expected by panproto-xrpc::NegotiateResult: { need, refs }
    // where refs is the list of (name, target) pairs the remote currently has.
    let refs: Vec<(String, String)> = fs_store
        .list_refs("refs/")
        .unwrap_or_default()
        .into_iter()
        .map(|(name, id)| (name, id.to_string()))
        .collect();

    Ok(Json(serde_json::json!({
        "need": need,
        "refs": refs,
    })))
}
