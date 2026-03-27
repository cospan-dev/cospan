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
    /// New state: "open" or "closed".
    pub state: String,
    pub reason: Option<String>,
    /// Temporary: the DID of the acting user (will be replaced by auth).
    pub did: String,
}

/// POST `/xrpc/dev.cospan.repo.issue.state.change`
pub async fn handler(
    State(state): State<Arc<AppState>>,
    Json(input): Json<Input>,
) -> Result<Json<serde_json::Value>, AppError> {
    if input.state != "open" && input.state != "closed" {
        return Err(AppError::InvalidRequest(
            "state must be 'open' or 'closed'".to_string(),
        ));
    }

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

    // Look up the issue to find its repo and current state.
    let issue = db::issue::get_by_pk(&state.db, issue_did, issue_rkey)
        .await?
        .ok_or_else(|| AppError::NotFound(format!("issue {} not found", input.issue)))?;

    let old_state = &issue.state;

    // Insert the state change record.
    let rkey = uuid::Uuid::new_v4().to_string();
    let now = Utc::now();

    let state_row = db::issue_state::IssueStateRow {
        did: input.did.clone(),
        rkey: rkey.clone(),
        issue_uri: input.issue.clone(),
        state: input.state.clone(),
        reason: input.reason.clone(),
        created_at: now,
        indexed_at: now,
    };
    db::issue_state::upsert(&state.db, &state_row).await?;

    // Update the issue state.
    db::issue::update_state(&state.db, issue_did, issue_rkey, &input.state).await?;

    // Update repo open_issue_count if transitioning between open/closed.
    if old_state == "open" && input.state == "closed" {
        sqlx::query(
            "UPDATE repos SET open_issue_count = GREATEST(open_issue_count - 1, 0), indexed_at = NOW() \
             WHERE did = $1 AND name = $2",
        )
        .bind(&issue.repo_did)
        .bind(&issue.repo_name)
        .execute(&state.db)
        .await?;
    } else if old_state == "closed" && input.state == "open" {
        sqlx::query(
            "UPDATE repos SET open_issue_count = open_issue_count + 1, indexed_at = NOW() \
             WHERE did = $1 AND name = $2",
        )
        .bind(&issue.repo_did)
        .bind(&issue.repo_name)
        .execute(&state.db)
        .await?;
    }

    Ok(Json(serde_json::json!({
        "uri": format!("at://{}/dev.cospan.repo.issue.state/{}", input.did, rkey),
        "rkey": rkey,
        "state": input.state,
        "reason": input.reason,
        "createdAt": now.to_rfc3339(),
    })))
}
