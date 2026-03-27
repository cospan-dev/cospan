use chrono::{DateTime, Utc};
use sqlx::PgPool;

#[derive(Debug, sqlx::FromRow, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct NodeRow {
    pub did: String,
    pub rkey: String,
    pub public_endpoint: Option<String>,
    pub created_at: DateTime<Utc>,
    pub indexed_at: DateTime<Utc>,
}

pub async fn upsert(pool: &PgPool, row: &NodeRow) -> Result<(), sqlx::Error> {
    sqlx::query(
        "INSERT INTO nodes (did, rkey, public_endpoint, created_at) \
         VALUES ($1, $2, $3, $4) \
         ON CONFLICT (did) DO UPDATE SET \
           rkey = EXCLUDED.rkey, \
           public_endpoint = EXCLUDED.public_endpoint, \
           indexed_at = NOW()",
    )
    .bind(&row.did)
    .bind(&row.rkey)
    .bind(&row.public_endpoint)
    .bind(row.created_at)
    .execute(pool)
    .await?;
    Ok(())
}

pub async fn delete(pool: &PgPool, did: &str) -> Result<(), sqlx::Error> {
    sqlx::query("DELETE FROM nodes WHERE did = $1")
        .bind(did)
        .execute(pool)
        .await?;
    Ok(())
}

pub async fn list(pool: &PgPool, limit: i64) -> Result<Vec<NodeRow>, sqlx::Error> {
    sqlx::query_as::<_, NodeRow>(
        "SELECT did, rkey, public_endpoint, created_at, indexed_at \
         FROM nodes ORDER BY created_at DESC LIMIT $1",
    )
    .bind(limit)
    .fetch_all(pool)
    .await
}
