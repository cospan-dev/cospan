//! Fork job tracking: rows live in the `fork_jobs` table.
//!
//! Each row represents a single background git-copy task spawned when a
//! user initiates a fork via POST /xrpc/dev.cospan.repo.fork. The PDS
//! record is created synchronously; the copy happens afterwards and
//! updates this row as it progresses.

use chrono::{DateTime, Utc};
use serde::Serialize;
use sqlx::PgPool;
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, sqlx::FromRow)]
pub struct ForkJob {
    pub id: Uuid,
    pub did: String,
    pub rkey: String,
    pub name: String,
    pub source_repo_uri: String,
    pub source_git_url: String,
    pub dest_git_url: String,
    pub state: String,
    pub refs_copied: i32,
    pub last_error: Option<String>,
    pub created_at: DateTime<Utc>,
    pub started_at: Option<DateTime<Utc>>,
    pub completed_at: Option<DateTime<Utc>>,
}

/// Insert a new fork job in the `pending` state. Returns the job id.
#[allow(clippy::too_many_arguments)]
pub async fn create(
    pool: &PgPool,
    did: &str,
    rkey: &str,
    name: &str,
    source_repo_uri: &str,
    source_git_url: &str,
    dest_git_url: &str,
) -> Result<Uuid, sqlx::Error> {
    let id = Uuid::new_v4();
    sqlx::query(
        "INSERT INTO fork_jobs \
         (id, did, rkey, name, source_repo_uri, source_git_url, dest_git_url, state) \
         VALUES ($1, $2, $3, $4, $5, $6, $7, 'pending')",
    )
    .bind(id)
    .bind(did)
    .bind(rkey)
    .bind(name)
    .bind(source_repo_uri)
    .bind(source_git_url)
    .bind(dest_git_url)
    .execute(pool)
    .await?;
    Ok(id)
}

pub async fn mark_running(pool: &PgPool, id: Uuid) -> Result<(), sqlx::Error> {
    sqlx::query(
        "UPDATE fork_jobs SET state = 'running', started_at = NOW() WHERE id = $1",
    )
    .bind(id)
    .execute(pool)
    .await?;
    Ok(())
}

pub async fn mark_completed(
    pool: &PgPool,
    id: Uuid,
    refs_copied: i32,
) -> Result<(), sqlx::Error> {
    sqlx::query(
        "UPDATE fork_jobs SET state = 'completed', refs_copied = $2, completed_at = NOW() \
         WHERE id = $1",
    )
    .bind(id)
    .bind(refs_copied)
    .execute(pool)
    .await?;
    Ok(())
}

pub async fn mark_failed(pool: &PgPool, id: Uuid, error: &str) -> Result<(), sqlx::Error> {
    sqlx::query(
        "UPDATE fork_jobs SET state = 'failed', last_error = $2, completed_at = NOW() \
         WHERE id = $1",
    )
    .bind(id)
    .bind(error)
    .execute(pool)
    .await?;
    Ok(())
}

pub async fn get(pool: &PgPool, id: Uuid) -> Result<Option<ForkJob>, sqlx::Error> {
    sqlx::query_as::<_, ForkJob>("SELECT * FROM fork_jobs WHERE id = $1")
        .bind(id)
        .fetch_optional(pool)
        .await
}

pub async fn list_for_repo(
    pool: &PgPool,
    did: &str,
    rkey: &str,
) -> Result<Vec<ForkJob>, sqlx::Error> {
    sqlx::query_as::<_, ForkJob>(
        "SELECT * FROM fork_jobs WHERE did = $1 AND rkey = $2 ORDER BY created_at DESC",
    )
    .bind(did)
    .bind(rkey)
    .fetch_all(pool)
    .await
}
