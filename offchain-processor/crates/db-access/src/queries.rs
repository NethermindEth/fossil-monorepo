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

pub async fn create_job_request_with_vault(
    db: Arc<OffchainProcessorDbConnection>,
    job_id: &str,
    status: JobStatus,
    vault_address: &str,
    expected_timestamp: i64,
) -> Result<(), sqlx::Error> {
    sqlx::query!(
        "INSERT INTO job_requests (job_id, status, vault_address, expected_timestamp) VALUES ($1, $2, $3, $4)",
        job_id,
        status.to_string(),
        vault_address,
        expected_timestamp
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
            vault_address,
            expected_timestamp,
            created_at,
            updated_at,
            result,
            l1_data,
            on_chain_confirmation
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
        SET status = $2, result = $3, updated_at = CURRENT_TIMESTAMP
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

pub async fn update_job_with_event_data(
    db: Arc<OffchainProcessorDbConnection>,
    job_id: &str,
    status: JobStatus,
    l1_data: serde_json::Value,
    on_chain_confirmation: serde_json::Value,
) -> Result<(), sqlx::Error> {
    sqlx::query!(
        r#"
        UPDATE job_requests
        SET status = $2, l1_data = $3, on_chain_confirmation = $4, updated_at = CURRENT_TIMESTAMP
        WHERE job_id = $1
        "#,
        job_id,
        status.to_string(),
        l1_data,
        on_chain_confirmation
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

pub async fn get_multiple_job_requests(
    db: Arc<OffchainProcessorDbConnection>,
    job_ids: &[String],
) -> Result<Vec<JobRequest>, sqlx::Error> {
    sqlx::query_as!(
        JobRequest,
        r#"
        SELECT
            job_id,
            status as "status: JobStatus",
            vault_address,
            expected_timestamp,
            created_at,
            updated_at,
            result,
            l1_data,
            on_chain_confirmation
        FROM job_requests
        WHERE job_id = ANY($1)
        ORDER BY created_at DESC
        "#,
        job_ids
    )
    .fetch_all(&db.db_connection().pool)
    .await
}

pub async fn get_pending_jobs_with_vaults(
    db: Arc<OffchainProcessorDbConnection>,
) -> Result<Vec<JobRequest>, sqlx::Error> {
    sqlx::query_as!(
        JobRequest,
        r#"
        SELECT
            job_id,
            status as "status: JobStatus",
            vault_address,
            expected_timestamp,
            created_at,
            updated_at,
            result,
            l1_data,
            on_chain_confirmation
        FROM job_requests
        WHERE status = 'Pending' AND vault_address IS NOT NULL
        ORDER BY created_at ASC
        "#
    )
    .fetch_all(&db.db_connection().pool)
    .await
}
