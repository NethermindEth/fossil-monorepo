use std::sync::Arc;

use crate::{
    types::{GetJobStatusResponseEnum, JobResponse, PitchLakeJobRequest},
    AppState,
};
use axum::{extract::State, http::StatusCode, Json};
use db_access::{
    models::JobStatus, queries::create_job_request, DbConnection, OffchainProcessorDbConnection,
};
use lazy_static::lazy_static;
use sqlx::postgres::PgPoolOptions;
use testcontainers::{clients::Cli, images::postgres::Postgres as PostgresImage, Container};

use super::{get_pricing_data::get_pricing_data, job_status::get_job_status};

lazy_static! {
    static ref DOCKER: Cli = Cli::default();
}

pub struct TestContext {
    pub app_state: AppState,
    pub offchain_processor_db: Arc<OffchainProcessorDbConnection>,
    pub _container: Container<'static, PostgresImage>,
}

impl TestContext {
    /// Creates a new test context with a PostgreSQL container and initializes the required tables.
    pub async fn new() -> Self {
        let container = DOCKER.run(PostgresImage::default());
        let port = container.get_host_port_ipv4(5432);
        let connection_string = format!("postgres://postgres:postgres@localhost:{}/postgres", port);

        let pool = PgPoolOptions::new()
            .max_connections(5)
            .connect(&connection_string)
            .await
            .expect("Failed to create database pool");

        // Create the `job_requests` table with the dynamic JSONB result column.
        sqlx::query(
            r#"
            CREATE TABLE IF NOT EXISTS job_requests (
                job_id TEXT PRIMARY KEY,
                status TEXT NOT NULL CHECK (status IN ('Completed', 'Pending', 'Failed')),
                result JSONB, -- Stores dynamic JSON responses
                created_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP
            )
            "#,
        )
        .execute(&pool)
        .await
        .expect("Failed to create job_requests table");

        // Create a single db connection for use by both involved db
        let db = Arc::new(DbConnection { pool: pool.clone() });
        let offchain_processor_db = Arc::new(OffchainProcessorDbConnection::new(db).await.unwrap());
        let app_state = AppState {
            offchain_processor_db: offchain_processor_db.clone(),
        };

        Self {
            app_state,
            offchain_processor_db,
            _container: container,
        }
    }

    /// Creates a new job request with a given status.
    pub async fn create_job(&self, job_id: &str, status: JobStatus) {
        create_job_request(self.offchain_processor_db.clone(), job_id, status)
            .await
            .expect("Failed to create job request");
    }

    pub async fn get_job_status(
        &self,
        job_id: &str,
    ) -> (StatusCode, Json<GetJobStatusResponseEnum>) {
        get_job_status(
            State(self.app_state.clone()),
            axum::extract::Path(job_id.to_string()),
        )
        .await
    }

    /// Sends a pricing data request and returns the status and response.
    pub async fn get_pricing_data(
        &self,
        payload: PitchLakeJobRequest,
    ) -> (StatusCode, Json<JobResponse>) {
        get_pricing_data(State(self.app_state.clone()), Json(payload)).await
    }

    pub async fn create_job_with_result(
        &self,
        job_id: &str,
        status: JobStatus,
        result: serde_json::Value,
    ) {
        sqlx::query(
            r#"
            INSERT INTO job_requests (job_id, status, result)
            VALUES ($1, $2, $3::jsonb)
            "#,
        )
        .bind(job_id)
        .bind(status.to_string())
        .bind(result)
        .execute(&self.offchain_processor_db.db_connection().pool)
        .await
        .expect("Failed to create job request with result");
    }
}
