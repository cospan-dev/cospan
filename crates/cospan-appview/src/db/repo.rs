use chrono::{DateTime, Utc};
use sqlx::PgPool;

#[derive(Debug, sqlx::FromRow, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RepoRow {
    pub did: String,
    pub rkey: String,
    pub name: String,
    pub description: Option<String>,
    pub protocol: String,
    pub node_did: String,
    pub node_url: String,
    pub default_branch: String,
    pub visibility: String,
    pub source_repo: Option<String>,
    pub star_count: i32,
    pub fork_count: i32,
    pub open_issue_count: i32,
    pub open_mr_count: i32,
    pub source: String,
    pub source_uri: Option<String>,
    pub created_at: DateTime<Utc>,
    pub indexed_at: DateTime<Utc>,
}

pub async fn upsert(pool: &PgPool, row: &RepoRow) -> Result<(), sqlx::Error> {
    sqlx::query(
        "INSERT INTO repos (did, rkey, name, description, protocol, node_did, node_url, \
              default_branch, visibility, source_repo, source, source_uri, created_at) \
         VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, $13) \
         ON CONFLICT (did, name) DO UPDATE SET \
           rkey = EXCLUDED.rkey, \
           description = EXCLUDED.description, \
           protocol = EXCLUDED.protocol, \
           node_did = EXCLUDED.node_did, \
           node_url = EXCLUDED.node_url, \
           default_branch = EXCLUDED.default_branch, \
           visibility = EXCLUDED.visibility, \
           source_repo = EXCLUDED.source_repo, \
           indexed_at = NOW()",
    )
    .bind(&row.did)
    .bind(&row.rkey)
    .bind(&row.name)
    .bind(&row.description)
    .bind(&row.protocol)
    .bind(&row.node_did)
    .bind(&row.node_url)
    .bind(&row.default_branch)
    .bind(&row.visibility)
    .bind(&row.source_repo)
    .bind(&row.source)
    .bind(&row.source_uri)
    .bind(row.created_at)
    .execute(pool)
    .await?;
    Ok(())
}

pub async fn get(pool: &PgPool, did: &str, name: &str) -> Result<Option<RepoRow>, sqlx::Error> {
    sqlx::query_as::<_, RepoRow>(
        "SELECT did, rkey, name, description, protocol, node_did, node_url, \
              default_branch, visibility, source_repo, star_count, fork_count, \
              open_issue_count, open_mr_count, source, source_uri, created_at, indexed_at \
         FROM repos WHERE did = $1 AND name = $2",
    )
    .bind(did)
    .bind(name)
    .fetch_optional(pool)
    .await
}

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
