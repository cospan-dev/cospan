use std::sync::Arc;

use axum::Json;
use axum::extract::State;
use chrono::Utc;
use serde::Deserialize;

use crate::db;
use crate::error::AppError;
use crate::middleware::auth::{RepoTarget, RequiredAuth, Role, require_role};
use crate::state::AppState;

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Input {
    /// AT-URI of the repo, e.g. `at://did/dev.cospan.repo/name`
    pub repo: String,
    /// DID of the collaborator to add.
    pub did: String,
    /// Role: "owner", "maintainer", "contributor", "reader".
    pub role: String,
}

/// POST `/xrpc/dev.cospan.repo.collaborator.add`
///
/// Requires:
/// - an authenticated session
/// - the caller to hold at least Maintainer role on the target repo
///   (or be the repo's owner DID)
///
/// The collaborator record is written under the *caller's* DID — there is
/// no `ownerDid` input parameter, because the caller's identity is the sole
/// source of authority.
pub async fn handler(
    State(state): State<Arc<AppState>>,
    RequiredAuth(caller): RequiredAuth,
    Json(input): Json<Input>,
) -> Result<Json<serde_json::Value>, AppError> {
    let uri = crate::at_uri::validate(&input.repo).map_err(AppError::InvalidRequest)?;
    let target = RepoTarget {
        repo_did: uri.did.clone(),
        repo_name: uri.rkey.clone(),
    };

    require_role(&state, &caller, &target, Role::Maintainer)
        .await
        .map_err(|e| AppError::Unauthorized(format!("{e:?}")))?;

    let rkey = uuid::Uuid::new_v4().to_string();
    let now = Utc::now();

    let row = db::collaborator::CollaboratorRow {
        did: caller.did.clone(),
        rkey: rkey.clone(),
        repo_did: target.repo_did.clone(),
        repo_name: target.repo_name.clone(),
        member_did: input.did.clone(),
        role: input.role.clone(),
        created_at: now,
        indexed_at: now,
    };
    db::collaborator::upsert(&state.db, &row).await?;

    Ok(Json(serde_json::json!({
        "uri": format!("at://{}/dev.cospan.repo.collaborator/{}", caller.did, rkey),
        "rkey": rkey,
        "did": input.did,
        "role": input.role,
        "createdAt": now.to_rfc3339(),
    })))
}
