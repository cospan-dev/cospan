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
    /// DID of the collaborator to add.
    pub did: String,
    /// Role: "admin", "contributor", "reader", etc.
    pub role: String,
    /// Temporary: the DID of the repo owner / acting user (will be replaced by auth).
    pub owner_did: String,
}

/// POST `/xrpc/dev.cospan.repo.collaborator.add`
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

    let row = db::collaborator::CollaboratorRow {
        did: input.owner_did.clone(),
        rkey: rkey.clone(),
        repo_did: repo_did.to_string(),
        repo_name: repo_name.to_string(),
        member_did: input.did.clone(),
        role: input.role.clone(),
        created_at: now,
        indexed_at: now,
    };
    db::collaborator::upsert(&state.db, &row).await?;

    Ok(Json(serde_json::json!({
        "uri": format!("at://{}/dev.cospan.repo.collaborator/{}", input.owner_did, rkey),
        "rkey": rkey,
        "did": input.did,
        "role": input.role,
        "createdAt": now.to_rfc3339(),
    })))
}
