# proving-service Crate

## Overview

The `proving-service` crate provides an HTTP API server for submitting proof generation jobs to the Fossil proving infrastructure. It acts as the entry point for job requests, accepting HTTP POST requests with time-range specifications for TWAP (Time-Weighted Average Price), reserve price, and max return calculations, then dispatching these jobs to AWS SQS for asynchronous processing.

This service is built with Axum and integrates with the `message-handler` crate for job dispatching and queue management.

**Key Responsibilities:**
- Accept HTTP job submission requests via REST API
- Validate and structure job requests with multiple time ranges
- Dispatch jobs to AWS SQS queue for processing
- Provide health check endpoint for service monitoring
- Handle graceful shutdown on SIGINT

## Crate Structure

```
proving-service/
├── Cargo.toml                    # Crate dependencies and configuration
└── src/
    ├── main.rs                   # Application entry point and server setup
    ├── lib.rs                    # Library exports and router creation
    ├── routes.rs                 # Router configuration and health check
    └── handlers/
        ├── mod.rs                # Handler module exports
        └── jobs.rs               # Job submission handler logic
```

### Module Organization

- **`main.rs`**: Initializes logging, loads environment configuration, sets up AWS SDK, creates the HTTP server, and handles graceful shutdown
- **`lib.rs`**: Provides the public `create_router` function for router initialization
- **`routes.rs`**: Defines the Axum router with endpoint mappings and the health check handler
- **`handlers/jobs.rs`**: Implements the job submission endpoint with request validation and job dispatching

## HTTP Server

### Server Setup

The HTTP server is built using Axum and runs on port 3001 by default. Here's the initialization flow from `main.rs`:

```rust
#[tokio::main]
async fn main() -> Result<()> {
    // Initialize tracing with INFO level default
    let subscriber = FmtSubscriber::builder()
        .with_max_level(Level::INFO)
        .finish();
    tracing::subscriber::set_global_default(subscriber)
        .map_err(|e| eyre::eyre!("setting default subscriber failed: {}", e))?;

    info!("Starting Fossil Prover HTTP Service");

    // Load .env file
    dotenv::dotenv().ok();

    // Get the queue URL from environment variable
    let queue_url = env::var("SQS_QUEUE_URL")
        .unwrap_or_else(|_| "http://localhost:4566/000000000000/fossilQueue".to_string());
    info!("Using SQS Queue URL: {}", queue_url);

    // Load AWS SDK config from environment variables
    let config = defaults(BehaviorVersion::latest()).load().await;
    info!("AWS configuration loaded");

    let queue = Arc::new(SqsMessageQueue::new(queue_url, config));

    // Create and start the HTTP server
    let app = create_router(queue).await;
    let addr = std::net::SocketAddr::from(([0, 0, 0, 0], 3001));
    info!("Starting HTTP server on {}", addr);

    let server = axum::serve(tokio::net::TcpListener::bind(addr).await?, app);

    // Handle Ctrl+C for graceful shutdown
    let handle = tokio::spawn(async move { server.await });

    info!("Waiting for shutdown signal...");
    signal::ctrl_c().await?;
    info!("Received shutdown signal, initiating graceful shutdown...");

    // Shutdown the HTTP server
    info!("Shutting down HTTP server...");
    handle.abort();

    info!("Shutdown complete");
    Ok(())
}
```

### Router Configuration

The router is configured in `routes.rs` with two endpoints:

```rust
pub async fn create_router(queue: Arc<SqsMessageQueue>) -> Router {
    info!("Setting up HTTP router");

    let dispatcher = Arc::new(JobDispatcher::new(queue));

    Router::new()
        .route("/health", get(health_check))
        .route("/api/job", post(handle_job_request))
        .with_state(dispatcher)
}
```

The `JobDispatcher` is created from the SQS queue and passed as shared state to all handlers.

## Endpoints

### GET /health

Health check endpoint for service monitoring and load balancer integration.

**Request:**
```bash
curl http://localhost:3001/health
```

**Response:**
```json
{
  "status": "healthy",
  "service": "proving-service"
}
```

**Status Code:** 200 OK

**Implementation:**
```rust
async fn health_check() -> Json<Value> {
    Json(json!({
        "status": "healthy",
        "service": "proving-service"
    }))
}
```

### POST /api/job

Submit a proof generation job with time ranges for TWAP, reserve price, and max return calculations.

**Request Body:**
```json
{
  "job_group_id": "unique-job-group-identifier",
  "twap": {
    "start_timestamp": 1609459200,
    "end_timestamp": 1609545600
  },
  "reserve_price": {
    "start_timestamp": 1609459200,
    "end_timestamp": 1609545600
  },
  "max_return": {
    "start_timestamp": 1609459200,
    "end_timestamp": 1609545600
  },
  "vault_address": "0x1234567890abcdef1234567890abcdef12345678",
  "vault_timestamp": 1609459200
}
```

**Request Fields:**
- `job_group_id` (string, required): Unique identifier for the job group, also used as the job ID
- `twap` (TimeRange, required): Time range for TWAP calculation
- `reserve_price` (TimeRange, required): Time range for reserve price calculation
- `max_return` (TimeRange, required): Time range for max return calculation
- `vault_address` (string, optional): Ethereum address of the vault contract
- `vault_timestamp` (i64, optional): Timestamp for vault state snapshot

**Success Response:**
```json
{
  "status": "success",
  "message": "Job dispatched successfully",
  "job_group_id": "unique-job-group-identifier"
}
```

**Status Code:** 200 OK

**Error Response:**
```json
{
  "status": "error",
  "message": "Failed to dispatch job: <error details>",
  "job_group_id": "unique-job-group-identifier"
}
```

**Status Code:** 500 Internal Server Error

**Example cURL Request:**
```bash
curl -X POST http://localhost:3001/api/job \
  -H "Content-Type: application/json" \
  -d '{
    "job_group_id": "test-job-001",
    "twap": {
      "start_timestamp": 1609459200,
      "end_timestamp": 1609545600
    },
    "reserve_price": {
      "start_timestamp": 1609459200,
      "end_timestamp": 1609545600
    },
    "max_return": {
      "start_timestamp": 1609459200,
      "end_timestamp": 1609545600
    },
    "vault_address": "0x1234567890abcdef1234567890abcdef12345678",
    "vault_timestamp": 1609459200
  }'
```

## Job Dispatcher

The job dispatcher is responsible for converting HTTP requests into queue messages and sending them to AWS SQS.

### Flow

1. **Request Reception**: The `handle_job_request` handler receives the HTTP request
2. **Job Creation**: The request is transformed into a `Job::RequestProof` variant
3. **Serialization**: The job is serialized to JSON
4. **Queue Dispatch**: The serialized job is sent to SQS via the `JobDispatcher`

### Implementation Details

From `handlers/jobs.rs`:

```rust
pub async fn handle_job_request(
    State(dispatcher): State<Arc<JobDispatcher<SqsMessageQueue>>>,
    Json(request): Json<JobRequest>,
) -> impl IntoResponse {
    info!("Received job request for group: {}", request.job_group_id);

    // Create a single job with ranges for all three components
    // Use job_group_id as the job_id to simplify identification
    let job_id = request.job_group_id.clone();

    let combined_job = Job::RequestProof(RequestProof {
        job_id: job_id.clone(),
        start_timestamp: request.twap.start_timestamp,
        end_timestamp: request.twap.end_timestamp,
        job_group_id: Some(request.job_group_id.clone()),
        twap_start_timestamp: Some(request.twap.start_timestamp),
        twap_end_timestamp: Some(request.twap.end_timestamp),
        reserve_price_start_timestamp: Some(request.reserve_price.start_timestamp),
        reserve_price_end_timestamp: Some(request.reserve_price.end_timestamp),
        max_return_start_timestamp: Some(request.max_return.start_timestamp),
        max_return_end_timestamp: Some(request.max_return.end_timestamp),
        vault_address: request.vault_address,
        vault_timestamp: request.vault_timestamp,
    });

    match dispatcher.dispatch_job(combined_job).await {
        Ok(_) => {
            info!("Successfully dispatched job with ID: {}", job_id);
            (
                StatusCode::OK,
                Json(Response {
                    status: "success".to_string(),
                    message: "Job dispatched successfully".to_string(),
                    job_group_id: request.job_group_id,
                }),
            )
        }
        Err(e) => {
            error!("Failed to dispatch job with ID: {}. Error: {}", job_id, e);
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(Response {
                    status: "error".to_string(),
                    message: format!("Failed to dispatch job: {e}"),
                    job_group_id: request.job_group_id,
                }),
            )
        }
    }
}
```

The `JobDispatcher` (from `message-handler` crate) serializes the job and sends it to SQS:

```rust
pub async fn dispatch_job(&self, job: Job) -> Result<()> {
    let message_body = serde_json::to_string(&job)?;

    self.queue
        .send_message(message_body)
        .await
        .map_err(|e| eyre::eyre!(e))?;
    Ok(())
}
```

## Request/Response Types

### TimeRange

Represents a time range with start and end timestamps.

```rust
#[derive(Debug, Deserialize)]
pub struct TimeRange {
    start_timestamp: i64,
    end_timestamp: i64,
}
```

### JobRequest

The HTTP request body structure for job submission.

```rust
#[derive(Debug, Deserialize)]
pub struct JobRequest {
    job_group_id: String,
    twap: TimeRange,
    reserve_price: TimeRange,
    max_return: TimeRange,
    vault_address: Option<String>,
    vault_timestamp: Option<i64>,
}
```

### Response

The HTTP response structure returned to clients.

```rust
#[derive(Debug, Serialize)]
pub struct Response {
    status: String,
    message: String,
    job_group_id: String,
}
```

### Job (from message-handler)

The internal job representation sent to the queue.

```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RequestProof {
    pub job_id: String,
    pub job_group_id: Option<String>,
    pub start_timestamp: i64,
    pub end_timestamp: i64,
    pub twap_start_timestamp: Option<i64>,
    pub twap_end_timestamp: Option<i64>,
    pub reserve_price_start_timestamp: Option<i64>,
    pub reserve_price_end_timestamp: Option<i64>,
    pub max_return_start_timestamp: Option<i64>,
    pub max_return_end_timestamp: Option<i64>,
    pub vault_address: Option<String>,
    pub vault_timestamp: Option<i64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(untagged)]
pub enum Job {
    RequestProof(RequestProof),
    ProofGenerated(Box<ProofGenerated>),
}
```

## Error Handling

The service uses a combination of Axum's response types and the `eyre` error handling library.

### Error Types

1. **Job Dispatch Errors**: When the job fails to be sent to SQS
   - Returns HTTP 500 with error message in response body
   - Logged as errors with full context

2. **Serialization Errors**: When job cannot be serialized to JSON
   - Handled by `eyre::Result` in dispatcher
   - Propagated as dispatch errors

3. **Queue Errors**: When SQS operations fail
   - Converted to `eyre::Error` via `map_err`
   - Returned as HTTP 500 errors

### Error Response Example

```json
{
  "status": "error",
  "message": "Failed to dispatch job: SendError(\"SQS connection timeout\")",
  "job_group_id": "failed-job-123"
}
```

### Logging

The service uses `tracing` for structured logging:

```rust
// Initialization in main.rs
let subscriber = FmtSubscriber::builder()
    .with_max_level(Level::INFO)
    .finish();
tracing::subscriber::set_global_default(subscriber)?;

// Usage in handlers
info!("Received job request for group: {}", request.job_group_id);
error!("Failed to dispatch job with ID: {}. Error: {}", job_id, e);
```

## Configuration

### Environment Variables

The service requires the following environment variables:

| Variable | Description | Default | Required |
|----------|-------------|---------|----------|
| `SQS_QUEUE_URL` | AWS SQS queue URL for job messages | `http://localhost:4566/000000000000/fossilQueue` | No |
| `AWS_REGION` | AWS region for SQS | N/A | Yes (via AWS SDK) |
| `AWS_ACCESS_KEY_ID` | AWS access key | N/A | Yes (for production) |
| `AWS_SECRET_ACCESS_KEY` | AWS secret key | N/A | Yes (for production) |
| `AWS_ENDPOINT_URL` | Custom AWS endpoint (for LocalStack) | N/A | No |

### Environment File

Configuration is loaded from `.env` files. The service uses the centralized environment management system:

1. Root level: `.env.local`
2. Service level overrides: `proving-service/.env.local`

**Example Configuration (from root `.env.example`):**
```bash
# AWS SQS Configuration
SQS_QUEUE_URL=http://localhost:4567/000000000000/fossilQueue
AWS_REGION=us-east-1
AWS_ENDPOINT_URL=http://localhost:4566  # For LocalStack
```

### Development Setup

For local development with LocalStack:

```bash
# Start LocalStack and PostgreSQL
make dev-services

# In another terminal, run the service
cd proving-service
cargo run --bin proving-service
```

## Usage Examples

### Starting the Server

**Development Mode:**
```bash
cd proving-service
cargo run --bin proving-service
```

**Production Mode:**
```bash
cd proving-service
cargo build --release
./target/release/proving-service
```

**Expected Output:**
```
2025-10-06T10:30:00.123456Z  INFO proving_service: Starting Fossil Prover HTTP Service
2025-10-06T10:30:00.234567Z  INFO proving_service: Using SQS Queue URL: http://localhost:4566/000000000000/fossilQueue
2025-10-06T10:30:00.345678Z  INFO proving_service: AWS configuration loaded
2025-10-06T10:30:00.456789Z  INFO proving_service: Setting up HTTP router
2025-10-06T10:30:00.567890Z  INFO proving_service: Starting HTTP server on 0.0.0.0:3001
2025-10-06T10:30:00.678901Z  INFO proving_service: Waiting for shutdown signal...
```

### Making Requests

**Health Check:**
```bash
curl http://localhost:3001/health
```

**Submit a Job:**
```bash
curl -X POST http://localhost:3001/api/job \
  -H "Content-Type: application/json" \
  -d '{
    "job_group_id": "production-job-001",
    "twap": {
      "start_timestamp": 1609459200,
      "end_timestamp": 1609545600
    },
    "reserve_price": {
      "start_timestamp": 1609459200,
      "end_timestamp": 1609545600
    },
    "max_return": {
      "start_timestamp": 1609459200,
      "end_timestamp": 1609545600
    },
    "vault_address": "0x1234567890abcdef1234567890abcdef12345678",
    "vault_timestamp": 1609459200
  }'
```

**Submit a Job Without Vault Information:**
```bash
curl -X POST http://localhost:3001/api/job \
  -H "Content-Type: application/json" \
  -d '{
    "job_group_id": "simple-job-002",
    "twap": {
      "start_timestamp": 1609459200,
      "end_timestamp": 1609545600
    },
    "reserve_price": {
      "start_timestamp": 1609459200,
      "end_timestamp": 1609545600
    },
    "max_return": {
      "start_timestamp": 1609459200,
      "end_timestamp": 1609545600
    }
  }'
```

**Using httpie (alternative to curl):**
```bash
http POST http://localhost:3001/api/job \
  job_group_id="test-job-003" \
  twap:='{"start_timestamp": 1609459200, "end_timestamp": 1609545600}' \
  reserve_price:='{"start_timestamp": 1609459200, "end_timestamp": 1609545600}' \
  max_return:='{"start_timestamp": 1609459200, "end_timestamp": 1609545600}'
```

### Graceful Shutdown

The service handles graceful shutdown on SIGINT (Ctrl+C):

```
^C2025-10-06T10:35:00.123456Z  INFO proving_service: Received shutdown signal, initiating graceful shutdown...
2025-10-06T10:35:00.234567Z  INFO proving_service: Shutting down HTTP server...
2025-10-06T10:35:00.345678Z  INFO proving_service: Shutdown complete
```

## Testing

### Test Structure

The crate includes comprehensive unit tests for all components:

1. **Router Tests** (`routes.rs`):
   - Router creation and configuration
   - Mock queue integration

2. **Handler Tests** (`handlers/jobs.rs`):
   - Job request handling with success scenarios
   - Job request handling with failure scenarios
   - Request deserialization
   - Response serialization

3. **Integration Tests** (`lib.rs`):
   - End-to-end router creation

### Running Tests

**Run all tests:**
```bash
cd proving-service
cargo test
```

**Run specific test:**
```bash
cargo test test_handle_job_request_success
```

**Run with output:**
```bash
cargo test -- --nocapture
```

### Test Examples

**Testing Request Deserialization:**
```rust
#[tokio::test]
async fn test_job_request_deserialization() {
    let json = r#"{
        "job_group_id": "test-group",
        "twap": {"start_timestamp": 1000, "end_timestamp": 2000},
        "reserve_price": {"start_timestamp": 3000, "end_timestamp": 4000},
        "max_return": {"start_timestamp": 5000, "end_timestamp": 6000}
    }"#;

    let request: JobRequest = serde_json::from_str(json).unwrap();

    assert_eq!(request.job_group_id, "test-group");
    assert_eq!(request.twap.start_timestamp, 1000);
    assert_eq!(request.twap.end_timestamp, 2000);
}
```

**Testing Handler Success:**
```rust
#[tokio::test]
async fn test_handle_job_request_success() {
    let mock_queue = MockQueue::new(false);
    let dispatcher = TestJobDispatcher::new(mock_queue);
    let dispatcher = Arc::new(dispatcher);

    let request = JobRequest {
        job_group_id: "test-group-123".to_string(),
        twap: TimeRange {
            start_timestamp: 1000,
            end_timestamp: 2000,
        },
        reserve_price: TimeRange {
            start_timestamp: 1000,
            end_timestamp: 2000,
        },
        max_return: TimeRange {
            start_timestamp: 1000,
            end_timestamp: 2000,
        },
        vault_address: None,
        vault_timestamp: None,
    };

    let response = handle_job_request_test(dispatcher, request).await;

    assert_eq!(response.0, StatusCode::OK);
    assert_eq!(response.1.status, "success");
}
```

### Mock Implementation

Tests use a `MockQueue` implementation to avoid AWS dependencies:

```rust
#[derive(Debug, Clone)]
struct MockQueue {
    should_fail: bool,
}

#[async_trait]
impl Queue for MockQueue {
    async fn send_message(&self, _message: String) -> Result<(), QueueError> {
        if self.should_fail {
            Err(QueueError::SendError("Mock send error".to_string()))
        } else {
            Ok(())
        }
    }
    // ... other methods
}
```

## Next Steps

- **[message-handler Crate](../message-handler/message-handler.md)**: Learn about job dispatching and SQS integration
- **[db Crate](../db/db.md)**: Understand job persistence and database models
- **[API Documentation](../../api/proving-service-api.md)**: Complete API reference
- **[Architecture Overview](../../architecture/proving-service-architecture.md)**: System design and flow

## Related Components

This crate integrates with:

1. **message-handler**: Provides `JobDispatcher` and `Job` types
2. **AWS SQS**: Message queue for job distribution
3. **LocalStack**: Development environment for AWS services
4. **Axum**: HTTP framework for REST API
5. **tokio**: Async runtime

## Dependencies

Key dependencies (from `Cargo.toml`):

```toml
[dependencies]
# Core dependencies
eyre = { workspace = true }              # Error handling
tokio = { workspace = true }             # Async runtime
serde = { workspace = true }             # Serialization
serde_json = { workspace = true }        # JSON support
tracing = { workspace = true }           # Logging
tracing-subscriber = { workspace = true } # Logging subscriber
dotenv = { workspace = true }            # Environment loading

# AWS
aws-config = { workspace = true }        # AWS SDK configuration
aws-sdk-sqs = { workspace = true }       # SQS client

# Web framework
axum = { workspace = true }              # HTTP server

# Internal dependencies
message-handler = { path = "../message-handler" }
```
