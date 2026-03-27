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
    /// AT-URI of the source repo, e.g. `at://did/dev.cospan.repo/name`
    pub source_repo: String,
    /// DID of the user forking the repo.
    pub did: String,
    /// Name for the forked repo (defaults to original name).
    pub name: Option<String>,
}

/// POST `/xrpc/dev.cospan.repo.fork`
pub async fn handler(
    State(state): State<Arc<AppState>>,
    Json(input): Json<Input>,
) -> Result<Json<serde_json::Value>, AppError> {
    // Parse source repo AT-URI: at://did/dev.cospan.repo/repo-name
    let parts: Vec<&str> = input
        .source_repo
        .strip_prefix("at://")
        .unwrap_or(&input.source_repo)
        .splitn(3, '/')
        .collect();
    if parts.len() < 3 {
        return Err(AppError::InvalidRequest(
            "sourceRepo must be a valid AT-URI like at://did/dev.cospan.repo/name".to_string(),
        ));
    }
    let source_did = parts[0];
    let source_name = parts[2];

    // Look up the source repo to copy its metadata.
    let source = db::repo::get(&state.db, source_did, source_name)
        .await?
        .ok_or_else(|| {
            AppError::NotFound(format!("source repo {} not found", input.source_repo))
        })?;

    let fork_name = input.name.unwrap_or_else(|| source.name.clone());
    let rkey = uuid::Uuid::new_v4().to_string();
    let now = Utc::now();

    let row = db::repo::RepoRow {
        did: input.did.clone(),
        rkey: rkey.clone(),
        name: fork_name.clone(),
        description: source.description.clone(),
        protocol: source.protocol.clone(),
        node_did: String::new(),
        node_url: String::new(),
        default_branch: source.default_branch.clone(),
        visibility: source.visibility.clone(),
        source_repo: Some(input.source_repo.clone()),
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

    // Increment fork_count on the source repo.
    sqlx::query(
        "UPDATE repos SET fork_count = fork_count + 1, indexed_at = NOW() \
         WHERE did = $1 AND name = $2",
    )
    .bind(source_did)
    .bind(source_name)
    .execute(&state.db)
    .await?;

    Ok(Json(serde_json::json!({
        "uri": format!("at://{}/dev.cospan.repo/{}", input.did, rkey),
        "rkey": rkey,
        "did": input.did,
        "name": fork_name,
        "sourceRepo": input.source_repo,
        "createdAt": now.to_rfc3339(),
    })))
}
