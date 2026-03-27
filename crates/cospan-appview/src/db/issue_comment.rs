use chrono::{DateTime, Utc};
use sqlx::PgPool;

#[derive(Debug, sqlx::FromRow, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct IssueCommentRow {
    pub did: String,
    pub rkey: String,
    pub issue_uri: String,
    pub body: String,
    pub created_at: DateTime<Utc>,
    pub indexed_at: DateTime<Utc>,
}

pub async fn upsert(pool: &PgPool, row: &IssueCommentRow) -> Result<(), sqlx::Error> {
    sqlx::query(
        "INSERT INTO issue_comments (did, rkey, issue_uri, body, created_at) \
         VALUES ($1, $2, $3, $4, $5) \
         ON CONFLICT (did, rkey) DO UPDATE SET \
           body = EXCLUDED.body, \
           indexed_at = NOW()",
    )
    .bind(&row.did)
    .bind(&row.rkey)
    .bind(&row.issue_uri)
    .bind(&row.body)
    .bind(row.created_at)
    .execute(pool)
    .await?;
    Ok(())
}

pub async fn delete(pool: &PgPool, did: &str, rkey: &str) -> Result<(), sqlx::Error> {
    sqlx::query("DELETE FROM issue_comments WHERE did = $1 AND rkey = $2")
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
) -> Result<Option<IssueCommentRow>, sqlx::Error> {
    sqlx::query_as::<_, IssueCommentRow>(
        "SELECT did, rkey, issue_uri, body, created_at, indexed_at \
         FROM issue_comments WHERE did = $1 AND rkey = $2",
    )
    .bind(did)
    .bind(rkey)
    .fetch_optional(pool)
    .await
}

pub async fn list_for_issue(
    pool: &PgPool,
    issue_uri: &str,
    limit: i64,
    cursor: Option<&str>,
) -> Result<Vec<IssueCommentRow>, sqlx::Error> {
    if let Some(cursor_ts) = cursor {
        let ts: DateTime<Utc> = cursor_ts
            .parse()
            .map_err(|_| sqlx::Error::Protocol("invalid cursor".into()))?;
        sqlx::query_as::<_, IssueCommentRow>(
            "SELECT did, rkey, issue_uri, body, created_at, indexed_at \
             FROM issue_comments WHERE issue_uri = $1 AND created_at > $2 \
             ORDER BY created_at ASC LIMIT $3",
        )
        .bind(issue_uri)
        .bind(ts)
        .bind(limit)
        .fetch_all(pool)
        .await
    } else {
        sqlx::query_as::<_, IssueCommentRow>(
            "SELECT did, rkey, issue_uri, body, created_at, indexed_at \
             FROM issue_comments WHERE issue_uri = $1 \
             ORDER BY created_at ASC LIMIT $2",
        )
        .bind(issue_uri)
        .bind(limit)
        .fetch_all(pool)
        .await
    }
}
