use std::sync::Arc;

use axum::Json;
use axum::extract::State;
use chrono::Utc;
use serde::Deserialize;

use crate::db;
use crate::error::AppError;
use crate::state::AppState;

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Input {
    pub did: String,
    pub name: String,
    pub description: Option<String>,
    pub protocol: Option<String>,
    pub visibility: Option<String>,
}

/// POST `/xrpc/dev.cospan.repo.create`
pub async fn handler(
    State(state): State<Arc<AppState>>,
    Json(input): Json<Input>,
) -> Result<Json<serde_json::Value>, AppError> {
    if input.name.is_empty() {
        return Err(AppError::InvalidRequest("name is required".to_string()));
    }

    let rkey = uuid::Uuid::new_v4().to_string();
    let now = Utc::now();
    let protocol = input.protocol.unwrap_or_else(|| "typescript".to_string());
    let visibility = input.visibility.unwrap_or_else(|| "public".to_string());

    let row = db::repo::RepoRow {
        did: input.did.clone(),
        rkey: rkey.clone(),
        name: input.name.clone(),
        description: input.description.clone(),
        protocol: protocol.clone(),
        node_did: String::new(),
        node_url: String::new(),
        default_branch: "main".to_string(),
        visibility: visibility.clone(),
        source_repo: None,
        star_count: 0,
        fork_count: 0,
        open_issue_count: 0,
        open_mr_count: 0,
        source: "api".to_string(),
        source_uri: None,
        created_at: now,
        indexed_at: now,
    };
    db::repo::upsert(&state.db, &row).await?;

    Ok(Json(serde_json::json!({
        "uri": format!("at://{}/dev.cospan.repo/{}", input.did, rkey),
        "rkey": rkey,
        "did": input.did,
        "name": input.name,
        "description": input.description,
        "protocol": protocol,
        "visibility": visibility,
        "createdAt": now.to_rfc3339(),
    })))
}
