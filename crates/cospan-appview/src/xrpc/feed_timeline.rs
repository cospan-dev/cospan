//! GET /xrpc/dev.cospan.feed.getTimeline
//!
//! Returns a chronological feed of recent activity (ref updates, issues,
//! pulls, stars) for repos the user follows/stars. Combines multiple
//! record types sorted by created_at.

use std::sync::Arc;

use axum::Json;
use axum::extract::{Query, State};
use chrono::{DateTime, Utc};
use serde::Deserialize;

use crate::error::AppError;
use crate::state::AppState;

#[derive(Deserialize)]
pub struct Params {
    /// DID of the user whose timeline to fetch. Returns activity for
    /// repos they have starred.
    pub did: String,
    pub limit: Option<i64>,
    pub cursor: Option<String>,
}

/// A timeline item combining different activity types.
#[derive(serde::Serialize)]
#[serde(tag = "type", rename_all = "camelCase")]
enum TimelineItem {
    RefUpdate {
        repo_did: String,
        repo_name: String,
        ref_name: String,
        new_target: String,
        committer_did: String,
        breaking_change_count: i32,
        created_at: DateTime<Utc>,
    },
    Issue {
        repo_did: String,
        repo_name: String,
        rkey: String,
        title: String,
        author_did: String,
        state: String,
        created_at: DateTime<Utc>,
    },
    Pull {
        repo_did: String,
        repo_name: String,
        rkey: String,
        title: String,
        author_did: String,
        state: String,
        created_at: DateTime<Utc>,
    },
    Star {
        did: String,
        subject: String,
        created_at: DateTime<Utc>,
    },
}

impl TimelineItem {
    fn created_at(&self) -> DateTime<Utc> {
        match self {
            Self::RefUpdate { created_at, .. }
            | Self::Issue { created_at, .. }
            | Self::Pull { created_at, .. }
            | Self::Star { created_at, .. } => *created_at,
        }
    }
}

pub async fn handler(
    State(state): State<Arc<AppState>>,
    Query(params): Query<Params>,
) -> Result<Json<serde_json::Value>, AppError> {
    let limit = params.limit.unwrap_or(25).min(100) as usize;
    let cursor_ts: Option<DateTime<Utc>> = params.cursor.as_deref().and_then(|c| c.parse().ok());

    // Fetch starred repos for this user to know which repos to show activity for.
    let stars = crate::db::star::list_by_user(&state.db, &params.did, 1000, None).await?;

    if stars.is_empty() {
        return Ok(Json(serde_json::json!({
            "items": serde_json::Value::Array(vec![]),
            "cursor": null,
        })));
    }

    // Extract repo (did, name) pairs from star subjects.
    let starred_repos: Vec<(String, String)> = stars
        .iter()
        .filter_map(|s| {
            crate::at_uri::validate(&s.subject)
                .ok()
                .map(|uri| (uri.did, uri.rkey))
        })
        .collect();

    let mut items: Vec<TimelineItem> = Vec::new();

    // Fetch recent ref updates for starred repos.
    for (repo_did, repo_name) in &starred_repos {
        let updates = crate::db::ref_update::list_for_repo(
            &state.db,
            repo_did,
            repo_name,
            (limit + 1) as i64,
            None,
        )
        .await?;

        for u in updates {
            if let Some(ts) = cursor_ts
                && u.created_at >= ts
            {
                continue;
            }
            items.push(TimelineItem::RefUpdate {
                repo_did: u.repo_did,
                repo_name: u.repo_name,
                ref_name: u.ref_name,
                new_target: u.new_target,
                committer_did: u.committer_did,
                breaking_change_count: u.breaking_change_count,
                created_at: u.created_at,
            });
        }
    }

    // Fetch recent issues for starred repos.
    for (repo_did, repo_name) in &starred_repos {
        let issues = crate::db::issue::list_for_repo(
            &state.db,
            repo_did,
            repo_name,
            None,
            (limit + 1) as i64,
            None,
        )
        .await?;

        for i in issues {
            if let Some(ts) = cursor_ts
                && i.created_at >= ts
            {
                continue;
            }
            items.push(TimelineItem::Issue {
                repo_did: i.repo_did,
                repo_name: i.repo_name,
                rkey: i.rkey,
                title: i.title,
                author_did: i.did,
                state: i.state,
                created_at: i.created_at,
            });
        }
    }

    // Fetch recent pulls for starred repos.
    for (repo_did, repo_name) in &starred_repos {
        let pulls = crate::db::pull::list_for_repo(
            &state.db,
            repo_did,
            repo_name,
            None,
            (limit + 1) as i64,
            None,
        )
        .await?;

        for p in pulls {
            if let Some(ts) = cursor_ts
                && p.created_at >= ts
            {
                continue;
            }
            items.push(TimelineItem::Pull {
                repo_did: p.repo_did,
                repo_name: p.repo_name,
                rkey: p.rkey,
                title: p.title,
                author_did: p.did,
                state: p.state,
                created_at: p.created_at,
            });
        }
    }

    // Include recent stars from anyone on starred repos (social activity).
    for star in &stars {
        if let Some(ts) = cursor_ts
            && star.created_at >= ts
        {
            continue;
        }
        items.push(TimelineItem::Star {
            did: star.did.clone(),
            subject: star.subject.clone(),
            created_at: star.created_at,
        });
    }

    // Sort all items by created_at descending and take the requested limit.
    items.sort_by_key(|b| std::cmp::Reverse(b.created_at()));
    items.truncate(limit + 1);

    let has_more = items.len() > limit;
    items.truncate(limit);

    let cursor = if has_more {
        items.last().map(|i| i.created_at().to_rfc3339())
    } else {
        None
    };

    Ok(Json(serde_json::json!({
        "items": items,
        "cursor": cursor,
    })))
}
