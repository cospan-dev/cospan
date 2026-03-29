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
}

pub async fn handler(
    State(state): State<Arc<AppState>>,
    Query(params): Query<Params>,
) -> Result<Json<serde_json::Value>, AppError> {
    let limit = params.limit.unwrap_or(50).min(200);
    let nodes = db::node::list(&state.db, limit, None).await?;

    Ok(Json(serde_json::json!({ "nodes": nodes })))
}
