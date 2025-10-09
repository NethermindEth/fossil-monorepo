# Server Crate

The `server` crate provides the HTTP server and REST API for the Fossil API service. It handles pricing data requests, job status queries, API key management, and StarkNet event monitoring for on-chain confirmations.

## Overview

The server crate is the main entry point for the Fossil API, providing:
- RESTful HTTP endpoints for pricing data and job management
- API key authentication middleware
- Background StarkNet event monitoring for vault callbacks
- Integration with the proving service
- CORS configuration and HTTP tracing

**Key Dependencies:**
- `axum` - Web framework
- `tower-http` - HTTP middleware (CORS, tracing, auth)
- `db-access` - Database access layer
- `sqlx` - PostgreSQL database driver
- `reqwest` - HTTP client for proving service calls

## Crate Structure

```
server/
├── src/
│   ├── lib.rs                    # Application setup and router configuration
│   ├── main.rs                   # Server startup and event monitor initialization
│   ├── types.rs                  # Request/response types
│   ├── handlers/
│   │   ├── mod.rs
│   │   ├── health_check.rs       # Health check endpoint
│   │   ├── api_key.rs            # API key creation
│   │   ├── get_pricing_data.rs   # Main pricing data handler
│   │   ├── job_status.rs         # Job status queries
│   │   ├── pl_integration.rs     # PitchLake integration endpoints
│   │   └── fixtures/
│   │       └── mod.rs            # Test fixtures
│   ├── middlewares/
│   │   ├── mod.rs
│   │   └── auth.rs               # API key authentication middleware
│   ├── starknet_provider.rs      # StarkNet RPC client
│   ├── event_monitor.rs          # Vault event monitoring logic
│   ├── bin/
│   │   └── event_monitor.rs      # Standalone event monitor binary
│   └── scripts/
│       └── create_api_key.rs     # API key creation script
└── Cargo.toml
```

## Application Setup

### Router Configuration

The application uses Axum for routing and middleware. Routes are divided into secured (requiring API key) and public routes:

```rust
pub async fn create_app(offchain_processor_db: Arc<OffchainProcessorDbConnection>) -> Router {
    let app_state = AppState {
        offchain_processor_db,
    };

    // Secured routes requiring API key authentication
    let secured_routes = Router::new()
        .route("/pricing_data", post(handlers::get_pricing_data::get_pricing_data))
        .route("/job_result/{job_id}", get(handlers::pl_integration::get_job_result))
        .route("/batch_job_status", post(handlers::pl_integration::get_batch_job_status))
        .layer(from_fn_with_state(app_state.clone(), simple_apikey_auth));

    // Public routes (no authentication required)
    let public_routes = Router::new()
        .route("/health", get(handlers::health_check::health_check))
        .route("/api_key", post(handlers::api_key::create_api_key))
        .route("/job_status/{job_id}", get(handlers::job_status::get_job_status))
        .route("/webhook/{job_id}", post(handlers::pl_integration::job_callback_webhook))
        .layer(CorsLayer::permissive());

    Router::new()
        .merge(secured_routes)
        .merge(public_routes)
        .layer(TraceLayer::new_for_http())
        .layer(cors_layer)
        .with_state(app_state)
}
```

### CORS Configuration

CORS is configured using environment variables:

```rust
let allowed_origins = std::env::var("ALLOWED_ORIGINS")
    .unwrap_or_default()
    .split(',')
    .filter(|s| !s.is_empty())
    .filter_map(|s| s.parse().ok())
    .collect::<Vec<_>>();

let cors_layer = CorsLayer::new()
    .allow_origin(AllowOrigin::list(allowed_origins))
    .allow_methods(AllowMethods::any())
    .allow_headers(AllowHeaders::any())
    .max_age(Duration::from_secs(3600));
```

## Application State

The `AppState` struct contains shared application state accessible to all handlers:

```rust
#[derive(Clone)]
pub struct AppState {
    pub offchain_processor_db: Arc<OffchainProcessorDbConnection>,
}
```

This state is passed to handlers via Axum's state extraction mechanism.

## Handlers

### Health Check (`/health`)

Simple health check endpoint returning "OK". Currently implements basic liveness checking.

**Path:** `/health`
**Method:** `GET`
**Authentication:** None
**Response:** `200 OK` with body `"OK"`

```rust
pub async fn health_check() -> &'static str {
    // TODO: should actually also check db connection and more
    // right now its essentially a liveness check.
    "OK"
}
```

### Create API Key (`/api_key`)

Creates a new API key for authentication.

**Path:** `/api_key`
**Method:** `POST`
**Authentication:** None
**Request Body:**
```json
{
    "name": "my-api-key"
}
```

**Response:**
```json
{
    "api_key": "550e8400-e29b-41d4-a716-446655440000"
}
```

**Handler Implementation:**
```rust
pub async fn create_api_key(
    State(state): State<AppState>,
    Json(payload): Json<ApiKeyRequest>,
) -> Result<Json<ApiKeyResponse>, StatusCode> {
    let api_key = Uuid::new_v4().to_string();

    if let Err(e) = add_api_key(state.offchain_processor_db, api_key.clone(), payload.name).await {
        tracing::error!("Failed to store API key: {:?}", e);
        return Err(StatusCode::INTERNAL_SERVER_ERROR);
    }

    Ok(Json(ApiKeyResponse { api_key }))
}
```

### Get Pricing Data (`/pricing_data`)

Main endpoint for requesting pricing data calculations. Creates a new job, dispatches it to the proving service, and returns the job ID.

**Path:** `/pricing_data`
**Method:** `POST`
**Authentication:** Required (API key)
**Request Body:**
```json
{
    "program_id": "fossil-twap-v1",
    "params": {
        "twap": [1704067200, 1704153600],
        "max_return": [1704067200, 1704153600],
        "reserve_price": [1704067200, 1704153600]
    },
    "vault_address": "0x1234567890abcdef"
}
```

**Response (New Job - 201 Created):**
```json
{
    "job_id": "550e8400-e29b-41d4-a716-446655440000",
    "message": "New job request registered and processing initiated.",
    "status": "Pending",
    "vault_address": "0x1234567890abcdef",
    "expected_timestamp": 1704153600
}
```

**Response (Existing Pending Job - 409 Conflict):**
```json
{
    "job_id": "550e8400-e29b-41d4-a716-446655440000",
    "message": "Job is already pending. Use the status endpoint to monitor progress."
}
```

**Handler Flow:**
1. Validates request parameters (program_id, time ranges)
2. Generates job ID (deterministic in tests, random UUID in production)
3. Checks for existing job with same ID
4. Creates database record with `Pending` status
5. Spawns background task to call proving service
6. Returns job ID immediately

**Key Functions:**
```rust
// Main handler
pub async fn get_pricing_data(
    State(state): State<AppState>,
    Json(payload): Json<PitchLakeJobRequest>,
) -> (StatusCode, Json<JobResponse>)

// Validation
fn validate_request(payload: &PitchLakeJobRequest) -> Result<(), Box<(StatusCode, JobResponse)>>

// Job processing
async fn process_job(
    offchain_processor_db: Arc<OffchainProcessorDbConnection>,
    job_id: String,
    payload: PitchLakeJobRequest,
)

// Proving service integration
async fn call_proving_service(
    job_id: &str,
    payload: &PitchLakeJobRequest,
) -> Result<serde_json::Value, eyre::Error>
```

### Get Job Status (`/job_status/{job_id}`)

Query the status of a submitted job.

**Path:** `/job_status/{job_id}`
**Method:** `GET`
**Authentication:** None
**Response (Job Found - 200 OK):**
```json
{
    "job_id": "550e8400-e29b-41d4-a716-446655440000",
    "message": "Job is pending on-chain confirmation via FossilCallbackSuccess event",
    "status": "Pending",
    "vault_address": "0x1234567890abcdef",
    "expected_timestamp": 1704153600,
    "l1_data": null,
    "on_chain_confirmation": null
}
```

**Response (Completed Job):**
```json
{
    "job_id": "550e8400-e29b-41d4-a716-446655440000",
    "message": "Job completed with on-chain confirmation",
    "status": "Completed",
    "vault_address": "0x1234567890abcdef",
    "expected_timestamp": 1704153600,
    "l1_data": {
        "twap": "0x1234",
        "max_return": "0x5678",
        "reserve_price": "0x9abc"
    },
    "on_chain_confirmation": {
        "block_number": 12345,
        "transaction_hash": "0xdef...",
        "event_timestamp": 1704153600
    }
}
```

**Response (Job Not Found - 404 Not Found):**
```json
{
    "error": "Job not found"
}
```

### Get Job Result (`/job_result/{job_id}`)

Enhanced endpoint for retrieving detailed job results with timestamps.

**Path:** `/job_result/{job_id}`
**Method:** `GET`
**Authentication:** Required (API key)
**Response:**
```json
{
    "job_id": "550e8400-e29b-41d4-a716-446655440000",
    "status": "Completed",
    "result": {
        "twap": 1234.56,
        "max_return": 789.01,
        "reserve_price": 2345.67
    },
    "created_at": "2024-01-01T00:00:00Z",
    "completed_at": "2024-01-01T00:05:00Z"
}
```

### Batch Job Status (`/batch_job_status`)

Query multiple jobs efficiently in a single request.

**Path:** `/batch_job_status`
**Method:** `POST`
**Authentication:** Required (API key)
**Request Body:**
```json
{
    "job_ids": [
        "job-1",
        "job-2",
        "job-3"
    ]
}
```

**Response:**
```json
{
    "jobs": [
        {
            "job_id": "job-1",
            "status": "Completed",
            "result": {...},
            "created_at": "2024-01-01T00:00:00Z",
            "completed_at": "2024-01-01T00:05:00Z"
        },
        {
            "job_id": "job-2",
            "status": "Pending",
            "result": null,
            "created_at": "2024-01-01T00:01:00Z",
            "completed_at": null
        }
    ],
    "not_found": ["job-3"]
}
```

**Validation:**
- Empty request: Returns `400 Bad Request`
- More than 100 jobs: Returns `400 Bad Request`

### Webhook Callback (`/webhook/{job_id}`)

Webhook endpoint for external notifications (e.g., from PitchLake).

**Path:** `/webhook/{job_id}`
**Method:** `POST`
**Authentication:** None
**Request Body:** Any JSON payload
**Response:**
```json
{
    "status": "received",
    "job_id": "550e8400-e29b-41d4-a716-446655440000",
    "timestamp": "2024-01-01T00:00:00Z"
}
```

## Middleware

### Authentication Middleware

The authentication middleware validates API keys for secured endpoints.

**Implementation:**
```rust
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
            Ok(next.run(request).await)
        }
        Err(response) => Ok(*response),
    }
}
```

**Key Extraction:**
```rust
fn extract_api_key(headers: &HeaderMap) -> Result<&str, Box<Response>> {
    let incoming_api_key = headers
        .get("x-api-key")
        .ok_or_else(|| Box::new(create_auth_error("No API key provided in headers")))?;

    incoming_api_key.to_str().map_err(|_| {
        tracing::warn!("Authentication failed: Invalid API key format");
        Box::new(create_auth_error("Invalid API key format"))
    })
}
```

**Usage:**
- API key must be provided in the `x-api-key` header
- Returns `401 Unauthorized` if key is missing or invalid
- Queries database to verify key exists and is valid

## Types

### Request Types

**`PitchLakeJobRequest`** - Main pricing data request:
```rust
#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct PitchLakeJobRequest {
    pub program_id: String,
    pub params: PitchLakeJobRequestParams,
    pub vault_address: String,
}
```

**`PitchLakeJobRequestParams`** - Time ranges for calculations:
```rust
#[derive(Debug, Default, Deserialize, Serialize, Clone)]
pub struct PitchLakeJobRequestParams {
    pub twap: (i64, i64),           // (start_timestamp, end_timestamp)
    pub max_return: (i64, i64),
    pub reserve_price: (i64, i64),
}
```

**`BatchJobStatusRequest`** - Batch query request:
```rust
#[derive(Debug, Serialize, Deserialize)]
pub struct BatchJobStatusRequest {
    pub job_ids: Vec<String>,
}
```

**`ApiKeyRequest`** - API key creation:
```rust
#[derive(Deserialize)]
pub struct ApiKeyRequest {
    name: String,
}
```

### Response Types

**`JobResponse`** - Standard job response:
```rust
#[derive(Debug, Deserialize, Serialize)]
pub struct JobResponse {
    pub job_id: String,
    pub message: Option<String>,
    pub status: Option<JobStatus>,
    pub vault_address: Option<String>,
    pub expected_timestamp: Option<i64>,
    pub l1_data: Option<serde_json::Value>,
    pub on_chain_confirmation: Option<serde_json::Value>,
}
```

**`JobResultResponse`** - Detailed job result:
```rust
#[derive(Debug, Serialize, Deserialize)]
pub struct JobResultResponse {
    pub job_id: String,
    pub status: JobStatus,
    pub result: Option<serde_json::Value>,
    pub created_at: Option<String>,
    pub completed_at: Option<String>,
}
```

**`BatchJobStatusResponse`** - Batch query response:
```rust
#[derive(Debug, Serialize, Deserialize)]
pub struct BatchJobStatusResponse {
    pub jobs: Vec<JobResultResponse>,
    pub not_found: Vec<String>,
}
```

**`ErrorResponse`** - Error response:
```rust
#[derive(Debug, Serialize, Deserialize)]
pub struct ErrorResponse {
    pub error: String,
}
```

**`GetJobStatusResponseEnum`** - Success or error:
```rust
#[derive(Serialize, Deserialize, Debug)]
#[serde(untagged)]
pub enum GetJobStatusResponseEnum {
    Success(JobResponse),
    Error(ErrorResponse),
}
```

## StarkNet Provider

The `StarknetProvider` handles communication with StarkNet RPC nodes for event monitoring.

**Initialization:**
```rust
pub fn new(rpc_url: &str) -> Result<Self> {
    let _parsed_url = Url::parse(rpc_url)?;

    Ok(Self {
        client: Client::new(),
        rpc_url: rpc_url.to_string(),
    })
}
```

**Key Methods:**

**`block_number()`** - Get current block number:
```rust
pub async fn block_number(&self) -> Result<u64> {
    let payload = json!({
        "jsonrpc": "2.0",
        "method": "starknet_blockNumber",
        "params": [],
        "id": 1
    });

    let response: Value = self.client
        .post(&self.rpc_url)
        .json(&payload)
        .send()
        .await?
        .json()
        .await?;

    // Parse hex or numeric response
    let block_number = if let Some(block_number_str) = response["result"].as_str() {
        u64::from_str_radix(block_number_str.trim_start_matches("0x"), 16)?
    } else if let Some(block_number_num) = response["result"].as_u64() {
        block_number_num
    } else {
        return Err(eyre::eyre!("Invalid block number response"));
    };

    Ok(block_number)
}
```

**`get_events()`** - Fetch events with filtering:
```rust
pub async fn get_events(
    &self,
    filter: EventFilter,
    _continuation_token: Option<String>,
    _chunk_size: usize,
) -> Result<EventsPage>
```

**Event Types:**
```rust
#[derive(Debug, Serialize, Deserialize)]
pub struct StarknetEvent {
    pub data: Vec<String>,
    pub keys: Vec<String>,
    pub block_number: Option<u64>,
    pub transaction_hash: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct EventFilter {
    pub from_block: Option<u64>,
    pub to_block: Option<u64>,
    pub address: Option<String>,
    pub keys: Option<Vec<Vec<String>>>,
}
```

## Event Monitor

The `VaultEventMonitor` continuously monitors StarkNet for `FossilCallbackSuccess` events and updates job status accordingly.

**Initialization:**
```rust
pub const fn new(
    provider: StarknetProvider,
    db: Arc<OffchainProcessorDbConnection>,
    polling_interval_secs: u64,
    blocks_per_scan: u64,
) -> Self {
    Self {
        provider,
        db,
        polling_interval: Duration::from_secs(polling_interval_secs),
        blocks_per_scan,
    }
}
```

**Main Loop:**
```rust
pub async fn start_monitoring(&self) -> Result<()> {
    info!("Starting vault event monitoring");

    loop {
        if let Err(e) = self.process_fossil_callback_events().await {
            error!("Error processing events: {:?}", e);
        }

        sleep(self.polling_interval).await;
    }
}
```

**Event Processing Flow:**

1. **Fetch Pending Jobs:**
```rust
let pending_jobs = get_pending_jobs_with_vaults(self.db.clone()).await?;
```

2. **Group by Vault Address:**
```rust
let mut jobs_by_vault: HashMap<String, Vec<&JobRequest>> = HashMap::new();
for job in &pending_jobs {
    if let Some(vault_addr) = &job.vault_address {
        jobs_by_vault.entry(vault_addr.clone()).or_default().push(job);
    }
}
```

3. **Fetch Events for Each Vault:**
```rust
let events = self.fetch_callback_events(
    vault_address,
    from_block,
    current_block
).await?;
```

4. **Match Events to Jobs:**
```rust
for job in jobs {
    for event in &events {
        if self.matches_job_criteria(job, event) {
            self.complete_job_from_event(&job.job_id, event).await?;
        }
    }
}
```

**Event Parsing:**

The `FossilCallbackSuccess` event has this structure:
```
Event selector: 0x028daf8c351269d325d0652973bd8ae38ea95c476dcc11eaf0e2495c440689cb
Data fields:
  [0] twap (felt252/hex)
  [1] max_return (felt252/hex)
  [2] reserve_price (felt252/hex)
  [3] unknown field
  [4] unknown field
  [5] timestamp (u64)
```

**Matching Criteria:**
```rust
fn matches_job_criteria(&self, job: &JobRequest, event: &FossilCallbackSuccessEvent) -> bool {
    // Match by timestamp with ±60 seconds tolerance
    if let Some(expected_timestamp) = job.expected_timestamp {
        let timestamp_diff = (event.timestamp as i64 - expected_timestamp).abs();
        if timestamp_diff > 60 {
            return false;
        }
    }
    true
}
```

**Job Completion:**
```rust
async fn complete_job_from_event(
    &self,
    job_id: &str,
    event: &FossilCallbackSuccessEvent,
) -> Result<()> {
    let l1_data = L1Data {
        twap: event.l1_data.twap.clone(),
        max_return: event.l1_data.max_return.clone(),
        reserve_price: event.l1_data.reserve_price.clone(),
    };

    let on_chain_confirmation = OnChainConfirmation {
        block_number: event.block_number,
        transaction_hash: event.transaction_hash.clone(),
        event_timestamp: event.timestamp,
    };

    update_job_with_event_data(
        self.db.clone(),
        job_id,
        JobStatus::Completed,
        serde_json::to_value(&l1_data)?,
        serde_json::to_value(&on_chain_confirmation)?,
    ).await?;

    Ok(())
}
```

## Configuration

### Environment Variables

**Server Configuration:**
- `ALLOWED_ORIGINS` - Comma-separated list of allowed CORS origins (default: empty)

**Database:**
- `OFFCHAIN_PROCESSOR_DATABASE_URL` - PostgreSQL connection string for main database

**StarkNet Integration:**
- `STARKNET_RPC_URL` - StarkNet RPC endpoint URL (default: `http://localhost:5050`)
- `EVENT_MONITOR_POLLING_INTERVAL` - Polling interval in seconds (default: `30`)
- `EVENT_MONITOR_BLOCKS_PER_SCAN` - Number of blocks to scan per poll (default: `1000`)

**Proving Service:**
- `PROVING_SERVICE_URL` - URL for the proving service (default: `http://127.0.0.1:3000`)

**Logging:**
- `RUST_LOG` - Log level configuration (default: `info,tracing=info,sqlx=error`)

## Binary: Server (`main.rs`)

The main server binary initializes the HTTP server and background event monitor.

**Startup Sequence:**

1. **Load Environment:**
```rust
dotenv().ok();
```

2. **Connect to Database:**
```rust
let offchain_processor_db = Arc::new(OffchainProcessorDbConnection::from_env().await?);
offchain_processor_db.migrate().await?;
```

3. **Create Axum App:**
```rust
let app = create_app(offchain_processor_db.clone()).await;
let listener = tokio::net::TcpListener::bind("0.0.0.0:3000").await?;
```

4. **Initialize Logging:**
```rust
let fmt_layer = fmt::layer()
    .event_format(fmt::format())
    .with_timer(fmt::time::UtcTime::rfc_3339())
    .with_thread_names(true)
    .with_thread_ids(true);

let filter_layer = EnvFilter::try_from_default_env()
    .unwrap_or_else(|_| EnvFilter::new("info,tracing=info,sqlx=error"));

Registry::default()
    .with(fmt_layer)
    .with(filter_layer)
    .init();
```

5. **Initialize Event Monitor:**
```rust
let provider = StarknetProvider::new(&starknet_rpc_url)?;
let event_monitor = VaultEventMonitor::new(
    provider,
    offchain_processor_db.clone(),
    polling_interval_secs,
    blocks_per_scan,
);
```

6. **Start Event Monitor in Background:**
```rust
let event_monitor_handle = tokio::spawn(async move {
    if let Err(e) = event_monitor.start_monitoring().await {
        error!("Event monitor failed: {:?}", e);
    }
});
```

7. **Start HTTP Server:**
```rust
axum::serve(listener, app.into_make_service()).await?;
```

**Graceful Shutdown:**
When the server stops, the event monitor task is aborted:
```rust
event_monitor_handle.abort();
```

## Binary: Standalone Event Monitor

A standalone event monitor binary is available for running event monitoring separately from the HTTP server.

**Path:** `src/bin/event_monitor.rs`

**Usage:**
```bash
cargo run --bin event_monitor
```

**Configuration:** Uses the same environment variables as the main server.

## Binary: Create API Key Script

A utility script for creating API keys from the command line.

**Path:** `src/scripts/create_api_key.rs`

**Usage:**
```bash
cargo run --bin create_api_key
```

## Usage Examples

### Running the Server

**Development:**
```bash
cd fossil-api/crates/server
cargo run
```

**Production:**
```bash
cargo run --release
```

**With Custom Configuration:**
```bash
STARKNET_RPC_URL=https://starknet-mainnet.infura.io/v3/YOUR_KEY \
EVENT_MONITOR_POLLING_INTERVAL=60 \
EVENT_MONITOR_BLOCKS_PER_SCAN=2000 \
cargo run --release
```

### Making Requests

**1. Create API Key:**
```bash
curl -X POST http://localhost:3000/api_key \
  -H "Content-Type: application/json" \
  -d '{"name": "my-api-key"}'
```

**Response:**
```json
{
  "api_key": "550e8400-e29b-41d4-a716-446655440000"
}
```

**2. Request Pricing Data:**
```bash
curl -X POST http://localhost:3000/pricing_data \
  -H "Content-Type: application/json" \
  -H "x-api-key: 550e8400-e29b-41d4-a716-446655440000" \
  -d '{
    "program_id": "fossil-twap-v1",
    "params": {
      "twap": [1704067200, 1704153600],
      "max_return": [1704067200, 1704153600],
      "reserve_price": [1704067200, 1704153600]
    },
    "vault_address": "0x1234567890abcdef"
  }'
```

**Response:**
```json
{
  "job_id": "f47ac10b-58cc-4372-a567-0e02b2c3d479",
  "message": "New job request registered and processing initiated.",
  "status": "Pending",
  "vault_address": "0x1234567890abcdef",
  "expected_timestamp": 1704153600
}
```

**3. Check Job Status:**
```bash
curl http://localhost:3000/job_status/f47ac10b-58cc-4372-a567-0e02b2c3d479
```

**Response (Pending):**
```json
{
  "job_id": "f47ac10b-58cc-4372-a567-0e02b2c3d479",
  "message": "Job is pending on-chain confirmation via FossilCallbackSuccess event",
  "status": "Pending",
  "vault_address": "0x1234567890abcdef",
  "expected_timestamp": 1704153600
}
```

**Response (Completed):**
```json
{
  "job_id": "f47ac10b-58cc-4372-a567-0e02b2c3d479",
  "message": "Job completed with on-chain confirmation",
  "status": "Completed",
  "vault_address": "0x1234567890abcdef",
  "expected_timestamp": 1704153600,
  "l1_data": {
    "twap": "0x1234abcd",
    "max_return": "0x5678ef01",
    "reserve_price": "0x9abcdef0"
  },
  "on_chain_confirmation": {
    "block_number": 12345,
    "transaction_hash": "0x4c55b3c8765e0e1904c50c3e758f34e7e9ef9721adad322ba4632967ea8b711",
    "event_timestamp": 1704153600
  }
}
```

**4. Get Job Result (Authenticated):**
```bash
curl http://localhost:3000/job_result/f47ac10b-58cc-4372-a567-0e02b2c3d479 \
  -H "x-api-key: 550e8400-e29b-41d4-a716-446655440000"
```

**5. Batch Job Status Query:**
```bash
curl -X POST http://localhost:3000/batch_job_status \
  -H "Content-Type: application/json" \
  -H "x-api-key: 550e8400-e29b-41d4-a716-446655440000" \
  -d '{
    "job_ids": [
      "f47ac10b-58cc-4372-a567-0e02b2c3d479",
      "a67bc20c-68dd-5483-b678-1f13c3d4e580",
      "nonexistent-job"
    ]
  }'
```

**Response:**
```json
{
  "jobs": [
    {
      "job_id": "f47ac10b-58cc-4372-a567-0e02b2c3d479",
      "status": "Completed",
      "result": {...},
      "created_at": "2024-01-01T00:00:00Z",
      "completed_at": "2024-01-01T00:05:00Z"
    },
    {
      "job_id": "a67bc20c-68dd-5483-b678-1f13c3d4e580",
      "status": "Pending",
      "result": null,
      "created_at": "2024-01-01T00:01:00Z",
      "completed_at": null
    }
  ],
  "not_found": ["nonexistent-job"]
}
```

**6. Health Check:**
```bash
curl http://localhost:3000/health
```

**Response:**
```
OK
```

## Testing

The server crate includes comprehensive integration tests using `testcontainers` for PostgreSQL.

### Test Fixtures

The `fixtures` module provides test infrastructure:

```rust
pub struct TestContext {
    pub app_state: AppState,
    pub offchain_processor_db: Arc<OffchainProcessorDbConnection>,
    pub _container: Container<'static, PostgresImage>,
}

impl TestContext {
    pub async fn new() -> Self {
        // Creates PostgreSQL container and initializes schema
    }

    pub async fn create_job(&self, job_id: &str, status: JobStatus) {
        // Helper to create test jobs
    }

    pub async fn get_job_status(&self, job_id: &str)
        -> (StatusCode, Json<GetJobStatusResponseEnum>) {
        // Test job status endpoint
    }

    pub async fn get_pricing_data(&self, payload: PitchLakeJobRequest)
        -> (StatusCode, Json<JobResponse>) {
        // Test pricing data endpoint
    }
}
```

### Example Tests

**Health Check Test:**
```rust
#[tokio::test]
async fn test_root() {
    let app = Router::new().route("/", get(health_check));
    let server = TestServer::new(app).unwrap();

    let response = server.get("/").await;

    assert_eq!(response.status_code(), StatusCode::OK);
    assert_eq!(response.text(), "OK");
}
```

**Job Status Tests:**
```rust
#[tokio::test]
async fn test_get_job_status_pending() {
    let ctx = TestContext::new().await;
    let job_id = "pending_job_id";

    ctx.create_job(job_id, JobStatus::Pending).await;

    let (status, Json(response)) = ctx.get_job_status(job_id).await;

    let response = match response {
        GetJobStatusResponseEnum::Success(success_res) => success_res,
        GetJobStatusResponseEnum::Error(_) => panic!("Unexpected response status"),
    };

    assert_eq!(status, StatusCode::OK);
    assert_eq!(response.job_id, job_id);
    assert_eq!(response.status.unwrap(), JobStatus::Pending);
}
```

**Pricing Data Tests:**
```rust
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
```

**Batch Status Tests:**
```rust
#[tokio::test]
async fn test_batch_job_status() {
    let ctx = TestContext::new().await;

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
```

### Running Tests

**All tests:**
```bash
cd fossil-api/crates/server
cargo test
```

**Specific test:**
```bash
cargo test test_get_pricing_data_new_job
```

**With output:**
```bash
cargo test -- --nocapture
```

**Integration tests require Docker** for testcontainers to spin up PostgreSQL instances.

## Next Steps

- [Database Access Crate](./db-access.md) - Database models, migrations, and queries
- [Main Fossil API Documentation](../../README.md) - Project overview
- [Proving Service Integration](../../proving-service/README.md) - Understanding the proving service API

## Related Files

**Core Files:**
- `/home/ametel/source/fossil-monorepo/fossil-api/crates/server/src/lib.rs` - Application setup
- `/home/ametel/source/fossil-monorepo/fossil-api/crates/server/src/main.rs` - Server entry point
- `/home/ametel/source/fossil-monorepo/fossil-api/crates/server/src/types.rs` - Request/response types

**Handlers:**
- `/home/ametel/source/fossil-monorepo/fossil-api/crates/server/src/handlers/health_check.rs`
- `/home/ametel/source/fossil-monorepo/fossil-api/crates/server/src/handlers/api_key.rs`
- `/home/ametel/source/fossil-monorepo/fossil-api/crates/server/src/handlers/get_pricing_data.rs`
- `/home/ametel/source/fossil-monorepo/fossil-api/crates/server/src/handlers/job_status.rs`
- `/home/ametel/source/fossil-monorepo/fossil-api/crates/server/src/handlers/pl_integration.rs`

**Middleware:**
- `/home/ametel/source/fossil-monorepo/fossil-api/crates/server/src/middlewares/auth.rs`

**StarkNet Integration:**
- `/home/ametel/source/fossil-monorepo/fossil-api/crates/server/src/starknet_provider.rs`
- `/home/ametel/source/fossil-monorepo/fossil-api/crates/server/src/event_monitor.rs`

**Binaries:**
- `/home/ametel/source/fossil-monorepo/fossil-api/crates/server/src/bin/event_monitor.rs`
- `/home/ametel/source/fossil-monorepo/fossil-api/crates/server/src/scripts/create_api_key.rs`

**Tests:**
- `/home/ametel/source/fossil-monorepo/fossil-api/crates/server/src/handlers/fixtures/mod.rs`
