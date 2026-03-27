use std::sync::Arc;

use axum::Json;
use axum::extract::{Query, State};
use serde::Deserialize;

use crate::db;
use crate::error::AppError;
use crate::state::AppState;

#[derive(Deserialize)]
pub struct Params {
    pub limit: Option<i64>,
    pub cursor: Option<String>,
}

pub async fn handler(
    State(state): State<Arc<AppState>>,
    Query(params): Query<Params>,
) -> Result<Json<serde_json::Value>, AppError> {
    let limit = params.limit.unwrap_or(25).min(100);

    let orgs = db::org::list(&state.db, limit + 1, params.cursor.as_deref()).await?;

    let has_more = orgs.len() as i64 > limit;
    let orgs: Vec<_> = orgs.into_iter().take(limit as usize).collect();
    let cursor = if has_more {
        orgs.last().map(|r| r.created_at.to_rfc3339())
    } else {
        None
    };

    Ok(Json(serde_json::json!({
        "orgs": orgs,
        "cursor": cursor,
    })))
}
