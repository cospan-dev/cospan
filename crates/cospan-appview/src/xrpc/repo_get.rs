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
    pub name: String,
}

pub async fn handler(
    State(state): State<Arc<AppState>>,
    Query(params): Query<Params>,
) -> Result<Json<serde_json::Value>, AppError> {
    let repo = db::repo::get(&state.db, &params.did, &params.name)
        .await?
        .ok_or_else(|| {
            AppError::NotFound(format!("repo {}/{} not found", params.did, params.name))
        })?;

    Ok(Json(serde_json::to_value(repo).unwrap()))
}
