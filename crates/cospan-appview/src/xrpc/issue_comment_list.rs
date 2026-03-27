use std::sync::Arc;

use axum::Json;
use axum::extract::{Query, State};
use serde::Deserialize;

use crate::db;
use crate::error::AppError;
use crate::state::AppState;

#[derive(Deserialize)]
pub struct Params {
    pub issue: String,
    pub limit: Option<i64>,
    pub cursor: Option<String>,
}

pub async fn handler(
    State(state): State<Arc<AppState>>,
    Query(params): Query<Params>,
) -> Result<Json<serde_json::Value>, AppError> {
    let limit = params.limit.unwrap_or(25).min(100);

    let comments = db::issue_comment::list_for_issue(
        &state.db,
        &params.issue,
        limit + 1,
        params.cursor.as_deref(),
    )
    .await?;

    let has_more = comments.len() as i64 > limit;
    let comments: Vec<_> = comments.into_iter().take(limit as usize).collect();
    let cursor = if has_more {
        comments.last().map(|r| r.created_at.to_rfc3339())
    } else {
        None
    };

    Ok(Json(serde_json::json!({
        "comments": comments,
        "cursor": cursor,
    })))
}
