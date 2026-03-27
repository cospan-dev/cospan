use std::sync::Arc;

use axum::Json;
use axum::extract::{Query, State};
use serde::Deserialize;

use crate::db;
use crate::error::AppError;
use crate::state::AppState;

/// Query by `did` with `direction` = "following" (default) or "followers".
#[derive(Deserialize)]
pub struct Params {
    pub did: String,
    pub direction: Option<String>,
    pub limit: Option<i64>,
    pub cursor: Option<String>,
}

pub async fn handler(
    State(state): State<Arc<AppState>>,
    Query(params): Query<Params>,
) -> Result<Json<serde_json::Value>, AppError> {
    let limit = params.limit.unwrap_or(25).min(100);
    let direction = params.direction.as_deref().unwrap_or("following");

    let follows = match direction {
        "followers" => {
            db::follow::list_followers(&state.db, &params.did, limit + 1, params.cursor.as_deref())
                .await?
        }
        _ => {
            db::follow::list_following(&state.db, &params.did, limit + 1, params.cursor.as_deref())
                .await?
        }
    };

    let has_more = follows.len() as i64 > limit;
    let follows: Vec<_> = follows.into_iter().take(limit as usize).collect();
    let cursor = if has_more {
        follows.last().map(|r| r.created_at.to_rfc3339())
    } else {
        None
    };

    Ok(Json(serde_json::json!({
        "follows": follows,
        "cursor": cursor,
    })))
}
