pub use super::generated::crud::pull_states::{delete, get, list, upsert};
pub use super::generated::types::PullStateRow;

use chrono::{DateTime, Utc};
use sqlx::PgPool;

pub async fn list_by_pull(
    pool: &PgPool,
    pull_uri: &str,
    limit: i64,
    cursor: Option<&str>,
) -> Result<Vec<PullStateRow>, sqlx::Error> {
    if let Some(cursor_ts) = cursor {
        let ts: DateTime<Utc> = cursor_ts
            .parse()
            .map_err(|_| sqlx::Error::Protocol("invalid cursor".into()))?;
        sqlx::query_as::<_, PullStateRow>(
            "SELECT did, rkey, pull_uri, state, merge_commit_id, created_at, indexed_at \
             FROM pull_states WHERE pull_uri = $1 AND created_at < $2 \
             ORDER BY created_at DESC LIMIT $3",
        )
        .bind(pull_uri)
        .bind(ts)
        .bind(limit)
        .fetch_all(pool)
        .await
    } else {
        sqlx::query_as::<_, PullStateRow>(
            "SELECT did, rkey, pull_uri, state, merge_commit_id, created_at, indexed_at \
             FROM pull_states WHERE pull_uri = $1 \
             ORDER BY created_at DESC LIMIT $2",
        )
        .bind(pull_uri)
        .bind(limit)
        .fetch_all(pool)
        .await
    }
}
