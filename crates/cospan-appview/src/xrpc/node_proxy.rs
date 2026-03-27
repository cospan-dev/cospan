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
