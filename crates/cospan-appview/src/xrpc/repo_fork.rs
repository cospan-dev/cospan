use std::sync::Arc;

use axum::Json;
use axum::extract::State;
use axum::http::HeaderMap;
use chrono::Utc;
use serde::Deserialize;

use crate::auth::oauth::extract_session_id;
use crate::auth::pds_client::{self, PdsClientError};
use crate::db;
use crate::error::AppError;
use crate::state::AppState;

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Input {
    /// AT-URI of the source repo, e.g. `at://did/dev.cospan.repo/name`
    pub source_repo: String,
    /// Optional name for the fork (defaults to the source's name).
    pub name: Option<String>,
}

/// POST `/xrpc/dev.cospan.repo.fork`
///
/// Requires an authenticated session. The forked repo is created on the
/// user's PDS with a strong ref to the source, and is assigned to the
/// Cospan default node for git data hosting.
///
/// Note: this creates the PDS record and DB row. Copying the git objects
/// from the source node to the destination node is a separate concern
/// handled asynchronously by the node after the record is observed via
/// the firehose.
pub async fn handler(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Json(input): Json<Input>,
) -> Result<Json<serde_json::Value>, AppError> {
    // 1. Require an authenticated session.
    let session_id = extract_session_id(&headers)
        .ok_or_else(|| AppError::Unauthorized("sign in required to fork".to_string()))?;
    let session = state
        .session_store
        .get_session(&session_id)
        .await
        .map_err(|e| AppError::Upstream(format!("session lookup failed: {e}")))?
        .ok_or_else(|| AppError::Unauthorized("session not found".to_string()))?;

    // 2. Parse and validate the source repo AT-URI.
    let uri = crate::at_uri::validate(&input.source_repo).map_err(AppError::InvalidRequest)?;
    let source_did = uri.did.clone();
    let source_rkey_or_name = uri.rkey.clone();

    // 3. Look up the source repo.
    let source = db::repo::get(&state.db, &source_did, &source_rkey_or_name)
        .await?
        .ok_or_else(|| {
            AppError::NotFound(format!("source repo {} not found", input.source_repo))
        })?;

    let fork_name = input.name.unwrap_or_else(|| source.name.clone());

    // 4. Choose the destination node — for now, always the cospan default.
    //    Later: allow users to specify or select from their own nodes.
    if state.config.default_node_did.is_empty() {
        return Err(AppError::Upstream(
            "DEFAULT_NODE_DID not configured on this appview instance".to_string(),
        ));
    }
    let dest_node_did = state.config.default_node_did.clone();
    let dest_node_url = state.config.default_node_url.clone();

    // 5. Build the dev.cospan.repo record to write to the PDS.
    let now = Utc::now();
    let record = serde_json::json!({
        "$type": "dev.cospan.repo",
        "name": fork_name,
        "description": source.description.clone().unwrap_or_default(),
        "protocol": source.protocol.clone(),
        "node": format!("at://{}/dev.panproto.node/self", dest_node_did),
        "defaultBranch": source.default_branch.clone(),
        "visibility": source.visibility.clone(),
        "sourceRepo": input.source_repo.clone(),
        "createdAt": now.to_rfc3339(),
    });

    // 6. Create the record on the user's PDS via authenticated OAuth/DPoP.
    let created = pds_client::create_record(
        &state.http_client,
        &session,
        "dev.cospan.repo",
        None,
        &record,
    )
    .await
    .map_err(|e| match e {
        PdsClientError::PdsError { status, ref body } if status == 401 || status == 403 => {
            AppError::Unauthorized(format!("PDS rejected write: {body}"))
        }
        other => AppError::Upstream(format!("PDS createRecord failed: {other}")),
    })?;

    // 7. Extract the rkey from the returned URI.
    let rkey = created
        .uri
        .rsplit('/')
        .next()
        .unwrap_or("")
        .to_string();
    if rkey.is_empty() {
        return Err(AppError::Upstream(format!(
            "PDS returned malformed URI: {}",
            created.uri
        )));
    }

    // 8. Optimistic local DB insert so the UI sees the repo immediately.
    //    The firehose will upsert the canonical version shortly.
    let row = db::repo::RepoRow {
        did: session.did.clone(),
        rkey: rkey.clone(),
        name: fork_name.clone(),
        description: source.description.clone(),
        protocol: source.protocol.clone(),
        node_did: dest_node_did.clone(),
        node_url: dest_node_url.clone(),
        default_branch: source.default_branch.clone(),
        visibility: source.visibility.clone(),
        source_repo: Some(input.source_repo.clone()),
        star_count: 0,
        fork_count: 0,
        open_issue_count: 0,
        open_mr_count: 0,
        source: "cospan".to_string(),
        source_uri: Some(created.uri.clone()),
        created_at: now,
        indexed_at: now,
    };
    db::repo::upsert(&state.db, &row).await?;

    // 9. Increment fork_count on the source repo.
    sqlx::query(
        "UPDATE repos SET fork_count = fork_count + 1, indexed_at = NOW() \
         WHERE did = $1 AND (rkey = $2 OR name = $2)",
    )
    .bind(&source_did)
    .bind(&source_rkey_or_name)
    .execute(&state.db)
    .await?;

    Ok(Json(serde_json::json!({
        "uri": created.uri,
        "cid": created.cid,
        "rkey": rkey,
        "did": session.did,
        "name": fork_name,
        "sourceRepo": input.source_repo,
        "nodeDid": dest_node_did,
        "nodeUrl": dest_node_url,
        "createdAt": now.to_rfc3339(),
    })))
}
