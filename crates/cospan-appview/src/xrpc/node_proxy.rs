//! Proxy endpoints that forward requests to the appropriate cospan node.
//!
//! These allow the frontend to fetch objects, refs, and schemas from nodes
//! without needing to know the node URL directly.

use std::sync::Arc;

use axum::Json;
use axum::extract::{Query, State};
use serde::Deserialize;

use crate::error::AppError;
use crate::node_proxy;
use crate::state::AppState;

#[derive(Deserialize)]
pub struct RepoParams {
    pub did: String,
    pub repo: String,
}

#[derive(Deserialize)]
pub struct ObjectParams {
    pub did: String,
    pub repo: String,
    pub id: String,
}

#[derive(Deserialize)]
pub struct ListCommitsParams {
    pub did: String,
    pub repo: String,
    #[serde(rename = "ref")]
    pub ref_name: Option<String>,
    pub limit: Option<i64>,
}

#[derive(Deserialize)]
pub struct DiffCommitsParams {
    pub did: String,
    pub repo: String,
    pub from: String,
    pub to: String,
    #[serde(rename = "contextLines")]
    pub context_lines: Option<i64>,
}

/// GET /xrpc/dev.cospan.node.proxy.listRefs
/// Proxies to the node hosting this repo.
pub async fn proxy_list_refs(
    State(state): State<Arc<AppState>>,
    Query(params): Query<RepoParams>,
) -> Result<Json<serde_json::Value>, AppError> {
    let refs = node_proxy::list_refs(&state, &params.did, &params.repo)
        .await
        .map_err(AppError::NotFound)?;

    let refs_json: Vec<serde_json::Value> = refs
        .into_iter()
        .map(|(name, id)| serde_json::json!({ "ref": name, "target": id.to_string() }))
        .collect();

    Ok(Json(serde_json::json!({ "refs": refs_json })))
}

/// GET /xrpc/dev.cospan.node.proxy.getHead
pub async fn proxy_get_head(
    State(state): State<Arc<AppState>>,
    Query(params): Query<RepoParams>,
) -> Result<Json<serde_json::Value>, AppError> {
    let head = node_proxy::get_head(&state, &params.did, &params.repo)
        .await
        .map_err(AppError::NotFound)?;

    let head_json = match head {
        panproto_core::vcs::HeadState::Branch(name) => {
            serde_json::json!({ "type": "branch", "ref": name })
        }
        panproto_core::vcs::HeadState::Detached(id) => {
            serde_json::json!({ "type": "detached", "target": id.to_string() })
        }
    };

    Ok(Json(serde_json::json!({ "head": head_json })))
}

/// GET /xrpc/dev.cospan.node.proxy.listCommits
pub async fn proxy_list_commits(
    State(state): State<Arc<AppState>>,
    Query(params): Query<ListCommitsParams>,
) -> Result<Json<serde_json::Value>, AppError> {
    let mut extra: Vec<(&str, String)> = Vec::new();
    if let Some(ref r) = params.ref_name {
        extra.push(("ref", r.clone()));
    }
    if let Some(l) = params.limit {
        extra.push(("limit", l.to_string()));
    }

    let result = node_proxy::proxy_get_json(
        &state,
        &params.did,
        &params.repo,
        "dev.cospan.node.listCommits",
        &extra,
    )
    .await
    .map_err(AppError::Upstream)?;

    Ok(Json(result))
}

/// GET /xrpc/dev.cospan.node.proxy.diffCommits
pub async fn proxy_diff_commits(
    State(state): State<Arc<AppState>>,
    Query(params): Query<DiffCommitsParams>,
) -> Result<Json<serde_json::Value>, AppError> {
    let mut extra: Vec<(&str, String)> = vec![
        ("from", params.from.clone()),
        ("to", params.to.clone()),
    ];
    if let Some(c) = params.context_lines {
        extra.push(("contextLines", c.to_string()));
    }

    let result = node_proxy::proxy_get_json(
        &state,
        &params.did,
        &params.repo,
        "dev.cospan.node.diffCommits",
        &extra,
    )
    .await
    .map_err(AppError::Upstream)?;

    Ok(Json(result))
}

// ─── Schema intelligence proxies ───────────────────────────────────

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ProjectSchemaParams {
    pub did: String,
    pub repo: String,
    pub commit: Option<String>,
    pub max_files: Option<i64>,
}

/// GET /xrpc/dev.panproto.node.proxy.getProjectSchema
pub async fn proxy_get_project_schema(
    State(state): State<Arc<AppState>>,
    Query(params): Query<ProjectSchemaParams>,
) -> Result<Json<serde_json::Value>, AppError> {
    let mut extra: Vec<(&str, String)> = Vec::new();
    if let Some(ref c) = params.commit {
        extra.push(("commit", c.clone()));
    }
    if let Some(m) = params.max_files {
        extra.push(("maxFiles", m.to_string()));
    }
    let result = node_proxy::proxy_get_json(
        &state,
        &params.did,
        &params.repo,
        "dev.panproto.node.getProjectSchema",
        &extra,
    )
    .await
    .map_err(AppError::Upstream)?;
    Ok(Json(result))
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CommitSchemaStatsParams {
    pub did: String,
    pub repo: String,
    #[serde(rename = "ref")]
    pub ref_name: Option<String>,
    pub limit: Option<i64>,
}

/// GET /xrpc/dev.panproto.node.proxy.getCommitSchemaStats
pub async fn proxy_get_commit_schema_stats(
    State(state): State<Arc<AppState>>,
    Query(params): Query<CommitSchemaStatsParams>,
) -> Result<Json<serde_json::Value>, AppError> {
    let mut extra: Vec<(&str, String)> = Vec::new();
    if let Some(ref r) = params.ref_name {
        extra.push(("ref", r.clone()));
    }
    if let Some(l) = params.limit {
        extra.push(("limit", l.to_string()));
    }
    let result = node_proxy::proxy_get_json(
        &state,
        &params.did,
        &params.repo,
        "dev.panproto.node.getCommitSchemaStats",
        &extra,
    )
    .await
    .map_err(AppError::Upstream)?;
    Ok(Json(result))
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct FileSchemaParams {
    pub did: String,
    pub repo: String,
    pub commit: String,
    pub path: String,
}

/// GET /xrpc/dev.panproto.node.proxy.getFileSchema
pub async fn proxy_get_file_schema(
    State(state): State<Arc<AppState>>,
    Query(params): Query<FileSchemaParams>,
) -> Result<Json<serde_json::Value>, AppError> {
    let extra: Vec<(&str, String)> = vec![
        ("commit", params.commit.clone()),
        ("path", params.path.clone()),
    ];
    let result = node_proxy::proxy_get_json(
        &state,
        &params.did,
        &params.repo,
        "dev.panproto.node.getFileSchema",
        &extra,
    )
    .await
    .map_err(AppError::Upstream)?;
    Ok(Json(result))
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CompareBranchSchemasParams {
    pub did: String,
    pub repo: String,
    pub base: String,
    pub head: String,
}

/// GET /xrpc/dev.panproto.node.proxy.compareBranchSchemas
pub async fn proxy_compare_branch_schemas(
    State(state): State<Arc<AppState>>,
    Query(params): Query<CompareBranchSchemasParams>,
) -> Result<Json<serde_json::Value>, AppError> {
    let extra: Vec<(&str, String)> = vec![
        ("base", params.base.clone()),
        ("head", params.head.clone()),
    ];
    let result = node_proxy::proxy_get_json(
        &state,
        &params.did,
        &params.repo,
        "dev.panproto.node.compareBranchSchemas",
        &extra,
    )
    .await
    .map_err(AppError::Upstream)?;
    Ok(Json(result))
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DependencyGraphParams {
    pub did: String,
    pub repo: String,
    pub commit: Option<String>,
    pub max_files: Option<i64>,
}

/// GET /xrpc/dev.panproto.node.proxy.getDependencyGraph
pub async fn proxy_get_dependency_graph(
    State(state): State<Arc<AppState>>,
    Query(params): Query<DependencyGraphParams>,
) -> Result<Json<serde_json::Value>, AppError> {
    let mut extra: Vec<(&str, String)> = Vec::new();
    if let Some(ref c) = params.commit {
        extra.push(("commit", c.clone()));
    }
    if let Some(m) = params.max_files {
        extra.push(("maxFiles", m.to_string()));
    }
    let result = node_proxy::proxy_get_json(
        &state,
        &params.did,
        &params.repo,
        "dev.panproto.node.getDependencyGraph",
        &extra,
    )
    .await
    .map_err(AppError::Upstream)?;
    Ok(Json(result))
}

/// GET /xrpc/dev.panproto.node.proxy.getImportStatus
pub async fn proxy_get_import_status(
    State(state): State<Arc<AppState>>,
    Query(params): Query<RepoParams>,
) -> Result<Json<serde_json::Value>, AppError> {
    let result = node_proxy::proxy_get_json(
        &state,
        &params.did,
        &params.repo,
        "dev.panproto.node.getImportStatus",
        &[],
    )
    .await
    .map_err(AppError::Upstream)?;
    Ok(Json(result))
}

/// GET /xrpc/dev.cospan.node.proxy.getObject
pub async fn proxy_get_object(
    State(state): State<Arc<AppState>>,
    Query(params): Query<ObjectParams>,
) -> Result<Json<serde_json::Value>, AppError> {
    let object = node_proxy::fetch_object(&state, &params.did, &params.repo, &params.id)
        .await
        .map_err(AppError::NotFound)?;

    // Serialize the object type and key metadata
    let obj_json = match &object {
        panproto_core::vcs::Object::Schema(s) => serde_json::json!({
            "type": "schema",
            "protocol": s.protocol,
            "vertexCount": s.vertices.len(),
            "edgeCount": s.edges.len(),
        }),
        panproto_core::vcs::Object::Commit(c) => serde_json::json!({
            "type": "commit",
            "message": c.message,
            "author": c.author,
            "timestamp": c.timestamp,
            "schemaId": c.schema_id.to_string(),
            "parents": c.parents.iter().map(|p| p.to_string()).collect::<Vec<_>>(),
            "migrationId": c.migration_id.as_ref().map(|m| m.to_string()),
        }),
        panproto_core::vcs::Object::Migration { src, tgt, .. } => serde_json::json!({
            "type": "migration",
            "src": src.to_string(),
            "tgt": tgt.to_string(),
        }),
        _ => serde_json::json!({ "type": "other" }),
    };

    Ok(Json(serde_json::json!({
        "id": params.id,
        "object": obj_json,
    })))
}
