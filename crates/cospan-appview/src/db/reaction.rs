use chrono::{DateTime, Utc};
use sqlx::PgPool;

#[derive(Debug, sqlx::FromRow, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ReactionRow {
    pub did: String,
    pub rkey: String,
    pub subject: String,
    pub emoji: String,
    pub created_at: DateTime<Utc>,
    pub indexed_at: DateTime<Utc>,
}

pub async fn upsert(pool: &PgPool, row: &ReactionRow) -> Result<(), sqlx::Error> {
    sqlx::query(
        "INSERT INTO reactions (did, rkey, subject, emoji, created_at) \
         VALUES ($1, $2, $3, $4, $5) \
         ON CONFLICT (did, rkey) DO UPDATE SET \
           subject = EXCLUDED.subject, \
           emoji = EXCLUDED.emoji, \
           indexed_at = NOW()",
    )
    .bind(&row.did)
    .bind(&row.rkey)
    .bind(&row.subject)
    .bind(&row.emoji)
    .bind(row.created_at)
    .execute(pool)
    .await?;
    Ok(())
}

pub async fn delete(pool: &PgPool, did: &str, rkey: &str) -> Result<(), sqlx::Error> {
    sqlx::query("DELETE FROM reactions WHERE did = $1 AND rkey = $2")
        .bind(did)
        .bind(rkey)
        .execute(pool)
        .await?;
    Ok(())
}

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
