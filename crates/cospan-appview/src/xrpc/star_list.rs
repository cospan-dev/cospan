use std::sync::Arc;

use axum::Json;
use axum::extract::{Query, State};
use serde::Deserialize;

use crate::db;
use crate::error::AppError;
use crate::state::AppState;

/// Query by `did` to get a user's stars, or by `subject` to get a repo's stargazers.
#[derive(Deserialize)]
pub struct Params {
    pub did: Option<String>,
    pub subject: Option<String>,
    pub limit: Option<i64>,
    pub cursor: Option<String>,
}

pub async fn handler(
    State(state): State<Arc<AppState>>,
    Query(params): Query<Params>,
) -> Result<Json<serde_json::Value>, AppError> {
    let limit = params.limit.unwrap_or(25).min(100);

    let stars = if let Some(ref did) = params.did {
        db::star::list_by_user(&state.db, did, limit + 1, params.cursor.as_deref()).await?
    } else if let Some(ref subject) = params.subject {
        db::star::list_for_subject(&state.db, subject, limit + 1, params.cursor.as_deref()).await?
    } else {
        return Err(AppError::NotFound(
            "either 'did' or 'subject' parameter is required".to_string(),
        ));
    };

    let has_more = stars.len() as i64 > limit;
    let stars: Vec<_> = stars.into_iter().take(limit as usize).collect();
    let cursor = if has_more {
        stars.last().map(|r| r.created_at.to_rfc3339())
    } else {
        None
    };

    Ok(Json(serde_json::json!({
        "stars": stars,
        "cursor": cursor,
    })))
}
