pub use super::generated::crud::repos::{get, list, upsert};
pub use super::generated::types::RepoRow;

use chrono::{DateTime, Utc};
use sqlx::PgPool;

pub async fn list_by_did(
    pool: &PgPool,
    did: &str,
    limit: i64,
    cursor: Option<&str>,
) -> Result<Vec<RepoRow>, sqlx::Error> {
    if let Some(cursor_ts) = cursor {
        let ts: DateTime<Utc> = cursor_ts
            .parse()
            .map_err(|_| sqlx::Error::Protocol("invalid cursor".into()))?;
        sqlx::query_as::<_, RepoRow>(
            "SELECT did, rkey, name, description, protocol, node_did, node_url, \
                  default_branch, visibility, source_repo, star_count, fork_count, \
                  open_issue_count, open_mr_count, source, source_uri, created_at, indexed_at \
             FROM repos WHERE did = $1 AND created_at < $2 \
             ORDER BY created_at DESC LIMIT $3",
        )
        .bind(did)
        .bind(ts)
        .bind(limit)
        .fetch_all(pool)
        .await
    } else {
        sqlx::query_as::<_, RepoRow>(
            "SELECT did, rkey, name, description, protocol, node_did, node_url, \
                  default_branch, visibility, source_repo, star_count, fork_count, \
                  open_issue_count, open_mr_count, source, source_uri, created_at, indexed_at \
             FROM repos WHERE did = $1 \
             ORDER BY created_at DESC LIMIT $2",
        )
        .bind(did)
        .bind(limit)
        .fetch_all(pool)
        .await
    }
}

pub async fn list_by_source(
    pool: &PgPool,
    source: &str,
    limit: i64,
    cursor: Option<&str>,
) -> Result<Vec<RepoRow>, sqlx::Error> {
    if let Some(cursor_ts) = cursor {
        let ts: DateTime<Utc> = cursor_ts
            .parse()
            .map_err(|_| sqlx::Error::Protocol("invalid cursor".into()))?;
        sqlx::query_as::<_, RepoRow>(
            "SELECT did, rkey, name, description, protocol, node_did, node_url, \
                  default_branch, visibility, source_repo, star_count, fork_count, \
                  open_issue_count, open_mr_count, source, source_uri, created_at, indexed_at \
             FROM repos WHERE source = $1 AND created_at < $2 \
             ORDER BY created_at DESC LIMIT $3",
        )
        .bind(source)
        .bind(ts)
        .bind(limit)
        .fetch_all(pool)
        .await
    } else {
        sqlx::query_as::<_, RepoRow>(
            "SELECT did, rkey, name, description, protocol, node_did, node_url, \
                  default_branch, visibility, source_repo, star_count, fork_count, \
                  open_issue_count, open_mr_count, source, source_uri, created_at, indexed_at \
             FROM repos WHERE source = $1 \
             ORDER BY created_at DESC LIMIT $2",
        )
        .bind(source)
        .bind(limit)
        .fetch_all(pool)
        .await
    }
}

pub async fn list_recent(
    pool: &PgPool,
    limit: i64,
    cursor: Option<&str>,
) -> Result<Vec<RepoRow>, sqlx::Error> {
    if let Some(cursor_ts) = cursor {
        let ts: DateTime<Utc> = cursor_ts
            .parse()
            .map_err(|_| sqlx::Error::Protocol("invalid cursor".into()))?;
        sqlx::query_as::<_, RepoRow>(
            "SELECT did, rkey, name, description, protocol, node_did, node_url, \
                  default_branch, visibility, source_repo, star_count, fork_count, \
                  open_issue_count, open_mr_count, source, source_uri, created_at, indexed_at \
             FROM repos WHERE created_at < $1 \
             ORDER BY created_at DESC LIMIT $2",
        )
        .bind(ts)
        .bind(limit)
        .fetch_all(pool)
        .await
    } else {
        sqlx::query_as::<_, RepoRow>(
            "SELECT did, rkey, name, description, protocol, node_did, node_url, \
                  default_branch, visibility, source_repo, star_count, fork_count, \
                  open_issue_count, open_mr_count, source, source_uri, created_at, indexed_at \
             FROM repos ORDER BY created_at DESC LIMIT $1",
        )
        .bind(limit)
        .fetch_all(pool)
        .await
    }
}

pub async fn search(
    pool: &PgPool,
    query: &str,
    limit: i64,
    cursor: Option<&str>,
) -> Result<Vec<RepoRow>, sqlx::Error> {
    if let Some(cursor_ts) = cursor {
        let ts: DateTime<Utc> = cursor_ts
            .parse()
            .map_err(|_| sqlx::Error::Protocol("invalid cursor".into()))?;
        sqlx::query_as::<_, RepoRow>(
            "SELECT did, rkey, name, description, protocol, node_did, node_url, \
                  default_branch, visibility, source_repo, star_count, fork_count, \
                  open_issue_count, open_mr_count, source, source_uri, created_at, indexed_at \
             FROM repos \
             WHERE to_tsvector('english', coalesce(name, '') || ' ' || coalesce(description, '')) \
                   @@ plainto_tsquery('english', $1) \
                   AND created_at < $2 \
             ORDER BY created_at DESC LIMIT $3",
        )
        .bind(query)
        .bind(ts)
        .bind(limit)
        .fetch_all(pool)
        .await
    } else {
        sqlx::query_as::<_, RepoRow>(
            "SELECT did, rkey, name, description, protocol, node_did, node_url, \
                  default_branch, visibility, source_repo, star_count, fork_count, \
                  open_issue_count, open_mr_count, source, source_uri, created_at, indexed_at \
             FROM repos \
             WHERE to_tsvector('english', coalesce(name, '') || ' ' || coalesce(description, '')) \
                   @@ plainto_tsquery('english', $1) \
             ORDER BY created_at DESC LIMIT $2",
        )
        .bind(query)
        .bind(limit)
        .fetch_all(pool)
        .await
    }
}
