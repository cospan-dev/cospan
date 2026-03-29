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
    pub title: String,
    pub body: Option<String>,
    /// Temporary: the DID of the acting user (will be replaced by auth).
    pub did: String,
}

/// POST `/xrpc/dev.cospan.repo.issue.create`
pub async fn handler(
    State(state): State<Arc<AppState>>,
    Json(input): Json<Input>,
) -> Result<Json<serde_json::Value>, AppError> {
    // Parse the repo AT-URI: at://did/dev.cospan.repo/repo-name
    let uri = crate::at_uri::validate(&input.repo).map_err(AppError::InvalidRequest)?;
    let repo_did = &uri.did;
    let repo_name = &uri.rkey;

    let rkey = uuid::Uuid::new_v4().to_string();
    let now = Utc::now();

    let row = db::issue::IssueRow {
        did: input.did.clone(),
        rkey: rkey.clone(),
        repo_did: repo_did.to_string(),
        repo_name: repo_name.to_string(),
        title: input.title.clone(),
        body: input.body.clone(),
        state: "open".to_string(),
        comment_count: 0,
        created_at: now,
        indexed_at: now,
    };
    db::issue::upsert(&state.db, &row).await?;

    // Increment open_issue_count on the repo.
    sqlx::query(
        "UPDATE repos SET open_issue_count = open_issue_count + 1, indexed_at = NOW() \
         WHERE did = $1 AND name = $2",
    )
    .bind(repo_did)
    .bind(repo_name)
    .execute(&state.db)
    .await?;

    Ok(Json(serde_json::json!({
        "uri": format!("at://{}/dev.cospan.repo.issue/{}", input.did, rkey),
        "rkey": rkey,
        "title": input.title,
        "body": input.body,
        "state": "open",
        "createdAt": now.to_rfc3339(),
    })))
}
