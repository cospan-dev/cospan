use chrono::{DateTime, Utc};
use sqlx::PgPool;

#[derive(Debug, sqlx::FromRow, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DependencyRow {
    pub did: String,
    pub rkey: String,
    pub source_repo_did: String,
    pub source_repo_name: String,
    pub target_repo_did: String,
    pub target_repo_name: String,
    pub morphism_id: String,
    pub source_protocol: Option<String>,
    pub target_protocol: Option<String>,
    pub description: Option<String>,
    pub created_at: DateTime<Utc>,
    pub indexed_at: DateTime<Utc>,
}

pub async fn upsert(pool: &PgPool, row: &DependencyRow) -> Result<(), sqlx::Error> {
    sqlx::query(
        "INSERT INTO dependencies (did, rkey, source_repo_did, source_repo_name, \
              target_repo_did, target_repo_name, morphism_id, source_protocol, target_protocol, \
              description, created_at) \
         VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11) \
         ON CONFLICT (did, rkey) DO UPDATE SET \
           morphism_id = EXCLUDED.morphism_id, \
           source_protocol = EXCLUDED.source_protocol, \
           target_protocol = EXCLUDED.target_protocol, \
           description = EXCLUDED.description, \
           indexed_at = NOW()",
    )
    .bind(&row.did)
    .bind(&row.rkey)
    .bind(&row.source_repo_did)
    .bind(&row.source_repo_name)
    .bind(&row.target_repo_did)
    .bind(&row.target_repo_name)
    .bind(&row.morphism_id)
    .bind(&row.source_protocol)
    .bind(&row.target_protocol)
    .bind(&row.description)
    .bind(row.created_at)
    .execute(pool)
    .await?;
    Ok(())
}

pub async fn delete(pool: &PgPool, did: &str, rkey: &str) -> Result<(), sqlx::Error> {
    sqlx::query("DELETE FROM dependencies WHERE did = $1 AND rkey = $2")
        .bind(did)
        .bind(rkey)
        .execute(pool)
        .await?;
    Ok(())
}

pub async fn list_for_repo(
    pool: &PgPool,
    source_did: &str,
    source_name: &str,
    limit: i64,
    cursor: Option<&str>,
) -> Result<Vec<DependencyRow>, sqlx::Error> {
    if let Some(cursor_ts) = cursor {
        let ts: DateTime<Utc> = cursor_ts
            .parse()
            .map_err(|_| sqlx::Error::Protocol("invalid cursor".into()))?;
        sqlx::query_as::<_, DependencyRow>(
            "SELECT did, rkey, source_repo_did, source_repo_name, target_repo_did, \
                  target_repo_name, morphism_id, source_protocol, target_protocol, \
                  description, created_at, indexed_at \
             FROM dependencies WHERE source_repo_did = $1 AND source_repo_name = $2 \
             AND created_at < $3 ORDER BY created_at DESC LIMIT $4",
        )
        .bind(source_did)
        .bind(source_name)
        .bind(ts)
        .bind(limit)
        .fetch_all(pool)
        .await
    } else {
        sqlx::query_as::<_, DependencyRow>(
            "SELECT did, rkey, source_repo_did, source_repo_name, target_repo_did, \
                  target_repo_name, morphism_id, source_protocol, target_protocol, \
                  description, created_at, indexed_at \
             FROM dependencies WHERE source_repo_did = $1 AND source_repo_name = $2 \
             ORDER BY created_at DESC LIMIT $3",
        )
        .bind(source_did)
        .bind(source_name)
        .bind(limit)
        .fetch_all(pool)
        .await
    }
}

pub async fn list_dependents(
    pool: &PgPool,
    target_did: &str,
    target_name: &str,
    limit: i64,
    cursor: Option<&str>,
) -> Result<Vec<DependencyRow>, sqlx::Error> {
    if let Some(cursor_ts) = cursor {
        let ts: DateTime<Utc> = cursor_ts
            .parse()
            .map_err(|_| sqlx::Error::Protocol("invalid cursor".into()))?;
        sqlx::query_as::<_, DependencyRow>(
            "SELECT did, rkey, source_repo_did, source_repo_name, target_repo_did, \
                  target_repo_name, morphism_id, source_protocol, target_protocol, \
                  description, created_at, indexed_at \
             FROM dependencies WHERE target_repo_did = $1 AND target_repo_name = $2 \
             AND created_at < $3 ORDER BY created_at DESC LIMIT $4",
        )
        .bind(target_did)
        .bind(target_name)
        .bind(ts)
        .bind(limit)
        .fetch_all(pool)
        .await
    } else {
        sqlx::query_as::<_, DependencyRow>(
            "SELECT did, rkey, source_repo_did, source_repo_name, target_repo_did, \
                  target_repo_name, morphism_id, source_protocol, target_protocol, \
                  description, created_at, indexed_at \
             FROM dependencies WHERE target_repo_did = $1 AND target_repo_name = $2 \
             ORDER BY created_at DESC LIMIT $3",
        )
        .bind(target_did)
        .bind(target_name)
        .bind(limit)
        .fetch_all(pool)
        .await
    }
}
