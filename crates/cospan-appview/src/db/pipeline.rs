pub use super::generated::crud::pipelines::{delete, get, list, upsert};
pub use super::generated::types::PipelineRow;

use chrono::{DateTime, Utc};
use sqlx::PgPool;

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
