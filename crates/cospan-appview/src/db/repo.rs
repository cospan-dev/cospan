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

pub async fn list_by_source_popular(
    pool: &PgPool,
    source: &str,
    limit: i64,
    cursor: Option<&str>,
) -> Result<Vec<RepoRow>, sqlx::Error> {
    if let Some(cursor_stars) = cursor {
        let stars: i32 = cursor_stars.parse().unwrap_or(0);
        sqlx::query_as::<_, RepoRow>(
            "SELECT did, rkey, name, description, protocol, node_did, node_url, \
                  default_branch, visibility, source_repo, star_count, fork_count, \
                  open_issue_count, open_mr_count, source, source_uri, created_at, indexed_at \
             FROM repos WHERE source = $1 AND star_count < $2 \
             ORDER BY star_count DESC, created_at DESC LIMIT $3",
        )
        .bind(source)
        .bind(stars)
        .bind(limit)
        .fetch_all(pool)
        .await
    } else {
        sqlx::query_as::<_, RepoRow>(
            "SELECT did, rkey, name, description, protocol, node_did, node_url, \
                  default_branch, visibility, source_repo, star_count, fork_count, \
                  open_issue_count, open_mr_count, source, source_uri, created_at, indexed_at \
             FROM repos WHERE source = $1 \
             ORDER BY star_count DESC, created_at DESC LIMIT $2",
        )
        .bind(source)
        .bind(limit)
        .fetch_all(pool)
        .await
    }
}

/// Resolve a repo rkey to its human-readable name.
/// Returns the rkey unchanged if no repo is found.
pub async fn resolve_rkey_to_name(
    pool: &PgPool,
    did: &str,
    rkey: &str,
) -> Result<String, sqlx::Error> {
    let row: Option<(String,)> = sqlx::query_as(
        "SELECT name FROM repos WHERE did = $1 AND rkey = $2",
    )
    .bind(did)
    .bind(rkey)
    .fetch_optional(pool)
    .await?;
    Ok(row.map(|r| r.0).unwrap_or_else(|| rkey.to_string()))
}

/// Insert a stub repo row if one doesn't already exist for (did, name).
/// Used during backfill when child records (issues, pulls, etc.) arrive
/// before their parent repo. The stub will be overwritten with full data
/// when the repo record itself is processed.
pub async fn ensure_exists(
    pool: &PgPool,
    did: &str,
    name: &str,
    source: &str,
) -> Result<(), sqlx::Error> {
    sqlx::query(
        "INSERT INTO repos (did, rkey, name, protocol, node_did, node_url, source, created_at) \
         VALUES ($1, '', $2, 'git', '', '', $3, NOW()) \
         ON CONFLICT (did, name) DO NOTHING",
    )
    .bind(did)
    .bind(name)
    .bind(source)
    .execute(pool)
    .await?;
    Ok(())
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
