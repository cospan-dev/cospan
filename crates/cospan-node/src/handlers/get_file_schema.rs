//! `GET /xrpc/dev.panproto.node.getFileSchema`
//!
//! Parses a single file at a specific commit and returns its complete
//! schema graph with human-readable labels. Powers the file browser's
//! schema sidebar.

use std::sync::Arc;

use axum::Json;
use axum::extract::{Query, State};
use serde::Deserialize;
use serde_json::{Value, json};

use crate::error::NodeError;
use crate::state::NodeState;

use super::list_commits::{resolve_default, resolve_ref};
use super::structural::humanize_vertex;

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Params {
    pub did: String,
    pub repo: String,
    pub commit: String,
    pub path: String,
}

pub async fn get_file_schema(
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

    // Resolve commit
    let commit_oid = match params.commit.as_str() {
        "HEAD" => resolve_default(&mirror)?,
        name => resolve_ref(&mirror, name)?,
    };

    let commit = mirror
        .find_commit(commit_oid)
        .map_err(|e| NodeError::Internal(format!("find commit: {e}")))?;
    let tree = commit
        .tree()
        .map_err(|e| NodeError::Internal(format!("commit tree: {e}")))?;

    // Find the blob at the given path
    let entry = tree
        .get_path(std::path::Path::new(&params.path))
        .map_err(|_| {
            NodeError::ObjectNotFound(format!("file '{}' not found in commit", params.path))
        })?;

    let blob = mirror
        .find_blob(entry.id())
        .map_err(|e| NodeError::Internal(format!("find blob: {e}")))?;

    let registry = panproto_parse::ParserRegistry::new();
    let parsed = super::structural::parse_any(&registry, &params.path, blob.content());

    let (schema, language) = match parsed {
        Some(pair) => pair,
        None => {
            return Ok(Json(json!({
                "path": params.path,
                "commit": commit_oid.to_string(),
                "language": null,
                "vertexCount": 0,
                "edgeCount": 0,
                "vertices": [],
                "edges": [],
            })));
        }
    };

    // Build vertex list with human labels, filtering pure-anonymous vertices
    let mut vertices: Vec<Value> = Vec::new();
    for (vid, vertex) in &schema.vertices {
        let vid_str: &str = vid;
        let human = humanize_vertex(vid_str);
        // Skip purely anonymous vertices (the label is just the raw ID)
        if human == vid_str {
            continue;
        }
        // Extract the leaf name
        let name = if human.starts_with('`') {
            let end = human.find("` in").unwrap_or(human.len() - 1);
            human[1..end].to_string()
        } else {
            human.clone()
        };
        vertices.push(json!({
            "id": vid_str,
            "name": name,
            "kind": vertex.kind.as_ref(),
            "humanLabel": human,
        }));
    }
    // Sort by name for stable output
    vertices.sort_by(|a, b| {
        a["name"].as_str().cmp(&b["name"].as_str())
    });

    // Build edge list with human labels
    let mut edges: Vec<Value> = Vec::new();
    for (edge, _) in &schema.edges {
        let src_str: &str = &edge.src;
        let tgt_str: &str = &edge.tgt;
        let src_human = humanize_vertex(src_str);
        let tgt_human = humanize_vertex(tgt_str);
        // Skip edges where both ends are anonymous
        if src_human == src_str && tgt_human == tgt_str {
            continue;
        }
        let edge_name: Option<&str> = edge.name.as_deref();
        let human_label = match edge_name {
            Some(n) if !n.starts_with('$') => {
                format!("{src_human} -> {tgt_human} (via `{n}`)")
            }
            _ => format!("{src_human} -> {tgt_human}"),
        };
        edges.push(json!({
            "src": src_str,
            "tgt": tgt_str,
            "kind": edge.kind.as_ref(),
            "name": edge_name,
            "humanLabel": human_label,
        }));
    }

    Ok(Json(json!({
        "path": params.path,
        "commit": commit_oid.to_string(),
        "language": language,
        "vertexCount": schema.vertices.len(),
        "edgeCount": schema.edges.len(),
        "vertices": vertices,
        "edges": edges,
    })))
}
