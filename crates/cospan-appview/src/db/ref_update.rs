pub use super::generated::crud::ref_updates::{get, list, upsert};
pub use super::generated::types::RefUpdateRow;

use sqlx::PgPool;

/// Delete a ref_update by committer DID and rkey (firehose delete event).
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
