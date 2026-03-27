use chrono::{DateTime, Utc};
use sqlx::PgPool;

#[derive(Debug, sqlx::FromRow, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct OrgMemberRow {
    pub did: String,
    pub rkey: String,
    pub org_uri: String,
    pub member_did: String,
    pub role: String,
    pub created_at: DateTime<Utc>,
    pub indexed_at: DateTime<Utc>,
}

pub async fn upsert(pool: &PgPool, row: &OrgMemberRow) -> Result<(), sqlx::Error> {
    sqlx::query(
        "INSERT INTO org_members (did, rkey, org_uri, member_did, role, created_at) \
         VALUES ($1, $2, $3, $4, $5, $6) \
         ON CONFLICT (did, rkey) DO UPDATE SET \
           role = EXCLUDED.role, \
           indexed_at = NOW()",
    )
    .bind(&row.did)
    .bind(&row.rkey)
    .bind(&row.org_uri)
    .bind(&row.member_did)
    .bind(&row.role)
    .bind(row.created_at)
    .execute(pool)
    .await?;
    Ok(())
}

pub async fn delete(pool: &PgPool, did: &str, rkey: &str) -> Result<(), sqlx::Error> {
    sqlx::query("DELETE FROM org_members WHERE did = $1 AND rkey = $2")
        .bind(did)
        .bind(rkey)
        .execute(pool)
        .await?;
    Ok(())
}

pub async fn get(
    pool: &PgPool,
    did: &str,
    rkey: &str,
) -> Result<Option<OrgMemberRow>, sqlx::Error> {
    sqlx::query_as::<_, OrgMemberRow>(
        "SELECT did, rkey, org_uri, member_did, role, created_at, indexed_at \
         FROM org_members WHERE did = $1 AND rkey = $2",
    )
    .bind(did)
    .bind(rkey)
    .fetch_optional(pool)
    .await
}

pub async fn list_for_org(
    pool: &PgPool,
    org_uri: &str,
    limit: i64,
    cursor: Option<&str>,
) -> Result<Vec<OrgMemberRow>, sqlx::Error> {
    if let Some(cursor_ts) = cursor {
        let ts: DateTime<Utc> = cursor_ts
            .parse()
            .map_err(|_| sqlx::Error::Protocol("invalid cursor".into()))?;
        sqlx::query_as::<_, OrgMemberRow>(
            "SELECT did, rkey, org_uri, member_did, role, created_at, indexed_at \
             FROM org_members WHERE org_uri = $1 AND created_at < $2 \
             ORDER BY created_at DESC LIMIT $3",
        )
        .bind(org_uri)
        .bind(ts)
        .bind(limit)
        .fetch_all(pool)
        .await
    } else {
        sqlx::query_as::<_, OrgMemberRow>(
            "SELECT did, rkey, org_uri, member_did, role, created_at, indexed_at \
             FROM org_members WHERE org_uri = $1 \
             ORDER BY created_at DESC LIMIT $2",
        )
        .bind(org_uri)
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
) -> Result<Vec<OrgMemberRow>, sqlx::Error> {
    if let Some(cursor_ts) = cursor {
        let ts: DateTime<Utc> = cursor_ts
            .parse()
            .map_err(|_| sqlx::Error::Protocol("invalid cursor".into()))?;
        sqlx::query_as::<_, OrgMemberRow>(
            "SELECT did, rkey, org_uri, member_did, role, created_at, indexed_at \
             FROM org_members WHERE member_did = $1 AND created_at < $2 \
             ORDER BY created_at DESC LIMIT $3",
        )
        .bind(member_did)
        .bind(ts)
        .bind(limit)
        .fetch_all(pool)
        .await
    } else {
        sqlx::query_as::<_, OrgMemberRow>(
            "SELECT did, rkey, org_uri, member_did, role, created_at, indexed_at \
             FROM org_members WHERE member_did = $1 \
             ORDER BY created_at DESC LIMIT $2",
        )
        .bind(member_did)
        .bind(limit)
        .fetch_all(pool)
        .await
    }
}
