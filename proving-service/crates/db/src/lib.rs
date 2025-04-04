use eyre::{Result, eyre};
use sqlx::postgres::PgPoolOptions;
use sqlx::{Pool, Postgres};
use std::sync::Arc;

pub mod models;

pub struct DbConnection {
    pub pool: Pool<Postgres>,
}

// Use Arc to allow thread-safe cloning
impl DbConnection {
    pub async fn new(database_url: &str) -> Result<Arc<Self>> {
        let pool = PgPoolOptions::new()
            .max_connections(5)
            .connect(database_url)
            .await
            .map_err(|e| eyre!("Failed to connect to database: {}", e))?;

        Ok(Arc::new(Self { pool }))
    }
}
