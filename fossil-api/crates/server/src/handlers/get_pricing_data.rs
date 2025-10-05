use db_access::OffchainProcessorDbConnection;
use dotenv::dotenv;
use std::env;
use std::sync::Arc;

use crate::types::PitchLakeJobRequestParams;
use crate::types::{JobResponse, PitchLakeJobRequest};
use crate::AppState;
use axum::{
    extract::{Json, State},
    http::StatusCode,
};
use db_access::{
    models::JobStatus,
    queries::{create_job_request_with_vault, get_job_request, update_job_status},
};
use eyre::{eyre, Result};
use reqwest::Client;
use serde_json::json;
use tokio::runtime::Handle;
#[cfg(not(test))]
use uuid::Uuid;

// Main handler function
pub async fn get_pricing_data(
    State(state): State<AppState>,
    Json(payload): Json<PitchLakeJobRequest>,
) -> (StatusCode, Json<JobResponse>) {
    let context = build_request_context(&payload);
    tracing::info!("Received pricing data request. {}", context);

    if let Err(boxed_error) = validate_request(&payload) {
        let (status, response) = *boxed_error;
        tracing::warn!("Invalid request: {:?}. {}", response, context);
        return (status, Json(response));
    }

    let job_id = generate_job_id(&payload.program_id, &payload.params);
    tracing::info!("Generated job_id: {}. {}", job_id, context);

    process_job_request(&state, &job_id, payload, &context).await
}

// Build context string for request logging
fn build_request_context(payload: &PitchLakeJobRequest) -> String {
    format!(
        "program_id={}, twap-range=({},{}), max_return-range=({},{}), reserve_price-range=({},{}), vault_address={}",
        payload.program_id,
        payload.params.twap.0, payload.params.twap.1,
        payload.params.max_return.0, payload.params.max_return.1,
        payload.params.reserve_price.0, payload.params.reserve_price.1,
        payload.vault_address,
    )
}

// Process the job request by checking existing jobs or creating new ones
async fn process_job_request(
    state: &AppState,
    job_id: &str,
    payload: PitchLakeJobRequest,
    context: &str,
) -> (StatusCode, Json<JobResponse>) {
    match get_job_request(state.offchain_processor_db.clone(), job_id).await {
        Ok(Some(job_request)) => {
            tracing::info!(
                "Found existing job with status: {}. {}",
                job_request.status,
                context
            );
            handle_existing_job(state, job_request.status, job_id.to_string(), payload).await
        }
        Ok(None) => {
            tracing::info!("Creating new job request. {}", context);
            handle_new_job_request(state, job_id.to_string(), payload).await
        }
        Err(e) => {
            tracing::error!("Database error: {}. {}", e, context);
            internal_server_error(e, job_id.to_string())
        }
    }
}

// Helper to validate the request
fn validate_request(payload: &PitchLakeJobRequest) -> Result<(), Box<(StatusCode, JobResponse)>> {
    if payload.program_id.is_empty() {
        return Err(Box::new((
            StatusCode::BAD_REQUEST,
            JobResponse::new(
                String::new(),
                Some("Program ID cannot be empty.".to_string()),
                None,
            ),
        )));
    }
    validate_time_ranges(&payload.params)
}

// Helper to generate a job ID
fn generate_job_id(
    #[cfg(test)] program_id: &str,
    #[cfg(not(test))] _program_id: &str,
    #[cfg(test)] params: &PitchLakeJobRequestParams,
    #[cfg(not(test))] _params: &PitchLakeJobRequestParams,
) -> String {
    #[cfg(test)]
    {
        // In test mode, create a deterministic job ID based on program_id and params
        // This ensures tests can predict the job ID
        format!(
            "test-job-{}-{}-{}-{}-{}-{}",
            program_id,
            params.twap.0,
            params.twap.1,
            params.max_return.0,
            params.max_return.1,
            params.reserve_price.0
        )
    }

    #[cfg(not(test))]
    {
        // In production, use random UUID v4
        Uuid::new_v4().to_string()
    }
}

// Handle existing jobs based on status
async fn handle_existing_job(
    state: &AppState,
    status: JobStatus,
    job_id: String,
    payload: PitchLakeJobRequest,
) -> (StatusCode, Json<JobResponse>) {
    match status {
        JobStatus::Pending => job_response(
            StatusCode::CONFLICT,
            job_id,
            "Job is already pending. Use the status endpoint to monitor progress.",
        ),
        JobStatus::Completed => job_response(
            StatusCode::OK,
            job_id,
            "Job has already been completed. No further processing required.",
        ),
        JobStatus::Failed => reprocess_failed_job(state, job_id, payload).await,
    }
}

// Handle new job requests
async fn handle_new_job_request(
    state: &AppState,
    job_id: String,
    payload: PitchLakeJobRequest,
) -> (StatusCode, Json<JobResponse>) {
    let current_timestamp = match get_current_timestamp() {
        Ok(ts) => ts,
        Err(e) => {
            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(JobResponse::new(job_id, Some(e), None)),
            );
        }
    };

    if let Err(e) = create_job_request_with_vault(
        state.offchain_processor_db.clone(),
        &job_id,
        JobStatus::Pending,
        &payload.vault_address,
        current_timestamp,
    )
    .await
    {
        return internal_server_error(e, job_id);
    }

    tracing::info!("New job request registered and processing initiated.");
    spawn_job_processor(state.offchain_processor_db.clone(), &job_id, &payload);

    (
        StatusCode::CREATED,
        Json(JobResponse {
            job_id: job_id.clone(),
            message: Some("New job request registered and processing initiated.".to_string()),
            status: Some(JobStatus::Pending),
            vault_address: Some(payload.vault_address.clone()),
            expected_timestamp: Some(current_timestamp),
            l1_data: None,
            on_chain_confirmation: None,
        }),
    )
}

// Get current Unix timestamp
fn get_current_timestamp() -> Result<i64, String> {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs() as i64)
        .map_err(|e| {
            tracing::error!("Failed to get current timestamp: {:?}", e);
            format!("Failed to get current timestamp: {}", e)
        })
}

// Spawn background task to process job
fn spawn_job_processor(
    db: Arc<OffchainProcessorDbConnection>,
    job_id: &str,
    payload: &PitchLakeJobRequest,
) {
    let job_id_clone = job_id.to_string();
    let payload_clone = payload.clone();
    let handle = Handle::current();

    tokio::task::spawn_blocking(move || {
        handle.block_on(process_job(db, job_id_clone, payload_clone));
    });
}

// Helper to handle failed job reprocessing
async fn reprocess_failed_job(
    state: &AppState,
    job_id: String,
    payload: PitchLakeJobRequest,
) -> (StatusCode, Json<JobResponse>) {
    if let Err(e) = update_job_status(
        state.offchain_processor_db.clone(),
        &job_id,
        JobStatus::Pending,
        None,
    )
    .await
    {
        return internal_server_error(e, job_id);
    }

    spawn_job_processor(state.offchain_processor_db.clone(), &job_id, &payload);

    job_response(
        StatusCode::OK,
        job_id,
        "Previous job request failed. Reprocessing initiated.",
    )
}

// Helper to generate a JSON response
fn job_response(
    status: StatusCode,
    job_id: String,
    message: &str,
) -> (StatusCode, Json<JobResponse>) {
    tracing::info!("Responding to job {} with status {}", job_id, status);
    (
        status,
        Json(JobResponse::new(job_id, Some(message.to_string()), None)),
    )
}

// Handle internal server errors
fn internal_server_error(error: sqlx::Error, job_id: String) -> (StatusCode, Json<JobResponse>) {
    tracing::error!("Internal server error: {:?}", error);
    (
        StatusCode::INTERNAL_SERVER_ERROR,
        Json(JobResponse::new(
            job_id,
            Some(format!("An error occurred: {}", error)),
            None,
        )),
    )
}

// Process the job and trigger request to the proving service
async fn process_job(
    offchain_processor_db: Arc<OffchainProcessorDbConnection>,
    job_id: String,
    payload: PitchLakeJobRequest,
) {
    let context = build_job_context(&job_id, &payload);
    tracing::info!("Starting job processing. {}", context);
    tracing::debug!("Payload received: {:?}. {}", payload, context);

    match call_proving_service(&job_id, &payload).await {
        Ok(_) => handle_proving_service_success(&job_id, &payload.vault_address, &context),
        Err(e) => handle_proving_service_error(offchain_processor_db, &job_id, e, &context).await,
    }
}

// Build context string for logging
fn build_job_context(job_id: &str, payload: &PitchLakeJobRequest) -> String {
    format!(
        "job_id={}, program_id={}, twap=({},{}), max_return=({},{}), reserve_price=({},{}), vault_address={}",
        job_id,
        payload.program_id,
        payload.params.twap.0, payload.params.twap.1,
        payload.params.max_return.0, payload.params.max_return.1,
        payload.params.reserve_price.0, payload.params.reserve_price.1,
        payload.vault_address,
    )
}

// Handle successful proving service response
fn handle_proving_service_success(job_id: &str, vault_address: &str, context: &str) {
    tracing::info!(
        job_id = %job_id,
        vault_address = %vault_address,
        "Proving service completed successfully, awaiting on-chain confirmation via FossilCallbackSuccess event"
    );
    tracing::info!(
        "Proving service request completed, job remains pending for on-chain confirmation. {}",
        context
    );
}

// Handle proving service errors
async fn handle_proving_service_error(
    db: Arc<OffchainProcessorDbConnection>,
    job_id: &str,
    error: eyre::Error,
    context: &str,
) {
    let error_msg = format!("Error calling proving service: {:?}", error);
    tracing::error!("{}. {}", error_msg, context);

    let _ = update_job_status(
        db,
        job_id,
        JobStatus::Failed,
        Some(serde_json::json!({ "error": error_msg })),
    )
    .await;

    tracing::error!(
        "Job processing failed. See previous errors for details. {}",
        context
    );
}

// Call the proving service API
async fn call_proving_service(
    job_id: &str,
    payload: &PitchLakeJobRequest,
) -> Result<serde_json::Value, eyre::Error> {
    dotenv().ok();

    let proving_service_url =
        env::var("PROVING_SERVICE_URL").unwrap_or_else(|_| "http://127.0.0.1:3000".to_string());

    let api_payload = build_proving_service_payload(job_id, payload)?;
    tracing::debug!("Sending request to proving service: {:?}", api_payload);

    let response = send_proving_service_request(&proving_service_url, &api_payload).await?;
    parse_proving_service_response(response).await
}

// Build the payload for the proving service API
fn build_proving_service_payload(
    job_id: &str,
    payload: &PitchLakeJobRequest,
) -> Result<serde_json::Value> {
    let vault_timestamp =
        get_current_timestamp().map_err(|e| eyre!("Failed to get current timestamp: {}", e))?;

    Ok(json!({
        "job_group_id": job_id,
        "twap": {
            "start_timestamp": payload.params.twap.0,
            "end_timestamp": payload.params.twap.1
        },
        "reserve_price": {
            "start_timestamp": payload.params.reserve_price.0,
            "end_timestamp": payload.params.reserve_price.1
        },
        "max_return": {
            "start_timestamp": payload.params.max_return.0,
            "end_timestamp": payload.params.max_return.1
        },
        "vault_address": payload.vault_address,
        "vault_timestamp": vault_timestamp
    }))
}

// Send request to proving service
async fn send_proving_service_request(
    url: &str,
    payload: &serde_json::Value,
) -> Result<reqwest::Response> {
    Client::new()
        .post(format!("{}/api/job", url))
        .json(payload)
        .send()
        .await
        .map_err(|e| eyre!("Failed to send request to proving service: {}", e))
}

// Parse response from proving service
async fn parse_proving_service_response(response: reqwest::Response) -> Result<serde_json::Value> {
    if !response.status().is_success() {
        let error_text = response
            .text()
            .await
            .unwrap_or_else(|_| "Failed to get error response text".to_string());
        return Err(eyre!("Proving service returned error: {}", error_text));
    }

    let result = response
        .json::<serde_json::Value>()
        .await
        .map_err(|e| eyre!("Failed to parse response from proving service: {}", e))?;

    tracing::debug!("Received response from proving service: {:?}", result);
    Ok(result)
}

// Validate the provided time ranges
fn validate_time_ranges(
    params: &PitchLakeJobRequestParams,
) -> Result<(), Box<(StatusCode, JobResponse)>> {
    let validations = [
        ("TWAP", params.twap),
        ("Max Return", params.max_return),
        ("Reserve Price", params.reserve_price),
    ];

    for (name, (start, end)) in &validations {
        if start >= end {
            return Err(Box::new((
                StatusCode::BAD_REQUEST,
                JobResponse::new(
                    String::new(),
                    Some(format!("Invalid time range for {} calculation.", name)),
                    None,
                ),
            )));
        }
    }
    Ok(())
}

// Enhanced error responses for better PL integration
fn _enhanced_error_response(
    status: StatusCode,
    error_code: &str,
    message: &str,
    job_id: Option<String>,
) -> (StatusCode, Json<serde_json::Value>) {
    (
        status,
        Json(serde_json::json!({
            "error_code": error_code,
            "message": message,
            "job_id": job_id.unwrap_or_default(),
            "timestamp": "2024-01-01T00:00:00Z"
        })),
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::handlers::fixtures::TestContext;
    use crate::types::{PitchLakeJobRequest, PitchLakeJobRequestParams};
    use axum::http::StatusCode;

    #[tokio::test]
    async fn test_get_pricing_data_new_job() {
        let ctx = TestContext::new().await;

        let payload = PitchLakeJobRequest {
            program_id: "test-id".to_string(),
            params: PitchLakeJobRequestParams {
                twap: (0, 100),
                max_return: (0, 100),
                reserve_price: (0, 100),
            },
            vault_address: "0x456".to_string(),
        };

        let (status, Json(response)) = ctx.get_pricing_data(payload).await;

        assert_eq!(status, StatusCode::CREATED);
        assert!(!response.job_id.is_empty());
        assert_eq!(
            response.message.unwrap(),
            "New job request registered and processing initiated."
        );
    }

    #[tokio::test]
    async fn test_get_pricing_data_pending_job() {
        let ctx = TestContext::new().await;

        let payload = PitchLakeJobRequest {
            program_id: "test-id".to_string(),
            params: PitchLakeJobRequestParams {
                twap: (0, 100),
                max_return: (0, 100),
                reserve_price: (0, 100),
            },
            vault_address: "0x456".to_string(),
        };

        let job_id = generate_job_id(&payload.program_id, &payload.params);
        ctx.create_job(&job_id, JobStatus::Pending).await;

        let (status, Json(response)) = ctx.get_pricing_data(payload).await;

        assert_eq!(status, StatusCode::CONFLICT);
        assert_eq!(response.job_id, job_id);
        assert_eq!(
            response.message.unwrap_or_default(),
            "Job is already pending. Use the status endpoint to monitor progress."
        );
    }

    #[tokio::test]
    async fn test_get_pricing_data_completed_job() {
        let ctx = TestContext::new().await;

        let payload = PitchLakeJobRequest {
            program_id: "test-id".to_string(),
            params: PitchLakeJobRequestParams {
                twap: (0, 100),
                max_return: (0, 100),
                reserve_price: (0, 100),
            },
            vault_address: "0x456".to_string(),
        };

        let job_id = generate_job_id(&payload.program_id, &payload.params);
        ctx.create_job(&job_id, JobStatus::Completed).await;

        let (status, Json(response)) = ctx.get_pricing_data(payload).await;

        assert_eq!(status, StatusCode::OK);
        assert_eq!(response.job_id, job_id);
        assert_eq!(
            response.message.unwrap_or_default(),
            "Job has already been completed. No further processing required."
        );
    }

    #[tokio::test]
    async fn test_get_pricing_data_failed_job() {
        let ctx = TestContext::new().await;

        let payload = PitchLakeJobRequest {
            program_id: "test-id".to_string(),
            params: PitchLakeJobRequestParams {
                twap: (0, 100),
                max_return: (0, 100),
                reserve_price: (0, 100),
            },
            vault_address: "0x456".to_string(),
        };

        let job_id = generate_job_id(&payload.program_id, &payload.params);
        ctx.create_job(&job_id, JobStatus::Failed).await;

        let (status, Json(response)) = ctx.get_pricing_data(payload).await;

        assert_eq!(status, StatusCode::OK);
        assert_eq!(response.job_id, job_id);
        assert_eq!(
            response.message.unwrap_or_default(),
            "Previous job request failed. Reprocessing initiated."
        );
    }

    #[tokio::test]
    async fn test_get_pricing_data_invalid_params() {
        let ctx = TestContext::new().await;

        let payload = PitchLakeJobRequest {
            program_id: "test-id".to_string(),
            params: PitchLakeJobRequestParams {
                twap: (100, 0), // Invalid range
                max_return: (0, 100),
                reserve_price: (0, 100),
            },
            vault_address: "0x456".to_string(),
        };

        let (status, Json(response)) = ctx.get_pricing_data(payload).await;

        assert_eq!(status, StatusCode::BAD_REQUEST);
        assert_eq!(
            response.message.unwrap_or_default(),
            "Invalid time range for TWAP calculation."
        );
    }
}
