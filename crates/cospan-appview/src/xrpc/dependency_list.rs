use std::sync::Arc;

use axum::Json;
use axum::extract::{Query, State};
use serde::Deserialize;

use crate::db;
use crate::error::AppError;
use crate::state::AppState;

/// Query with `direction` = "dependencies" (default) to list what this repo depends on,
/// or "dependents" to list repos that depend on this one.
#[derive(Deserialize)]
pub struct Params {
    pub did: String,
    pub repo: String,
    pub direction: Option<String>,
    pub limit: Option<i64>,
    pub cursor: Option<String>,
}

pub async fn handler(
    State(state): State<Arc<AppState>>,
    Query(params): Query<Params>,
) -> Result<Json<serde_json::Value>, AppError> {
    let limit = params.limit.unwrap_or(25).min(100);
    let direction = params.direction.as_deref().unwrap_or("dependencies");

    let deps = match direction {
        "dependents" => {
            db::dependency::list_dependents(
                &state.db,
                &params.did,
                &params.repo,
                limit + 1,
                params.cursor.as_deref(),
            )
            .await?
        }
        _ => {
            db::dependency::list_for_repo(
                &state.db,
                &params.did,
                &params.repo,
                limit + 1,
                params.cursor.as_deref(),
            )
            .await?
        }
    };

    let has_more = deps.len() as i64 > limit;
    let deps: Vec<_> = deps.into_iter().take(limit as usize).collect();
    let cursor = if has_more {
        deps.last().map(|r| r.created_at.to_rfc3339())
    } else {
        None
    };

    Ok(Json(serde_json::json!({
        "dependencies": deps,
        "cursor": cursor,
    })))
}
