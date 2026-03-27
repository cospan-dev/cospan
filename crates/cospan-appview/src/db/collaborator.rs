use chrono::{DateTime, Utc};
use sqlx::PgPool;

#[derive(Debug, sqlx::FromRow, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CollaboratorRow {
    pub did: String,
    pub rkey: String,
    pub repo_did: String,
    pub repo_name: String,
    pub member_did: String,
    pub role: String,
    pub created_at: DateTime<Utc>,
    pub indexed_at: DateTime<Utc>,
}

pub async fn upsert(pool: &PgPool, row: &CollaboratorRow) -> Result<(), sqlx::Error> {
    sqlx::query(
        "INSERT INTO collaborators (did, rkey, repo_did, repo_name, member_did, role, created_at) \
         VALUES ($1, $2, $3, $4, $5, $6, $7) \
         ON CONFLICT (did, rkey) DO UPDATE SET \
           role = EXCLUDED.role, \
           indexed_at = NOW()",
    )
    .bind(&row.did)
    .bind(&row.rkey)
    .bind(&row.repo_did)
    .bind(&row.repo_name)
    .bind(&row.member_did)
    .bind(&row.role)
    .bind(row.created_at)
    .execute(pool)
    .await?;
    Ok(())
}

pub async fn delete(pool: &PgPool, did: &str, rkey: &str) -> Result<(), sqlx::Error> {
    sqlx::query("DELETE FROM collaborators WHERE did = $1 AND rkey = $2")
        .bind(did)
        .bind(rkey)
        .execute(pool)
        .await?;
    Ok(())
}

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
