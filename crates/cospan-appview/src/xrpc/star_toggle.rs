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
    /// The AT-URI of the repo being starred, e.g. `at://did/dev.cospan.repo/name`
    pub subject: String,
    /// `true` to star, `false` to unstar.
    pub starred: bool,
    /// Temporary: the DID of the acting user (will be replaced by auth).
    pub did: String,
}

/// POST `/xrpc/dev.cospan.feed.star.toggle`
pub async fn handler(
    State(state): State<Arc<AppState>>,
    Json(input): Json<Input>,
) -> Result<Json<serde_json::Value>, AppError> {
    // Parse the subject AT-URI to extract repo_did and repo_name.
    let uri = crate::at_uri::validate(&input.subject).map_err(AppError::InvalidRequest)?;
    let repo_did = &uri.did;
    let repo_name = &uri.rkey;

    if input.starred {
        let rkey = uuid::Uuid::new_v4().to_string();
        let row = db::star::StarRow {
            did: input.did.clone(),
            rkey: rkey.clone(),
            subject: input.subject.clone(),
            created_at: Utc::now(),
            indexed_at: Utc::now(),
        };
        db::star::upsert(&state.db, &row).await?;
        db::star::increment_repo_star_count(&state.db, repo_did, repo_name).await?;

        Ok(Json(serde_json::json!({
            "uri": format!("at://{}/dev.cospan.feed.star/{}", input.did, rkey),
            "starred": true,
        })))
    } else {
        // Find the existing star for this user + subject, then delete it.
        let existing = sqlx::query_as::<_, db::star::StarRow>(
            "SELECT did, rkey, subject, created_at, indexed_at \
             FROM stars WHERE did = $1 AND subject = $2 LIMIT 1",
        )
        .bind(&input.did)
        .bind(&input.subject)
        .fetch_optional(&state.db)
        .await?;

        if let Some(star) = existing {
            db::star::delete(&state.db, &star.did, &star.rkey).await?;
            db::star::decrement_repo_star_count(&state.db, repo_did, repo_name).await?;
        }

        Ok(Json(serde_json::json!({
            "starred": false,
        })))
    }
}
