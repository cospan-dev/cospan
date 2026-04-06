use std::sync::Arc;

use axum::Json;
use axum::extract::State;
use serde::Deserialize;

use crate::auth::DidAuth;
use crate::error::NodeError;
use crate::state::NodeState;

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SetRefInput {
    pub did: String,
    pub repo: String,
    #[serde(rename = "ref")]
    pub ref_name: String,
    pub old_target: Option<String>,
    pub new_target: String,
    pub protocol: String,
    pub commit_count: Option<i64>,
}

pub async fn set_ref(
    State(state): State<Arc<NodeState>>,
    auth: DidAuth,
    Json(input): Json<SetRefInput>,
) -> Result<Json<serde_json::Value>, NodeError> {
    // 1. Authorization
    state.authz.check_push(&auth.did, &input.did, &input.repo)?;

    let new_id: panproto_core::vcs::ObjectId = input.new_target.parse().map_err(|_| {
        NodeError::InvalidRequest(format!("invalid object ID: {}", input.new_target))
    })?;

    let store = state.store.lock().await;

    // 2. Verify the new target object exists
    if !store
        .has_object(&input.did, &input.repo, &new_id)
        .map_err(|e| NodeError::Internal(format!("store error: {e}")))?
    {
        return Err(NodeError::ObjectNotFound(input.new_target.clone()));
    }

    // 3. Check old target matches (optimistic concurrency)
    if let Some(expected_old) = &input.old_target {
        let current = store
            .get_ref(&input.did, &input.repo, &input.ref_name)
            .map_err(|e| NodeError::Internal(format!("store error: {e}")))?;
        let current_str = current.map(|id| id.to_string());
        if current_str.as_deref() != Some(expected_old.as_str()) {
            return Err(NodeError::InvalidRequest(format!(
                "ref {} has moved: expected {}, got {:?}",
                input.ref_name, expected_old, current_str
            )));
        }
    }

    // 4. Run validation pipeline
    let validation = state
        .validator
        .validate(
            &state.store,
            &input.did,
            &input.repo,
            &input.protocol,
            input.old_target.as_deref(),
            &input.new_target,
        )
        .await
        .map_err(NodeError::ValidationFailed)?;

    // 5. Update the ref
    store
        .set_ref(&input.did, &input.repo, &input.ref_name, new_id)
        .map_err(|e| NodeError::Internal(format!("store error: {e}")))?;

    // 6. Emit dev.cospan.vcs.refUpdate record to user's PDS
    let ref_update_record = serde_json::json!({
        "repo": format!("at://{}/{}", input.did, input.repo),
        "ref": input.ref_name,
        "oldTarget": input.old_target,
        "newTarget": input.new_target,
        "committerDid": auth.did,
        "protocol": input.protocol,
        "breakingChanges": validation.breaking_changes,
        "lensId": validation.lens_id,
        "lensQuality": validation.lens_quality,
        "commitCount": input.commit_count,
        "createdAt": chrono::Utc::now().to_rfc3339(),
    });

    let at_uri = state
        .pds_client
        .create_record(&auth.did, "dev.panproto.vcs.refUpdate", &ref_update_record)
        .await?;

    Ok(Json(serde_json::json!({
        "ref": input.ref_name,
        "target": input.new_target,
        "refUpdateUri": at_uri,
        "validation": validation,
    })))
}
