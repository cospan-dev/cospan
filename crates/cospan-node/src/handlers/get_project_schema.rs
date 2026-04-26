//! `GET /xrpc/dev.panproto.node.getProjectSchema`
//!
//! Returns project-level schema statistics by walking the commit's
//! `SchemaTree` (panproto issue #49) and collecting stats from each
//! `FileSchemaObject` leaf directly. Per-file vertex / edge counts,
//! top-level names, and language come from the structured per-file
//! schemas — no flat assembly, no vertex-id string parsing.

use std::collections::HashMap;
use std::sync::{Arc, LazyLock, Mutex};

use axum::Json;
use axum::extract::{Query, State};
use panproto_core::vcs::{self, FileSchemaObject, Object, Store};
use serde::Deserialize;
use serde_json::{Value, json};

use crate::error::NodeError;
use crate::state::NodeState;

use super::list_commits::{resolve_default, resolve_ref};
use super::structural::humanize_vertex;

/// Cache for on-demand project schema results. Keyed by (did, repo, commit_oid).
static SCHEMA_CACHE: LazyLock<Mutex<HashMap<String, Value>>> =
    LazyLock::new(|| Mutex::new(HashMap::new()));

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

    let vcs_store = store_guard.open(&params.did, &params.repo).ok();
    let marks = store_guard.load_import_marks(&params.did, &params.repo);
    drop(store_guard);

    let commit_oid = match params.commit.as_deref() {
        Some("HEAD") | None => resolve_default(&mirror)?,
        Some(name) => resolve_ref(&mirror, name)?,
    };

    let cache_key = format!("{}:{}:{}", params.did, params.repo, commit_oid);
    if let Ok(cache) = SCHEMA_CACHE.lock()
        && let Some(cached) = cache.get(&cache_key)
    {
        return Ok(Json(cached.clone()));
    }

    // Walk the git tree purely for file count / language extension stats
    // (cheap and covers files the parser skipped).
    let commit = mirror
        .find_commit(commit_oid)
        .map_err(|e| NodeError::Internal(format!("find commit: {e}")))?;
    let tree = commit
        .tree()
        .map_err(|e| NodeError::Internal(format!("commit tree: {e}")))?;

    let registry = panproto_parse::ParserRegistry::new();

    let mut git_file_count = 0usize;
    let mut lang_file_counts: HashMap<String, usize> = HashMap::new();
    tree.walk(git2::TreeWalkMode::PreOrder, |dir, entry| {
        if entry.kind() == Some(git2::ObjectType::Blob) {
            git_file_count += 1;
            let name = entry.name().unwrap_or("");
            let path = if dir.is_empty() {
                name.to_string()
            } else {
                format!("{dir}{name}")
            };
            if let Some(lang) = registry.detect_language(std::path::Path::new(&path)) {
                *lang_file_counts.entry(lang.to_string()).or_default() += 1;
            }
        }
        git2::TreeWalkResult::Ok
    })
    .map_err(|e| NodeError::Internal(format!("tree walk: {e}")))?;

    // Walk the schema tree and collect per-file leaves.
    let leaves: Vec<(String, FileSchemaObject)> = match (vcs_store, marks.get(&commit_oid)) {
        (Some(store), Some(pp_id)) => match store.get(pp_id) {
            Ok(Object::Commit(pp_commit)) => {
                let mut acc: Vec<(String, FileSchemaObject)> = Vec::new();
                vcs::walk_tree(&store, &pp_commit.schema_id, |path, file| {
                    acc.push((path.to_string_lossy().into_owned(), file.clone()));
                    Ok(())
                })
                .map_err(|e| NodeError::Internal(format!("walk schema tree: {e}")))?;
                acc
            }
            _ => Vec::new(),
        },
        _ => Vec::new(),
    };

    // No schema data yet (repo hasn't been pushed via git-remote-cospan).
    if leaves.is_empty() {
        let mut languages: Vec<Value> = lang_file_counts
            .iter()
            .map(|(name, fc)| json!({"name": name, "fileCount": fc, "vertexCount": 0}))
            .collect();
        languages.sort_by(|a, b| b["fileCount"].as_u64().cmp(&a["fileCount"].as_u64()));
        let protocol = lang_file_counts
            .iter()
            .max_by_key(|(_, fc)| *fc)
            .map(|(name, _)| name.clone())
            .unwrap_or_default();

        let result = json!({
            "commit": commit_oid.to_string(),
            "protocol": protocol,
            "totalVertexCount": 0,
            "totalEdgeCount": 0,
            "fileCount": git_file_count,
            "parsedFileCount": 0,
            "languages": languages,
            "fileSchemas": [],
            "needsGitRemoteCospan": true,
        });
        if let Ok(mut cache) = SCHEMA_CACHE.lock() {
            cache.insert(cache_key, result.clone());
        }
        return Ok(Json(result));
    }

    // Aggregate per-file stats directly from each leaf.
    let mut total_vc = 0usize;
    let mut total_ec = 0usize;
    let mut file_schemas: Vec<Value> = Vec::with_capacity(leaves.len());
    let mut lang_vertex_counts: HashMap<String, usize> = HashMap::new();

    for (path, file) in &leaves {
        let vc = file.schema.vertices.len();
        let ec = file.schema.edges.len() + file.cross_file_edges.len();
        total_vc += vc;
        total_ec += ec;

        let language = registry
            .detect_language(std::path::Path::new(path))
            .unwrap_or(file.protocol.as_str())
            .to_string();
        *lang_vertex_counts.entry(language.clone()).or_default() += vc;

        let mut top_names: Vec<String> = Vec::new();
        for vid in file.schema.vertices.keys() {
            if top_names.len() >= 8 {
                break;
            }
            let vid_str: &str = vid;
            let human = humanize_vertex(vid_str);
            if human == vid_str || human.contains(" in ") {
                continue;
            }
            if let Some(start) = human.find('`')
                && let Some(end) = human[start + 1..].find('`')
            {
                let name = human[start + 1..start + 1 + end].to_string();
                if !name.starts_with('$') && !name.is_empty() && !top_names.contains(&name) {
                    top_names.push(name);
                }
            }
        }

        file_schemas.push(json!({
            "path": path,
            "protocol": file.protocol,
            "language": language,
            "vertexCount": vc,
            "edgeCount": ec,
            "topNames": top_names,
        }));
    }
    file_schemas.sort_by(|a, b| b["vertexCount"].as_u64().cmp(&a["vertexCount"].as_u64()));

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

    let result = json!({
        "commit": commit_oid.to_string(),
        "protocol": protocol,
        "totalVertexCount": total_vc,
        "totalEdgeCount": total_ec,
        "fileCount": git_file_count,
        "parsedFileCount": leaves.len(),
        "languages": languages,
        "fileSchemas": file_schemas,
    });
    if let Ok(mut cache) = SCHEMA_CACHE.lock() {
        cache.insert(cache_key, result.clone());
    }
    Ok(Json(result))
}
