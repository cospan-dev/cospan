use std::sync::Arc;

use axum::Json;
use axum::extract::State;
use axum::http::HeaderMap;
use serde::Deserialize;

use crate::auth::oauth::extract_session_id;
use crate::auth::pds_client;
use crate::db;
use crate::error::AppError;
use crate::state::AppState;

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Input {
    /// DID of the repo owner.
    pub did: String,
    /// Name of the repo to delete.
    pub name: String,
}

/// POST /xrpc/dev.cospan.repo.delete
///
/// Deletes a repo from the appview DB. If the session has PDS
/// credentials (full OAuth, not bridge-only), also deletes the
/// dev.cospan.repo record from the user's PDS.
///
/// Requires an authenticated session. The session's DID must match
/// the repo's DID (you can only delete your own repos).
pub async fn handler(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Json(input): Json<Input>,
) -> Result<Json<serde_json::Value>, AppError> {
    let session_id = extract_session_id(&headers)
        .ok_or_else(|| AppError::Unauthorized("sign in required".to_string()))?;
    let session = state
        .session_store
        .get_session(&session_id)
        .await
        .map_err(|e| AppError::Upstream(format!("session lookup: {e}")))?
        .ok_or_else(|| AppError::Unauthorized("session not found".to_string()))?;

    if session.did != input.did {
        return Err(AppError::Unauthorized(
            "you can only delete repos you own".to_string(),
        ));
    }

    // Look up the repo to get its rkey (needed for PDS deletion).
    let repo = db::repo::get(&state.db, &input.did, &input.name)
        .await?
        .ok_or_else(|| {
            AppError::NotFound(format!("repo {}/{} not found", input.did, input.name))
        })?;

    // Try to delete the PDS record if the session has PDS credentials.
    if !session.pds_url.is_empty() && !session.access_token.is_empty() {
        match pds_client::delete_record(&state.http_client, &session, "dev.cospan.repo", &repo.rkey)
            .await
        {
            Ok(_) => tracing::info!(did = %input.did, name = %input.name, "PDS record deleted"),
            Err(e) => tracing::warn!(
                did = %input.did, name = %input.name, error = %e,
                "PDS record deletion failed (continuing with DB delete)"
            ),
        }
    }

    // Delete from the appview DB.
    db::repo::delete(&state.db, &input.did, &input.name).await?;

    tracing::info!(did = %input.did, name = %input.name, "repo deleted");

    Ok(Json(serde_json::json!({
        "ok": true,
        "did": input.did,
        "name": input.name,
    })))
}
