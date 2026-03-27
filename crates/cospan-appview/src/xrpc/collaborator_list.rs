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
    pub limit: Option<i64>,
    pub cursor: Option<String>,
}

pub async fn handler(
    State(state): State<Arc<AppState>>,
    Query(params): Query<Params>,
) -> Result<Json<serde_json::Value>, AppError> {
    let limit = params.limit.unwrap_or(25).min(100);

    let collaborators = db::collaborator::list_for_repo(
        &state.db,
        &params.did,
        &params.repo,
        limit + 1,
        params.cursor.as_deref(),
    )
    .await?;

    let has_more = collaborators.len() as i64 > limit;
    let collaborators: Vec<_> = collaborators.into_iter().take(limit as usize).collect();
    let cursor = if has_more {
        collaborators.last().map(|r| r.created_at.to_rfc3339())
    } else {
        None
    };

    Ok(Json(serde_json::json!({
        "collaborators": collaborators,
        "cursor": cursor,
    })))
}
