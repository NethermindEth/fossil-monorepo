use std::sync::Arc;

use crate::models::{JobRequest, JobStatus};
use crate::OffchainProcessorDbConnection;
use eyre::Result;

pub async fn create_job_request(
    db: Arc<OffchainProcessorDbConnection>,
    job_id: &str,
    status: JobStatus,
) -> Result<(), sqlx::Error> {
    sqlx::query!(
        "INSERT INTO job_requests (job_id, status) VALUES ($1, $2)",
        job_id,
        status.to_string()
    )
    .execute(&db.db_connection().pool)
    .await?;

    Ok(())
}

pub async fn get_job_request(
    db: Arc<OffchainProcessorDbConnection>,
    job_id: &str,
) -> Result<Option<JobRequest>, sqlx::Error> {
    sqlx::query_as!(
        JobRequest,
        r#"
        SELECT 
            job_id,
            status as "status: JobStatus",
            created_at,
            result
        FROM job_requests
        WHERE job_id = $1
        "#,
        job_id
    )
    .fetch_optional(&db.db_connection().pool)
    .await
}

pub async fn update_job_status(
    db: Arc<OffchainProcessorDbConnection>,
    job_id: &str,
    status: JobStatus,
    result: Option<serde_json::Value>,
) -> Result<(), sqlx::Error> {
    sqlx::query!(
        r#"
        UPDATE job_requests
        SET status = $2, result = $3
        WHERE job_id = $1
        "#,
        job_id,
        status.to_string(),
        result
    )
    .execute(&db.db_connection().pool)
    .await?;

    Ok(())
}

pub async fn update_job_result(
    db: Arc<OffchainProcessorDbConnection>,
    job_id: &str,
    status: &str,
    result: serde_json::Value,
) -> Result<()> {
    sqlx::query!(
        r#"
        UPDATE job_requests
        SET status = $2, result = $3
        WHERE job_id = $1
        "#,
        job_id,
        status,
        result
    )
    .execute(&db.db_connection().pool)
    .await?;

    Ok(())
}
