# Fossil API Architecture

This document provides a comprehensive overview of the Fossil API service, including its architecture, components, data flows, and integration patterns.

## Table of Contents
- [Overview](#overview)
- [Crate Organization](#crate-organization)
- [Architecture Diagram](#architecture-diagram)
- [Server Layer](#server-layer)
- [Database Access Layer](#database-access-layer)
- [API Endpoints](#api-endpoints)
- [Authentication](#authentication)
- [Request Flow](#request-flow)
- [Event Monitoring](#event-monitoring)
- [Configuration](#configuration)
- [Testing](#testing)
- [Next Steps](#next-steps)

## Overview

### Purpose

The Fossil API is the client-facing HTTP service that serves as the entry point for the Fossil proof generation system. It provides a REST API for submitting pricing data calculation requests, monitoring job status, and retrieving verified results from the StarkNet blockchain.

### Key Responsibilities

1. **API Key Authentication** - Secure access control using API key-based authentication
2. **Request Validation** - Validate client requests and parameters before processing
3. **Job Management** - Create, track, and manage proof generation jobs
4. **Proving Service Integration** - Forward validated requests to the Proving Service
5. **Event Monitoring** - Monitor StarkNet for proof verification events
6. **Result Delivery** - Return verified pricing data and proof information to clients

### Technology Stack

| Component | Technology | Version | Purpose |
|-----------|-----------|---------|---------|
| Language | Rust | Edition 2021 | Core implementation |
| HTTP Framework | Axum | 0.8+ | Async web server |
| Database | PostgreSQL | 14+ | Data persistence |
| ORM | SQLx | 0.8+ | Type-safe SQL queries |
| Async Runtime | Tokio | 1.39+ | Async execution |
| HTTP Client | Reqwest | 0.11+ | Proving Service calls |
| StarkNet RPC | Custom provider | - | Event monitoring |
| Logging | Tracing | 0.1+ | Structured logging |

## Crate Organization

The Fossil API is organized as a Rust workspace with two main crates:

```
fossil-api/
├── Cargo.toml                    # Workspace definition
├── crates/
│   ├── server/                   # HTTP server implementation
│   │   ├── Cargo.toml
│   │   ├── src/
│   │   │   ├── main.rs           # Application entry point
│   │   │   ├── lib.rs            # Server library
│   │   │   ├── types.rs          # Request/response types
│   │   │   ├── event_monitor.rs  # StarkNet event monitoring
│   │   │   ├── starknet_provider.rs # StarkNet RPC client
│   │   │   ├── handlers/         # HTTP request handlers
│   │   │   │   ├── mod.rs
│   │   │   │   ├── health_check.rs
│   │   │   │   ├── api_key.rs
│   │   │   │   ├── get_pricing_data.rs
│   │   │   │   ├── job_status.rs
│   │   │   │   └── pl_integration.rs
│   │   │   ├── middlewares/      # HTTP middleware
│   │   │   │   ├── mod.rs
│   │   │   │   └── auth.rs
│   │   │   ├── scripts/          # Utility scripts
│   │   │   │   └── create_api_key.rs
│   │   │   └── bin/              # Additional binaries
│   │   │       └── event_monitor.rs
│   │   └── tests/                # Integration tests
│   └── db-access/                # Database access layer
│       ├── Cargo.toml
│       ├── src/
│       │   ├── lib.rs            # Database connection
│       │   ├── models.rs         # Data models
│       │   ├── queries.rs        # SQL queries
│       │   ├── auth.rs           # Auth queries
│       │   └── types.rs          # Type definitions
│       └── migrations/           # Database migrations
│           ├── 20241024044528_job_requests.up.sql
│           ├── 20241024044538_api_keys.up.sql
│           ├── 20250112120000_add_updated_at.up.sql
│           └── 20250117140000_add_vault_event_fields.up.sql
└── .env.example                  # Environment configuration
```

### Crate Descriptions

#### `server` Crate

The server crate contains all HTTP server logic, request handling, and external integrations.

**Key Files:**
- `main.rs` - Application bootstrap, database migrations, server initialization
- `lib.rs` - Router configuration, CORS setup, middleware application
- `handlers/` - HTTP endpoint implementations
- `middlewares/` - Authentication and other middleware
- `event_monitor.rs` - Background service for StarkNet event monitoring
- `starknet_provider.rs` - StarkNet RPC client wrapper

#### `db-access` Crate

The database access crate provides a clean abstraction over PostgreSQL, handling connections, migrations, and queries.

**Key Files:**
- `lib.rs` - Database connection pool and migration runner
- `models.rs` - Rust structs representing database entities
- `queries.rs` - SQL query functions using SQLx
- `auth.rs` - Authentication-specific queries
- `migrations/` - SQL migration files

## Architecture Diagram

### Component Architecture

```
┌─────────────────────────────────────────────────────────────────────────┐
│                            Fossil API Service                           │
│                                                                         │
│  ┌───────────────────────────────────────────────────────────────────┐ │
│  │                        HTTP Server (Axum)                         │ │
│  │                        Port: 3000                                 │ │
│  └────────────────────────────┬──────────────────────────────────────┘ │
│                               │                                         │
│         ┌─────────────────────┼────────────────────────┐               │
│         │                     │                        │               │
│         ▼                     ▼                        ▼               │
│  ┌────────────┐      ┌────────────────┐      ┌──────────────┐        │
│  │  Public    │      │   Secured      │      │  Event       │        │
│  │  Routes    │      │   Routes       │      │  Monitor     │        │
│  │            │      │  (API Key)     │      │ (Background) │        │
│  └─────┬──────┘      └────────┬───────┘      └──────┬───────┘        │
│        │                      │                      │                │
│        │  ┌───────────────────┼──────────────────────┘                │
│        │  │                   │                                       │
│        ▼  ▼                   ▼                                       │
│  ┌────────────────────────────────────────────┐                      │
│  │            Request Handlers                │                      │
│  │  ┌──────────────┐  ┌─────────────────┐   │                      │
│  │  │ Health Check │  │ API Key Mgmt    │   │                      │
│  │  └──────────────┘  └─────────────────┘   │                      │
│  │  ┌──────────────┐  ┌─────────────────┐   │                      │
│  │  │ Job Status   │  │ Get Pricing Data│   │                      │
│  │  └──────────────┘  └─────────────────┘   │                      │
│  │  ┌──────────────┐  ┌─────────────────┐   │                      │
│  │  │ Job Result   │  │ Batch Status    │   │                      │
│  │  └──────────────┘  └─────────────────┘   │                      │
│  └────────────────┬───────────────────┬──────┘                      │
│                   │                   │                             │
│                   │  ┌────────────────┘                             │
│                   │  │                                               │
│                   ▼  ▼                                               │
│  ┌─────────────────────────────────────────────────────────────┐   │
│  │               Database Access Layer                         │   │
│  │  ┌────────────┐  ┌────────────┐  ┌────────────────────┐   │   │
│  │  │  Models    │  │  Queries   │  │  Auth Functions    │   │   │
│  │  └────────────┘  └────────────┘  └────────────────────┘   │   │
│  └──────────────────────────┬──────────────────────────────────┘   │
│                             │                                       │
└─────────────────────────────┼───────────────────────────────────────┘
                              │
                              ▼
                    ┌──────────────────┐
                    │   PostgreSQL DB  │
                    │                  │
                    │  • job_requests  │
                    │  • api_keys      │
                    └──────────────────┘

          External Integrations:

    ┌─────────────────────┐        ┌─────────────────────┐
    │  Proving Service    │        │   StarkNet RPC      │
    │  (HTTP API)         │        │   (Event Monitor)   │
    │  Port: 3001         │        │                     │
    └─────────────────────┘        └─────────────────────┘
```

### Data Flow Architecture

```
Client Request Flow:
┌────────┐     ┌──────────┐     ┌──────────┐     ┌──────────┐     ┌──────────┐
│        │ (1) │          │ (2) │          │ (3) │          │ (4) │ Proving  │
│ Client ├────▶│   Auth   ├────▶│ Handler  ├────▶│    DB    ├────▶│ Service  │
│        │     │Middleware│     │          │     │          │     │   API    │
└────┬───┘     └──────────┘     └──────────┘     └──────────┘     └──────────┘
     │
     │ (5) Response
     └───────────────────────────────────────────────────────────────────┘

Event Monitoring Flow:
┌──────────┐     ┌──────────┐     ┌──────────┐     ┌──────────┐
│ Event    │ (1) │ StarkNet │ (2) │  Parse   │ (3) │ Update   │
│ Monitor  ├────▶│   RPC    ├────▶│  Event   ├────▶│    DB    │
│ (Loop)   │     │   Poll   │     │   Data   │     │  Status  │
└──────────┘     └──────────┘     └──────────┘     └──────────┘
```

## Server Layer

The server layer is implemented using Axum, a modern async web framework built on Tokio.

### Application Initialization

**`main.rs` - Bootstrap Process:**

```rust
#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    // 1. Load environment variables
    dotenv().ok();

    // 2. Initialize database connection pool
    let offchain_processor_db = Arc::new(
        OffchainProcessorDbConnection::from_env().await?
    );

    // 3. Run database migrations
    offchain_processor_db.migrate().await?;

    // 4. Create Axum application
    let app = create_app(offchain_processor_db.clone()).await;

    // 5. Bind TCP listener
    let listener = tokio::net::TcpListener::bind("0.0.0.0:3000").await?;

    // 6. Initialize logging
    Registry::default()
        .with(fmt_layer)
        .with(filter_layer)
        .init();

    // 7. Start event monitor in background
    let event_monitor = VaultEventMonitor::new(/* ... */);
    tokio::spawn(async move {
        event_monitor.start_monitoring().await
    });

    // 8. Start HTTP server
    axum::serve(listener, app.into_make_service()).await?;

    Ok(())
}
```

### Router Configuration

**`lib.rs` - Route Setup:**

```rust
pub async fn create_app(offchain_processor_db: Arc<OffchainProcessorDbConnection>) -> Router {
    let app_state = AppState { offchain_processor_db };

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

    // Combine routes and apply global middleware
    Router::new()
        .merge(secured_routes)
        .merge(public_routes)
        .layer(TraceLayer::new_for_http())
        .layer(cors_layer)
        .with_state(app_state)
}
```

### Application State

**Shared State Across Handlers:**

```rust
#[derive(Clone)]
pub struct AppState {
    pub offchain_processor_db: Arc<OffchainProcessorDbConnection>,
}
```

The `AppState` is cloned for each request and provides access to the database connection pool.

### Middleware

#### Authentication Middleware

**`middlewares/auth.rs`:**

```rust
pub async fn simple_apikey_auth(
    State(state): State<AppState>,
    headers: HeaderMap,
    request: Request<axum::body::Body>,
    next: Next,
) -> Result<Response, StatusCode> {
    // 1. Extract API key from X-API-Key header
    let api_key_str = match extract_api_key(&headers) {
        Ok(key) => key,
        Err(response) => return Ok(*response),
    };

    // 2. Validate API key against database
    match validate_api_key(&state, api_key_str).await {
        Ok(()) => Ok(next.run(request).await),
        Err(response) => Ok(*response),
    }
}
```

**Features:**
- Header-based API key extraction (`X-API-Key`)
- Database validation via `find_api_key()` query
- Structured error responses
- Logging for security events

### Request Handlers

#### Health Check Handler

**`handlers/health_check.rs`:**

```rust
pub async fn health_check() -> StatusCode {
    StatusCode::OK
}
```

Simple endpoint for service health monitoring.

#### API Key Creation Handler

**`handlers/api_key.rs`:**

```rust
pub async fn create_api_key(
    State(state): State<AppState>,
    Json(payload): Json<CreateApiKeyRequest>,
) -> (StatusCode, Json<CreateApiKeyResponse>) {
    // Generate secure API key
    let api_key = generate_secure_key();

    // Store in database
    match add_api_key(state.offchain_processor_db, api_key.clone(), payload.name).await {
        Ok(_) => (StatusCode::CREATED, Json(CreateApiKeyResponse { api_key })),
        Err(e) => (StatusCode::INTERNAL_SERVER_ERROR, Json(error_response(e))),
    }
}
```

#### Get Pricing Data Handler

**`handlers/get_pricing_data.rs`:**

The main endpoint for submitting pricing data calculation requests.

**Request Flow:**

```rust
pub async fn get_pricing_data(
    State(state): State<AppState>,
    Json(payload): Json<PitchLakeJobRequest>,
) -> (StatusCode, Json<JobResponse>) {
    // 1. Validate request parameters
    validate_request(&payload)?;

    // 2. Generate unique job ID
    let job_id = generate_job_id(&payload.program_id, &payload.params);

    // 3. Check for existing job
    match get_job_request(state.offchain_processor_db.clone(), &job_id).await {
        Ok(Some(job_request)) => handle_existing_job(state, job_request, job_id, payload).await,
        Ok(None) => handle_new_job_request(state, job_id, payload).await,
        Err(e) => internal_server_error(e, job_id),
    }
}
```

**Request Type:**

```rust
#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct PitchLakeJobRequest {
    pub program_id: String,
    pub params: PitchLakeJobRequestParams,
    pub vault_address: String,
}

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct PitchLakeJobRequestParams {
    pub twap: (i64, i64),           // (start_timestamp, end_timestamp)
    pub max_return: (i64, i64),
    pub reserve_price: (i64, i64),
}
```

**Job Creation Logic:**

```rust
async fn handle_new_job_request(
    state: &AppState,
    job_id: String,
    payload: PitchLakeJobRequest,
) -> (StatusCode, Json<JobResponse>) {
    // Get current timestamp for job tracking
    let current_timestamp = get_current_timestamp()?;

    // Create job record in database
    create_job_request_with_vault(
        state.offchain_processor_db.clone(),
        &job_id,
        JobStatus::Pending,
        &payload.vault_address,
        current_timestamp,
    ).await?;

    // Spawn background task to call proving service
    spawn_job_processor(state.offchain_processor_db.clone(), &job_id, &payload);

    // Return immediate response
    (StatusCode::CREATED, Json(JobResponse {
        job_id: job_id.clone(),
        message: Some("New job request registered and processing initiated.".to_string()),
        status: Some(JobStatus::Pending),
        vault_address: Some(payload.vault_address.clone()),
        expected_timestamp: Some(current_timestamp),
        l1_data: None,
        on_chain_confirmation: None,
    }))
}
```

**Proving Service Integration:**

```rust
async fn call_proving_service(
    job_id: &str,
    payload: &PitchLakeJobRequest,
) -> Result<serde_json::Value, eyre::Error> {
    let proving_service_url = env::var("PROVING_SERVICE_URL")
        .unwrap_or_else(|_| "http://127.0.0.1:3001".to_string());

    // Transform request format
    let api_payload = json!({
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
        "vault_timestamp": get_current_timestamp()?
    });

    // Send HTTP POST request
    let response = Client::new()
        .post(format!("{}/api/job", proving_service_url))
        .json(&api_payload)
        .send()
        .await?;

    // Parse response
    response.json::<serde_json::Value>().await
}
```

#### Job Status Handler

**`handlers/job_status.rs`:**

```rust
pub async fn get_job_status(
    State(state): State<AppState>,
    Path(job_id): Path<String>,
) -> (StatusCode, Json<GetJobStatusResponseEnum>) {
    match get_job_request(state.offchain_processor_db, &job_id).await {
        Ok(Some(job)) => {
            let message = build_status_message(&job);
            (StatusCode::OK, Json(GetJobStatusResponseEnum::Success(
                JobResponse::from_job_request(&job, message)
            )))
        }
        Ok(None) => (StatusCode::NOT_FOUND, Json(GetJobStatusResponseEnum::Error(
            ErrorResponse { error: "Job not found".to_string() }
        ))),
        Err(e) => (StatusCode::INTERNAL_SERVER_ERROR, Json(GetJobStatusResponseEnum::Error(
            ErrorResponse { error: "Internal error".to_string() }
        ))),
    }
}
```

**Status Messages:**

```rust
fn build_status_message(job: &JobRequest) -> Option<String> {
    match job.status {
        JobStatus::Pending => {
            if job.vault_address.is_some() {
                Some("Job is pending on-chain confirmation via FossilCallbackSuccess event".to_string())
            } else {
                Some("Job is pending".to_string())
            }
        }
        JobStatus::Completed => {
            if job.l1_data.is_some() {
                Some("Job completed with on-chain confirmation".to_string())
            } else {
                Some("Job completed".to_string())
            }
        }
        JobStatus::Failed => Some("Job failed".to_string()),
    }
}
```

#### PitchLake Integration Handlers

**`handlers/pl_integration.rs`:**

**Get Job Result:**

```rust
pub async fn get_job_result(
    State(state): State<AppState>,
    Path(job_id): Path<String>,
) -> (StatusCode, Json<Result<JobResultResponse, ErrorResponse>>) {
    match get_job_request(state.offchain_processor_db, &job_id).await {
        Ok(Some(job)) => (StatusCode::OK, Json(Ok(JobResultResponse {
            job_id: job.job_id,
            status: job.status,
            result: job.result,
            created_at: Some(job.created_at.format("%Y-%m-%dT%H:%M:%SZ").to_string()),
            completed_at: job.updated_at.map(|t| t.format("%Y-%m-%dT%H:%M:%SZ").to_string()),
        }))),
        Ok(None) => (StatusCode::NOT_FOUND, Json(Err(ErrorResponse {
            error: "Job not found".to_string()
        }))),
        Err(e) => (StatusCode::INTERNAL_SERVER_ERROR, Json(Err(ErrorResponse {
            error: "Internal error".to_string()
        }))),
    }
}
```

**Batch Job Status:**

```rust
pub async fn get_batch_job_status(
    State(state): State<AppState>,
    Json(payload): Json<BatchJobStatusRequest>,
) -> (StatusCode, Json<BatchJobStatusResponse>) {
    // Validate batch size (max 100 jobs)
    if payload.job_ids.len() > 100 {
        return (StatusCode::BAD_REQUEST, Json(BatchJobStatusResponse {
            jobs: vec![],
            not_found: payload.job_ids,
        }));
    }

    // Fetch all requested jobs in single query
    match get_multiple_job_requests(state.offchain_processor_db, &payload.job_ids).await {
        Ok(jobs) => {
            let found_jobs: HashSet<_> = jobs.iter().map(|j| j.job_id.clone()).collect();
            let not_found: Vec<String> = payload.job_ids.into_iter()
                .filter(|id| !found_jobs.contains(id))
                .collect();

            let job_responses: Vec<JobResultResponse> = jobs.into_iter()
                .map(job_to_result)
                .collect();

            (StatusCode::OK, Json(BatchJobStatusResponse {
                jobs: job_responses,
                not_found,
            }))
        }
        Err(e) => (StatusCode::INTERNAL_SERVER_ERROR, Json(BatchJobStatusResponse {
            jobs: vec![],
            not_found: payload.job_ids,
        })),
    }
}
```

## Database Access Layer

The database access layer provides a clean, type-safe interface to PostgreSQL using SQLx.

### Connection Management

**`db-access/src/lib.rs`:**

```rust
#[derive(Debug)]
pub struct DbConnection {
    pub pool: Pool<Postgres>,
}

impl DbConnection {
    pub async fn new(database_url: &str) -> Result<Arc<Self>> {
        let pool = PgPoolOptions::new()
            .max_connections(5)
            .connect(database_url)
            .await?;

        Ok(Arc::new(Self { pool }))
    }
}

pub struct OffchainProcessorDbConnection(Arc<DbConnection>);

impl OffchainProcessorDbConnection {
    pub async fn from_env() -> Result<Self> {
        let database_url = env::var("OFFCHAIN_PROCESSOR_DATABASE_URL")?;
        let db_connection = DbConnection::new(&database_url).await?;
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
```

### Data Models

**`db-access/src/models.rs`:**

**API Key Model:**

```rust
#[derive(sqlx::FromRow, Debug)]
pub struct ApiKey {
    pub key: String,
    pub name: Option<String>,
}
```

**Job Status Enum:**

```rust
#[derive(sqlx::Type, Debug, PartialEq, Eq, Serialize, Deserialize, Clone)]
#[sqlx(type_name = "TEXT")]
pub enum JobStatus {
    Pending,
    Completed,
    Failed,
}

impl fmt::Display for JobStatus {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Pending => write!(f, "Pending"),
            Self::Completed => write!(f, "Completed"),
            Self::Failed => write!(f, "Failed"),
        }
    }
}
```

**Job Request Model:**

```rust
#[derive(sqlx::FromRow, Debug)]
pub struct JobRequest {
    pub job_id: String,
    pub status: JobStatus,
    pub vault_address: Option<String>,
    pub expected_timestamp: Option<i64>,
    pub created_at: chrono::NaiveDateTime,
    pub updated_at: Option<chrono::NaiveDateTime>,
    pub result: Option<serde_json::Value>,
    pub l1_data: Option<serde_json::Value>,
    pub on_chain_confirmation: Option<serde_json::Value>,
}
```

**L1 Data and Confirmation Models:**

```rust
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct L1Data {
    pub twap: String,          // u256 as hex string
    pub max_return: String,    // u128 as hex string
    pub reserve_price: String, // u256 as hex string
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct OnChainConfirmation {
    pub block_number: u64,
    pub transaction_hash: String,
    pub event_timestamp: u64,
}
```

### Database Queries

**`db-access/src/queries.rs`:**

**Create Job Request:**

```rust
pub async fn create_job_request_with_vault(
    db: Arc<OffchainProcessorDbConnection>,
    job_id: &str,
    status: JobStatus,
    vault_address: &str,
    expected_timestamp: i64,
) -> Result<(), sqlx::Error> {
    sqlx::query!(
        "INSERT INTO job_requests (job_id, status, vault_address, expected_timestamp)
         VALUES ($1, $2, $3, $4)",
        job_id,
        status.to_string(),
        vault_address,
        expected_timestamp
    )
    .execute(&db.db_connection().pool)
    .await?;

    Ok(())
}
```

**Get Job Request:**

```rust
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
```

**Update Job Status:**

```rust
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
```

**Update Job with Event Data:**

```rust
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
```

**Get Multiple Jobs (Batch):**

```rust
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
```

**Get Pending Jobs with Vaults:**

```rust
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
```

### Authentication Queries

**`db-access/src/auth.rs`:**

**Add API Key:**

```rust
pub async fn add_api_key(
    db: Arc<OffchainProcessorDbConnection>,
    api_key: String,
    name: String,
) -> Result<(), sqlx::Error> {
    sqlx::query!(
        "INSERT INTO api_keys (key, name, created_at) VALUES ($1, $2, now())",
        api_key,
        name
    )
    .execute(&db.db_connection().pool)
    .await?;
    Ok(())
}
```

**Find API Key:**

```rust
pub async fn find_api_key(
    db: Arc<OffchainProcessorDbConnection>,
    key: String,
) -> Result<ApiKey, sqlx::Error> {
    let api_key = sqlx::query_as!(
        ApiKey,
        r#"
        SELECT key, name as "name?"
        FROM api_keys
        WHERE key = $1
        "#,
        key
    )
    .fetch_one(&db.db_connection().pool)
    .await?;

    Ok(api_key)
}
```

**Validate API Key:**

```rust
pub async fn validate_api_key(
    db: Arc<OffchainProcessorDbConnection>,
    api_key: &str,
) -> Result<(), sqlx::Error> {
    let result = sqlx::query!(
        r#"
        SELECT key
        FROM api_keys
        WHERE key = $1
        "#,
        api_key
    )
    .fetch_optional(&db.db_connection().pool)
    .await?;

    match result {
        Some(_) => Ok(()),
        None => Err(sqlx::Error::RowNotFound),
    }
}
```

### Database Migrations

**Job Requests Table:**

`migrations/20241024044528_job_requests.up.sql`:

```sql
CREATE TABLE IF NOT EXISTS public.job_requests (
    job_id VARCHAR(255) NOT NULL,
    created_at TIMESTAMP WITHOUT TIME ZONE NOT NULL DEFAULT CURRENT_TIMESTAMP,
    status VARCHAR(20) NOT NULL,
    result JSONB,
    CONSTRAINT job_requests_pkey PRIMARY KEY (job_id),
    CONSTRAINT job_requests_status_check CHECK (
        status::TEXT = ANY (ARRAY['Completed'::TEXT, 'Pending'::TEXT, 'Failed'::TEXT])
    )
);

ALTER TABLE IF EXISTS public.job_requests OWNER TO postgres;
```

**API Keys Table:**

`migrations/20241024044538_api_keys.up.sql`:

```sql
CREATE TABLE IF NOT EXISTS public.api_keys (
    id SERIAL PRIMARY KEY,
    key TEXT NOT NULL,
    name VARCHAR(255) NOT NULL,
    created_at TIMESTAMP WITHOUT TIME ZONE NOT NULL DEFAULT NOW(),
    CONSTRAINT api_keys_key_key UNIQUE (key)
);

ALTER TABLE IF EXISTS public.api_keys OWNER TO postgres;
```

**Add Updated At Column:**

`migrations/20250112120000_add_updated_at.up.sql`:

```sql
ALTER TABLE job_requests
ADD COLUMN updated_at TIMESTAMP WITHOUT TIME ZONE;
```

**Add Vault Event Fields:**

`migrations/20250117140000_add_vault_event_fields.up.sql`:

```sql
ALTER TABLE job_requests
ADD COLUMN vault_address TEXT,
ADD COLUMN expected_timestamp BIGINT,
ADD COLUMN l1_data JSONB,
ADD COLUMN on_chain_confirmation JSONB;
```

## API Endpoints

### Endpoint Summary

| Endpoint | Method | Auth | Description |
|----------|--------|------|-------------|
| `/health` | GET | No | Health check endpoint |
| `/api_key` | POST | No | Create new API key |
| `/pricing_data` | POST | Yes | Submit pricing data calculation request |
| `/job_status/{job_id}` | GET | No | Get job status |
| `/job_result/{job_id}` | GET | Yes | Get detailed job result with proof data |
| `/batch_job_status` | POST | Yes | Get status for multiple jobs |
| `/webhook/{job_id}` | POST | No | Webhook callback endpoint |

### Detailed Endpoint Documentation

#### POST `/pricing_data`

Submit a new pricing data calculation request.

**Authentication:** Required (`X-API-Key` header)

**Request Body:**

```json
{
  "program_id": "RISC0_MOCK_PROOF_TEST",
  "vault_address": "0x004018ae0157b10d08cb1f70d34e32fd8b25e4ad1d70afc89616c3b300257fd9",
  "params": {
    "twap": [1672531200, 1672617600],
    "max_return": [1672531200, 1672617600],
    "reserve_price": [1672531200, 1672617600]
  }
}
```

**Response (201 Created):**

```json
{
  "job_id": "550e8400-e29b-41d4-a716-446655440000",
  "message": "New job request registered and processing initiated.",
  "status": "Pending",
  "vault_address": "0x004018ae0157b10d08cb1f70d34e32fd8b25e4ad1d70afc89616c3b300257fd9",
  "expected_timestamp": 1672531200,
  "l1_data": null,
  "on_chain_confirmation": null
}
```

**Error Responses:**

- `400 Bad Request` - Invalid parameters
- `401 Unauthorized` - Missing or invalid API key
- `409 Conflict` - Job already pending
- `500 Internal Server Error` - Server error

#### GET `/job_status/{job_id}`

Get the current status of a job.

**Authentication:** Not required

**Response (200 OK):**

```json
{
  "job_id": "550e8400-e29b-41d4-a716-446655440000",
  "message": "Job is pending on-chain confirmation via FossilCallbackSuccess event",
  "status": "Pending",
  "vault_address": "0x004018ae...",
  "expected_timestamp": 1672531200,
  "l1_data": null,
  "on_chain_confirmation": null
}
```

**Completed Job Response:**

```json
{
  "job_id": "550e8400-e29b-41d4-a716-446655440000",
  "message": "Job completed with on-chain confirmation",
  "status": "Completed",
  "vault_address": "0x004018ae...",
  "expected_timestamp": 1672531200,
  "l1_data": {
    "twap": "0x1234...",
    "max_return": "0x5678...",
    "reserve_price": "0x9abc..."
  },
  "on_chain_confirmation": {
    "block_number": 12345,
    "transaction_hash": "0xdef0...",
    "event_timestamp": 1672531260
  }
}
```

#### GET `/job_result/{job_id}`

Get detailed job result with complete proof data.

**Authentication:** Required (`X-API-Key` header)

**Response (200 OK):**

```json
{
  "job_id": "550e8400-e29b-41d4-a716-446655440000",
  "status": "Completed",
  "result": {
    "twap": "156789.42",
    "max_return": "0.087",
    "reserve_price": "145000.00"
  },
  "created_at": "2024-01-15T10:30:00Z",
  "completed_at": "2024-01-15T10:35:00Z"
}
```

#### POST `/batch_job_status`

Get status for multiple jobs in a single request.

**Authentication:** Required (`X-API-Key` header)

**Request Body:**

```json
{
  "job_ids": [
    "job-id-1",
    "job-id-2",
    "job-id-3"
  ]
}
```

**Response (200 OK):**

```json
{
  "jobs": [
    {
      "job_id": "job-id-1",
      "status": "Completed",
      "result": { "twap": "123.45" },
      "created_at": "2024-01-15T10:30:00Z",
      "completed_at": "2024-01-15T10:35:00Z"
    },
    {
      "job_id": "job-id-2",
      "status": "Pending",
      "result": null,
      "created_at": "2024-01-15T10:31:00Z",
      "completed_at": null
    }
  ],
  "not_found": ["job-id-3"]
}
```

**Limits:**
- Maximum 100 job IDs per request
- Empty request returns `400 Bad Request`
- Requests exceeding limit return `400 Bad Request`

## Authentication

### API Key Management

**Key Generation:**

API keys are generated using a secure random process and stored in the database.

**Creating API Keys:**

Using the CLI utility:

```bash
cd fossil-api/crates/server
cargo run --bin create_api_key -- --name "My Application"
```

Or via HTTP endpoint:

```bash
curl -X POST http://localhost:3000/api_key \
  -H "Content-Type: application/json" \
  -d '{"name": "My Application"}'
```

**Response:**

```json
{
  "api_key": "sk_test_1234567890abcdef",
  "name": "My Application",
  "created_at": "2024-01-15T10:00:00Z"
}
```

### Authentication Flow

```
1. Client Request
   ├── Include X-API-Key header
   └── Send to secured endpoint

2. Middleware Intercepts
   ├── Extract API key from header
   └── Validate format

3. Database Lookup
   ├── Query api_keys table
   └── Check if key exists

4. Authorization Decision
   ├── Valid key → Allow request
   └── Invalid key → Return 401

5. Request Processing
   └── Handler executes with authenticated context
```

### Security Considerations

**Best Practices:**
- Store API keys securely (environment variables, secret managers)
- Use HTTPS in production to prevent key interception
- Rotate keys periodically
- Implement rate limiting per API key (future enhancement)
- Log authentication failures for security monitoring

**Key Format:**
- Prefix: `sk_` (secret key)
- Environment indicator: `test_` or `live_`
- Random suffix: Base64-encoded random bytes

## Request Flow

### Complete Request-Response Flow

```
┌────────┐                                                     ┌──────────────┐
│        │  (1) POST /pricing_data                            │              │
│ Client ├────────────────────────────────────────────────────▶  Fossil API  │
│        │      X-API-Key: sk_test_xxx                        │              │
└────────┘      {program_id, params, vault_address}           └──────┬───────┘
    │                                                                 │
    │                                                                 │ (2) Auth
    │                                                                 │ Middleware
    │                                                                 ▼
    │                                                          ┌─────────────┐
    │                                                          │  Validate   │
    │                                                          │  API Key    │
    │                                                          └──────┬──────┘
    │                                                                 │
    │                                                                 │ (3) Handler
    │                                                                 ▼
    │                                                          ┌─────────────┐
    │                                                          │  Validate   │
    │                                                          │  Request    │
    │                                                          └──────┬──────┘
    │                                                                 │
    │                                                                 │ (4) Generate
    │                                                                 │ job_id
    │                                                                 ▼
    │                                                          ┌─────────────┐
    │                                                          │  Check DB   │
    │                                                          │  for existing│
    │                                                          └──────┬──────┘
    │                                                                 │
    │                                                                 │ (5) Create
    │                                                                 │ job record
    │                                                                 ▼
    │                                                          ┌─────────────┐
    │                                                          │  Database   │
    │                                                          │  INSERT     │
    │                                                          └──────┬──────┘
    │                                                                 │
    │  (6) Response {job_id, status: Pending}                        │
    │◀────────────────────────────────────────────────────────────────┘
    │
    │
    │                                   Background Task
    │                                   ┌──────────────┐
    │                                   │  Spawn async │
    │                                   │  processor   │
    │                                   └──────┬───────┘
    │                                          │
    │                                          │ (7) Transform
    │                                          │ request
    │                                          ▼
    │                                   ┌──────────────┐
    │                                   │ POST to      │
    │                                   │ Proving      │
    │                                   │ Service API  │
    │                                   └──────┬───────┘
    │                                          │
    │                                          │ (8) Queue job
    │                                          │ in SQS
    │                                          ▼
    │                                   ┌──────────────┐
    │                                   │  Proving     │
    │                                   │  Service     │
    │                                   │  responds    │
    │                                   └──────────────┘
    │
    │  (9) Poll for status
    │  GET /job_status/{job_id}
    ├─────────────────────────────────▶
    │
    │  (10) Response {job_id, status}
    │◀─────────────────────────────────
    │
    │  ... (polling continues) ...
    │
    │  (11) Job completed via event
    │  GET /job_status/{job_id}
    ├─────────────────────────────────▶
    │
    │  (12) Response {status: Completed, l1_data, confirmation}
    │◀─────────────────────────────────
    │
```

### Job State Transitions

```
┌─────────────┐
│   Created   │  Initial job creation
└──────┬──────┘
       │
       │ Job persisted to DB
       ▼
┌─────────────┐
│   Pending   │  Awaiting proving service processing
└──────┬──────┘
       │
       │ Forwarded to Proving Service
       ▼
┌─────────────┐
│  Processing │  Proof generation in progress
└──────┬──────┘
       │
       ├────────────────┬────────────────┐
       │                │                │
       │ Success        │ Timeout        │ Error
       ▼                ▼                ▼
┌─────────────┐  ┌─────────────┐  ┌─────────────┐
│  Completed  │  │  Retrying   │  │   Failed    │
│             │  │             │  │             │
│ l1_data     │  │ (3x max)    │  │ error_msg   │
│ confirmation│  └──────┬──────┘  └─────────────┘
└─────────────┘         │
                        │ Max retries
                        ▼
                  ┌─────────────┐
                  │   Failed    │
                  └─────────────┘
```

## Event Monitoring

The Fossil API includes a background event monitor that watches for proof verification events on StarkNet.

### Event Monitor Architecture

**`event_monitor.rs`:**

```rust
pub struct VaultEventMonitor {
    provider: StarknetProvider,
    db: Arc<OffchainProcessorDbConnection>,
    polling_interval: Duration,
    blocks_per_scan: u64,
}

impl VaultEventMonitor {
    pub async fn start_monitoring(&self) -> Result<()> {
        info!("Starting vault event monitoring");

        loop {
            if let Err(e) = self.process_fossil_callback_events().await {
                error!("Error processing events: {:?}", e);
            }

            sleep(self.polling_interval).await;
        }
    }
}
```

### Event Processing Flow

```
┌──────────────────────────────────────────────────────────────┐
│                    Event Monitor Loop                        │
│                                                              │
│  1. Get pending jobs from database                           │
│     ├── Query: status = 'Pending' AND vault_address IS NOT NULL│
│     └── Group by vault_address                               │
│                                                              │
│  2. Get current StarkNet block number                        │
│     └── RPC call: starknet_blockNumber                       │
│                                                              │
│  3. For each vault address:                                  │
│     ├── Calculate block range (current - blocks_per_scan)    │
│     ├── Fetch FossilCallbackSuccess events                   │
│     │   └── Event selector: 0x028daf8c...                    │
│     └── Parse event data                                     │
│         ├── L1 data (twap, max_return, reserve_price)        │
│         ├── Timestamp                                        │
│         └── Transaction hash and block number                │
│                                                              │
│  4. Match events to jobs:                                    │
│     ├── Compare vault_address                                │
│     ├── Compare timestamp (±60 seconds tolerance)            │
│     └── Update job with event data                           │
│                                                              │
│  5. Sleep for polling_interval                               │
│     └── Default: 30 seconds                                  │
│                                                              │
│  6. Repeat                                                   │
└──────────────────────────────────────────────────────────────┘
```

### Event Structure

**FossilCallbackSuccess Event:**

```rust
pub struct FossilCallbackSuccessEvent {
    pub l1_data: L1DataEvent,
    pub timestamp: u64,
    pub block_number: u64,
    pub transaction_hash: String,
}

pub struct L1DataEvent {
    pub twap: String,          // u256 as hex string
    pub max_return: String,    // u128 as hex string
    pub reserve_price: String, // u256 as hex string
}
```

**Event Data Format (from StarkNet):**

```
Event Keys:
[0] = 0x028daf8c351269d325d0652973bd8ae38ea95c476dcc11eaf0e2495c440689cb  (selector)

Event Data:
[0] = twap (felt252)
[1] = max_return (felt252)
[2] = reserve_price (felt252)
[3] = unknown1
[4] = unknown2
[5] = timestamp (u64 as felt252)
```

### Job Completion via Events

```rust
async fn complete_job_from_event(
    &self,
    job_id: &str,
    event: &FossilCallbackSuccessEvent,
) -> Result<()> {
    // Prepare L1 data from event
    let l1_data = L1Data {
        twap: event.l1_data.twap.clone(),
        max_return: event.l1_data.max_return.clone(),
        reserve_price: event.l1_data.reserve_price.clone(),
    };

    // Prepare on-chain confirmation data
    let on_chain_confirmation = OnChainConfirmation {
        block_number: event.block_number,
        transaction_hash: event.transaction_hash.clone(),
        event_timestamp: event.timestamp,
    };

    // Update job in database
    update_job_with_event_data(
        self.db.clone(),
        job_id,
        JobStatus::Completed,
        serde_json::to_value(&l1_data)?,
        serde_json::to_value(&on_chain_confirmation)?,
    ).await?;

    info!(
        job_id = job_id,
        twap = %l1_data.twap,
        max_return = %l1_data.max_return,
        reserve_price = %l1_data.reserve_price,
        "Job completed via FossilCallbackSuccess event"
    );

    Ok(())
}
```

### StarkNet Provider

**`starknet_provider.rs`:**

Custom StarkNet RPC client for event fetching:

```rust
pub struct StarknetProvider {
    rpc_url: String,
    client: Client,
}

pub struct EventFilter {
    pub from_block: Option<u64>,
    pub to_block: Option<u64>,
    pub address: Option<String>,
    pub keys: Option<Vec<Vec<String>>>,
}

impl StarknetProvider {
    pub fn new(rpc_url: &str) -> Result<Self> {
        Ok(Self {
            rpc_url: rpc_url.to_string(),
            client: Client::new(),
        })
    }

    pub async fn get_events(
        &self,
        filter: EventFilter,
        continuation_token: Option<String>,
        chunk_size: u64,
    ) -> Result<EventsPage> {
        // Prepare RPC request
        let params = json!({
            "filter": {
                "from_block": filter.from_block.map(|b| json!({"block_number": b})),
                "to_block": filter.to_block.map(|b| json!({"block_number": b})),
                "address": filter.address,
                "keys": filter.keys
            },
            "continuation_token": continuation_token,
            "chunk_size": chunk_size
        });

        // Call starknet_getEvents RPC method
        let response = self.client
            .post(&self.rpc_url)
            .json(&json!({
                "jsonrpc": "2.0",
                "method": "starknet_getEvents",
                "params": params,
                "id": 1
            }))
            .send()
            .await?;

        // Parse response
        let events_page: EventsPage = response.json().await?;
        Ok(events_page)
    }

    pub async fn block_number(&self) -> Result<u64> {
        // Call starknet_blockNumber RPC method
        // ...
    }
}
```

## Configuration

### Environment Variables

The Fossil API uses environment variables for all configuration:

**Database:**
```bash
OFFCHAIN_PROCESSOR_DATABASE_URL=postgresql://user:password@localhost:5434/fossil_api
```

**Proving Service:**
```bash
PROVING_SERVICE_URL=http://127.0.0.1:3001
```

**StarkNet RPC:**
```bash
STARKNET_RPC_URL=https://starknet-sepolia.g.alchemy.com/v2/YOUR_API_KEY
```

**Event Monitor:**
```bash
EVENT_MONITOR_POLLING_INTERVAL=30      # seconds
EVENT_MONITOR_BLOCKS_PER_SCAN=1000     # blocks to scan per poll
```

**CORS:**
```bash
ALLOWED_ORIGINS=http://localhost:3000,https://app.example.com
```

**Logging:**
```bash
RUST_LOG=info,server=debug,sqlx=error
```

### Environment Files

**`.env.local` (Development):**
```bash
OFFCHAIN_PROCESSOR_DATABASE_URL=postgresql://postgres:postgres@localhost:5434/fossil_api
PROVING_SERVICE_URL=http://127.0.0.1:3001
STARKNET_RPC_URL=http://localhost:5050
EVENT_MONITOR_POLLING_INTERVAL=10
EVENT_MONITOR_BLOCKS_PER_SCAN=100
ALLOWED_ORIGINS=http://localhost:3000
RUST_LOG=debug
```

**`.env.docker` (Docker Compose):**
```bash
OFFCHAIN_PROCESSOR_DATABASE_URL=postgresql://postgres:postgres@postgres-op:5432/fossil_api
PROVING_SERVICE_URL=http://proving-service:3001
STARKNET_RPC_URL=http://katana:5050
EVENT_MONITOR_POLLING_INTERVAL=30
EVENT_MONITOR_BLOCKS_PER_SCAN=1000
```

**`.env.production` (Production):**
```bash
OFFCHAIN_PROCESSOR_DATABASE_URL=${DATABASE_URL}
PROVING_SERVICE_URL=https://proving-service.internal.example.com
STARKNET_RPC_URL=https://starknet-mainnet.g.alchemy.com/v2/${ALCHEMY_API_KEY}
EVENT_MONITOR_POLLING_INTERVAL=30
EVENT_MONITOR_BLOCKS_PER_SCAN=1000
ALLOWED_ORIGINS=https://app.example.com
RUST_LOG=info,server=info,sqlx=warn
```

### Configuration Loading

Configuration is loaded in this order:
1. `.env.local` file (if present)
2. Environment variables
3. Default values (fallback)

```rust
// Example from main.rs
dotenv().ok();  // Load .env.local

let database_url = env::var("OFFCHAIN_PROCESSOR_DATABASE_URL")
    .expect("OFFCHAIN_PROCESSOR_DATABASE_URL must be set");

let proving_service_url = env::var("PROVING_SERVICE_URL")
    .unwrap_or_else(|_| "http://127.0.0.1:3001".to_string());
```

## Testing

### Test Structure

The Fossil API includes comprehensive tests at multiple levels:

**Unit Tests:**
- Handler logic tests
- Validation tests
- Query tests

**Integration Tests:**
- Full request/response cycle
- Database interaction
- API endpoint tests

### Test Infrastructure

**Test Context (`handlers/fixtures/mod.rs`):**

```rust
pub struct TestContext {
    db: Arc<OffchainProcessorDbConnection>,
    state: AppState,
}

impl TestContext {
    pub async fn new() -> Self {
        // Setup test database
        let db = setup_test_db().await;
        let state = AppState {
            offchain_processor_db: db.clone(),
        };
        Self { db, state }
    }

    pub async fn create_job(&self, job_id: &str, status: JobStatus) {
        create_job_request(self.db.clone(), job_id, status).await.unwrap();
    }

    pub async fn get_job_status(&self, job_id: &str) -> (StatusCode, Json<GetJobStatusResponseEnum>) {
        get_job_status(State(self.state.clone()), Path(job_id.to_string())).await
    }
}
```

### Example Tests

**Handler Test:**

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

**Validation Test:**

```rust
#[tokio::test]
async fn test_get_pricing_data_invalid_params() {
    let ctx = TestContext::new().await;

    let payload = PitchLakeJobRequest {
        program_id: "test-id".to_string(),
        params: PitchLakeJobRequestParams {
            twap: (100, 0),  // Invalid: end < start
            max_return: (0, 100),
            reserve_price: (0, 100),
        },
        vault_address: "0x456".to_string(),
    };

    let (status, Json(response)) = ctx.get_pricing_data(payload).await;

    assert_eq!(status, StatusCode::BAD_REQUEST);
    assert_eq!(
        response.message.unwrap(),
        "Invalid time range for TWAP calculation."
    );
}
```

**Batch Status Test:**

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

**Run all tests:**
```bash
cd fossil-api
make test
```

**Run specific test:**
```bash
cd fossil-api/crates/server
cargo test test_get_pricing_data_new_job
```

**Run with logging:**
```bash
RUST_LOG=debug cargo test -- --nocapture
```

### Test Database

Tests use Docker to spin up a PostgreSQL instance:

```bash
# Started automatically by test suite
docker run -d \
  --name fossil-api-test-db \
  -e POSTGRES_PASSWORD=postgres \
  -e POSTGRES_DB=fossil_api_test \
  -p 5434:5432 \
  postgres:14
```

## Next Steps

### Related Documentation

- **[Architecture Overview](overview.md)** - System-wide architecture and component relationships
- **[Data Flow Documentation](data-flow.md)** - End-to-end request processing flow
- **[Proving Service Architecture](proving-service.md)** - Proof generation service details
- **[StarkNet Contracts Architecture](starknet-contracts.md)** - On-chain verification contracts

### API Reference

- **[Fossil API Endpoints](../api-reference/fossil-api-endpoints.md)** - Complete API reference with examples
- **[Error Codes](../api-reference/error-codes.md)** - Error handling and status codes
- **[Authentication Guide](../guides/authentication.md)** - API key management and security

### Development Guides

- **[Local Development](../getting-started/local-development.md)** - Setting up development environment
- **[Database Migrations](../guides/database-migrations.md)** - Managing schema changes
- **[Testing Guide](../guides/testing.md)** - Writing and running tests
- **[Deployment Guide](../guides/deployment.md)** - Deploying to production

### Integration Examples

- **[Client SDKs](../examples/client-sdks/)** - Client libraries in various languages
- **[PitchLake Integration](../examples/pitchlake-integration.md)** - Vault integration example
- **[Webhook Integration](../examples/webhooks.md)** - Setting up callbacks
