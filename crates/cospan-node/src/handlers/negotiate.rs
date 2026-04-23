use std::collections::HashSet;
use std::sync::Arc;

use axum::Json;
use axum::extract::State;
use serde::Deserialize;

use panproto_core::vcs::{Object, Store};

use crate::auth::DidAuth;
use crate::error::NodeError;
use crate::state::NodeState;

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct NegotiateInput {
    pub did: String,
    pub repo: String,
    /// Object IDs the client already has.
    pub have: Vec<String>,
    /// Refs the client wants.
    pub want: Vec<String>,
}

pub async fn negotiate(
    State(state): State<Arc<NodeState>>,
    _auth: DidAuth,
    Json(input): Json<NegotiateInput>,
) -> Result<Json<serde_json::Value>, NodeError> {
    let store = state.store.lock().await;
    // Auto-init on first push: an authenticated client negotiating against
    // a not-yet-existing repo means "I'm about to push you everything".
    let fs_store = store
        .open_or_init(&input.did, &input.repo)
        .map_err(|e| NodeError::Internal(format!("store error: {e}")))?;

    // Build set of objects the client already has
    let have_set: HashSet<String> = input.have.into_iter().collect();

    // Resolve wanted refs to object IDs
    let mut want_ids = Vec::new();
    for ref_name in &input.want {
        if let Ok(Some(target)) = fs_store.get_ref(ref_name) {
            want_ids.push(target);
        }
    }

    // Walk the commit DAG from each wanted object, collecting all
    // reachable objects that the client doesn't have.
    let mut need = Vec::new();
    let mut visited = HashSet::new();
    let mut queue: Vec<panproto_core::vcs::ObjectId> = want_ids.clone();

    while let Some(id) = queue.pop() {
        let id_str = id.to_string();
        if have_set.contains(&id_str) || !visited.insert(id_str.clone()) {
            continue;
        }
        need.push(id_str);

        // If this object is a commit, follow its parents and schema ref
        if let Ok(obj) = fs_store.get(&id)
            && let Object::Commit(commit) = obj
        {
            for parent in &commit.parents {
                queue.push(*parent);
            }
            queue.push(commit.schema_id);
            if let Some(ref mig) = commit.migration_id {
                queue.push(*mig);
            }
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
