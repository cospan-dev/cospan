use chrono::{DateTime, Utc};
use sqlx::PgPool;

#[derive(Debug, sqlx::FromRow, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PullStateRow {
    pub did: String,
    pub rkey: String,
    pub pull_uri: String,
    pub state: String,
    pub merge_commit_id: Option<String>,
    pub created_at: DateTime<Utc>,
    pub indexed_at: DateTime<Utc>,
}

pub async fn upsert(pool: &PgPool, row: &PullStateRow) -> Result<(), sqlx::Error> {
    sqlx::query(
        "INSERT INTO pull_states (did, rkey, pull_uri, state, merge_commit_id, created_at) \
         VALUES ($1, $2, $3, $4, $5, $6) \
         ON CONFLICT (did, rkey) DO UPDATE SET \
           state = EXCLUDED.state, \
           merge_commit_id = EXCLUDED.merge_commit_id, \
           indexed_at = NOW()",
    )
    .bind(&row.did)
    .bind(&row.rkey)
    .bind(&row.pull_uri)
    .bind(&row.state)
    .bind(&row.merge_commit_id)
    .bind(row.created_at)
    .execute(pool)
    .await?;
    Ok(())
}

pub async fn delete(pool: &PgPool, did: &str, rkey: &str) -> Result<(), sqlx::Error> {
    sqlx::query("DELETE FROM pull_states WHERE did = $1 AND rkey = $2")
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
) -> Result<Option<PullStateRow>, sqlx::Error> {
    sqlx::query_as::<_, PullStateRow>(
        "SELECT did, rkey, pull_uri, state, merge_commit_id, created_at, indexed_at \
         FROM pull_states WHERE did = $1 AND rkey = $2",
    )
    .bind(did)
    .bind(rkey)
    .fetch_optional(pool)
    .await
}

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
