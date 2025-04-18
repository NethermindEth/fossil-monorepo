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
    // Extract the API key from headers.
    if let Some(incoming_api_key) = headers.get("x-api-key") {
        if let Ok(api_key_str) = incoming_api_key.to_str() {
            tracing::info!("Attempting authentication with API key");
            tracing::debug!("Received API key: {}", api_key_str);
            // Access the database connection from the state.
            let matching_api_key =
                find_api_key(state.offchain_processor_db, api_key_str.to_string()).await;

            match matching_api_key {
                Ok(_) => {
                    tracing::info!("Authentication successful");
                    tracing::debug!("API key authenticated successfully");
                    Ok(next.run(request).await)
                }
                Err(err) => {
                    tracing::warn!("Authentication failed: Invalid API key");
                    tracing::debug!("Authentication failed: {:?}", err);

                    // Create a more informative error response
                    let error_detail = match err {
                        sqlx::Error::RowNotFound => "API key not found",
                        _ => "Database error occurred while validating API key",
                    };

                    let response_data = ErrorResponse {
                        error: format!("Authentication failed: {}", error_detail),
                    };

                    Ok((StatusCode::UNAUTHORIZED, Json(response_data)).into_response())
                }
            }
        } else {
            tracing::warn!("Authentication failed: Invalid API key format");

            let response_data = ErrorResponse {
                error: "Authentication failed: Invalid API key format".to_string(),
            };

            Ok((StatusCode::UNAUTHORIZED, Json(response_data)).into_response())
        }
    } else {
        tracing::warn!("Authentication failed: No API key provided");
        tracing::debug!("No API key found in headers");

        let response_data = ErrorResponse {
            error: "Authentication failed: No API key provided in headers".to_string(),
        };

        Ok((StatusCode::UNAUTHORIZED, Json(response_data)).into_response())
    }
}
