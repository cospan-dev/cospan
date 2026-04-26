pub use super::generated::crud::repos::{delete, get, list, upsert};
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
    let row: Option<(String,)> =
        sqlx::query_as("SELECT name FROM repos WHERE did = $1 AND rkey = $2")
            .bind(did)
            .bind(rkey)
            .fetch_optional(pool)
            .await?;
    Ok(row.map(|r| r.0).unwrap_or_else(|| rkey.to_string()))
}

/// Ensure a repo exists for (did, name) so FK constraints are satisfied.
/// If the name is already a real repo name, this is a no-op.
/// If the name is a TID/rkey (no real repo found yet), check if a repo
/// with this rkey exists and use its real name. Otherwise skip: the
/// child record will fail FK and be retried when the repo arrives.
pub async fn ensure_exists(
    pool: &PgPool,
    did: &str,
    name: &str,
    _source: &str,
) -> Result<(), sqlx::Error> {
    // Check if repo already exists by name
    let exists: Option<(i32,)> = sqlx::query_as("SELECT 1 FROM repos WHERE did = $1 AND name = $2")
        .bind(did)
        .bind(name)
        .fetch_optional(pool)
        .await?;
    if exists.is_some() {
        return Ok(());
    }
    // Check if it exists by rkey (name might be an unresolved rkey)
    let exists_by_rkey: Option<(i64,)> =
        sqlx::query_as("SELECT 1 FROM repos WHERE did = $1 AND rkey = $2")
            .bind(did)
            .bind(name)
            .fetch_optional(pool)
            .await?;
    if exists_by_rkey.is_some() {
        // Repo exists under a different name: the caller should
        // resolve the rkey to the real name before inserting.
        return Ok(());
    }
    // No repo found at all: don't create a stub with a TID as name,
    // it creates garbage data. Let the FK constraint fail; the record
    // will be retried when the repo arrives via backfill.
    Ok(())
}

pub async fn search(
    pool: &PgPool,
    query: &str,
    source: Option<&str>,
    limit: i64,
    _cursor: Option<&str>,
) -> Result<Vec<RepoRow>, sqlx::Error> {
    let pattern = format!("%{query}%");
    let source_clause = if source.is_some() {
        "AND source = $3"
    } else {
        ""
    };
    let sql = format!(
        "SELECT did, rkey, name, description, protocol, node_did, node_url, \
              default_branch, visibility, source_repo, star_count, fork_count, \
              open_issue_count, open_mr_count, source, source_uri, created_at, indexed_at \
         FROM repos \
         WHERE (name ILIKE $1 OR coalesce(description, '') ILIKE $1) \
         {source_clause} \
         ORDER BY star_count DESC, created_at DESC LIMIT $2"
    );
    let mut q = sqlx::query_as::<_, RepoRow>(&sql)
        .bind(&pattern)
        .bind(limit);
    if let Some(s) = source {
        q = q.bind(s);
    }
    q.fetch_all(pool).await
}
