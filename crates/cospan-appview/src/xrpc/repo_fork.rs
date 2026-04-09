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
use crate::git_copy::{self, CopyOptions};
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
/// Requires an authenticated session.
///
/// Synchronous work:
///   1. Validate the session and source repo.
///   2. Create a `dev.cospan.repo` record on the user's PDS via OAuth/DPoP.
///   3. Insert an optimistic row into the local `repos` table.
///   4. Insert a `fork_jobs` row tracking the copy task.
///   5. Spawn the git object copy as a background tokio task.
///
/// The copy task fetches objects from the source's git URL (cospan node
/// git smart HTTP, Tangled knot git HTTP, etc.) and pushes them to the
/// destination node via git-receive-pack.
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

    // 3. Look up the source repo so we can copy its metadata and derive
    //    the git URL for the object copy.
    let source = db::repo::get(&state.db, &source_did, &source_rkey_or_name)
        .await?
        .ok_or_else(|| {
            AppError::NotFound(format!("source repo {} not found", input.source_repo))
        })?;

    let fork_name = input.name.unwrap_or_else(|| source.name.clone());

    // 4. Choose the destination node — for now always the cospan default.
    if state.config.default_node_did.is_empty() {
        return Err(AppError::Upstream(
            "DEFAULT_NODE_DID not configured on this appview instance".to_string(),
        ));
    }
    let dest_node_did = state.config.default_node_did.clone();
    let dest_node_url = state.config.default_node_url.clone();

    // 5. Derive the source git URL. For cospan/tangled repos the repo
    //    source row has a node_url; the git smart HTTP endpoint lives at
    //    {node_url}/{did}/{repo_name}.
    let source_git_url = derive_git_url(
        &source.node_url,
        &source_did,
        &source.name,
        source.source.as_str(),
    );
    if source_git_url.is_empty() {
        return Err(AppError::Upstream(format!(
            "cannot determine git URL for source repo {} (node_url is empty)",
            input.source_repo
        )));
    }

    // 6. Build the dev.cospan.repo record to write to the PDS.
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

    // 7. Create the record on the user's PDS via authenticated OAuth/DPoP.
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

    // 8. Extract the rkey from the returned URI.
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

    // 9. Optimistic local DB insert so the UI sees the repo immediately.
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

    // 10. Increment fork_count on the source repo.
    sqlx::query(
        "UPDATE repos SET fork_count = fork_count + 1, indexed_at = NOW() \
         WHERE did = $1 AND (rkey = $2 OR name = $2)",
    )
    .bind(&source_did)
    .bind(&source_rkey_or_name)
    .execute(&state.db)
    .await?;

    // 11. Create a fork_jobs row tracking the async copy.
    // The destination node git URL uses the node's did + the fork's rkey
    // as the repo path (matching the receive-pack route pattern).
    let dest_git_url = format!(
        "{}/{}/{}",
        dest_node_url.trim_end_matches('/'),
        session.did,
        rkey
    );
    let job_id = db::fork_job::create(
        &state.db,
        &session.did,
        &rkey,
        &fork_name,
        &input.source_repo,
        &source_git_url,
        &dest_git_url,
    )
    .await?;

    // 12. Spawn the background git copy task.
    let copy_state = state.clone();
    let copy_source = source_git_url.clone();
    let copy_dest = dest_git_url.clone();
    tokio::spawn(async move {
        run_copy_job(copy_state, job_id, copy_source, copy_dest).await;
    });

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
        "forkJobId": job_id.to_string(),
    })))
}

/// Derive the git smart HTTP base URL for a repo given its node URL,
/// owner DID, and name. The convention on cospan-node is
/// `{node_url}/{did}/{repo_name}`; Tangled knots expose an equivalent
/// path under `{knot_url}/{did}/{repo_name}`. Both are served by the
/// same git smart HTTP protocol.
fn derive_git_url(node_url: &str, did: &str, name: &str, _source: &str) -> String {
    if node_url.is_empty() {
        return String::new();
    }
    format!("{}/{}/{}", node_url.trim_end_matches('/'), did, name)
}

/// Run a single fork_job to completion: fetch objects from source,
/// push to destination, record status in the DB.
async fn run_copy_job(
    state: Arc<AppState>,
    job_id: uuid::Uuid,
    source_git_url: String,
    dest_git_url: String,
) {
    if let Err(e) = db::fork_job::mark_running(&state.db, job_id).await {
        tracing::error!(%job_id, error = %e, "failed to mark fork job running");
        return;
    }

    // git2 operations block; run on a dedicated blocking thread.
    let result = tokio::task::spawn_blocking(move || {
        // Dev-auth credentials for pushing to our own cospan-node.
        // In production the destination is the cospan-dev/cospan-node
        // which accepts bearer tokens for allowed DIDs.
        let dest_creds = git_copy::basic_auth_creds("cospan-appview".to_string(), String::new());
        let options = CopyOptions {
            source_creds: None,
            dest_creds: Some(dest_creds),
            ..CopyOptions::default()
        };
        git_copy::copy_repo(&source_git_url, &dest_git_url, options)
    })
    .await;

    match result {
        Ok(Ok(report)) => {
            tracing::info!(
                %job_id,
                refs = report.refs_copied,
                "fork git copy completed"
            );
            if let Err(e) =
                db::fork_job::mark_completed(&state.db, job_id, report.refs_copied as i32).await
            {
                tracing::error!(%job_id, error = %e, "failed to mark fork job completed");
            }
        }
        Ok(Err(e)) => {
            tracing::error!(%job_id, error = %e, "fork git copy failed");
            if let Err(e2) = db::fork_job::mark_failed(&state.db, job_id, &e.to_string()).await {
                tracing::error!(%job_id, error = %e2, "failed to mark fork job failed");
            }
        }
        Err(join_err) => {
            tracing::error!(%job_id, error = %join_err, "fork copy task panicked");
            let _ = db::fork_job::mark_failed(&state.db, job_id, &join_err.to_string()).await;
        }
    }
}
