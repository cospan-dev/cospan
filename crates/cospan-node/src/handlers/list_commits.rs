//! `GET /xrpc/dev.panproto.node.listCommits` and its cospan alias.
//!
//! Walks the repo's persistent git mirror using libgit2's RevWalk and
//! returns a flat list of commits with the parent pointers needed to
//! draw a DAG. This is the data source for the frontend CommitGraph
//! visualization: we return things in topological order newest-first
//! so the UI can lay out lanes without re-sorting.
//!
//! Query parameters:
//!   - `did`: repo owner
//!   - `repo`: repo name (rkey or human name)
//!   - `ref`: optional ref to start from (default: HEAD → first branch)
//!   - `limit`: optional max commits to return (default 50, max 500)

use std::sync::Arc;

use axum::Json;
use axum::extract::{Query, State};
use serde::Deserialize;
use serde_json::json;

use crate::error::NodeError;
use crate::state::NodeState;

#[derive(Deserialize)]
pub struct ListCommitsParams {
    pub did: String,
    pub repo: String,
    #[serde(rename = "ref")]
    pub ref_name: Option<String>,
    pub limit: Option<usize>,
}

pub async fn list_commits(
    State(state): State<Arc<NodeState>>,
    Query(params): Query<ListCommitsParams>,
) -> Result<Json<serde_json::Value>, NodeError> {
    let limit = params.limit.unwrap_or(50).min(500);

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

    // Figure out where to start the walk.
    let start_oid = match params.ref_name.as_deref() {
        Some(name) => resolve_ref(&mirror, name)?,
        None => resolve_default(&mirror)?,
    };

    let mut walk = mirror
        .revwalk()
        .map_err(|e| NodeError::Internal(format!("revwalk: {e}")))?;
    walk.set_sorting(git2::Sort::TOPOLOGICAL | git2::Sort::TIME)
        .map_err(|e| NodeError::Internal(format!("set sorting: {e}")))?;
    walk.push(start_oid)
        .map_err(|e| NodeError::Internal(format!("push start: {e}")))?;

    let mut commits = Vec::with_capacity(limit);
    for (i, oid_result) in walk.enumerate() {
        if i >= limit {
            break;
        }
        let oid = match oid_result {
            Ok(o) => o,
            Err(e) => {
                tracing::warn!(error = %e, "revwalk step failed; stopping");
                break;
            }
        };
        let commit = match mirror.find_commit(oid) {
            Ok(c) => c,
            Err(e) => {
                tracing::warn!(oid = %oid, error = %e, "find_commit failed; skipping");
                continue;
            }
        };

        let author = commit.author();
        let committer = commit.committer();
        let summary = commit.summary().unwrap_or_default().to_string();
        let message = commit.message().unwrap_or_default().to_string();
        let timestamp = commit.time().seconds();
        let parents: Vec<String> = commit.parent_ids().map(|p| p.to_string()).collect();

        commits.push(json!({
            "oid": oid.to_string(),
            "parents": parents,
            "summary": summary,
            "message": message,
            "author": {
                "name": author.name().unwrap_or_default(),
                "email": author.email().unwrap_or_default(),
            },
            "committer": {
                "name": committer.name().unwrap_or_default(),
                "email": committer.email().unwrap_or_default(),
            },
            "timestamp": timestamp,
            "treeOid": commit.tree_id().to_string(),
        }));
    }

    Ok(Json(json!({
        "commits": commits,
        "count": commits.len(),
        "start": start_oid.to_string(),
    })))
}

/// Resolve a ref name to its target OID. Accepts short names like
/// "main" as well as fully-qualified "refs/heads/main".
fn resolve_ref(mirror: &git2::Repository, name: &str) -> Result<git2::Oid, NodeError> {
    // Try fully qualified first.
    if let Ok(r) = mirror.find_reference(name) {
        if let Some(oid) = r.target() {
            return Ok(oid);
        }
    }
    // Try under refs/heads/.
    let candidate = format!("refs/heads/{name}");
    if let Ok(r) = mirror.find_reference(&candidate) {
        if let Some(oid) = r.target() {
            return Ok(oid);
        }
    }
    // Try under refs/tags/.
    let candidate = format!("refs/tags/{name}");
    if let Ok(r) = mirror.find_reference(&candidate) {
        if let Some(oid) = r.target() {
            return Ok(oid);
        }
        // Annotated tags point at a tag object, not the commit directly.
        if let Ok(tag) = r.peel_to_commit() {
            return Ok(tag.id());
        }
    }
    // Maybe it's already a raw oid.
    if let Ok(oid) = git2::Oid::from_str(name) {
        if mirror.find_commit(oid).is_ok() {
            return Ok(oid);
        }
    }
    Err(NodeError::RefNotFound(format!("ref '{name}' not found")))
}

/// Pick a sensible default start point: HEAD if set, else the first
/// branch found in the mirror.
fn resolve_default(mirror: &git2::Repository) -> Result<git2::Oid, NodeError> {
    if let Ok(head) = mirror.head() {
        if let Some(oid) = head.target() {
            return Ok(oid);
        }
    }
    // Fall back to the first branch.
    let branches = mirror
        .references()
        .map_err(|e| NodeError::Internal(format!("references: {e}")))?;
    for r in branches.flatten() {
        let Some(name) = r.name() else { continue };
        if !name.starts_with("refs/heads/") {
            continue;
        }
        if let Some(oid) = r.target() {
            return Ok(oid);
        }
    }
    Err(NodeError::RefNotFound("repository has no commits".to_string()))
}
