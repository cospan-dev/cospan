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
    /// AT-URI of the issue, e.g. `at://did/dev.cospan.repo.issue/rkey`
    pub issue: String,
    pub body: String,
    /// Temporary: the DID of the acting user (will be replaced by auth).
    pub did: String,
}

/// POST `/xrpc/dev.cospan.repo.issue.comment.create`
pub async fn handler(
    State(state): State<Arc<AppState>>,
    Json(input): Json<Input>,
) -> Result<Json<serde_json::Value>, AppError> {
    // Parse the issue AT-URI: at://did/dev.cospan.repo.issue/rkey
    let parts: Vec<&str> = input
        .issue
        .strip_prefix("at://")
        .unwrap_or(&input.issue)
        .splitn(3, '/')
        .collect();
    if parts.len() < 3 {
        return Err(AppError::InvalidRequest(
            "issue must be a valid AT-URI like at://did/dev.cospan.repo.issue/rkey".to_string(),
        ));
    }
    let issue_did = parts[0];
    let issue_rkey = parts[2];

    let rkey = uuid::Uuid::new_v4().to_string();
    let now = Utc::now();

    let row = db::issue_comment::IssueCommentRow {
        did: input.did.clone(),
        rkey: rkey.clone(),
        issue_uri: input.issue.clone(),
        body: input.body.clone(),
        created_at: now,
        indexed_at: now,
    };
    db::issue_comment::upsert(&state.db, &row).await?;

    // Increment comment_count on the issue.
    db::issue::increment_comment_count(&state.db, issue_did, issue_rkey).await?;

    Ok(Json(serde_json::json!({
        "uri": format!("at://{}/dev.cospan.repo.issue.comment/{}", input.did, rkey),
        "rkey": rkey,
        "body": input.body,
        "createdAt": now.to_rfc3339(),
    })))
}
