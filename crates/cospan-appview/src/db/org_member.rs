pub use super::generated::crud::org_members::{delete, get, list, upsert};
pub use super::generated::types::OrgMemberRow;

use chrono::{DateTime, Utc};
use sqlx::PgPool;

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
