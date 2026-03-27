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
    /// AT-URI of the repo, e.g. `at://did/dev.cospan.repo/name`
    pub repo: String,
    pub name: String,
    pub color: String,
    pub description: Option<String>,
    /// Temporary: the DID of the acting user (will be replaced by auth).
    pub did: String,
}

/// POST `/xrpc/dev.cospan.label.definition.create`
pub async fn handler(
    State(state): State<Arc<AppState>>,
    Json(input): Json<Input>,
) -> Result<Json<serde_json::Value>, AppError> {
    // Parse the repo AT-URI: at://did/dev.cospan.repo/repo-name
    let parts: Vec<&str> = input
        .repo
        .strip_prefix("at://")
        .unwrap_or(&input.repo)
        .splitn(3, '/')
        .collect();
    if parts.len() < 3 {
        return Err(AppError::InvalidRequest(
            "repo must be a valid AT-URI like at://did/dev.cospan.repo/name".to_string(),
        ));
    }
    let repo_did = parts[0];
    let repo_name = parts[2];

    let rkey = uuid::Uuid::new_v4().to_string();
    let now = Utc::now();

    let row = db::label::LabelRow {
        did: input.did.clone(),
        rkey: rkey.clone(),
        repo_did: repo_did.to_string(),
        repo_name: repo_name.to_string(),
        name: input.name.clone(),
        color: input.color.clone(),
        description: input.description.clone(),
        created_at: now,
        indexed_at: now,
    };
    db::label::upsert(&state.db, &row).await?;

    Ok(Json(serde_json::json!({
        "uri": format!("at://{}/dev.cospan.label.definition/{}", input.did, rkey),
        "rkey": rkey,
        "name": input.name,
        "color": input.color,
        "description": input.description,
        "createdAt": now.to_rfc3339(),
    })))
}
