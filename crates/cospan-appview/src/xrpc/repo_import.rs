use std::sync::Arc;

use axum::Json;
use axum::extract::State;
use chrono::Utc;
use serde::Deserialize;

use crate::at_uri;
use crate::db;
use crate::error::AppError;
use crate::state::AppState;

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Input {
    pub source_uri: String,
    pub name: Option<String>,
    pub did: String,
}

/// POST `/xrpc/dev.cospan.repo.import`
pub async fn handler(
    State(state): State<Arc<AppState>>,
    Json(input): Json<Input>,
) -> Result<Json<serde_json::Value>, AppError> {
    // Parse the source AT-URI to find the Tangled repo
    let parsed = at_uri::validate(&input.source_uri).map_err(AppError::InvalidRequest)?;

    // Look up the source repo in the DB
    let source_repo = db::repo::get(&state.db, &parsed.did, &parsed.rkey)
        .await?
        .ok_or_else(|| {
            AppError::InvalidRequest(format!("source repo not found: {}", input.source_uri))
        })?;

    let repo_name = input.name.unwrap_or_else(|| source_repo.name.clone());
    let rkey = uuid::Uuid::new_v4().to_string();
    let now = Utc::now();

    let row = db::repo::RepoRow {
        did: input.did.clone(),
        rkey: rkey.clone(),
        name: repo_name.clone(),
        description: source_repo.description.clone(),
        protocol: source_repo.protocol.clone(),
        node_did: source_repo.node_did.clone(),
        node_url: source_repo.node_url.clone(),
        default_branch: source_repo.default_branch.clone(),
        visibility: source_repo.visibility.clone(),
        source_repo: Some(input.source_uri.clone()),
        star_count: 0,
        fork_count: 0,
        open_issue_count: 0,
        open_mr_count: 0,
        source: "imported".to_string(),
        source_uri: Some(input.source_uri.clone()),
        created_at: now,
        indexed_at: now,
    };
    db::repo::upsert(&state.db, &row).await?;

    Ok(Json(serde_json::json!({
        "uri": format!("at://{}/dev.cospan.repo/{}", input.did, rkey),
        "rkey": rkey,
        "did": input.did,
        "name": repo_name,
        "source": "imported",
        "sourceUri": input.source_uri,
        "createdAt": now.to_rfc3339(),
    })))
}
