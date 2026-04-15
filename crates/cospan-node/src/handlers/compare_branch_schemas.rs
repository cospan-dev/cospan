//! `GET /xrpc/dev.panproto.node.compareBranchSchemas`
//!
//! Structural comparison between two refs (branches or tags). Resolves
//! both refs, diffs the trees, runs panproto structural diff on each
//! changed file, and aggregates breaking/non-breaking change counts
//! with human-readable labels. Powers the PR compatibility badge and
//! the branch comparison feature.

use std::sync::Arc;

use axum::Json;
use axum::extract::{Query, State};
use serde::Deserialize;
use serde_json::{Value, json};

use crate::error::NodeError;
use crate::state::NodeState;

use super::list_commits::resolve_ref;

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Params {
    pub did: String,
    pub repo: String,
    pub base: String,
    pub head: String,
}

pub async fn compare_branch_schemas(
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

    let base_oid = resolve_ref(&mirror, &params.base)?;
    let head_oid = resolve_ref(&mirror, &params.head)?;

    let base_commit = mirror
        .find_commit(base_oid)
        .map_err(|_| NodeError::ObjectNotFound(params.base.clone()))?;
    let head_commit = mirror
        .find_commit(head_oid)
        .map_err(|_| NodeError::ObjectNotFound(params.head.clone()))?;

    let base_tree = base_commit
        .tree()
        .map_err(|e| NodeError::Internal(format!("base tree: {e}")))?;
    let head_tree = head_commit
        .tree()
        .map_err(|e| NodeError::Internal(format!("head tree: {e}")))?;

    let diff = mirror
        .diff_tree_to_tree(Some(&base_tree), Some(&head_tree), None)
        .map_err(|e| NodeError::Internal(format!("diff trees: {e}")))?;

    let registry = panproto_parse::ParserRegistry::new();

    let mut total_breaking = 0usize;
    let mut total_non_breaking = 0usize;
    let mut total_added_vertices = 0usize;
    let mut total_removed_vertices = 0usize;
    let mut total_added_edges = 0usize;
    let mut total_removed_edges = 0usize;
    let mut breaking_changes: Vec<Value> = Vec::new();
    let mut non_breaking_changes: Vec<Value> = Vec::new();
    let mut scope_changes: Vec<Value> = Vec::new();
    let mut changed_files: Vec<String> = Vec::new();
    let mut base_vertex_total = 0usize;
    let mut head_vertex_total = 0usize;

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

        changed_files.push(path.clone());

        let old_bytes = load_blob(&mirror, old_file.id());
        let new_bytes = load_blob(&mirror, new_file.id());

        if let Some(sd) = super::structural::try_structural_diff(
            &registry,
            &path,
            old_bytes.as_deref(),
            new_bytes.as_deref(),
        ) {
            total_breaking += sd.report.breaking.len();
            total_non_breaking += sd.report.non_breaking.len();
            total_added_vertices += sd.raw_diff.added_vertices.len();
            total_removed_vertices += sd.raw_diff.removed_vertices.len();
            total_added_edges += sd.raw_diff.added_edges.len();
            total_removed_edges += sd.raw_diff.removed_edges.len();
            base_vertex_total += sd.old_vertex_count;
            head_vertex_total += sd.new_vertex_count;

            // Collect individual changes (cap at 50 total for wire size)
            let sd_json = super::structural::structural_diff_to_json(
                &sd,
                old_bytes.as_deref(),
                new_bytes.as_deref(),
            );
            if let Some(bc) = sd_json["breakingChanges"].as_array() {
                for c in bc {
                    if breaking_changes.len() < 50 {
                        breaking_changes.push(c.clone());
                    }
                }
            }
            if let Some(nbc) = sd_json["nonBreakingChanges"].as_array() {
                for c in nbc {
                    if non_breaking_changes.len() < 50 {
                        non_breaking_changes.push(c.clone());
                    }
                }
            }
            // Include scope-level changes per file (tagged with path)
            if let Some(sc) = sd_json["scopeChanges"].as_array() {
                for c in sc {
                    if scope_changes.len() < 100 {
                        let mut tagged = c.clone();
                        if let Some(obj) = tagged.as_object_mut() {
                            obj.insert("path".to_string(), json!(path));
                        }
                        scope_changes.push(tagged);
                    }
                }
            }
        }
    }

    let compatible = total_breaking == 0;

    Ok(Json(json!({
        "base": { "ref": params.base, "oid": base_oid.to_string() },
        "head": { "ref": params.head, "oid": head_oid.to_string() },
        "compatible": compatible,
        "verdict": if compatible { "compatible" } else { "breaking" },
        "breakingCount": total_breaking,
        "nonBreakingCount": total_non_breaking,
        "addedVertices": total_added_vertices,
        "removedVertices": total_removed_vertices,
        "addedEdges": total_added_edges,
        "removedEdges": total_removed_edges,
        "breakingChanges": breaking_changes,
        "nonBreakingChanges": non_breaking_changes,
        "scopeChanges": scope_changes,
        "changedFiles": changed_files,
        "baseVertexCount": base_vertex_total,
        "headVertexCount": head_vertex_total,
    })))
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
