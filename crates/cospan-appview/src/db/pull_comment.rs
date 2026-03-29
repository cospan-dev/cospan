pub use super::generated::crud::pull_comments::{delete, get, list, upsert};
pub use super::generated::types::PullCommentRow;

use chrono::{DateTime, Utc};
use sqlx::PgPool;

pub async fn list_for_pull(
    pool: &PgPool,
    pull_uri: &str,
    limit: i64,
    cursor: Option<&str>,
) -> Result<Vec<PullCommentRow>, sqlx::Error> {
    if let Some(cursor_ts) = cursor {
        let ts: DateTime<Utc> = cursor_ts
            .parse()
            .map_err(|_| sqlx::Error::Protocol("invalid cursor".into()))?;
        sqlx::query_as::<_, PullCommentRow>(
            "SELECT did, rkey, pull_uri, body, review_decision, created_at, indexed_at \
             FROM pull_comments WHERE pull_uri = $1 AND created_at > $2 \
             ORDER BY created_at ASC LIMIT $3",
        )
        .bind(pull_uri)
        .bind(ts)
        .bind(limit)
        .fetch_all(pool)
        .await
    } else {
        sqlx::query_as::<_, PullCommentRow>(
            "SELECT did, rkey, pull_uri, body, review_decision, created_at, indexed_at \
             FROM pull_comments WHERE pull_uri = $1 \
             ORDER BY created_at ASC LIMIT $2",
        )
        .bind(pull_uri)
        .bind(limit)
        .fetch_all(pool)
        .await
    }
}
