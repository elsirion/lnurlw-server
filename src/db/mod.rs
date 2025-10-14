pub mod models;
pub mod queries;

use sqlx::{Pool, Sqlite, SqlitePool};
use anyhow::Result;

pub async fn init_pool(database_url: &str) -> Result<Pool<Sqlite>> {
    let pool = SqlitePool::connect(database_url).await?;
    
    // Run migrations
    sqlx::migrate!("./migrations")
        .run(&pool)
        .await?;
    
    Ok(pool)
}