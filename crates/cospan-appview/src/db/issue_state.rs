pub use super::generated::crud::issue_states::{delete, get, list, upsert};
pub use super::generated::types::IssueStateRow;

use chrono::{DateTime, Utc};
use sqlx::PgPool;

pub async fn list_for_issue(
    pool: &PgPool,
    issue_uri: &str,
    limit: i64,
    cursor: Option<&str>,
) -> Result<Vec<IssueStateRow>, sqlx::Error> {
    if let Some(cursor_ts) = cursor {
        let ts: DateTime<Utc> = cursor_ts
            .parse()
            .map_err(|_| sqlx::Error::Protocol("invalid cursor".into()))?;
        sqlx::query_as::<_, IssueStateRow>(
            "SELECT did, rkey, issue_uri, state, reason, created_at, indexed_at \
             FROM issue_states WHERE issue_uri = $1 AND created_at < $2 \
             ORDER BY created_at DESC LIMIT $3",
        )
        .bind(issue_uri)
        .bind(ts)
        .bind(limit)
        .fetch_all(pool)
        .await
    } else {
        sqlx::query_as::<_, IssueStateRow>(
            "SELECT did, rkey, issue_uri, state, reason, created_at, indexed_at \
             FROM issue_states WHERE issue_uri = $1 \
             ORDER BY created_at DESC LIMIT $2",
        )
        .bind(issue_uri)
        .bind(limit)
        .fetch_all(pool)
        .await
    }
}
