use chrono::{DateTime, Utc};
use sqlx::PgPool;

#[derive(Debug, sqlx::FromRow, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PipelineRow {
    pub did: String,
    pub rkey: String,
    pub repo_did: String,
    pub repo_name: String,
    pub commit_id: String,
    pub ref_name: Option<String>,
    pub status: String,
    pub gat_type_check: Option<String>,
    pub equation_verification: Option<String>,
    pub lens_law_check: Option<String>,
    pub breaking_change_check: Option<String>,
    pub created_at: DateTime<Utc>,
    pub completed_at: Option<DateTime<Utc>>,
    pub indexed_at: DateTime<Utc>,
}

pub async fn upsert(pool: &PgPool, row: &PipelineRow) -> Result<(), sqlx::Error> {
    sqlx::query(
        "INSERT INTO pipelines (did, rkey, repo_did, repo_name, commit_id, ref_name, status, \
              gat_type_check, equation_verification, lens_law_check, breaking_change_check, \
              created_at, completed_at) \
         VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, $13) \
         ON CONFLICT (did, rkey) DO UPDATE SET \
           status = EXCLUDED.status, \
           gat_type_check = EXCLUDED.gat_type_check, \
           equation_verification = EXCLUDED.equation_verification, \
           lens_law_check = EXCLUDED.lens_law_check, \
           breaking_change_check = EXCLUDED.breaking_change_check, \
           completed_at = EXCLUDED.completed_at, \
           indexed_at = NOW()",
    )
    .bind(&row.did)
    .bind(&row.rkey)
    .bind(&row.repo_did)
    .bind(&row.repo_name)
    .bind(&row.commit_id)
    .bind(&row.ref_name)
    .bind(&row.status)
    .bind(&row.gat_type_check)
    .bind(&row.equation_verification)
    .bind(&row.lens_law_check)
    .bind(&row.breaking_change_check)
    .bind(row.created_at)
    .bind(row.completed_at)
    .execute(pool)
    .await?;
    Ok(())
}

pub async fn delete(pool: &PgPool, did: &str, rkey: &str) -> Result<(), sqlx::Error> {
    sqlx::query("DELETE FROM pipelines WHERE did = $1 AND rkey = $2")
        .bind(did)
        .bind(rkey)
        .execute(pool)
        .await?;
    Ok(())
}

pub async fn get(pool: &PgPool, did: &str, rkey: &str) -> Result<Option<PipelineRow>, sqlx::Error> {
    sqlx::query_as::<_, PipelineRow>(
        "SELECT did, rkey, repo_did, repo_name, commit_id, ref_name, status, \
              gat_type_check, equation_verification, lens_law_check, breaking_change_check, \
              created_at, completed_at, indexed_at \
         FROM pipelines WHERE did = $1 AND rkey = $2",
    )
    .bind(did)
    .bind(rkey)
    .fetch_optional(pool)
    .await
}

pub async fn list_for_repo(
    pool: &PgPool,
    repo_did: &str,
    repo_name: &str,
    limit: i64,
    cursor: Option<&str>,
) -> Result<Vec<PipelineRow>, sqlx::Error> {
    if let Some(cursor_ts) = cursor {
        let ts: DateTime<Utc> = cursor_ts
            .parse()
            .map_err(|_| sqlx::Error::Protocol("invalid cursor".into()))?;
        sqlx::query_as::<_, PipelineRow>(
            "SELECT did, rkey, repo_did, repo_name, commit_id, ref_name, status, \
                  gat_type_check, equation_verification, lens_law_check, breaking_change_check, \
                  created_at, completed_at, indexed_at \
             FROM pipelines WHERE repo_did = $1 AND repo_name = $2 AND created_at < $3 \
             ORDER BY created_at DESC LIMIT $4",
        )
        .bind(repo_did)
        .bind(repo_name)
        .bind(ts)
        .bind(limit)
        .fetch_all(pool)
        .await
    } else {
        sqlx::query_as::<_, PipelineRow>(
            "SELECT did, rkey, repo_did, repo_name, commit_id, ref_name, status, \
                  gat_type_check, equation_verification, lens_law_check, breaking_change_check, \
                  created_at, completed_at, indexed_at \
             FROM pipelines WHERE repo_did = $1 AND repo_name = $2 \
             ORDER BY created_at DESC LIMIT $3",
        )
        .bind(repo_did)
        .bind(repo_name)
        .bind(limit)
        .fetch_all(pool)
        .await
    }
}

pub async fn list_by_commit(
    pool: &PgPool,
    repo_did: &str,
    repo_name: &str,
    commit_id: &str,
) -> Result<Vec<PipelineRow>, sqlx::Error> {
    sqlx::query_as::<_, PipelineRow>(
        "SELECT did, rkey, repo_did, repo_name, commit_id, ref_name, status, \
              gat_type_check, equation_verification, lens_law_check, breaking_change_check, \
              created_at, completed_at, indexed_at \
         FROM pipelines WHERE repo_did = $1 AND repo_name = $2 AND commit_id = $3 \
         ORDER BY created_at DESC",
    )
    .bind(repo_did)
    .bind(repo_name)
    .bind(commit_id)
    .fetch_all(pool)
    .await
}
