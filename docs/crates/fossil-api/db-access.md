# db-access Crate Documentation

## Overview

The `db-access` crate provides the complete database layer for the Fossil API service. It handles PostgreSQL database connections, schema migrations, data models, authentication, and all database queries. This crate serves as the persistence layer for job tracking, API key management, and vault event processing.

**Location:** `fossil-api/crates/db-access/`

**Key Responsibilities:**
- PostgreSQL connection pooling and management
- Database schema migrations using SQLx
- API key storage and validation
- Job request lifecycle management
- Vault event data persistence
- Type-safe database queries

## Crate Structure

```
db-access/
├── src/
│   ├── lib.rs           # Connection management and initialization
│   ├── models.rs        # Data models and types
│   ├── auth.rs          # API key authentication functions
│   ├── queries.rs       # Job request queries
│   └── types.rs         # Type definitions (currently empty)
├── migrations/          # SQLx database migrations
│   ├── 20241024044528_job_requests.{up,down}.sql
│   ├── 20241024044538_api_keys.{up,down}.sql
│   ├── 20250112120000_add_updated_at.{up,down}.sql
│   └── 20250117140000_add_vault_event_fields.{up,down}.sql
└── Cargo.toml
```

### Module Organization

- **`lib`** - Database connection pooling, initialization, and migration
- **`models`** - Core data structures (ApiKey, JobRequest, JobStatus, L1Data, OnChainConfirmation)
- **`auth`** - API key management and validation
- **`queries`** - Job request CRUD operations
- **`types`** - Reserved for additional type definitions

## Data Models

### ApiKey

Represents an API key for authenticating access to the Fossil API.

```rust
#[derive(sqlx::FromRow, Debug)]
pub struct ApiKey {
    pub key: String,           // Unique API key string
    pub name: Option<String>,  // Optional descriptive name
}
```

**Database Table:** `api_keys`
- `id` - Auto-incrementing primary key
- `key` - Unique text identifier (indexed)
- `name` - Optional descriptive name
- `created_at` - Timestamp of key creation

### JobStatus

Enumeration representing the lifecycle state of a job request.

```rust
#[derive(sqlx::Type, Debug, PartialEq, Eq, Serialize, Deserialize, Clone)]
#[sqlx(type_name = "TEXT")]
pub enum JobStatus {
    Pending,    // Job created, awaiting processing
    Completed,  // Job successfully processed
    Failed,     // Job processing failed
}
```

Implements `Display` trait for string conversion.

### JobRequest

Primary model for tracking proof generation jobs.

```rust
#[derive(sqlx::FromRow, Debug)]
pub struct JobRequest {
    pub job_id: String,                               // Unique job identifier
    pub status: JobStatus,                            // Current job state
    pub vault_address: Option<String>,                // Associated vault address
    pub expected_timestamp: Option<i64>,              // Expected event timestamp
    pub created_at: chrono::NaiveDateTime,           // Job creation time
    pub updated_at: Option<chrono::NaiveDateTime>,   // Last update time
    pub result: Option<serde_json::Value>,            // Job result data (JSON)
    pub l1_data: Option<serde_json::Value>,          // L1 blockchain data (JSON)
    pub on_chain_confirmation: Option<serde_json::Value>,  // Chain confirmation (JSON)
}
```

**Database Table:** `job_requests`
- Primary key: `job_id`
- Status constraint: Must be 'Pending', 'Completed', or 'Failed'
- Indexes:
  - `idx_job_requests_pending_vault` - Partial index on (status, vault_address) for pending jobs
  - `idx_job_requests_timestamp` - Partial index on expected_timestamp

### L1Data

Structure for Layer 1 blockchain data associated with vault events.

```rust
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct L1Data {
    pub twap: String,           // Time-weighted average price (u256 as hex)
    pub max_return: String,     // Maximum return value (u128 as hex)
    pub reserve_price: String,  // Reserve price (u256 as hex)
}
```

Stored as JSONB in the `job_requests.l1_data` column.

### OnChainConfirmation

Structure for on-chain event confirmation data.

```rust
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct OnChainConfirmation {
    pub block_number: u64,         // Block number of event
    pub transaction_hash: String,  // Transaction hash
    pub event_timestamp: u64,      // Unix timestamp of event
}
```

Stored as JSONB in the `job_requests.on_chain_confirmation` column.

## Database Connection Management

### DbConnection

Low-level database connection wrapper with connection pooling.

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
```

**Configuration:**
- Maximum connections: 5
- Returns `Arc<DbConnection>` for thread-safe sharing

### OffchainProcessorDbConnection

High-level connection wrapper for the Offchain Processor (Fossil API).

```rust
pub struct OffchainProcessorDbConnection(Arc<DbConnection>);

impl OffchainProcessorDbConnection {
    // Create from environment variable
    pub async fn from_env() -> Result<Self>;

    // Create from existing connection
    pub async fn new(db_connection: Arc<DbConnection>) -> Result<Self>;

    // Run database migrations
    pub async fn migrate(&self) -> Result<()>;

    // Get underlying connection
    pub fn db_connection(&self) -> Arc<DbConnection>;
}
```

**Environment Variable:** `OFFCHAIN_PROCESSOR_DATABASE_URL`

**Usage Example:**
```rust
use db_access::OffchainProcessorDbConnection;

// Initialize from environment
let db = OffchainProcessorDbConnection::from_env().await?;

// Run migrations
db.migrate().await?;

// Get connection pool for queries
let pool = &db.db_connection().pool;
```

## Authentication Module

The `auth` module provides API key management functions.

### add_api_key

Adds a new API key to the database.

```rust
pub async fn add_api_key(
    db: Arc<OffchainProcessorDbConnection>,
    api_key: String,
    name: String,
) -> Result<(), sqlx::Error>
```

**Example:**
```rust
use db_access::auth::add_api_key;
use uuid::Uuid;

let api_key = Uuid::new_v4().to_string();
add_api_key(db.clone(), api_key.clone(), "Production API".to_string()).await?;
println!("Created API key: {}", api_key);
```

### find_api_key

Retrieves an API key from the database.

```rust
pub async fn find_api_key(
    db: Arc<OffchainProcessorDbConnection>,
    key: String,
) -> Result<ApiKey, sqlx::Error>
```

**Example:**
```rust
use db_access::auth::find_api_key;

match find_api_key(db.clone(), api_key_string).await {
    Ok(api_key) => println!("Found key: {:?}", api_key.name),
    Err(sqlx::Error::RowNotFound) => println!("Invalid API key"),
    Err(e) => println!("Database error: {}", e),
}
```

### validate_api_key

Validates that an API key exists in the database.

```rust
pub async fn validate_api_key(
    db: Arc<OffchainProcessorDbConnection>,
    api_key: &str,
) -> Result<(), sqlx::Error>
```

Returns `Ok(())` if valid, `Err(sqlx::Error::RowNotFound)` if invalid.

**Example:**
```rust
use db_access::auth::validate_api_key;

if validate_api_key(&db, incoming_key).await.is_ok() {
    // API key is valid, proceed with request
} else {
    // Return 401 Unauthorized
}
```

## Query Functions

The `queries` module provides all job request database operations.

### create_job_request

Creates a new job request with basic status.

```rust
pub async fn create_job_request(
    db: Arc<OffchainProcessorDbConnection>,
    job_id: &str,
    status: JobStatus,
) -> Result<(), sqlx::Error>
```

**Example:**
```rust
use db_access::queries::create_job_request;
use db_access::models::JobStatus;

let job_id = uuid::Uuid::new_v4().to_string();
create_job_request(db.clone(), &job_id, JobStatus::Pending).await?;
```

### create_job_request_with_vault

Creates a job request associated with a vault and expected timestamp.

```rust
pub async fn create_job_request_with_vault(
    db: Arc<OffchainProcessorDbConnection>,
    job_id: &str,
    status: JobStatus,
    vault_address: &str,
    expected_timestamp: i64,
) -> Result<(), sqlx::Error>
```

**Example:**
```rust
use db_access::queries::create_job_request_with_vault;
use db_access::models::JobStatus;

create_job_request_with_vault(
    db.clone(),
    &job_id,
    JobStatus::Pending,
    "0x1234567890abcdef1234567890abcdef12345678",
    1705449600, // Unix timestamp
).await?;
```

### get_job_request

Retrieves a job request by ID.

```rust
pub async fn get_job_request(
    db: Arc<OffchainProcessorDbConnection>,
    job_id: &str,
) -> Result<Option<JobRequest>, sqlx::Error>
```

**Example:**
```rust
use db_access::queries::get_job_request;

if let Some(job) = get_job_request(db.clone(), &job_id).await? {
    println!("Job status: {}", job.status);
    println!("Created at: {}", job.created_at);

    if let Some(vault) = job.vault_address {
        println!("Vault: {}", vault);
    }
}
```

### update_job_status

Updates job status and optionally stores result data.

```rust
pub async fn update_job_status(
    db: Arc<OffchainProcessorDbConnection>,
    job_id: &str,
    status: JobStatus,
    result: Option<serde_json::Value>,
) -> Result<(), sqlx::Error>
```

**Example:**
```rust
use db_access::queries::update_job_status;
use db_access::models::JobStatus;
use serde_json::json;

let result_data = json!({
    "proof_id": "proof_123",
    "verified": true
});

update_job_status(
    db.clone(),
    &job_id,
    JobStatus::Completed,
    Some(result_data)
).await?;
```

### update_job_with_event_data

Updates job with L1 data and on-chain confirmation.

```rust
pub async fn update_job_with_event_data(
    db: Arc<OffchainProcessorDbConnection>,
    job_id: &str,
    status: JobStatus,
    l1_data: serde_json::Value,
    on_chain_confirmation: serde_json::Value,
) -> Result<(), sqlx::Error>
```

**Example:**
```rust
use db_access::queries::update_job_with_event_data;
use db_access::models::{JobStatus, L1Data, OnChainConfirmation};
use serde_json::to_value;

let l1_data = L1Data {
    twap: "0x1234".to_string(),
    max_return: "0x5678".to_string(),
    reserve_price: "0x9abc".to_string(),
};

let confirmation = OnChainConfirmation {
    block_number: 12345678,
    transaction_hash: "0xabcd...".to_string(),
    event_timestamp: 1705449600,
};

update_job_with_event_data(
    db.clone(),
    &job_id,
    JobStatus::Completed,
    to_value(l1_data)?,
    to_value(confirmation)?
).await?;
```

### update_job_result

Legacy function to update job result with string status.

```rust
pub async fn update_job_result(
    db: Arc<OffchainProcessorDbConnection>,
    job_id: &str,
    status: &str,
    result: serde_json::Value,
) -> Result<()>
```

**Note:** Consider using `update_job_status` instead for type safety.

### get_multiple_job_requests

Retrieves multiple job requests by IDs, ordered by creation time.

```rust
pub async fn get_multiple_job_requests(
    db: Arc<OffchainProcessorDbConnection>,
    job_ids: &[String],
) -> Result<Vec<JobRequest>, sqlx::Error>
```

**Example:**
```rust
use db_access::queries::get_multiple_job_requests;

let job_ids = vec![
    "job-123".to_string(),
    "job-456".to_string(),
    "job-789".to_string(),
];

let jobs = get_multiple_job_requests(db.clone(), &job_ids).await?;
for job in jobs {
    println!("{}: {}", job.job_id, job.status);
}
```

### get_pending_jobs_with_vaults

Retrieves all pending jobs that have associated vault addresses, ordered by creation time.

```rust
pub async fn get_pending_jobs_with_vaults(
    db: Arc<OffchainProcessorDbConnection>,
) -> Result<Vec<JobRequest>, sqlx::Error>
```

**Example:**
```rust
use db_access::queries::get_pending_jobs_with_vaults;

// Used by event monitor to find jobs awaiting vault events
let pending_jobs = get_pending_jobs_with_vaults(db.clone()).await?;
for job in pending_jobs {
    println!("Processing job {} for vault {}",
        job.job_id,
        job.vault_address.unwrap()
    );
}
```

**Performance:** Uses the `idx_job_requests_pending_vault` index for efficient queries.

## Database Migrations

The crate uses SQLx's migration system to manage database schema evolution.

### Migration Files

Migrations are located in `fossil-api/crates/db-access/migrations/`

#### 1. Initial Job Requests Table (20241024044528)

**File:** `20241024044528_job_requests.up.sql`

```sql
-- Create job_requests table if it doesn't exist
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

-- Set the owner of the job_requests table
ALTER TABLE IF EXISTS public.job_requests
    OWNER TO postgres;
```

**Rollback:** `20241024044528_job_requests.down.sql`
```sql
DROP TABLE IF EXISTS public.job_requests;
```

#### 2. API Keys Table (20241024044538)

**File:** `20241024044538_api_keys.up.sql`

```sql
-- Create api_keys table if it doesn't exist
CREATE TABLE IF NOT EXISTS public.api_keys (
    id SERIAL PRIMARY KEY,
    key TEXT NOT NULL,
    name VARCHAR(255) NOT NULL,
    created_at TIMESTAMP WITHOUT TIME ZONE NOT NULL DEFAULT NOW(),
    CONSTRAINT api_keys_key_key UNIQUE (key)
);

-- Set the owner of the api_keys table
ALTER TABLE IF EXISTS public.api_keys
    OWNER TO postgres;
```

**Rollback:** `20241024044538_api_keys.down.sql`
```sql
DROP TABLE IF EXISTS public.api_keys;
```

#### 3. Add Updated Timestamp (20250112120000)

**File:** `20250112120000_add_updated_at.up.sql`

```sql
-- Add updated_at column to job_requests table
ALTER TABLE job_requests ADD COLUMN IF NOT EXISTS updated_at TIMESTAMP WITHOUT TIME ZONE;
```

**Rollback:** `20250112120000_add_updated_at.down.sql`
```sql
ALTER TABLE job_requests DROP COLUMN IF EXISTS updated_at;
```

#### 4. Add Vault Event Fields (20250117140000)

**File:** `20250117140000_add_vault_event_fields.up.sql`

```sql
-- Add vault address and event tracking fields to job_requests table
ALTER TABLE job_requests
ADD COLUMN vault_address TEXT,
ADD COLUMN expected_timestamp BIGINT,
ADD COLUMN l1_data JSONB,
ADD COLUMN on_chain_confirmation JSONB;

-- Create index for efficient lookup of pending jobs with vault addresses
CREATE INDEX idx_job_requests_pending_vault ON job_requests (status, vault_address)
WHERE status = 'Pending' AND vault_address IS NOT NULL;

-- Create index for timestamp-based lookups
CREATE INDEX idx_job_requests_timestamp ON job_requests (expected_timestamp)
WHERE expected_timestamp IS NOT NULL;
```

**Rollback:** `20250117140000_add_vault_event_fields.down.sql`
```sql
DROP INDEX IF EXISTS idx_job_requests_pending_vault;
DROP INDEX IF EXISTS idx_job_requests_timestamp;

ALTER TABLE job_requests
DROP COLUMN IF EXISTS vault_address,
DROP COLUMN IF EXISTS expected_timestamp,
DROP COLUMN IF EXISTS l1_data,
DROP COLUMN IF EXISTS on_chain_confirmation;
```

### Running Migrations

Migrations are automatically run using the `migrate()` method:

```rust
use db_access::OffchainProcessorDbConnection;

let db = OffchainProcessorDbConnection::from_env().await?;
db.migrate().await?;
```

**Manual Migration (CLI):**
```bash
# From fossil-api directory
sqlx migrate run --database-url $OFFCHAIN_PROCESSOR_DATABASE_URL
```

## Complete Database Schema

### Current Schema (After All Migrations)

```sql
-- API Keys Table
CREATE TABLE public.api_keys (
    id SERIAL PRIMARY KEY,
    key TEXT NOT NULL UNIQUE,
    name VARCHAR(255) NOT NULL,
    created_at TIMESTAMP WITHOUT TIME ZONE NOT NULL DEFAULT NOW()
);

-- Job Requests Table
CREATE TABLE public.job_requests (
    job_id VARCHAR(255) NOT NULL PRIMARY KEY,
    status VARCHAR(20) NOT NULL CHECK (
        status IN ('Completed', 'Pending', 'Failed')
    ),
    created_at TIMESTAMP WITHOUT TIME ZONE NOT NULL DEFAULT CURRENT_TIMESTAMP,
    updated_at TIMESTAMP WITHOUT TIME ZONE,
    result JSONB,
    vault_address TEXT,
    expected_timestamp BIGINT,
    l1_data JSONB,
    on_chain_confirmation JSONB
);

-- Indexes
CREATE INDEX idx_job_requests_pending_vault
    ON job_requests (status, vault_address)
    WHERE status = 'Pending' AND vault_address IS NOT NULL;

CREATE INDEX idx_job_requests_timestamp
    ON job_requests (expected_timestamp)
    WHERE expected_timestamp IS NOT NULL;

-- Unique Constraints
CREATE UNIQUE INDEX api_keys_key_key ON api_keys (key);
```

### Index Usage Patterns

**idx_job_requests_pending_vault:**
- Optimizes queries for pending jobs with vault addresses
- Used by: `get_pending_jobs_with_vaults()`
- Partial index (only indexes matching rows)

**idx_job_requests_timestamp:**
- Optimizes timestamp-based lookups
- Used for temporal queries and event correlation
- Partial index (only indexes non-null timestamps)

## Usage Examples

### Complete Job Lifecycle Example

```rust
use db_access::{
    OffchainProcessorDbConnection,
    models::{JobStatus, L1Data, OnChainConfirmation},
    queries::*,
};
use serde_json::to_value;
use uuid::Uuid;

#[tokio::main]
async fn main() -> eyre::Result<()> {
    // Initialize database connection
    let db = OffchainProcessorDbConnection::from_env().await?;
    db.migrate().await?;

    // Create a new job for a vault
    let job_id = Uuid::new_v4().to_string();
    let vault_address = "0x1234567890abcdef1234567890abcdef12345678";
    let expected_timestamp = 1705449600i64;

    create_job_request_with_vault(
        db.clone(),
        &job_id,
        JobStatus::Pending,
        vault_address,
        expected_timestamp
    ).await?;

    println!("Created job: {}", job_id);

    // Later: Update with event data when vault event is detected
    let l1_data = L1Data {
        twap: "0x1234".to_string(),
        max_return: "0x5678".to_string(),
        reserve_price: "0x9abc".to_string(),
    };

    let confirmation = OnChainConfirmation {
        block_number: 12345678,
        transaction_hash: "0xabcd...".to_string(),
        event_timestamp: 1705449600,
    };

    update_job_with_event_data(
        db.clone(),
        &job_id,
        JobStatus::Completed,
        to_value(l1_data)?,
        to_value(confirmation)?
    ).await?;

    // Retrieve and display final job state
    if let Some(job) = get_job_request(db.clone(), &job_id).await? {
        println!("Job completed: {}", job.job_id);
        println!("Status: {}", job.status);
        println!("L1 Data: {:?}", job.l1_data);
        println!("Confirmation: {:?}", job.on_chain_confirmation);
    }

    Ok(())
}
```

### API Key Management Example

```rust
use db_access::{
    OffchainProcessorDbConnection,
    auth::{add_api_key, find_api_key, validate_api_key},
};
use uuid::Uuid;

#[tokio::main]
async fn main() -> eyre::Result<()> {
    let db = OffchainProcessorDbConnection::from_env().await?;

    // Create a new API key
    let api_key = Uuid::new_v4().to_string();
    add_api_key(
        db.clone(),
        api_key.clone(),
        "Production Client".to_string()
    ).await?;

    println!("Generated API key: {}", api_key);

    // Validate the key
    match validate_api_key(&db, &api_key).await {
        Ok(_) => println!("API key is valid"),
        Err(_) => println!("API key is invalid"),
    }

    // Retrieve key details
    let key_info = find_api_key(db.clone(), api_key).await?;
    println!("Key name: {:?}", key_info.name);

    Ok(())
}
```

### Event Monitor Pattern

```rust
use db_access::{
    OffchainProcessorDbConnection,
    queries::{get_pending_jobs_with_vaults, update_job_with_event_data},
    models::JobStatus,
};
use tokio::time::{interval, Duration};

async fn monitor_vault_events(db: Arc<OffchainProcessorDbConnection>) -> eyre::Result<()> {
    let mut ticker = interval(Duration::from_secs(30));

    loop {
        ticker.tick().await;

        // Get all pending jobs waiting for vault events
        let pending_jobs = get_pending_jobs_with_vaults(db.clone()).await?;

        for job in pending_jobs {
            let vault = job.vault_address.unwrap();
            let expected_ts = job.expected_timestamp.unwrap();

            // Check blockchain for event at expected_ts for vault
            if let Some((l1_data, confirmation)) = check_vault_event(&vault, expected_ts).await? {
                update_job_with_event_data(
                    db.clone(),
                    &job.job_id,
                    JobStatus::Completed,
                    serde_json::to_value(l1_data)?,
                    serde_json::to_value(confirmation)?
                ).await?;

                println!("Updated job {} with event data", job.job_id);
            }
        }
    }
}
```

### Authentication Middleware Example

```rust
use axum::{
    extract::State,
    http::{HeaderMap, Request, StatusCode},
    middleware::Next,
    response::Response,
};
use db_access::auth::find_api_key;
use std::sync::Arc;

pub async fn auth_middleware(
    State(db): State<Arc<OffchainProcessorDbConnection>>,
    headers: HeaderMap,
    request: Request<axum::body::Body>,
    next: Next,
) -> Result<Response, StatusCode> {
    // Extract API key from headers
    let api_key = headers
        .get("x-api-key")
        .and_then(|v| v.to_str().ok())
        .ok_or(StatusCode::UNAUTHORIZED)?;

    // Validate API key exists
    find_api_key(db, api_key.to_string())
        .await
        .map_err(|_| StatusCode::UNAUTHORIZED)?;

    // Continue to handler
    Ok(next.run(request).await)
}
```

## Testing

The `db-access` crate does not currently contain unit tests. Testing is performed at the integration level in the `server` crate, which uses the database layer in real-world scenarios.

### Testing Strategy

**Integration Tests (in `server` crate):**
- Test database operations through API endpoints
- Use Docker PostgreSQL for test database
- Test migrations and schema integrity
- Validate query performance with realistic data

**Test Database Setup:**
```bash
cd fossil-api
make test  # Starts Docker PostgreSQL and runs tests
```

### Future Testing Improvements

Consider adding:
1. Unit tests for query functions with mock database
2. Migration rollback tests
3. Concurrent access tests
4. Performance benchmarks for indexed queries
5. Data validation tests

## Configuration

### Environment Variables

**Required:**
- `OFFCHAIN_PROCESSOR_DATABASE_URL` - PostgreSQL connection string for Fossil API database

**Format:**
```bash
OFFCHAIN_PROCESSOR_DATABASE_URL=postgresql://user:password@localhost:5432/fossil_api
```

### Connection Pool Settings

Current configuration in `DbConnection::new()`:
- **Max Connections:** 5
- **Connection Timeout:** Default (SQLx default is 30 seconds)
- **Idle Timeout:** Default (SQLx default is 10 minutes)

To customize, modify `fossil-api/crates/db-access/src/lib.rs`:

```rust
let pool = PgPoolOptions::new()
    .max_connections(10)
    .acquire_timeout(Duration::from_secs(30))
    .idle_timeout(Duration::from_secs(600))
    .connect(database_url)
    .await?;
```

### SQLx Configuration

The crate uses SQLx's compile-time query verification. To work with this:

**Prepare for Offline Mode:**
```bash
cd fossil-api/crates/db-access
cargo sqlx prepare --database-url $OFFCHAIN_PROCESSOR_DATABASE_URL
```

This creates `.sqlx/` directory with query metadata for offline compilation.

## Dependencies

From `fossil-api/crates/db-access/Cargo.toml`:

```toml
[dependencies]
eyre = { workspace = true }           # Error handling
sqlx = { workspace = true }           # PostgreSQL async driver
tracing = { workspace = true }        # Logging and diagnostics
chrono = { workspace = true }         # DateTime handling
serde = { workspace = true }          # Serialization
serde_json = { workspace = true }     # JSON handling
```

**Key SQLx Features Used:**
- PostgreSQL driver with native TLS
- Runtime: Tokio
- Macros for compile-time query verification
- Migration support

## Best Practices

### Error Handling

All query functions return `Result<T, sqlx::Error>` or `eyre::Result<T>`. Always handle:
- `sqlx::Error::RowNotFound` - Entity not found
- `sqlx::Error::Database` - Database-level errors (constraints, etc.)
- Connection errors - Network or pool issues

### Thread Safety

- `DbConnection` is wrapped in `Arc` for safe cloning across threads
- All query functions accept `Arc<OffchainProcessorDbConnection>`
- Connection pool handles concurrent access automatically

### Performance Optimization

1. **Use Indexes:** Queries for pending vault jobs use partial indexes
2. **Batch Operations:** `get_multiple_job_requests` for bulk lookups
3. **JSON Fields:** Use JSONB for flexible schema without migrations
4. **Connection Pooling:** Reuse connections efficiently (5 max connections)

### Migration Guidelines

When adding new migrations:
1. Use timestamped filenames: `YYYYMMDDHHMMSS_description.{up,down}.sql`
2. Always provide rollback (`.down.sql`) migration
3. Use `IF EXISTS` and `IF NOT EXISTS` for idempotency
4. Add indexes for query patterns
5. Test both up and down migrations

## Common Patterns

### Creating Jobs with Optional Data

```rust
// Simple job without vault
create_job_request(db.clone(), &job_id, JobStatus::Pending).await?;

// Job with vault tracking
create_job_request_with_vault(
    db.clone(),
    &job_id,
    JobStatus::Pending,
    vault_address,
    expected_timestamp
).await?;
```

### Updating Jobs Progressively

```rust
// Initial creation
create_job_request_with_vault(...).await?;

// Update with event data
update_job_with_event_data(..., l1_data, confirmation).await?;

// Final result update
update_job_status(..., JobStatus::Completed, result).await?;
```

### Querying with Type Safety

```rust
// Single job
let job = get_job_request(db.clone(), &job_id).await?;
if let Some(job) = job {
    match job.status {
        JobStatus::Pending => { /* handle pending */ },
        JobStatus::Completed => { /* handle completed */ },
        JobStatus::Failed => { /* handle failed */ },
    }
}

// Multiple jobs
let jobs = get_multiple_job_requests(db.clone(), &job_ids).await?;
for job in jobs {
    // Process each job
}
```

## Troubleshooting

### Common Issues

**Migration Errors:**
```
Error: error applying migration 20250117140000
```
**Solution:** Check database state, manually verify migrations table, rollback if needed.

**Connection Pool Exhausted:**
```
Error: timed out while waiting for an open connection
```
**Solution:** Increase `max_connections` or audit connection usage for leaks.

**API Key Not Found:**
```
sqlx::Error::RowNotFound
```
**Solution:** Ensure API key exists in database, check key format (UUIDs vs custom strings).

**Type Conversion Errors:**
```
error: mismatched types, expected JobStatus, found String
```
**Solution:** Use `JobStatus` enum, not string literals. Use `JobStatus::to_string()` for conversion.

## Related Documentation

- [Fossil API Server Documentation](docs/crates/fossil-api/server.md)
- [Fossil API Architecture](docs/architecture/fossil-api.md)
- [Database Setup Guide](docs/setup/database.md)

## Next Steps

### For New Developers

1. **Set up local database:**
   ```bash
   cd fossil-api
   make dev-services  # Start PostgreSQL in Docker
   ```

2. **Run migrations:**
   ```bash
   cd crates/db-access
   cargo sqlx migrate run --database-url $OFFCHAIN_PROCESSOR_DATABASE_URL
   ```

3. **Create test API key:**
   ```bash
   cd crates/server
   cargo run --bin create_api_key -- "My Test Key"
   ```

4. **Explore the code:**
   - Read `fossil-api/crates/db-access/src/models.rs`
   - Study `fossil-api/crates/db-access/src/queries.rs`
   - Review migration files for schema evolution

### For Contributors

**Adding New Features:**
1. Create migration files for schema changes
2. Update models in `models.rs`
3. Add query functions in `queries.rs`
4. Update this documentation
5. Add integration tests in `server` crate

**Optimization Opportunities:**
1. Add caching layer for API key validation
2. Implement prepared statement caching
3. Add database metrics and monitoring
4. Create database seeding scripts for development
5. Add comprehensive unit tests

## Additional Resources

- [SQLx Documentation](https://docs.rs/sqlx/)
- [PostgreSQL JSON Types](https://www.postgresql.org/docs/current/datatype-json.html)
- [Fossil API CLAUDE.md](CLAUDE.md)

---

**Last Updated:** 2025-10-06
**Crate Version:** Workspace version
**Documentation Maintainer:** Development Team
