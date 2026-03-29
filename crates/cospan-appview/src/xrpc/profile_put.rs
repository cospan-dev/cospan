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
    pub did: String,
    pub display_name: Option<String>,
    pub description: Option<String>,
    pub bluesky: Option<String>,
}

/// POST `/xrpc/dev.cospan.actor.profile.put`
pub async fn handler(
    State(state): State<Arc<AppState>>,
    Json(input): Json<Input>,
) -> Result<Json<serde_json::Value>, AppError> {
    if input.did.is_empty() {
        return Err(AppError::InvalidRequest("did is required".to_string()));
    }

    let row = db::actor_profile::ActorProfileRow {
        did: input.did.clone(),
        rkey: "self".to_string(),
        bluesky: input.bluesky.clone().unwrap_or_default(),
        display_name: input.display_name.clone(),
        description: input.description.clone(),
        avatar_cid: None,
        indexed_at: Utc::now(),
    };
    db::actor_profile::upsert(&state.db, &row).await?;

    Ok(Json(serde_json::json!({
        "did": input.did,
        "displayName": input.display_name,
        "description": input.description,
        "bluesky": input.bluesky,
    })))
}
