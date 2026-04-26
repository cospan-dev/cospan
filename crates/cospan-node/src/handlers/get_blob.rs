//! `GET /xrpc/dev.cospan.node.getBlob`
//!
//! Returns the raw content of a file (blob) at a given path and ref
//! from the git mirror. This powers the code browser's file viewer.

use std::sync::Arc;

use axum::Json;
use axum::extract::{Query, State};
use serde::Deserialize;
use serde_json::{Value, json};

use crate::error::NodeError;
use crate::state::NodeState;

use super::list_commits::{resolve_default, resolve_ref};

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Params {
    pub did: String,
    pub repo: String,
    #[serde(rename = "ref")]
    pub ref_name: Option<String>,
    pub path: String,
}

pub async fn get_blob(
    State(state): State<Arc<NodeState>>,
    Query(params): Query<Params>,
) -> Result<Json<Value>, NodeError> {
    let store = state.store.lock().await;
    if !store.has_git_mirror(&params.did, &params.repo) {
        return Err(NodeError::RefNotFound(format!(
            "repo {}/{} not found",
            params.did, params.repo
        )));
    }
    let mirror = store
        .open_or_init_git_mirror(&params.did, &params.repo)
        .map_err(|e| NodeError::Internal(format!("open mirror: {e}")))?;
    drop(store);

    let commit_oid = match params.ref_name.as_deref() {
        Some(name) => resolve_ref(&mirror, name)?,
        None => resolve_default(&mirror)?,
    };

    let commit = mirror
        .find_commit(commit_oid)
        .map_err(|e| NodeError::Internal(format!("find commit: {e}")))?;
    let tree = commit
        .tree()
        .map_err(|e| NodeError::Internal(format!("commit tree: {e}")))?;

    let entry = tree
        .get_path(std::path::Path::new(&params.path))
        .map_err(|_| NodeError::ObjectNotFound(format!("file '{}' not found", params.path)))?;

    let blob = mirror
        .find_blob(entry.id())
        .map_err(|_| NodeError::ObjectNotFound(format!("'{}' is not a file", params.path)))?;

    let content = blob.content();
    let is_binary = content.contains(&0u8);

    if is_binary {
        Ok(Json(json!({
            "path": params.path,
            "commit": commit_oid.to_string(),
            "binary": true,
            "size": content.len(),
            "content": serde_json::Value::Null,
        })))
    } else {
        let text = String::from_utf8_lossy(content);
        Ok(Json(json!({
            "path": params.path,
            "commit": commit_oid.to_string(),
            "binary": false,
            "size": content.len(),
            "content": text,
        })))
    }
}
