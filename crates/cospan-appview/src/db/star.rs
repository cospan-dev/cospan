pub use super::generated::crud::stars::{delete, get, list, upsert};
pub use super::generated::types::StarRow;

use chrono::{DateTime, Utc};
use sqlx::PgPool;

/// Increment the star_count on the repo referenced by the AT-URI subject.
/// Subject format: at://did/collection/rkey
/// The rkey in the AT-URI may be the repo name OR the record rkey,
/// so we match on either.
pub async fn increment_repo_star_count(
    pool: &PgPool,
    repo_did: &str,
    repo_rkey: &str,
) -> Result<(), sqlx::Error> {
    sqlx::query(
        "UPDATE repos SET star_count = star_count + 1, indexed_at = NOW() \
         WHERE did = $1 AND (name = $2 OR rkey = $2)",
    )
    .bind(repo_did)
    .bind(repo_rkey)
    .execute(pool)
    .await?;
    Ok(())
}

pub async fn decrement_repo_star_count(
    pool: &PgPool,
    repo_did: &str,
    repo_rkey: &str,
) -> Result<(), sqlx::Error> {
    sqlx::query(
        "UPDATE repos SET star_count = GREATEST(star_count - 1, 0), indexed_at = NOW() \
         WHERE did = $1 AND (name = $2 OR rkey = $2)",
    )
    .bind(repo_did)
    .bind(repo_rkey)
    .execute(pool)
    .await?;
    Ok(())
}

/// Recompute all star counts from the stars table.
/// Handles backfill ordering where stars arrive before repos.
pub async fn recount_all_stars(pool: &PgPool) -> Result<u64, sqlx::Error> {
    let result = sqlx::query(
        "UPDATE repos SET star_count = COALESCE(s.cnt, 0) \
         FROM ( \
             SELECT \
                 split_part(replace(subject, 'at://', ''), '/', 1) AS repo_did, \
                 split_part(replace(subject, 'at://', ''), '/', 3) AS repo_rkey, \
                 COUNT(*) AS cnt \
             FROM stars \
             GROUP BY 1, 2 \
         ) s \
         WHERE repos.did = s.repo_did AND repos.rkey = s.repo_rkey \
         AND repos.star_count <> s.cnt",
    )
    .execute(pool)
    .await?;
    Ok(result.rows_affected())
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
