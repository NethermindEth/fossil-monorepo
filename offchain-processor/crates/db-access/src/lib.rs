#![deny(unused_crate_dependencies)]

pub mod auth;
pub mod models;
pub mod queries;

use eyre::{eyre, Result};
use sqlx::postgres::PgPoolOptions;
use sqlx::{Pool, Postgres};
use std::env;
use std::sync::Arc;

#[derive(Debug)]
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

pub struct OffchainProcessorDbConnection(Arc<DbConnection>);

impl OffchainProcessorDbConnection {
    pub async fn from_env() -> Result<Self> {
        let database_url =
            env::var("DATABASE_URL").map_err(|_| eyre!("DATABASE_URL must be set"))?;

        let db_connection = DbConnection::new(&database_url).await?;
        Ok(Self(db_connection))
    }

    pub async fn new(db_connection: Arc<DbConnection>) -> Result<Self> {
        Ok(Self(db_connection))
    }

    pub async fn migrate(&self) -> Result<()> {
        sqlx::migrate!("./migrations")
            .run(&self.db_connection().pool)
            .await?;
        Ok(())
    }

    pub fn db_connection(&self) -> Arc<DbConnection> {
        self.0.clone()
    }
}
