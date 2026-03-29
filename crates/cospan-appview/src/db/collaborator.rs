pub use super::generated::crud::collaborators::{delete, get, list, upsert};
pub use super::generated::types::CollaboratorRow;

use chrono::{DateTime, Utc};
use sqlx::PgPool;

pub async fn list_for_repo(
    pool: &PgPool,
    repo_did: &str,
    repo_name: &str,
    limit: i64,
    cursor: Option<&str>,
) -> Result<Vec<CollaboratorRow>, sqlx::Error> {
    if let Some(cursor_ts) = cursor {
        let ts: DateTime<Utc> = cursor_ts
            .parse()
            .map_err(|_| sqlx::Error::Protocol("invalid cursor".into()))?;
        sqlx::query_as::<_, CollaboratorRow>(
            "SELECT did, rkey, repo_did, repo_name, member_did, role, created_at, indexed_at \
             FROM collaborators WHERE repo_did = $1 AND repo_name = $2 AND created_at < $3 \
             ORDER BY created_at DESC LIMIT $4",
        )
        .bind(repo_did)
        .bind(repo_name)
        .bind(ts)
        .bind(limit)
        .fetch_all(pool)
        .await
    } else {
        sqlx::query_as::<_, CollaboratorRow>(
            "SELECT did, rkey, repo_did, repo_name, member_did, role, created_at, indexed_at \
             FROM collaborators WHERE repo_did = $1 AND repo_name = $2 \
             ORDER BY created_at DESC LIMIT $3",
        )
        .bind(repo_did)
        .bind(repo_name)
        .bind(limit)
        .fetch_all(pool)
        .await
    }
}

pub async fn list_by_member(
    pool: &PgPool,
    member_did: &str,
    limit: i64,
    cursor: Option<&str>,
) -> Result<Vec<CollaboratorRow>, sqlx::Error> {
    if let Some(cursor_ts) = cursor {
        let ts: DateTime<Utc> = cursor_ts
            .parse()
            .map_err(|_| sqlx::Error::Protocol("invalid cursor".into()))?;
        sqlx::query_as::<_, CollaboratorRow>(
            "SELECT did, rkey, repo_did, repo_name, member_did, role, created_at, indexed_at \
             FROM collaborators WHERE member_did = $1 AND created_at < $2 \
             ORDER BY created_at DESC LIMIT $3",
        )
        .bind(member_did)
        .bind(ts)
        .bind(limit)
        .fetch_all(pool)
        .await
    } else {
        sqlx::query_as::<_, CollaboratorRow>(
            "SELECT did, rkey, repo_did, repo_name, member_did, role, created_at, indexed_at \
             FROM collaborators WHERE member_did = $1 \
             ORDER BY created_at DESC LIMIT $2",
        )
        .bind(member_did)
        .bind(limit)
        .fetch_all(pool)
        .await
    }
}
