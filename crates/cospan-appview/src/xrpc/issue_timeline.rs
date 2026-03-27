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

    // Fetch comments and state changes in parallel, then interleave by created_at.
    let (comments, state_changes) = tokio::try_join!(
        db::issue_comment::list_for_issue(
            &state.db,
            &params.issue,
            limit + 1,
            params.cursor.as_deref(),
        ),
        db::issue_state::list_for_issue(
            &state.db,
            &params.issue,
            limit + 1,
            params.cursor.as_deref(),
        ),
    )?;

    // Convert both to timeline entries with a unified timestamp for sorting.
    let mut timeline: Vec<serde_json::Value> = Vec::new();

    for c in comments {
        let created_at = c.created_at.to_rfc3339();
        let mut val = serde_json::to_value(&c).unwrap();
        val.as_object_mut()
            .unwrap()
            .insert("kind".into(), serde_json::json!("comment"));
        val.as_object_mut()
            .unwrap()
            .insert("sortedAt".into(), serde_json::json!(created_at));
        timeline.push(val);
    }

    for s in state_changes {
        let created_at = s.created_at.to_rfc3339();
        let mut val = serde_json::to_value(&s).unwrap();
        val.as_object_mut()
            .unwrap()
            .insert("kind".into(), serde_json::json!("stateChange"));
        val.as_object_mut()
            .unwrap()
            .insert("sortedAt".into(), serde_json::json!(created_at));
        timeline.push(val);
    }

    // Sort by sortedAt ascending.
    timeline.sort_by(|a, b| {
        let a_ts = a.get("sortedAt").and_then(|v| v.as_str()).unwrap_or("");
        let b_ts = b.get("sortedAt").and_then(|v| v.as_str()).unwrap_or("");
        a_ts.cmp(b_ts)
    });

    // Apply cursor filtering: only include entries after the cursor timestamp.
    let timeline: Vec<_> = if let Some(ref cursor) = params.cursor {
        timeline
            .into_iter()
            .filter(|entry| {
                entry
                    .get("sortedAt")
                    .and_then(|v| v.as_str())
                    .map(|ts| ts > cursor.as_str())
                    .unwrap_or(false)
            })
            .collect()
    } else {
        timeline
    };

    let has_more = timeline.len() as i64 > limit;
    let timeline: Vec<_> = timeline.into_iter().take(limit as usize).collect();
    let cursor = if has_more {
        timeline
            .last()
            .and_then(|r| r.get("sortedAt"))
            .and_then(|v| v.as_str())
            .map(|s| s.to_string())
    } else {
        None
    };

    Ok(Json(serde_json::json!({
        "timeline": timeline,
        "cursor": cursor,
    })))
}
