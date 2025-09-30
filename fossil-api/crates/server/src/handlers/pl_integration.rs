use crate::types::{
    BatchJobStatusRequest, BatchJobStatusResponse, ErrorResponse, JobResultResponse,
};
use crate::AppState;
use axum::extract::{Json, State};
use axum::http::StatusCode;
use db_access::queries::{get_job_request, get_multiple_job_requests};

// Enhanced job status endpoint with detailed result information
#[axum::debug_handler]
pub async fn get_job_result(
    State(state): State<AppState>,
    axum::extract::Path(job_id): axum::extract::Path<String>,
) -> (StatusCode, Json<Result<JobResultResponse, ErrorResponse>>) {
    tracing::info!("Getting detailed result for job_id: {}", job_id);

    match get_job_request(state.offchain_processor_db, &job_id).await {
        Ok(Some(job)) => {
            tracing::info!(
                "Found job with status: {:?} for job_id: {}",
                job.status,
                job_id
            );
            (
                StatusCode::OK,
                Json(Ok(JobResultResponse {
                    job_id: job.job_id,
                    status: job.status,
                    result: job.result,
                    created_at: Some(job.created_at.format("%Y-%m-%dT%H:%M:%SZ").to_string()),
                    completed_at: job
                        .updated_at
                        .map(|t| t.format("%Y-%m-%dT%H:%M:%SZ").to_string()),
                })),
            )
        }
        Ok(None) => {
            tracing::info!("Job not found for job_id: {}", job_id);
            (
                StatusCode::NOT_FOUND,
                Json(Err(ErrorResponse {
                    error: "Job not found".to_string(),
                })),
            )
        }
        Err(e) => {
            tracing::error!("Failed to get job result for job_id {}: {:?}", job_id, e);
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(Err(ErrorResponse {
                    error: "An internal error occurred. Please try again later.".to_string(),
                })),
            )
        }
    }
}

// Batch job status endpoint for PitchLake to query multiple jobs efficiently
#[axum::debug_handler]
pub async fn get_batch_job_status(
    State(state): State<AppState>,
    Json(payload): Json<BatchJobStatusRequest>,
) -> (StatusCode, Json<BatchJobStatusResponse>) {
    tracing::info!("Getting batch status for {} jobs", payload.job_ids.len());

    if payload.job_ids.is_empty() {
        return (
            StatusCode::BAD_REQUEST,
            Json(BatchJobStatusResponse {
                jobs: vec![],
                not_found: vec![],
            }),
        );
    }

    if payload.job_ids.len() > 100 {
        tracing::warn!("Batch request too large: {} jobs", payload.job_ids.len());
        return (
            StatusCode::BAD_REQUEST,
            Json(BatchJobStatusResponse {
                jobs: vec![],
                not_found: payload.job_ids,
            }),
        );
    }

    match get_multiple_job_requests(state.offchain_processor_db, &payload.job_ids).await {
        Ok(jobs) => {
            let found_jobs: std::collections::HashSet<_> =
                jobs.iter().map(|j| j.job_id.clone()).collect();
            let not_found: Vec<String> = payload
                .job_ids
                .into_iter()
                .filter(|id| !found_jobs.contains(id))
                .collect();

            let job_responses: Vec<JobResultResponse> = jobs
                .into_iter()
                .map(|job| JobResultResponse {
                    job_id: job.job_id,
                    status: job.status,
                    result: job.result,
                    created_at: Some(job.created_at.format("%Y-%m-%dT%H:%M:%SZ").to_string()),
                    completed_at: job
                        .updated_at
                        .map(|t| t.format("%Y-%m-%dT%H:%M:%SZ").to_string()),
                })
                .collect();

            tracing::info!(
                "Found {} jobs, {} not found",
                job_responses.len(),
                not_found.len()
            );

            (
                StatusCode::OK,
                Json(BatchJobStatusResponse {
                    jobs: job_responses,
                    not_found,
                }),
            )
        }
        Err(e) => {
            tracing::error!("Failed to get batch job status: {:?}", e);
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(BatchJobStatusResponse {
                    jobs: vec![],
                    not_found: payload.job_ids,
                }),
            )
        }
    }
}

// Webhook callback endpoint for PitchLake notifications
#[axum::debug_handler]
pub async fn job_callback_webhook(
    State(_state): State<AppState>,
    axum::extract::Path(job_id): axum::extract::Path<String>,
    Json(payload): Json<serde_json::Value>,
) -> (StatusCode, Json<serde_json::Value>) {
    tracing::info!("Received callback for job_id: {} with payload", job_id);
    tracing::debug!("Callback payload: {:?}", payload);

    // This endpoint can be used by external systems (like PitchLake)
    // to send notifications or updates about job processing
    // For now, we just log the callback and return success

    (
        StatusCode::OK,
        Json(serde_json::json!({
            "status": "received",
            "job_id": job_id,
            "timestamp": "2024-01-01T00:00:00Z"
        })),
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::handlers::fixtures::TestContext;
    use db_access::models::JobStatus;
    use serde_json::json;

    #[tokio::test]
    async fn test_get_job_result_success() {
        let ctx = TestContext::new().await;
        let job_id = "test_result_job";
        let sample_result = json!({"twap": 123.45});

        ctx.create_job_with_result(job_id, JobStatus::Completed, sample_result.clone())
            .await;

        let (status, Json(response)) = ctx.get_job_result(job_id).await;

        assert_eq!(status, StatusCode::OK);
        let job_result = response.unwrap();
        assert_eq!(job_result.job_id, job_id);
        assert_eq!(job_result.status, JobStatus::Completed);
        assert_eq!(job_result.result, Some(sample_result));
    }

    #[tokio::test]
    async fn test_batch_job_status() {
        let ctx = TestContext::new().await;

        // Create some test jobs
        ctx.create_job("job1", JobStatus::Pending).await;
        ctx.create_job("job2", JobStatus::Completed).await;

        let request = BatchJobStatusRequest {
            job_ids: vec![
                "job1".to_string(),
                "job2".to_string(),
                "nonexistent".to_string(),
            ],
        };

        let (status, Json(response)) = ctx.get_batch_job_status(request).await;

        assert_eq!(status, StatusCode::OK);
        assert_eq!(response.jobs.len(), 2);
        assert_eq!(response.not_found.len(), 1);
        assert!(response.not_found.contains(&"nonexistent".to_string()));
    }

    #[tokio::test]
    async fn test_batch_job_status_empty_request() {
        let ctx = TestContext::new().await;

        let request = BatchJobStatusRequest { job_ids: vec![] };

        let (status, Json(response)) = ctx.get_batch_job_status(request).await;

        assert_eq!(status, StatusCode::BAD_REQUEST);
        assert!(response.jobs.is_empty());
        assert!(response.not_found.is_empty());
    }

    #[tokio::test]
    async fn test_batch_job_status_too_many() {
        let ctx = TestContext::new().await;

        let request = BatchJobStatusRequest {
            job_ids: (0..101).map(|i| format!("job{}", i)).collect(),
        };

        let (status, Json(response)) = ctx.get_batch_job_status(request).await;

        assert_eq!(status, StatusCode::BAD_REQUEST);
        assert!(response.jobs.is_empty());
        assert_eq!(response.not_found.len(), 101);
    }
}
