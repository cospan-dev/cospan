//! GET /xrpc/dev.cospan.feed.getBreakingChanges
//!
//! Returns ref_updates where breaking_change_count > 0, ordered by
//! created_at DESC. Optionally filtered by `did` (repos the caller
//! follows/stars).

use std::sync::Arc;

use axum::Json;
use axum::extract::{Query, State};
use chrono::{DateTime, Utc};
use serde::Deserialize;

use crate::db::ref_update::RefUpdateRow;
use crate::error::AppError;
use crate::state::AppState;

#[derive(Deserialize)]
pub struct Params {
    /// If provided, only return breaking changes for repos starred by this DID.
    pub did: Option<String>,
    pub limit: Option<i64>,
    pub cursor: Option<String>,
}

pub async fn handler(
    State(state): State<Arc<AppState>>,
    Query(params): Query<Params>,
) -> Result<Json<serde_json::Value>, AppError> {
    let limit = params.limit.unwrap_or(25).min(100);

    let updates = if let Some(did) = &params.did {
        list_breaking_for_starred_repos(&state.db, did, limit + 1, params.cursor.as_deref()).await?
    } else {
        list_breaking_all(&state.db, limit + 1, params.cursor.as_deref()).await?
    };

    let has_more = updates.len() as i64 > limit;
    let updates: Vec<_> = updates.into_iter().take(limit as usize).collect();
    let cursor = if has_more {
        updates.last().map(|u| u.created_at.to_rfc3339())
    } else {
        None
    };

    Ok(Json(serde_json::json!({
        "refUpdates": updates,
        "cursor": cursor,
    })))
}

/// List breaking changes across all repos.
async fn list_breaking_all(
    pool: &sqlx::PgPool,
    limit: i64,
    cursor: Option<&str>,
) -> Result<Vec<RefUpdateRow>, sqlx::Error> {
    if let Some(cursor_ts) = cursor {
        let ts: DateTime<Utc> = cursor_ts
            .parse()
            .map_err(|_| sqlx::Error::Protocol("invalid cursor".into()))?;
        sqlx::query_as::<_, RefUpdateRow>(
            "SELECT id, repo_did, repo_name, rkey, committer_did, ref_name, \
                  old_target, new_target, protocol, migration_id, breaking_change_count, \
                  lens_id, lens_quality, commit_count, created_at, indexed_at \
             FROM ref_updates \
             WHERE breaking_change_count > 0 AND created_at < $1 \
             ORDER BY created_at DESC LIMIT $2",
        )
        .bind(ts)
        .bind(limit)
        .fetch_all(pool)
        .await
    } else {
        sqlx::query_as::<_, RefUpdateRow>(
            "SELECT id, repo_did, repo_name, rkey, committer_did, ref_name, \
                  old_target, new_target, protocol, migration_id, breaking_change_count, \
                  lens_id, lens_quality, commit_count, created_at, indexed_at \
             FROM ref_updates \
             WHERE breaking_change_count > 0 \
             ORDER BY created_at DESC LIMIT $1",
        )
        .bind(limit)
        .fetch_all(pool)
        .await
    }
}

/// List breaking changes only for repos that `did` has starred.
async fn list_breaking_for_starred_repos(
    pool: &sqlx::PgPool,
    did: &str,
    limit: i64,
    cursor: Option<&str>,
) -> Result<Vec<RefUpdateRow>, sqlx::Error> {
    if let Some(cursor_ts) = cursor {
        let ts: DateTime<Utc> = cursor_ts
            .parse()
            .map_err(|_| sqlx::Error::Protocol("invalid cursor".into()))?;
        sqlx::query_as::<_, RefUpdateRow>(
            "SELECT ru.id, ru.repo_did, ru.repo_name, ru.rkey, ru.committer_did, ru.ref_name, \
                  ru.old_target, ru.new_target, ru.protocol, ru.migration_id, \
                  ru.breaking_change_count, ru.lens_id, ru.lens_quality, ru.commit_count, \
                  ru.created_at, ru.indexed_at \
             FROM ref_updates ru \
             JOIN stars s ON s.subject = 'at://' || ru.repo_did || '/dev.cospan.repo/' || ru.repo_name \
             WHERE s.did = $1 AND ru.breaking_change_count > 0 AND ru.created_at < $2 \
             ORDER BY ru.created_at DESC LIMIT $3",
        )
        .bind(did)
        .bind(ts)
        .bind(limit)
        .fetch_all(pool)
        .await
    } else {
        sqlx::query_as::<_, RefUpdateRow>(
            "SELECT ru.id, ru.repo_did, ru.repo_name, ru.rkey, ru.committer_did, ru.ref_name, \
                  ru.old_target, ru.new_target, ru.protocol, ru.migration_id, \
                  ru.breaking_change_count, ru.lens_id, ru.lens_quality, ru.commit_count, \
                  ru.created_at, ru.indexed_at \
             FROM ref_updates ru \
             JOIN stars s ON s.subject = 'at://' || ru.repo_did || '/dev.cospan.repo/' || ru.repo_name \
             WHERE s.did = $1 AND ru.breaking_change_count > 0 \
             ORDER BY ru.created_at DESC LIMIT $2",
        )
        .bind(did)
        .bind(limit)
        .fetch_all(pool)
        .await
    }
}
