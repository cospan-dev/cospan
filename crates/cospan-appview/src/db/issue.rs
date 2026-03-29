pub use super::generated::crud::issues::{delete, list, upsert};
pub use super::generated::types::IssueRow;

use chrono::{DateTime, Utc};
use sqlx::PgPool;

/// Get an issue by repo owner DID, repo name, and issue rkey.
pub async fn get(
    pool: &PgPool,
    repo_did: &str,
    repo_name: &str,
    rkey: &str,
) -> Result<Option<IssueRow>, sqlx::Error> {
    sqlx::query_as::<_, IssueRow>(
        "SELECT did, rkey, repo_did, repo_name, title, body, state, comment_count, \
              created_at, indexed_at \
         FROM issues WHERE repo_did = $1 AND repo_name = $2 AND rkey = $3",
    )
    .bind(repo_did)
    .bind(repo_name)
    .bind(rkey)
    .fetch_optional(pool)
    .await
}

/// Get an issue by record creator DID and rkey (primary key lookup).
pub async fn get_by_pk(
    pool: &PgPool,
    did: &str,
    rkey: &str,
) -> Result<Option<IssueRow>, sqlx::Error> {
    sqlx::query_as::<_, IssueRow>(
        "SELECT did, rkey, repo_did, repo_name, title, body, state, comment_count, \
              created_at, indexed_at \
         FROM issues WHERE did = $1 AND rkey = $2",
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
    sqlx::query("UPDATE issues SET state = $3, indexed_at = NOW() WHERE did = $1 AND rkey = $2")
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
        "UPDATE issues SET comment_count = comment_count + 1, indexed_at = NOW() \
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
        "UPDATE issues SET comment_count = GREATEST(comment_count - 1, 0), indexed_at = NOW() \
         WHERE did = $1 AND rkey = $2",
    )
    .bind(did)
    .bind(rkey)
    .execute(pool)
    .await?;
    Ok(())
}

/// List issues for a repo, optionally filtering by state.
pub async fn list_for_repo(
    pool: &PgPool,
    repo_did: &str,
    repo_name: &str,
    state_filter: Option<&str>,
    limit: i64,
    cursor: Option<&str>,
) -> Result<Vec<IssueRow>, sqlx::Error> {
    match (state_filter, cursor) {
        (Some(state), Some(cursor_ts)) => {
            let ts: DateTime<Utc> = cursor_ts
                .parse()
                .map_err(|_| sqlx::Error::Protocol("invalid cursor".into()))?;
            sqlx::query_as::<_, IssueRow>(
                "SELECT did, rkey, repo_did, repo_name, title, body, state, comment_count, \
                      created_at, indexed_at \
                 FROM issues WHERE repo_did = $1 AND repo_name = $2 AND state = $3 AND created_at < $4 \
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
            sqlx::query_as::<_, IssueRow>(
                "SELECT did, rkey, repo_did, repo_name, title, body, state, comment_count, \
                      created_at, indexed_at \
                 FROM issues WHERE repo_did = $1 AND repo_name = $2 AND state = $3 \
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
            sqlx::query_as::<_, IssueRow>(
                "SELECT did, rkey, repo_did, repo_name, title, body, state, comment_count, \
                      created_at, indexed_at \
                 FROM issues WHERE repo_did = $1 AND repo_name = $2 AND created_at < $3 \
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
            sqlx::query_as::<_, IssueRow>(
                "SELECT did, rkey, repo_did, repo_name, title, body, state, comment_count, \
                      created_at, indexed_at \
                 FROM issues WHERE repo_did = $1 AND repo_name = $2 \
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
