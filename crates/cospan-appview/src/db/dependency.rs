pub use super::generated::crud::dependencies::{delete, get, list, upsert};
pub use super::generated::types::DependencyRow;

use chrono::{DateTime, Utc};
use sqlx::PgPool;

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
