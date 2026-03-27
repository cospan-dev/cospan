use chrono::{DateTime, Utc};
use sqlx::PgPool;

#[derive(Debug, sqlx::FromRow, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct FollowRow {
    pub did: String,
    pub rkey: String,
    pub subject: String,
    pub created_at: DateTime<Utc>,
    pub indexed_at: DateTime<Utc>,
}

pub async fn upsert(pool: &PgPool, row: &FollowRow) -> Result<(), sqlx::Error> {
    sqlx::query(
        "INSERT INTO follows (did, rkey, subject, created_at) \
         VALUES ($1, $2, $3, $4) \
         ON CONFLICT (did, rkey) DO UPDATE SET \
           subject = EXCLUDED.subject, \
           indexed_at = NOW()",
    )
    .bind(&row.did)
    .bind(&row.rkey)
    .bind(&row.subject)
    .bind(row.created_at)
    .execute(pool)
    .await?;
    Ok(())
}

pub async fn delete(pool: &PgPool, did: &str, rkey: &str) -> Result<(), sqlx::Error> {
    sqlx::query("DELETE FROM follows WHERE did = $1 AND rkey = $2")
        .bind(did)
        .bind(rkey)
        .execute(pool)
        .await?;
    Ok(())
}

pub async fn get(pool: &PgPool, did: &str, rkey: &str) -> Result<Option<FollowRow>, sqlx::Error> {
    sqlx::query_as::<_, FollowRow>(
        "SELECT did, rkey, subject, created_at, indexed_at \
         FROM follows WHERE did = $1 AND rkey = $2",
    )
    .bind(did)
    .bind(rkey)
    .fetch_optional(pool)
    .await
}

pub async fn list_following(
    pool: &PgPool,
    did: &str,
    limit: i64,
    cursor: Option<&str>,
) -> Result<Vec<FollowRow>, sqlx::Error> {
    if let Some(cursor_ts) = cursor {
        let ts: DateTime<Utc> = cursor_ts
            .parse()
            .map_err(|_| sqlx::Error::Protocol("invalid cursor".into()))?;
        sqlx::query_as::<_, FollowRow>(
            "SELECT did, rkey, subject, created_at, indexed_at \
             FROM follows WHERE did = $1 AND created_at < $2 \
             ORDER BY created_at DESC LIMIT $3",
        )
        .bind(did)
        .bind(ts)
        .bind(limit)
        .fetch_all(pool)
        .await
    } else {
        sqlx::query_as::<_, FollowRow>(
            "SELECT did, rkey, subject, created_at, indexed_at \
             FROM follows WHERE did = $1 \
             ORDER BY created_at DESC LIMIT $2",
        )
        .bind(did)
        .bind(limit)
        .fetch_all(pool)
        .await
    }
}

pub async fn list_followers(
    pool: &PgPool,
    subject: &str,
    limit: i64,
    cursor: Option<&str>,
) -> Result<Vec<FollowRow>, sqlx::Error> {
    if let Some(cursor_ts) = cursor {
        let ts: DateTime<Utc> = cursor_ts
            .parse()
            .map_err(|_| sqlx::Error::Protocol("invalid cursor".into()))?;
        sqlx::query_as::<_, FollowRow>(
            "SELECT did, rkey, subject, created_at, indexed_at \
             FROM follows WHERE subject = $1 AND created_at < $2 \
             ORDER BY created_at DESC LIMIT $3",
        )
        .bind(subject)
        .bind(ts)
        .bind(limit)
        .fetch_all(pool)
        .await
    } else {
        sqlx::query_as::<_, FollowRow>(
            "SELECT did, rkey, subject, created_at, indexed_at \
             FROM follows WHERE subject = $1 \
             ORDER BY created_at DESC LIMIT $2",
        )
        .bind(subject)
        .bind(limit)
        .fetch_all(pool)
        .await
    }
}
