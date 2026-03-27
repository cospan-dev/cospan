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
    pub cursor: Option<i64>,
}

pub async fn handler(
    State(state): State<Arc<AppState>>,
    Query(params): Query<Params>,
) -> Result<Json<serde_json::Value>, AppError> {
    let limit = params.limit.unwrap_or(25).min(100);

    let updates = db::ref_update::list_for_repo(
        &state.db,
        &params.did,
        &params.repo,
        limit + 1,
        params.cursor,
    )
    .await?;

    let has_more = updates.len() as i64 > limit;
    let updates: Vec<_> = updates.into_iter().take(limit as usize).collect();
    let cursor = if has_more {
        updates.last().map(|r| r.id)
    } else {
        None
    };

    Ok(Json(serde_json::json!({
        "refUpdates": updates,
        "cursor": cursor,
    })))
}
