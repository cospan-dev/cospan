//! `GET /xrpc/dev.panproto.node.getCommitSchemaStats`
//!
//! For a range of commits, returns per-commit schema statistics:
//! total vertex/edge counts and breaking/non-breaking change counts
//! vs the parent commit. Powers the schema evolution sparkline.

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
    pub limit: Option<usize>,
}

pub async fn get_commit_schema_stats(
    State(state): State<Arc<NodeState>>,
    Query(params): Query<Params>,
) -> Result<Json<Value>, NodeError> {
    let limit = params.limit.unwrap_or(30).min(100);

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

    let registry = panproto_parse::ParserRegistry::new();
    let mut commits: Vec<Value> = Vec::new();

    for oid_result in walk.take(limit) {
        let oid = match oid_result {
            Ok(o) => o,
            Err(_) => break,
        };
        let commit = match mirror.find_commit(oid) {
            Ok(c) => c,
            Err(_) => continue,
        };

        let summary = commit.summary().unwrap_or_default().to_string();
        let timestamp = u64::try_from(commit.time().seconds()).unwrap_or(0);

        // Count total vertices by walking the tree
        let tree = match commit.tree() {
            Ok(t) => t,
            Err(_) => continue,
        };
        let (total_vc, total_ec, parsed_fc) =
            count_tree_schema_stats(&mirror, &registry, &tree);

        // Diff against first parent for breaking/non-breaking counts
        let (breaking, non_breaking) = if commit.parent_count() > 0 {
            if let Ok(parent) = commit.parent(0) {
                diff_commit_pair(&mirror, &registry, &parent, &commit)
            } else {
                (0, 0)
            }
        } else {
            (0, 0)
        };

        commits.push(json!({
            "oid": oid.to_string(),
            "timestamp": timestamp,
            "summary": summary,
            "totalVertexCount": total_vc,
            "totalEdgeCount": total_ec,
            "parsedFileCount": parsed_fc,
            "breakingChangeCount": breaking,
            "nonBreakingChangeCount": non_breaking,
        }));
    }

    Ok(Json(json!({ "commits": commits })))
}

/// Count total vertices, edges, and parsed files in a commit tree.
fn count_tree_schema_stats(
    mirror: &git2::Repository,
    registry: &panproto_parse::ParserRegistry,
    tree: &git2::Tree<'_>,
) -> (usize, usize, usize) {
    let mut total_vc = 0usize;
    let mut total_ec = 0usize;
    let mut parsed_fc = 0usize;

    let mut blobs: Vec<(String, git2::Oid)> = Vec::new();
    let _ = tree.walk(git2::TreeWalkMode::PreOrder, |dir, entry| {
        if entry.kind() == Some(git2::ObjectType::Blob) {
            let name = entry.name().unwrap_or("");
            let path = if dir.is_empty() {
                name.to_string()
            } else {
                format!("{dir}{name}")
            };
            blobs.push((path, entry.id()));
        }
        git2::TreeWalkResult::Ok
    });

    // Only parse up to 200 files to keep latency bounded
    for (path, blob_oid) in blobs.iter().take(200) {
        let blob = match mirror.find_blob(*blob_oid) {
            Ok(b) => b,
            Err(_) => continue,
        };
        if let Some((schema, _)) =
            super::structural::parse_any(registry, path, blob.content())
        {
            total_vc += schema.vertices.len();
            total_ec += schema.edges.len();
            parsed_fc += 1;
        }
    }

    (total_vc, total_ec, parsed_fc)
}

/// Diff two commits and return (breaking_count, non_breaking_count).
fn diff_commit_pair(
    mirror: &git2::Repository,
    registry: &panproto_parse::ParserRegistry,
    parent: &git2::Commit<'_>,
    child: &git2::Commit<'_>,
) -> (usize, usize) {
    let parent_tree = match parent.tree() {
        Ok(t) => t,
        Err(_) => return (0, 0),
    };
    let child_tree = match child.tree() {
        Ok(t) => t,
        Err(_) => return (0, 0),
    };

    let diff = match mirror.diff_tree_to_tree(
        Some(&parent_tree),
        Some(&child_tree),
        None,
    ) {
        Ok(d) => d,
        Err(_) => return (0, 0),
    };

    let mut breaking = 0usize;
    let mut non_breaking = 0usize;

    for delta_idx in 0..diff.deltas().len() {
        let delta = match diff.get_delta(delta_idx) {
            Some(d) => d,
            None => continue,
        };
        let new_file = delta.new_file();
        let old_file = delta.old_file();
        let path = new_file
            .path()
            .map(|p| p.to_string_lossy().into_owned())
            .unwrap_or_default();

        let old_bytes = load_blob(mirror, old_file.id());
        let new_bytes = load_blob(mirror, new_file.id());

        if let Some(sd) = super::structural::try_structural_diff(
            registry,
            &path,
            old_bytes.as_deref(),
            new_bytes.as_deref(),
        ) {
            breaking += sd.report.breaking.len();
            non_breaking += sd.report.non_breaking.len();
        }
    }

    (breaking, non_breaking)
}

fn load_blob(mirror: &git2::Repository, oid: git2::Oid) -> Option<Vec<u8>> {
    if oid.is_zero() {
        return None;
    }
    mirror
        .find_blob(oid)
        .ok()
        .map(|b| b.content().to_vec())
}
