use chrono::{DateTime, Utc};
use sqlx::PgPool;

#[derive(Debug, sqlx::FromRow, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct OrgRow {
    pub did: String,
    pub rkey: String,
    pub name: String,
    pub description: Option<String>,
    pub avatar_cid: Option<String>,
    pub created_at: DateTime<Utc>,
    pub indexed_at: DateTime<Utc>,
}

pub async fn upsert(pool: &PgPool, row: &OrgRow) -> Result<(), sqlx::Error> {
    sqlx::query(
        "INSERT INTO orgs (did, rkey, name, description, avatar_cid, created_at) \
         VALUES ($1, $2, $3, $4, $5, $6) \
         ON CONFLICT (did, rkey) DO UPDATE SET \
           name = EXCLUDED.name, \
           description = EXCLUDED.description, \
           avatar_cid = EXCLUDED.avatar_cid, \
           indexed_at = NOW()",
    )
    .bind(&row.did)
    .bind(&row.rkey)
    .bind(&row.name)
    .bind(&row.description)
    .bind(&row.avatar_cid)
    .bind(row.created_at)
    .execute(pool)
    .await?;
    Ok(())
}

pub async fn delete(pool: &PgPool, did: &str, rkey: &str) -> Result<(), sqlx::Error> {
    sqlx::query("DELETE FROM orgs WHERE did = $1 AND rkey = $2")
        .bind(did)
        .bind(rkey)
        .execute(pool)
        .await?;
    Ok(())
}

pub async fn get(pool: &PgPool, did: &str, rkey: &str) -> Result<Option<OrgRow>, sqlx::Error> {
    sqlx::query_as::<_, OrgRow>(
        "SELECT did, rkey, name, description, avatar_cid, created_at, indexed_at \
         FROM orgs WHERE did = $1 AND rkey = $2",
    )
    .bind(did)
    .bind(rkey)
    .fetch_optional(pool)
    .await
}

pub async fn list(
    pool: &PgPool,
    limit: i64,
    cursor: Option<&str>,
) -> Result<Vec<OrgRow>, sqlx::Error> {
    if let Some(cursor_ts) = cursor {
        let ts: DateTime<Utc> = cursor_ts
            .parse()
            .map_err(|_| sqlx::Error::Protocol("invalid cursor".into()))?;
        sqlx::query_as::<_, OrgRow>(
            "SELECT did, rkey, name, description, avatar_cid, created_at, indexed_at \
             FROM orgs WHERE created_at < $1 \
             ORDER BY created_at DESC LIMIT $2",
        )
        .bind(ts)
        .bind(limit)
        .fetch_all(pool)
        .await
    } else {
        sqlx::query_as::<_, OrgRow>(
            "SELECT did, rkey, name, description, avatar_cid, created_at, indexed_at \
             FROM orgs ORDER BY created_at DESC LIMIT $1",
        )
        .bind(limit)
        .fetch_all(pool)
        .await
    }
}
