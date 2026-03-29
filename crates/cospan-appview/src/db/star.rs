pub use super::generated::crud::stars::{delete, get, list, upsert};
pub use super::generated::types::StarRow;

use chrono::{DateTime, Utc};
use sqlx::PgPool;

/// Increment the star_count on the repo referenced by the AT-URI subject.
/// Subject format: at://did/dev.cospan.repo/repo-name
pub async fn increment_repo_star_count(
    pool: &PgPool,
    repo_did: &str,
    repo_name: &str,
) -> Result<(), sqlx::Error> {
    sqlx::query(
        "UPDATE repos SET star_count = star_count + 1, indexed_at = NOW() \
         WHERE did = $1 AND name = $2",
    )
    .bind(repo_did)
    .bind(repo_name)
    .execute(pool)
    .await?;
    Ok(())
}

pub async fn decrement_repo_star_count(
    pool: &PgPool,
    repo_did: &str,
    repo_name: &str,
) -> Result<(), sqlx::Error> {
    sqlx::query(
        "UPDATE repos SET star_count = GREATEST(star_count - 1, 0), indexed_at = NOW() \
         WHERE did = $1 AND name = $2",
    )
    .bind(repo_did)
    .bind(repo_name)
    .execute(pool)
    .await?;
    Ok(())
}

pub async fn list_by_user(
    pool: &PgPool,
    did: &str,
    limit: i64,
    cursor: Option<&str>,
) -> Result<Vec<StarRow>, sqlx::Error> {
    if let Some(cursor_ts) = cursor {
        let ts: DateTime<Utc> = cursor_ts
            .parse()
            .map_err(|_| sqlx::Error::Protocol("invalid cursor".into()))?;
        sqlx::query_as::<_, StarRow>(
            "SELECT did, rkey, subject, created_at, indexed_at \
             FROM stars WHERE did = $1 AND created_at < $2 \
             ORDER BY created_at DESC LIMIT $3",
        )
        .bind(did)
        .bind(ts)
        .bind(limit)
        .fetch_all(pool)
        .await
    } else {
        sqlx::query_as::<_, StarRow>(
            "SELECT did, rkey, subject, created_at, indexed_at \
             FROM stars WHERE did = $1 \
             ORDER BY created_at DESC LIMIT $2",
        )
        .bind(did)
        .bind(limit)
        .fetch_all(pool)
        .await
    }
}

pub async fn list_for_subject(
    pool: &PgPool,
    subject: &str,
    limit: i64,
    cursor: Option<&str>,
) -> Result<Vec<StarRow>, sqlx::Error> {
    if let Some(cursor_ts) = cursor {
        let ts: DateTime<Utc> = cursor_ts
            .parse()
            .map_err(|_| sqlx::Error::Protocol("invalid cursor".into()))?;
        sqlx::query_as::<_, StarRow>(
            "SELECT did, rkey, subject, created_at, indexed_at \
             FROM stars WHERE subject = $1 AND created_at < $2 \
             ORDER BY created_at DESC LIMIT $3",
        )
        .bind(subject)
        .bind(ts)
        .bind(limit)
        .fetch_all(pool)
        .await
    } else {
        sqlx::query_as::<_, StarRow>(
            "SELECT did, rkey, subject, created_at, indexed_at \
             FROM stars WHERE subject = $1 \
             ORDER BY created_at DESC LIMIT $2",
        )
        .bind(subject)
        .bind(limit)
        .fetch_all(pool)
        .await
    }
}
