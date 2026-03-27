use chrono::{DateTime, Utc};
use sqlx::PgPool;

#[derive(Debug, sqlx::FromRow, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PullRow {
    pub did: String,
    pub rkey: String,
    pub repo_did: String,
    pub repo_name: String,
    pub title: String,
    pub body: Option<String>,
    pub target_ref: String,
    pub source_ref: String,
    pub source_repo: Option<String>,
    pub state: String,
    pub comment_count: i32,
    pub created_at: DateTime<Utc>,
    pub indexed_at: DateTime<Utc>,
}

pub async fn upsert(pool: &PgPool, row: &PullRow) -> Result<(), sqlx::Error> {
    sqlx::query(
        "INSERT INTO pulls (did, rkey, repo_did, repo_name, title, body, target_ref, source_ref, \
              source_repo, state, created_at) \
         VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11) \
         ON CONFLICT (did, rkey) DO UPDATE SET \
           title = EXCLUDED.title, \
           body = EXCLUDED.body, \
           target_ref = EXCLUDED.target_ref, \
           source_ref = EXCLUDED.source_ref, \
           source_repo = EXCLUDED.source_repo, \
           indexed_at = NOW()",
    )
    .bind(&row.did)
    .bind(&row.rkey)
    .bind(&row.repo_did)
    .bind(&row.repo_name)
    .bind(&row.title)
    .bind(&row.body)
    .bind(&row.target_ref)
    .bind(&row.source_ref)
    .bind(&row.source_repo)
    .bind(&row.state)
    .bind(row.created_at)
    .execute(pool)
    .await?;
    Ok(())
}

pub async fn delete(pool: &PgPool, did: &str, rkey: &str) -> Result<(), sqlx::Error> {
    sqlx::query("DELETE FROM pulls WHERE did = $1 AND rkey = $2")
        .bind(did)
        .bind(rkey)
        .execute(pool)
        .await?;
    Ok(())
}

/// Get a pull by repo owner DID, repo name, and pull rkey.
pub async fn get(
    pool: &PgPool,
    repo_did: &str,
    repo_name: &str,
    rkey: &str,
) -> Result<Option<PullRow>, sqlx::Error> {
    sqlx::query_as::<_, PullRow>(
        "SELECT did, rkey, repo_did, repo_name, title, body, target_ref, source_ref, \
              source_repo, state, comment_count, created_at, indexed_at \
         FROM pulls WHERE repo_did = $1 AND repo_name = $2 AND rkey = $3",
    )
    .bind(repo_did)
    .bind(repo_name)
    .bind(rkey)
    .fetch_optional(pool)
    .await
}

/// Get a pull by record creator DID and rkey (primary key lookup).
pub async fn get_by_pk(
    pool: &PgPool,
    did: &str,
    rkey: &str,
) -> Result<Option<PullRow>, sqlx::Error> {
    sqlx::query_as::<_, PullRow>(
        "SELECT did, rkey, repo_did, repo_name, title, body, target_ref, source_ref, \
              source_repo, state, comment_count, created_at, indexed_at \
         FROM pulls WHERE did = $1 AND rkey = $2",
    )
    .bind(did)
    .bind(rkey)
    .fetch_optional(pool)
    .await
}

pub async fn update_state(
    pool: &PgPool,
    did: &str,
    rkey: &str,
    state: &str,
) -> Result<(), sqlx::Error> {
    sqlx::query("UPDATE pulls SET state = $3, indexed_at = NOW() WHERE did = $1 AND rkey = $2")
        .bind(did)
        .bind(rkey)
        .bind(state)
        .execute(pool)
        .await?;
    Ok(())
}

pub async fn increment_comment_count(
    pool: &PgPool,
    did: &str,
    rkey: &str,
) -> Result<(), sqlx::Error> {
    sqlx::query(
        "UPDATE pulls SET comment_count = comment_count + 1, indexed_at = NOW() \
         WHERE did = $1 AND rkey = $2",
    )
    .bind(did)
    .bind(rkey)
    .execute(pool)
    .await?;
    Ok(())
}

pub async fn decrement_comment_count(
    pool: &PgPool,
    did: &str,
    rkey: &str,
) -> Result<(), sqlx::Error> {
    sqlx::query(
        "UPDATE pulls SET comment_count = GREATEST(comment_count - 1, 0), indexed_at = NOW() \
         WHERE did = $1 AND rkey = $2",
    )
    .bind(did)
    .bind(rkey)
    .execute(pool)
    .await?;
    Ok(())
}

/// List pulls for a repo, optionally filtering by state.
pub async fn list_for_repo(
    pool: &PgPool,
    repo_did: &str,
    repo_name: &str,
    state_filter: Option<&str>,
    limit: i64,
    cursor: Option<&str>,
) -> Result<Vec<PullRow>, sqlx::Error> {
    match (state_filter, cursor) {
        (Some(state), Some(cursor_ts)) => {
            let ts: DateTime<Utc> = cursor_ts
                .parse()
                .map_err(|_| sqlx::Error::Protocol("invalid cursor".into()))?;
            sqlx::query_as::<_, PullRow>(
                "SELECT did, rkey, repo_did, repo_name, title, body, target_ref, source_ref, \
                      source_repo, state, comment_count, created_at, indexed_at \
                 FROM pulls WHERE repo_did = $1 AND repo_name = $2 AND state = $3 AND created_at < $4 \
                 ORDER BY created_at DESC LIMIT $5",
            )
            .bind(repo_did)
            .bind(repo_name)
            .bind(state)
            .bind(ts)
            .bind(limit)
            .fetch_all(pool)
            .await
        }
        (Some(state), None) => {
            sqlx::query_as::<_, PullRow>(
                "SELECT did, rkey, repo_did, repo_name, title, body, target_ref, source_ref, \
                      source_repo, state, comment_count, created_at, indexed_at \
                 FROM pulls WHERE repo_did = $1 AND repo_name = $2 AND state = $3 \
                 ORDER BY created_at DESC LIMIT $4",
            )
            .bind(repo_did)
            .bind(repo_name)
            .bind(state)
            .bind(limit)
            .fetch_all(pool)
            .await
        }
        (None, Some(cursor_ts)) => {
            let ts: DateTime<Utc> = cursor_ts
                .parse()
                .map_err(|_| sqlx::Error::Protocol("invalid cursor".into()))?;
            sqlx::query_as::<_, PullRow>(
                "SELECT did, rkey, repo_did, repo_name, title, body, target_ref, source_ref, \
                      source_repo, state, comment_count, created_at, indexed_at \
                 FROM pulls WHERE repo_did = $1 AND repo_name = $2 AND created_at < $3 \
                 ORDER BY created_at DESC LIMIT $4",
            )
            .bind(repo_did)
            .bind(repo_name)
            .bind(ts)
            .bind(limit)
            .fetch_all(pool)
            .await
        }
        (None, None) => {
            sqlx::query_as::<_, PullRow>(
                "SELECT did, rkey, repo_did, repo_name, title, body, target_ref, source_ref, \
                      source_repo, state, comment_count, created_at, indexed_at \
                 FROM pulls WHERE repo_did = $1 AND repo_name = $2 \
                 ORDER BY created_at DESC LIMIT $3",
            )
            .bind(repo_did)
            .bind(repo_name)
            .bind(limit)
            .fetch_all(pool)
            .await
        }
    }
}
