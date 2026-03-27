use chrono::{DateTime, Utc};
use sqlx::PgPool;

#[derive(Debug, sqlx::FromRow, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ActorProfileRow {
    pub did: String,
    pub bluesky: String,
    pub display_name: Option<String>,
    pub description: Option<String>,
    pub avatar_cid: Option<String>,
    pub indexed_at: DateTime<Utc>,
}

pub async fn upsert(pool: &PgPool, row: &ActorProfileRow) -> Result<(), sqlx::Error> {
    sqlx::query(
        "INSERT INTO actor_profiles (did, bluesky, display_name, description, avatar_cid) \
         VALUES ($1, $2, $3, $4, $5) \
         ON CONFLICT (did) DO UPDATE SET \
           bluesky = EXCLUDED.bluesky, \
           display_name = EXCLUDED.display_name, \
           description = EXCLUDED.description, \
           avatar_cid = EXCLUDED.avatar_cid, \
           indexed_at = NOW()",
    )
    .bind(&row.did)
    .bind(&row.bluesky)
    .bind(&row.display_name)
    .bind(&row.description)
    .bind(&row.avatar_cid)
    .execute(pool)
    .await?;
    Ok(())
}

pub async fn delete(pool: &PgPool, did: &str) -> Result<(), sqlx::Error> {
    sqlx::query("DELETE FROM actor_profiles WHERE did = $1")
        .bind(did)
        .execute(pool)
        .await?;
    Ok(())
}

pub async fn get(pool: &PgPool, did: &str) -> Result<Option<ActorProfileRow>, sqlx::Error> {
    sqlx::query_as::<_, ActorProfileRow>(
        "SELECT did, bluesky, display_name, description, avatar_cid, indexed_at \
         FROM actor_profiles WHERE did = $1",
    )
    .bind(did)
    .fetch_optional(pool)
    .await
}
