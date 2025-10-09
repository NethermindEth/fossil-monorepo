use crate::{types::ErrorResponse, AppState};
use axum::{
    extract::State,
    http::{HeaderMap, Request, StatusCode},
    middleware::Next,
    response::{IntoResponse, Response},
    Json,
};
use db_access::auth::find_api_key;

/// A simple API key authentication middleware.
/// TODO: Use the more comprehensive `tower_http` auth middleware.
pub async fn simple_apikey_auth(
    State(state): State<AppState>,
    headers: HeaderMap,
    request: Request<axum::body::Body>,
    next: Next,
) -> Result<Response, StatusCode> {
    let api_key_str = match extract_api_key(&headers) {
        Ok(key) => key,
        Err(response) => return Ok(*response),
    };

    match validate_api_key(&state, api_key_str).await {
        Ok(()) => {
            tracing::info!("Authentication successful");
            tracing::debug!("API key authenticated successfully");
            Ok(next.run(request).await)
        }
        Err(response) => Ok(*response),
    }
}

/// Extracts and validates the API key from request headers.
fn extract_api_key(headers: &HeaderMap) -> Result<&str, Box<Response>> {
    let incoming_api_key = headers
        .get("x-api-key")
        .ok_or_else(|| Box::new(create_auth_error("No API key provided in headers")))?;

    incoming_api_key.to_str().map_err(|_| {
        tracing::warn!("Authentication failed: Invalid API key format");
        Box::new(create_auth_error("Invalid API key format"))
    })
}

/// Validates the API key against the database.
async fn validate_api_key(state: &AppState, api_key_str: &str) -> Result<(), Box<Response>> {
    tracing::info!("Attempting authentication with API key");
    tracing::debug!("Received API key: {}", api_key_str);

    find_api_key(state.offchain_processor_db.clone(), api_key_str.to_string())
        .await
        .map_err(|err| {
            tracing::warn!("Authentication failed: Invalid API key");
            tracing::debug!("Authentication failed: {:?}", err);

            let error_detail = match err {
                sqlx::Error::RowNotFound => "API key not found",
                _ => "Database error occurred while validating API key",
            };

            Box::new(create_auth_error(error_detail))
        })?;

    Ok(())
}

/// Creates a standardized authentication error response.
fn create_auth_error(detail: &str) -> Response {
    tracing::warn!("Authentication failed: {}", detail);
    tracing::debug!("No API key found in headers");

    let response_data = ErrorResponse {
        error: format!("Authentication failed: {}", detail),
    };

    (StatusCode::UNAUTHORIZED, Json(response_data)).into_response()
}
