use sqlx::PgPool;

pub async fn get_cursor(pool: &PgPool) -> Result<Option<i64>, sqlx::Error> {
    let row: Option<(i64,)> = sqlx::query_as("SELECT cursor_us FROM firehose_cursor WHERE id = 1")
        .fetch_optional(pool)
        .await?;
    Ok(row.map(|r| r.0))
}

pub async fn save_cursor(pool: &PgPool, cursor_us: i64) -> Result<(), sqlx::Error> {
    sqlx::query(
        "INSERT INTO firehose_cursor (id, cursor_us, updated_at) \
         VALUES (1, $1, NOW()) \
         ON CONFLICT (id) DO UPDATE SET cursor_us = $1, updated_at = NOW()",
    )
    .bind(cursor_us)
    .execute(pool)
    .await?;
    Ok(())
}
