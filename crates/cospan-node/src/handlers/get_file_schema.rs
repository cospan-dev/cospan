//! `GET /xrpc/dev.panproto.node.getFileSchema`
//!
//! Returns the schema graph for a single file by reading the already-
//! imported project schema from the panproto-vcs store and filtering to
//! vertices/edges whose IDs start with the requested file path. Falls
//! back to on-demand parsing if the vcs store is unavailable.

use std::sync::Arc;

use axum::Json;
use axum::extract::{Query, State};
use panproto_core::vcs::{Object, Store};
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
    let store_guard = state.store.lock().await;
    if !store_guard.has_git_mirror(&params.did, &params.repo) {
        return Err(NodeError::RefNotFound(format!(
            "repo {}/{} not found",
            params.did, params.repo
        )));
    }
    let mirror = store_guard
        .open_or_init_git_mirror(&params.did, &params.repo)
        .map_err(|e| NodeError::Internal(format!("open mirror: {e}")))?;

    let vcs_store = store_guard.open(&params.did, &params.repo).ok();
    let marks = store_guard.load_import_marks(&params.did, &params.repo);
    drop(store_guard);

    // Resolve commit
    let commit_oid = match params.commit.as_str() {
        "HEAD" => resolve_default(&mirror)?,
        name => resolve_ref(&mirror, name)?,
    };

    let empty_response = || {
        Json(json!({
            "path": params.path,
            "commit": commit_oid.to_string(),
            "language": serde_json::Value::Null,
            "vertexCount": 0,
            "edgeCount": 0,
            "vertices": [],
            "edges": [],
        }))
    };

    // Detect language from extension
    let registry = panproto_parse::ParserRegistry::new();
    let language = registry
        .detect_language(std::path::Path::new(&params.path))
        .map(|s| s.to_string());

    // Try to read from the vcs store (fast path).
    let stored_schema = marks
        .get(&commit_oid)
        .and_then(|pp_id| vcs_store.as_ref()?.get(pp_id).ok())
        .and_then(|obj| match obj {
            Object::Commit(c) => vcs_store.as_ref()?.get(&c.schema_id).ok(),
            _ => None,
        })
        .and_then(|obj| match obj {
            Object::Schema(s) => Some(*s),
            _ => None,
        });

    if let Some(schema) = stored_schema {
        let file_prefix = format!("{}::", params.path);

        // Filter vertices belonging to this file
        let mut vertices: Vec<Value> = Vec::new();
        let mut total_vc = 0usize;
        for (vid, vertex) in &schema.vertices {
            let vid_str: &str = vid;
            if !vid_str.starts_with(&file_prefix) {
                continue;
            }
            total_vc += 1;
            let human = humanize_vertex(vid_str);
            if human == vid_str {
                continue; // Skip anonymous
            }
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
        vertices.sort_by(|a, b| a["name"].as_str().cmp(&b["name"].as_str()));

        // Filter edges belonging to this file
        let mut edges: Vec<Value> = Vec::new();
        let mut total_ec = 0usize;
        for (edge, _) in &schema.edges {
            let src_str: &str = &edge.src;
            let tgt_str: &str = &edge.tgt;
            if !src_str.starts_with(&file_prefix) && !tgt_str.starts_with(&file_prefix) {
                continue;
            }
            total_ec += 1;
            let src_human = humanize_vertex(src_str);
            let tgt_human = humanize_vertex(tgt_str);
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

        return Ok(Json(json!({
            "path": params.path,
            "commit": commit_oid.to_string(),
            "language": language,
            "vertexCount": total_vc,
            "edgeCount": total_ec,
            "vertices": vertices,
            "edges": edges,
        })));
    }

    // Fallback: parse on demand from the git blob.
    let commit = mirror
        .find_commit(commit_oid)
        .map_err(|e| NodeError::Internal(format!("find commit: {e}")))?;
    let tree = commit
        .tree()
        .map_err(|e| NodeError::Internal(format!("commit tree: {e}")))?;

    let entry = match tree.get_path(std::path::Path::new(&params.path)) {
        Ok(e) => e,
        Err(_) => return Ok(empty_response()),
    };

    let blob = match mirror.find_blob(entry.id()) {
        Ok(b) => b,
        Err(_) => return Ok(empty_response()),
    };

    let parsed = super::structural::parse_any(&registry, &params.path, blob.content());
    let (schema, lang) = match parsed {
        Some(pair) => pair,
        None => return Ok(empty_response()),
    };

    let mut vertices: Vec<Value> = Vec::new();
    for (vid, vertex) in &schema.vertices {
        let vid_str: &str = vid;
        let human = humanize_vertex(vid_str);
        if human == vid_str {
            continue;
        }
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
    vertices.sort_by(|a, b| a["name"].as_str().cmp(&b["name"].as_str()));

    let mut edges: Vec<Value> = Vec::new();
    for (edge, _) in &schema.edges {
        let src_str: &str = &edge.src;
        let tgt_str: &str = &edge.tgt;
        let src_human = humanize_vertex(src_str);
        let tgt_human = humanize_vertex(tgt_str);
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
        "language": lang,
        "vertexCount": schema.vertices.len(),
        "edgeCount": schema.edges.len(),
        "vertices": vertices,
        "edges": edges,
    })))
}
