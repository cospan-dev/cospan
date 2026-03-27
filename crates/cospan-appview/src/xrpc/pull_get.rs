use std::sync::Arc;

use axum::Json;
use axum::extract::{Query, State};
use serde::Deserialize;

use crate::db;
use crate::error::AppError;
use crate::state::AppState;

#[derive(Deserialize)]
pub struct Params {
    pub did: String,
    pub repo: String,
    pub rkey: String,
}

pub async fn handler(
    State(state): State<Arc<AppState>>,
    Query(params): Query<Params>,
) -> Result<Json<serde_json::Value>, AppError> {
    let pull = db::pull::get(&state.db, &params.did, &params.repo, &params.rkey)
        .await?
        .ok_or_else(|| {
            AppError::NotFound(format!(
                "pull {}/{}/{} not found",
                params.did, params.repo, params.rkey
            ))
        })?;

    Ok(Json(serde_json::to_value(pull).unwrap()))
}
