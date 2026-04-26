//! `GET /xrpc/dev.panproto.node.getFileSchema`
//!
//! Returns the schema graph for a single file. With per-file content
//! addressing (panproto issue #49) each commit's `schema_id` points
//! at a `SchemaTree` whose leaves are `FileSchemaObject`s; we walk
//! that tree, find the leaf whose `path` matches the request, and
//! return its schema natively. No vertex-id prefix parsing.

use std::sync::Arc;

use axum::Json;
use axum::extract::{Query, State};
use panproto_core::vcs::{self, FileSchemaObject, Object, Store};
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

    let commit_oid = match params.commit.as_str() {
        "HEAD" => resolve_default(&mirror)?,
        name => resolve_ref(&mirror, name)?,
    };

    let registry = panproto_parse::ParserRegistry::new();
    let language = registry
        .detect_language(std::path::Path::new(&params.path))
        .map(|s| s.to_string());

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

    // Locate the FileSchemaObject for this path by walking the commit's tree.
    let Some(store) = vcs_store else {
        return Ok(empty_response());
    };
    let Some(pp_id) = marks.get(&commit_oid) else {
        return Ok(empty_response());
    };
    let Ok(Object::Commit(commit)) = store.get(pp_id) else {
        return Ok(empty_response());
    };

    let mut found: Option<FileSchemaObject> = None;
    let walk_result = vcs::walk_tree(&store, &commit.schema_id, |path, file| {
        if found.is_some() {
            return Ok(());
        }
        if path.to_string_lossy() == params.path {
            found = Some(file.clone());
        }
        Ok(())
    });
    if let Err(e) = walk_result {
        return Err(NodeError::Internal(format!("walk schema tree: {e}")));
    }

    let Some(file) = found else {
        return Ok(empty_response());
    };

    let schema = &file.schema;

    let mut vertices: Vec<Value> = Vec::with_capacity(schema.vertices.len());
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

    let mut edges: Vec<Value> = Vec::with_capacity(schema.edges.len());
    for edge in schema.edges.keys() {
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
        "language": language.or(Some(file.protocol.clone())),
        "protocol": file.protocol,
        "vertexCount": schema.vertices.len(),
        "edgeCount": schema.edges.len(),
        "vertices": vertices,
        "edges": edges,
    })))
}
