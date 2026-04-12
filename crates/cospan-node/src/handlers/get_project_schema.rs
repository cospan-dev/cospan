//! `GET /xrpc/dev.panproto.node.getProjectSchema`
//!
//! Walks the commit tree at HEAD (or a specified commit), parses every
//! file via panproto's ParserRegistry, and returns per-file schema
//! statistics: language detection, vertex/edge counts, top-level named
//! elements. This powers the repo overview's Schema Health Card.

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
    pub commit: Option<String>,
    pub max_files: Option<usize>,
}

pub async fn get_project_schema(
    State(state): State<Arc<NodeState>>,
    Query(params): Query<Params>,
) -> Result<Json<Value>, NodeError> {
    let max_files = params.max_files.unwrap_or(500).min(1000);

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

    // Walk tree, collect file blobs
    let registry = panproto_parse::ParserRegistry::new();
    let mut file_schemas: Vec<Value> = Vec::new();
    let mut lang_counts: std::collections::HashMap<String, (usize, usize)> =
        std::collections::HashMap::new();
    let mut total_vertices = 0usize;
    let mut total_edges = 0usize;
    let mut parsed_count = 0usize;

    // Collect all blobs from the tree
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

    let file_count = blobs.len();

    for (path, blob_oid) in blobs.iter().take(max_files) {
        let blob = match mirror.find_blob(*blob_oid) {
            Ok(b) => b,
            Err(_) => continue,
        };
        let bytes = blob.content();

        let parsed = super::structural::parse_any(&registry, path, bytes);
        if let Some((schema, language)) = parsed {
            let vc = schema.vertices.len();
            let ec = schema.edges.len();
            total_vertices += vc;
            total_edges += ec;
            parsed_count += 1;

            // Extract top-level named elements (non-anonymous, non-file-path vertices)
            let mut top_names: Vec<String> = Vec::new();
            for vid in schema.vertices.keys() {
                let vid_str: &str = vid;
                let name = humanize_vertex(vid_str);
                // Only keep simple top-level names (no "in" = top scope)
                if !name.contains(" in ") && name.starts_with('`') && name.ends_with('`') {
                    top_names.push(name[1..name.len() - 1].to_string());
                }
            }
            top_names.sort();
            top_names.dedup();
            top_names.truncate(8);

            let entry = lang_counts.entry(language.clone()).or_insert((0, 0));
            entry.0 += 1;
            entry.1 += vc;

            file_schemas.push(json!({
                "path": path,
                "language": language,
                "vertexCount": vc,
                "edgeCount": ec,
                "topNames": top_names,
            }));
        }
    }

    // Sort languages by file count descending
    let mut languages: Vec<Value> = lang_counts
        .iter()
        .map(|(name, (fc, vc))| {
            json!({
                "name": name,
                "fileCount": fc,
                "vertexCount": vc,
            })
        })
        .collect();
    languages.sort_by(|a, b| {
        b["fileCount"]
            .as_u64()
            .cmp(&a["fileCount"].as_u64())
    });

    // Dominant protocol
    let protocol = lang_counts
        .iter()
        .max_by_key(|(_, (fc, _))| *fc)
        .map(|(name, _)| name.clone())
        .unwrap_or_default();

    // Sort file schemas by vertex count descending
    file_schemas.sort_by(|a, b| {
        b["vertexCount"]
            .as_u64()
            .cmp(&a["vertexCount"].as_u64())
    });

    Ok(Json(json!({
        "commit": commit_oid.to_string(),
        "protocol": protocol,
        "totalVertexCount": total_vertices,
        "totalEdgeCount": total_edges,
        "fileCount": file_count,
        "parsedFileCount": parsed_count,
        "languages": languages,
        "fileSchemas": file_schemas,
    })))
}
