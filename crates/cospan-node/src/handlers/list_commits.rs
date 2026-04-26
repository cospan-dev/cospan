//! `GET /xrpc/dev.panproto.node.listCommits` and its cospan alias.
//!
//! Walks the repo's persistent git mirror using libgit2's RevWalk and
//! returns a flat list of commits with the parent pointers needed to
//! draw a DAG. Uses panproto-xrpc's typed response structs to guarantee
//! wire-format compatibility with panproto clients.
//!
//! Query parameters:
//!   - `did`: repo owner
//!   - `repo`: repo name (rkey or human name)
//!   - `ref`: optional ref to start from (default: HEAD / first branch)
//!   - `limit`: optional max commits to return (default 50, max 500)

use std::sync::Arc;

use axum::Json;
use axum::extract::{Query, State};
use panproto_xrpc::{CommitEntry, CommitIdentity, ListCommitsResult};
use serde::Deserialize;

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
) -> Result<Json<ListCommitsResult>, NodeError> {
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

        commits.push(CommitEntry {
            oid: oid.to_string(),
            parents: commit.parent_ids().map(|p| p.to_string()).collect(),
            summary: commit.summary().unwrap_or_default().to_string(),
            message: commit.message().unwrap_or_default().to_string(),
            author: CommitIdentity {
                name: author.name().unwrap_or_default().to_string(),
                email: Some(author.email().unwrap_or_default().to_string()),
            },
            committer: Some(CommitIdentity {
                name: committer.name().unwrap_or_default().to_string(),
                email: Some(committer.email().unwrap_or_default().to_string()),
            }),
            timestamp: u64::try_from(commit.time().seconds()).unwrap_or(0),
            tree_oid: Some(commit.tree_id().to_string()),
        });
    }

    let count = commits.len() as u64;
    Ok(Json(ListCommitsResult {
        commits,
        count,
        start: Some(start_oid.to_string()),
    }))
}

/// Resolve a ref name to its target OID. Accepts short names like
/// "main" as well as fully-qualified "refs/heads/main".
pub(crate) fn resolve_ref(mirror: &git2::Repository, name: &str) -> Result<git2::Oid, NodeError> {
    // Try fully qualified first.
    if let Ok(r) = mirror.find_reference(name)
        && let Some(oid) = r.target()
    {
        return Ok(oid);
    }
    // Try under refs/heads/.
    let candidate = format!("refs/heads/{name}");
    if let Ok(r) = mirror.find_reference(&candidate)
        && let Some(oid) = r.target()
    {
        return Ok(oid);
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
    if let Ok(oid) = git2::Oid::from_str(name)
        && mirror.find_commit(oid).is_ok()
    {
        return Ok(oid);
    }
    Err(NodeError::RefNotFound(format!("ref '{name}' not found")))
}

/// Pick a sensible default start point: HEAD if set, else the first
/// branch found in the mirror.
pub(crate) fn resolve_default(mirror: &git2::Repository) -> Result<git2::Oid, NodeError> {
    if let Ok(head) = mirror.head()
        && let Some(oid) = head.target()
    {
        return Ok(oid);
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
    Err(NodeError::RefNotFound(
        "repository has no commits".to_string(),
    ))
}
