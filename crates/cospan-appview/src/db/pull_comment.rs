use chrono::{DateTime, Utc};
use sqlx::PgPool;

#[derive(Debug, sqlx::FromRow, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PullCommentRow {
    pub did: String,
    pub rkey: String,
    pub pull_uri: String,
    pub body: String,
    pub review_decision: Option<String>,
    pub created_at: DateTime<Utc>,
    pub indexed_at: DateTime<Utc>,
}

pub async fn upsert(pool: &PgPool, row: &PullCommentRow) -> Result<(), sqlx::Error> {
    sqlx::query(
        "INSERT INTO pull_comments (did, rkey, pull_uri, body, review_decision, created_at) \
         VALUES ($1, $2, $3, $4, $5, $6) \
         ON CONFLICT (did, rkey) DO UPDATE SET \
           body = EXCLUDED.body, \
           review_decision = EXCLUDED.review_decision, \
           indexed_at = NOW()",
    )
    .bind(&row.did)
    .bind(&row.rkey)
    .bind(&row.pull_uri)
    .bind(&row.body)
    .bind(&row.review_decision)
    .bind(row.created_at)
    .execute(pool)
    .await?;
    Ok(())
}

pub async fn delete(pool: &PgPool, did: &str, rkey: &str) -> Result<(), sqlx::Error> {
    sqlx::query("DELETE FROM pull_comments WHERE did = $1 AND rkey = $2")
        .bind(did)
        .bind(rkey)
        .execute(pool)
        .await?;
    Ok(())
}

pub async fn get(
    pool: &PgPool,
    did: &str,
    rkey: &str,
) -> Result<Option<PullCommentRow>, sqlx::Error> {
    sqlx::query_as::<_, PullCommentRow>(
        "SELECT did, rkey, pull_uri, body, review_decision, created_at, indexed_at \
         FROM pull_comments WHERE did = $1 AND rkey = $2",
    )
    .bind(did)
    .bind(rkey)
    .fetch_optional(pool)
    .await
}

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
