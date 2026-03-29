pub use super::generated::crud::reactions::{delete, get, list, upsert};
pub use super::generated::types::ReactionRow;

use chrono::{DateTime, Utc};
use sqlx::PgPool;

pub async fn list_for_subject(
    pool: &PgPool,
    subject: &str,
    limit: i64,
    cursor: Option<&str>,
) -> Result<Vec<ReactionRow>, sqlx::Error> {
    if let Some(cursor_ts) = cursor {
        let ts: DateTime<Utc> = cursor_ts
            .parse()
            .map_err(|_| sqlx::Error::Protocol("invalid cursor".into()))?;
        sqlx::query_as::<_, ReactionRow>(
            "SELECT did, rkey, subject, emoji, created_at, indexed_at \
             FROM reactions WHERE subject = $1 AND created_at < $2 \
             ORDER BY created_at DESC LIMIT $3",
        )
        .bind(subject)
        .bind(ts)
        .bind(limit)
        .fetch_all(pool)
        .await
    } else {
        sqlx::query_as::<_, ReactionRow>(
            "SELECT did, rkey, subject, emoji, created_at, indexed_at \
             FROM reactions WHERE subject = $1 \
             ORDER BY created_at DESC LIMIT $2",
        )
        .bind(subject)
        .bind(limit)
        .fetch_all(pool)
        .await
    }
}

pub async fn list_by_did(
    pool: &PgPool,
    did: &str,
    limit: i64,
    cursor: Option<&str>,
) -> Result<Vec<ReactionRow>, sqlx::Error> {
    if let Some(cursor_ts) = cursor {
        let ts: DateTime<Utc> = cursor_ts
            .parse()
            .map_err(|_| sqlx::Error::Protocol("invalid cursor".into()))?;
        sqlx::query_as::<_, ReactionRow>(
            "SELECT did, rkey, subject, emoji, created_at, indexed_at \
             FROM reactions WHERE did = $1 AND created_at < $2 \
             ORDER BY created_at DESC LIMIT $3",
        )
        .bind(did)
        .bind(ts)
        .bind(limit)
        .fetch_all(pool)
        .await
    } else {
        sqlx::query_as::<_, ReactionRow>(
            "SELECT did, rkey, subject, emoji, created_at, indexed_at \
             FROM reactions WHERE did = $1 \
             ORDER BY created_at DESC LIMIT $2",
        )
        .bind(did)
        .bind(limit)
        .fetch_all(pool)
        .await
    }
}
