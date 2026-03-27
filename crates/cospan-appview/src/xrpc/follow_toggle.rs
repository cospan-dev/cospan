use std::sync::Arc;

use axum::Json;
use axum::extract::State;
use chrono::Utc;
use serde::Deserialize;

use crate::db;
use crate::error::AppError;
use crate::state::AppState;

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Input {
    /// The DID of the user to follow/unfollow.
    pub subject: String,
    /// `true` to follow, `false` to unfollow.
    pub following: bool,
    /// Temporary: the DID of the acting user (will be replaced by auth).
    pub did: String,
}

/// POST `/xrpc/dev.cospan.graph.follow.toggle`
pub async fn handler(
    State(state): State<Arc<AppState>>,
    Json(input): Json<Input>,
) -> Result<Json<serde_json::Value>, AppError> {
    if input.following {
        let rkey = uuid::Uuid::new_v4().to_string();
        let row = db::follow::FollowRow {
            did: input.did.clone(),
            rkey: rkey.clone(),
            subject: input.subject.clone(),
            created_at: Utc::now(),
            indexed_at: Utc::now(),
        };
        db::follow::upsert(&state.db, &row).await?;

        Ok(Json(serde_json::json!({
            "uri": format!("at://{}/dev.cospan.graph.follow/{}", input.did, rkey),
            "following": true,
        })))
    } else {
        // Find the existing follow for this user + subject, then delete it.
        let existing = sqlx::query_as::<_, db::follow::FollowRow>(
            "SELECT did, rkey, subject, created_at, indexed_at \
             FROM follows WHERE did = $1 AND subject = $2 LIMIT 1",
        )
        .bind(&input.did)
        .bind(&input.subject)
        .fetch_optional(&state.db)
        .await?;

        if let Some(follow) = existing {
            db::follow::delete(&state.db, &follow.did, &follow.rkey).await?;
        }

        Ok(Json(serde_json::json!({
            "following": false,
        })))
    }
}
