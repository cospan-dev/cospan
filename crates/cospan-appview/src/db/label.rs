pub use super::generated::crud::labels::{delete, get, list, upsert};
pub use super::generated::types::LabelRow;

use sqlx::PgPool;

pub async fn list_by_repo(
    pool: &PgPool,
    repo_did: &str,
    repo_name: &str,
    limit: i64,
) -> Result<Vec<LabelRow>, sqlx::Error> {
    sqlx::query_as::<_, LabelRow>(
        "SELECT did, rkey, repo_did, repo_name, name, color, description, created_at, indexed_at \
         FROM labels WHERE repo_did = $1 AND repo_name = $2 \
         ORDER BY name ASC LIMIT $3",
    )
    .bind(repo_did)
    .bind(repo_name)
    .bind(limit)
    .fetch_all(pool)
    .await
}
