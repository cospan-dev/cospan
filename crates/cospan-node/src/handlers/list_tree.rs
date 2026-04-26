//! `GET /xrpc/dev.cospan.node.listTree`
//!
//! Lists the entries (files and directories) at a given path within
//! a commit tree. This is the git tree walker that powers the code
//! browser's file/directory navigation.

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
    /// Branch, tag, or commit OID. Defaults to HEAD.
    #[serde(rename = "ref")]
    pub ref_name: Option<String>,
    /// Directory path within the tree. Empty or "/" for root.
    pub path: Option<String>,
}

pub async fn list_tree(
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

    // Resolve ref to commit OID
    let commit_oid = match params.ref_name.as_deref() {
        Some(name) => resolve_ref(&mirror, name)?,
        None => resolve_default(&mirror)?,
    };

    let commit = mirror
        .find_commit(commit_oid)
        .map_err(|e| NodeError::Internal(format!("find commit: {e}")))?;
    let root_tree = commit
        .tree()
        .map_err(|e| NodeError::Internal(format!("commit tree: {e}")))?;

    // Navigate to the requested subtree
    let path = params.path.as_deref().unwrap_or("");
    let tree = if path.is_empty() || path == "/" {
        root_tree
    } else {
        let entry = root_tree
            .get_path(std::path::Path::new(path))
            .map_err(|_| NodeError::ObjectNotFound(format!("path '{path}' not found in tree")))?;
        mirror
            .find_tree(entry.id())
            .map_err(|_| NodeError::ObjectNotFound(format!("'{path}' is not a directory")))?
    };

    // List entries in the tree
    let mut entries: Vec<Value> = Vec::new();
    for entry in tree.iter() {
        let name = entry.name().unwrap_or("").to_string();
        let kind = match entry.kind() {
            Some(git2::ObjectType::Tree) => "dir",
            Some(git2::ObjectType::Blob) => "file",
            _ => continue,
        };
        let oid = entry.id().to_string();

        // For blobs, get the file size
        let size = if kind == "file" {
            mirror.find_blob(entry.id()).ok().map(|b| b.size())
        } else {
            None
        };

        entries.push(json!({
            "name": name,
            "type": kind,
            "oid": oid,
            "size": size,
        }));
    }

    // Sort: directories first, then files, both alphabetical
    entries.sort_by(|a, b| {
        let a_type = a["type"].as_str().unwrap_or("");
        let b_type = b["type"].as_str().unwrap_or("");
        let a_name = a["name"].as_str().unwrap_or("");
        let b_name = b["name"].as_str().unwrap_or("");
        a_type.cmp(b_type).reverse().then(a_name.cmp(b_name))
    });

    Ok(Json(json!({
        "ref": params.ref_name.as_deref().unwrap_or("HEAD"),
        "commit": commit_oid.to_string(),
        "path": path,
        "entries": entries,
    })))
}
