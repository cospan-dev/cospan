//! `GET /xrpc/dev.panproto.node.getProjectSchema`
//!
//! Returns project-level schema statistics by reading the already-imported
//! schema from the panproto-vcs store. The schema was parsed and stored
//! during git push via `import_git_repo_incremental`, so this is a cheap
//! read operation. Language detection uses file extensions from the git
//! tree (no re-parsing). Per-file vertex counts are extracted from the
//! stored schema's vertex IDs (which encode the file path prefix).

use std::collections::HashMap;
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
    pub commit: Option<String>,
    pub max_files: Option<usize>,
}

pub async fn get_project_schema(
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

    // Try to read from the panproto-vcs store first (fast path).
    let vcs_store = store_guard.open(&params.did, &params.repo).ok();
    let marks = store_guard.load_import_marks(&params.did, &params.repo);
    drop(store_guard);

    // Resolve commit
    let commit_oid = match params.commit.as_deref() {
        Some("HEAD") | None => resolve_default(&mirror)?,
        Some(name) => resolve_ref(&mirror, name)?,
    };

    // Try to load the schema from the vcs store via import marks.
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

    // Walk the git tree for file listing and language detection.
    let commit = mirror
        .find_commit(commit_oid)
        .map_err(|e| NodeError::Internal(format!("find commit: {e}")))?;
    let tree = commit
        .tree()
        .map_err(|e| NodeError::Internal(format!("commit tree: {e}")))?;

    let registry = panproto_parse::ParserRegistry::new();

    // Collect all file paths from the tree.
    let mut file_paths: Vec<String> = Vec::new();
    tree.walk(git2::TreeWalkMode::PreOrder, |dir, entry| {
        if entry.kind() == Some(git2::ObjectType::Blob) {
            let name = entry.name().unwrap_or("");
            let path = if dir.is_empty() {
                name.to_string()
            } else {
                format!("{dir}{name}")
            };
            file_paths.push(path);
        }
        git2::TreeWalkResult::Ok
    })
    .map_err(|e| NodeError::Internal(format!("tree walk: {e}")))?;

    let file_count = file_paths.len();

    // Language detection from file extensions (instant, no parsing).
    let mut lang_file_counts: HashMap<String, usize> = HashMap::new();
    for path in &file_paths {
        let p = std::path::Path::new(path);
        if let Some(lang) = registry.detect_language(p) {
            *lang_file_counts.entry(lang.to_string()).or_default() += 1;
        }
    }

    // If we have a stored schema, extract stats from it directly.
    if let Some(ref schema) = stored_schema {
        let total_vc = schema.vertices.len();
        let total_ec = schema.edges.len();

        // Extract per-file vertex counts from vertex IDs.
        // Vertex IDs are prefixed with the file path: "src/repo.ts::Repo::field"
        let mut file_vertex_counts: HashMap<String, usize> = HashMap::new();
        let mut file_top_names: HashMap<String, Vec<String>> = HashMap::new();

        for vid in schema.vertices.keys() {
            let vid_str: &str = vid;
            // Extract file path from vertex ID (everything before the first "::")
            let file_path = if vid_str.contains("::") {
                vid_str.split("::").next().unwrap_or(vid_str)
            } else if vid_str.contains(':') {
                // Lexicon style: "dev.cospan.repo:body.field" - no file path
                continue;
            } else {
                continue;
            };

            *file_vertex_counts.entry(file_path.to_string()).or_default() += 1;

            // Extract top-level names for this file
            let human = humanize_vertex(vid_str);
            if human != vid_str && !human.contains(" in ") {
                if let Some(start) = human.find('`') {
                    if let Some(end) = human[start + 1..].find('`') {
                        let name = human[start + 1..start + 1 + end].to_string();
                        if !name.starts_with('$') && !name.is_empty() {
                            let names = file_top_names
                                .entry(file_path.to_string())
                                .or_default();
                            if !names.contains(&name) && names.len() < 8 {
                                names.push(name);
                            }
                        }
                    }
                }
            }
        }

        // Count per-file edges
        let mut file_edge_counts: HashMap<String, usize> = HashMap::new();
        for (edge, _) in &schema.edges {
            let src_str: &str = &edge.src;
            if src_str.contains("::") {
                let file_path = src_str.split("::").next().unwrap_or(src_str);
                *file_edge_counts.entry(file_path.to_string()).or_default() += 1;
            }
        }

        // Build per-file schema entries
        let mut file_schemas: Vec<Value> = file_vertex_counts
            .iter()
            .map(|(path, vc)| {
                let ec = file_edge_counts.get(path).copied().unwrap_or(0);
                let lang = {
                    let p = std::path::Path::new(path);
                    registry
                        .detect_language(p)
                        .unwrap_or("unknown")
                        .to_string()
                };
                let top_names = file_top_names
                    .get(path)
                    .cloned()
                    .unwrap_or_default();
                json!({
                    "path": path,
                    "language": lang,
                    "vertexCount": vc,
                    "edgeCount": ec,
                    "topNames": top_names,
                })
            })
            .collect();
        file_schemas.sort_by(|a, b| {
            b["vertexCount"].as_u64().cmp(&a["vertexCount"].as_u64())
        });

        // Add per-language vertex counts from the stored schema
        let mut lang_vertex_counts: HashMap<String, usize> = HashMap::new();
        for (path, vc) in &file_vertex_counts {
            let p = std::path::Path::new(path);
            if let Some(lang) = registry.detect_language(p) {
                *lang_vertex_counts.entry(lang.to_string()).or_default() += *vc;
            }
        }

        let mut languages: Vec<Value> = lang_file_counts
            .iter()
            .map(|(name, fc)| {
                json!({
                    "name": name,
                    "fileCount": fc,
                    "vertexCount": lang_vertex_counts.get(name).copied().unwrap_or(0),
                })
            })
            .collect();
        languages.sort_by(|a, b| b["fileCount"].as_u64().cmp(&a["fileCount"].as_u64()));

        let protocol = lang_file_counts
            .iter()
            .max_by_key(|(_, fc)| *fc)
            .map(|(name, _)| name.clone())
            .unwrap_or_default();

        let parsed_count = file_vertex_counts.len();

        return Ok(Json(json!({
            "commit": commit_oid.to_string(),
            "protocol": protocol,
            "totalVertexCount": total_vc,
            "totalEdgeCount": total_ec,
            "fileCount": file_count,
            "parsedFileCount": parsed_count,
            "languages": languages,
            "fileSchemas": file_schemas,
        })));
    }

    // Fallback: no vcs store data. Return language stats from extensions only.
    let mut languages: Vec<Value> = lang_file_counts
        .iter()
        .map(|(name, fc)| json!({ "name": name, "fileCount": fc, "vertexCount": 0 }))
        .collect();
    languages.sort_by(|a, b| b["fileCount"].as_u64().cmp(&a["fileCount"].as_u64()));

    let protocol = lang_file_counts
        .iter()
        .max_by_key(|(_, fc)| *fc)
        .map(|(name, _)| name.clone())
        .unwrap_or_default();

    Ok(Json(json!({
        "commit": commit_oid.to_string(),
        "protocol": protocol,
        "totalVertexCount": 0,
        "totalEdgeCount": 0,
        "fileCount": file_count,
        "parsedFileCount": 0,
        "languages": languages,
        "fileSchemas": [],
    })))
}
