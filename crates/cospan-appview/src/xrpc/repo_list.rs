use std::sync::Arc;

use axum::Json;
use axum::extract::{Query, State};
use serde::Deserialize;

use crate::db;
use crate::error::AppError;
use crate::state::AppState;

#[derive(Deserialize)]
pub struct Params {
    pub did: Option<String>,
    pub source: Option<String>,
    pub limit: Option<i64>,
    pub cursor: Option<String>,
}

pub async fn handler(
    State(state): State<Arc<AppState>>,
    Query(params): Query<Params>,
) -> Result<Json<serde_json::Value>, AppError> {
    let limit = params.limit.unwrap_or(25).min(100);

    let repos = if let Some(did) = &params.did {
        db::repo::list_by_did(&state.db, did, limit + 1, params.cursor.as_deref()).await?
    } else if let Some(source) = &params.source {
        db::repo::list_by_source(&state.db, source, limit + 1, params.cursor.as_deref()).await?
    } else {
        db::repo::list_recent(&state.db, limit + 1, params.cursor.as_deref()).await?
    };

    let has_more = repos.len() as i64 > limit;
    let repos: Vec<_> = repos.into_iter().take(limit as usize).collect();
    let cursor = if has_more {
        repos.last().map(|r| r.created_at.to_rfc3339())
    } else {
        None
    };

    Ok(Json(serde_json::json!({
        "repos": repos,
        "cursor": cursor,
    })))
}
