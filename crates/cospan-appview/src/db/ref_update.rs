use chrono::{DateTime, Utc};
use sqlx::PgPool;

#[derive(Debug, sqlx::FromRow, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RefUpdateRow {
    pub id: i64,
    pub repo_did: String,
    pub repo_name: String,
    pub rkey: String,
    pub committer_did: String,
    pub ref_name: String,
    pub old_target: Option<String>,
    pub new_target: String,
    pub protocol: String,
    pub migration_id: Option<String>,
    pub breaking_change_count: i32,
    pub lens_id: Option<String>,
    pub lens_quality: Option<f32>,
    pub commit_count: i32,
    pub created_at: DateTime<Utc>,
    pub indexed_at: DateTime<Utc>,
}

pub async fn upsert(pool: &PgPool, row: &RefUpdateRow) -> Result<(), sqlx::Error> {
    sqlx::query(
        "INSERT INTO ref_updates (repo_did, repo_name, rkey, committer_did, ref_name, \
              old_target, new_target, protocol, migration_id, breaking_change_count, \
              lens_id, lens_quality, commit_count, created_at) \
         VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, $13, $14) \
         ON CONFLICT (committer_did, rkey) DO UPDATE SET \
           ref_name = EXCLUDED.ref_name, \
           old_target = EXCLUDED.old_target, \
           new_target = EXCLUDED.new_target, \
           breaking_change_count = EXCLUDED.breaking_change_count, \
           lens_id = EXCLUDED.lens_id, \
           lens_quality = EXCLUDED.lens_quality, \
           indexed_at = NOW()",
    )
    .bind(&row.repo_did)
    .bind(&row.repo_name)
    .bind(&row.rkey)
    .bind(&row.committer_did)
    .bind(&row.ref_name)
    .bind(&row.old_target)
    .bind(&row.new_target)
    .bind(&row.protocol)
    .bind(&row.migration_id)
    .bind(row.breaking_change_count)
    .bind(&row.lens_id)
    .bind(row.lens_quality)
    .bind(row.commit_count)
    .bind(row.created_at)
    .execute(pool)
    .await?;
    Ok(())
}

pub async fn delete(pool: &PgPool, committer_did: &str, rkey: &str) -> Result<(), sqlx::Error> {
    sqlx::query("DELETE FROM ref_updates WHERE committer_did = $1 AND rkey = $2")
        .bind(committer_did)
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
    cursor: Option<i64>,
) -> Result<Vec<RefUpdateRow>, sqlx::Error> {
    if let Some(cursor_id) = cursor {
        sqlx::query_as::<_, RefUpdateRow>(
            "SELECT id, repo_did, repo_name, rkey, committer_did, ref_name, \
                  old_target, new_target, protocol, migration_id, breaking_change_count, \
                  lens_id, lens_quality, commit_count, created_at, indexed_at \
             FROM ref_updates \
             WHERE repo_did = $1 AND repo_name = $2 AND id < $3 \
             ORDER BY created_at DESC LIMIT $4",
        )
        .bind(repo_did)
        .bind(repo_name)
        .bind(cursor_id)
        .bind(limit)
        .fetch_all(pool)
        .await
    } else {
        sqlx::query_as::<_, RefUpdateRow>(
            "SELECT id, repo_did, repo_name, rkey, committer_did, ref_name, \
                  old_target, new_target, protocol, migration_id, breaking_change_count, \
                  lens_id, lens_quality, commit_count, created_at, indexed_at \
             FROM ref_updates \
             WHERE repo_did = $1 AND repo_name = $2 \
             ORDER BY created_at DESC LIMIT $3",
        )
        .bind(repo_did)
        .bind(repo_name)
        .bind(limit)
        .fetch_all(pool)
        .await
    }
}
