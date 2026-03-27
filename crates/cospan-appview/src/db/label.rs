use chrono::{DateTime, Utc};
use sqlx::PgPool;

#[derive(Debug, sqlx::FromRow, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct LabelRow {
    pub did: String,
    pub rkey: String,
    pub repo_did: String,
    pub repo_name: String,
    pub name: String,
    pub color: String,
    pub description: Option<String>,
    pub created_at: DateTime<Utc>,
    pub indexed_at: DateTime<Utc>,
}

pub async fn upsert(pool: &PgPool, row: &LabelRow) -> Result<(), sqlx::Error> {
    sqlx::query(
        "INSERT INTO labels (did, rkey, repo_did, repo_name, name, color, description, created_at) \
         VALUES ($1, $2, $3, $4, $5, $6, $7, $8) \
         ON CONFLICT (did, rkey) DO UPDATE SET \
           name = EXCLUDED.name, \
           color = EXCLUDED.color, \
           description = EXCLUDED.description, \
           indexed_at = NOW()",
    )
    .bind(&row.did)
    .bind(&row.rkey)
    .bind(&row.repo_did)
    .bind(&row.repo_name)
    .bind(&row.name)
    .bind(&row.color)
    .bind(&row.description)
    .bind(row.created_at)
    .execute(pool)
    .await?;
    Ok(())
}

pub async fn delete(pool: &PgPool, did: &str, rkey: &str) -> Result<(), sqlx::Error> {
    sqlx::query("DELETE FROM labels WHERE did = $1 AND rkey = $2")
        .bind(did)
        .bind(rkey)
        .execute(pool)
        .await?;
    Ok(())
}

pub async fn list_by_repo(
    pool: &PgPool,
    repo_did: &str,
    repo_name: &str,
    limit: i64,
) -> Result<Vec<LabelRow>, sqlx::Error> {
    sqlx::query_as::<_, LabelRow>(
        "SELECT did, rkey, repo_did, repo_name, name, color, description, created_at, indexed_at \
         FROM labels WHERE repo_did = $1 AND repo_name = $2 \
         ORDER BY name ASC LIMIT $3",
    )
    .bind(repo_did)
    .bind(repo_name)
    .bind(limit)
    .fetch_all(pool)
    .await
}
