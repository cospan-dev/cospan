//! `GET /xrpc/dev.panproto.node.getDependencyGraph`
//!
//! Builds a cross-file dependency graph using panproto's ProjectBuilder.
//! Parses all files in the commit tree, assembles the project-level
//! coproduct schema, then finds edges whose src and tgt vertices belong
//! to different files. Powers the interactive dependency graph view.

use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::Arc;

use axum::Json;
use axum::extract::{Query, State};
use panproto_project::ProjectBuilder;
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
    pub commit: Option<String>,
    pub max_files: Option<usize>,
}

pub async fn get_dependency_graph(
    State(state): State<Arc<NodeState>>,
    Query(params): Query<Params>,
) -> Result<Json<Value>, NodeError> {
    let max_files = params.max_files.unwrap_or(200).min(500);

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

    let commit_oid = match params.commit.as_deref() {
        Some("HEAD") | None => resolve_default(&mirror)?,
        Some(name) => resolve_ref(&mirror, name)?,
    };

    let commit = mirror
        .find_commit(commit_oid)
        .map_err(|e| NodeError::Internal(format!("find commit: {e}")))?;
    let tree = commit
        .tree()
        .map_err(|e| NodeError::Internal(format!("commit tree: {e}")))?;

    // Collect all blobs
    let mut blobs: Vec<(String, git2::Oid)> = Vec::new();
    tree.walk(git2::TreeWalkMode::PreOrder, |dir, entry| {
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
    })
    .map_err(|e| NodeError::Internal(format!("tree walk: {e}")))?;

    // Build project schema via ProjectBuilder
    let mut builder = ProjectBuilder::new();
    let mut file_count = 0usize;

    for (path, blob_oid) in blobs.iter().take(max_files) {
        let blob = match mirror.find_blob(*blob_oid) {
            Ok(b) => b,
            Err(_) => continue,
        };
        let p = PathBuf::from(path);
        if builder.add_file(&p, blob.content()).is_ok() {
            file_count += 1;
        }
    }

    if file_count == 0 {
        return Ok(Json(json!({
            "commit": commit_oid.to_string(),
            "nodes": [],
            "edges": [],
        })));
    }

    let project = match builder.build() {
        Ok(p) => p,
        Err(e) => {
            tracing::warn!(error = %e, "project build failed for dependency graph");
            return Ok(Json(json!({
                "commit": commit_oid.to_string(),
                "nodes": [],
                "edges": [],
            })));
        }
    };

    // Build vertex -> file mapping (Name -> file path string)
    let mut vertex_to_file: HashMap<String, String> = HashMap::new();
    for (path, names) in &project.file_map {
        let path_str = path.to_string_lossy().to_string();
        for name in names {
            vertex_to_file.insert(name.to_string(), path_str.clone());
        }
    }

    // Build nodes: one per file that had parseable content
    let mut nodes: Vec<Value> = Vec::new();
    let mut file_vertex_counts: HashMap<String, usize> = HashMap::new();
    for (path, names) in &project.file_map {
        let path_str = path.to_string_lossy().to_string();
        file_vertex_counts.insert(path_str, names.len());
    }

    for (path, vc) in &file_vertex_counts {
        let language = project
            .protocol_map
            .get(&PathBuf::from(path))
            .map(|s| s.as_str())
            .unwrap_or("");
        let label = PathBuf::from(path)
            .file_name()
            .map(|f| f.to_string_lossy().to_string())
            .unwrap_or_else(|| path.clone());
        nodes.push(json!({
            "id": path,
            "language": language,
            "vertexCount": vc,
            "label": label,
        }));
    }

    // Find cross-file edges by looking up src/tgt vertex names in the file map
    let mut edge_set: std::collections::HashSet<(String, String)> =
        std::collections::HashSet::new();
    let mut dep_edges: Vec<Value> = Vec::new();

    for edge in project.schema.edges.keys() {
        let src_str: &str = &edge.src;
        let tgt_str: &str = &edge.tgt;
        let src_file = vertex_to_file.get(src_str);
        let tgt_file = vertex_to_file.get(tgt_str);

        if let (Some(sf), Some(tf)) = (src_file, tgt_file)
            && sf != tf
            && edge_set.insert((sf.clone(), tf.clone()))
        {
            dep_edges.push(json!({
                "src": sf,
                "tgt": tf,
                "kind": edge.kind.as_ref(),
                "label": edge.kind.as_ref(),
            }));
        }
    }

    // Sort nodes by vertex count descending
    nodes.sort_by(|a, b| b["vertexCount"].as_u64().cmp(&a["vertexCount"].as_u64()));

    Ok(Json(json!({
        "commit": commit_oid.to_string(),
        "nodes": nodes,
        "edges": dep_edges,
    })))
}
