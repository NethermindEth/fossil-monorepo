# DB Crate Documentation

## Overview

The `db` crate provides the database layer for the Proving Service. It handles PostgreSQL database connections, connection pooling, and data access for blockchain block header information. This crate is designed to query block header data from the indexer database, specifically focusing on retrieving base fee per gas information for proof generation.

**Location**: `/home/ametel/source/fossil-monorepo/proving-service/crates/db/`

**Purpose**:
- Manage PostgreSQL database connections with connection pooling
- Query blockchain block header data by time ranges
- Provide type-safe data models for block header information
- Support the proving service's need for historical gas fee data

## Crate Structure

```
proving-service/crates/db/
├── Cargo.toml          # Crate dependencies and metadata
└── src/
    ├── lib.rs          # Database connection management
    └── models.rs       # Data models and query functions
```

### Key Files

- **lib.rs**: Exports the `DbConnection` struct for managing database connection pools
- **models.rs**: Contains the `BlockHeader` data model and query functions for retrieving block data

## Dependencies

From `Cargo.toml`:

```toml
[dependencies]
eyre = { workspace = true }       # Error handling
tokio = { workspace = true }      # Async runtime
tracing = { workspace = true }    # Logging and instrumentation
sqlx = { workspace = true }       # PostgreSQL driver with async support

[dev-dependencies]
testcontainers = { version = "0.14" }  # Docker containers for testing
lazy_static = { workspace = true }     # Static initialization for test infrastructure
```

## Database Connection

### DbConnection Struct

The `DbConnection` struct manages a PostgreSQL connection pool using SQLx.

**File**: `/home/ametel/source/fossil-monorepo/proving-service/crates/db/src/lib.rs`

```rust
use eyre::{Result, eyre};
use sqlx::postgres::PgPoolOptions;
use sqlx::{Pool, Postgres};
use std::sync::Arc;

pub struct DbConnection {
    pub pool: Pool<Postgres>,
}

impl DbConnection {
    pub async fn new(database_url: &str) -> Result<Arc<Self>> {
        let pool = PgPoolOptions::new()
            .max_connections(5)
            .connect(database_url)
            .await
            .map_err(|e| eyre!("Failed to connect to database: {}", e))?;

        Ok(Arc::new(Self { pool }))
    }
}
```

### Connection Features

- **Connection Pooling**: Maintains a pool of up to 5 concurrent database connections
- **Thread-Safe**: Returns `Arc<DbConnection>` for safe sharing across threads
- **Error Handling**: Uses `eyre::Result` for rich error context
- **Async**: Built on tokio and SQLx for non-blocking database operations

### Usage Example

```rust
use db::DbConnection;
use std::sync::Arc;

#[tokio::main]
async fn main() -> eyre::Result<()> {
    let database_url = "postgresql://postgres:postgres@localhost:5435/postgres";
    let db: Arc<DbConnection> = DbConnection::new(database_url).await?;

    // Use db for queries...

    Ok(())
}
```

## Data Models

### BlockHeader

The `BlockHeader` struct represents a complete Ethereum block header record from the database.

**File**: `/home/ametel/source/fossil-monorepo/proving-service/crates/db/src/models.rs`

```rust
#[derive(sqlx::FromRow, Debug)]
pub struct BlockHeader {
    pub block_hash: Option<String>,
    pub number: i64,
    pub gas_limit: Option<i64>,
    pub gas_used: Option<i64>,
    pub nonce: Option<String>,
    pub transaction_root: Option<String>,
    pub base_fee_per_gas: Option<String>,  // Primary field for proof generation
    pub receipts_root: Option<String>,
    pub state_root: Option<String>,
    pub timestamp: Option<i64>,
}
```

### Field Descriptions

| Field | Type | Description |
|-------|------|-------------|
| `block_hash` | `Option<String>` | Unique hash identifier for the block (0x-prefixed hex) |
| `number` | `i64` | Block number (height) in the blockchain |
| `gas_limit` | `Option<i64>` | Maximum gas allowed in the block |
| `gas_used` | `Option<i64>` | Actual gas consumed by transactions in the block |
| `nonce` | `Option<String>` | Proof-of-work nonce value |
| `transaction_root` | `Option<String>` | Merkle root of transactions in the block |
| `base_fee_per_gas` | `Option<String>` | Base fee per gas unit (hex string) - **primary field used for proofs** |
| `receipts_root` | `Option<String>` | Merkle root of transaction receipts |
| `state_root` | `Option<String>` | Merkle root of the state trie |
| `timestamp` | `Option<i64>` | Unix timestamp when the block was mined |

**Note**: The `base_fee_per_gas` field is the primary focus for the proving service, used to generate proofs about historical gas prices.

## Query Functions

### get_block_headers_by_time_range

Retrieves complete block header information for all blocks within a specified time range.

**Signature**:
```rust
pub async fn get_block_headers_by_time_range(
    db: Arc<DbConnection>,
    start_timestamp: i64,
    end_timestamp: i64,
) -> Result<Vec<BlockHeader>, Error>
```

**Parameters**:
- `db`: Thread-safe reference to database connection
- `start_timestamp`: Unix timestamp for range start (inclusive)
- `end_timestamp`: Unix timestamp for range end (inclusive)

**Returns**: Vector of `BlockHeader` structs ordered by block number ascending

**SQL Query**:
```sql
SELECT
    block_hash,
    number,
    gas_limit,
    gas_used,
    base_fee_per_gas,
    nonce,
    transaction_root,
    receipts_root,
    state_root,
    timestamp
FROM blockheaders
WHERE CAST(timestamp AS BIGINT) BETWEEN $1 AND $2
ORDER BY number ASC
```

**Usage Example**:
```rust
use db::{DbConnection, models::get_block_headers_by_time_range};
use std::sync::Arc;

#[tokio::main]
async fn main() -> eyre::Result<()> {
    let db = DbConnection::new("postgresql://...").await?;

    // Get all blocks from January 1, 2025 to January 2, 2025
    let start = 1735689600; // 2025-01-01 00:00:00 UTC
    let end = 1735776000;   // 2025-01-02 00:00:00 UTC

    let headers = get_block_headers_by_time_range(db, start, end).await?;

    for header in headers {
        println!("Block {}: base_fee={:?}",
                 header.number,
                 header.base_fee_per_gas);
    }

    Ok(())
}
```

### get_block_base_fee_by_time_range

Optimized function to retrieve only the base fee per gas for blocks in a time range. This is faster than retrieving complete block headers when only gas fee data is needed.

**Signature**:
```rust
pub async fn get_block_base_fee_by_time_range(
    db: Arc<DbConnection>,
    start_timestamp: i64,
    end_timestamp: i64,
) -> Result<Vec<String>, Error>
```

**Parameters**:
- `db`: Thread-safe reference to database connection
- `start_timestamp`: Unix timestamp for range start (inclusive)
- `end_timestamp`: Unix timestamp for range end (inclusive)

**Returns**: Vector of base fee strings (hex-encoded) ordered by block number ascending

**SQL Query**:
```sql
SELECT base_fee_per_gas
FROM blockheaders
WHERE CAST(timestamp AS BIGINT) BETWEEN $1 AND $2
ORDER BY number ASC
```

**Usage Example**:
```rust
use db::{DbConnection, models::get_block_base_fee_by_time_range};

#[tokio::main]
async fn main() -> eyre::Result<()> {
    let db = DbConnection::new("postgresql://...").await?;

    let start = 1743249000;
    let end = 1743249120;

    let base_fees = get_block_base_fee_by_time_range(db, start, end).await?;

    println!("Retrieved {} base fee values", base_fees.len());
    for (i, fee) in base_fees.iter().enumerate() {
        println!("Block {}: {}", i, fee);
    }

    Ok(())
}
```

**Performance Note**: This function is the recommended choice for production code when only base fee data is needed, as it minimizes data transfer and query processing time.

## Database Schema

### blockheaders Table

The complete schema for the `blockheaders` table as defined in `/home/ametel/source/fossil-monorepo/proving-service/tests/init.sql`:

```sql
CREATE TABLE blockheaders (
    block_hash CHAR(66) UNIQUE,
    number BIGINT PRIMARY KEY,
    gas_limit BIGINT NOT NULL,
    gas_used BIGINT NOT NULL,
    base_fee_per_gas VARCHAR(78),
    nonce VARCHAR(78) NOT NULL,
    transaction_root CHAR(66),
    receipts_root CHAR(66),
    state_root CHAR(66),
    parent_hash VARCHAR(66),
    miner VARCHAR(42),
    logs_bloom VARCHAR(1024),
    difficulty VARCHAR(78),
    totalDifficulty VARCHAR(78),
    sha3_uncles VARCHAR(66),
    timestamp VARCHAR(100),
    extra_data VARCHAR(1024),
    mix_hash VARCHAR(66),
    withdrawals_root VARCHAR(66),
    blob_gas_used VARCHAR(78),
    excess_blob_gas VARCHAR(78),
    parent_beacon_block_root VARCHAR(66),
    requests_hash TEXT
);
```

### Key Indexes

- **Primary Key**: `number` - Block number provides unique identification
- **Unique Constraint**: `block_hash` - Ensures no duplicate block hashes

### Important Notes

1. **Timestamp Type**: The `timestamp` column is stored as `VARCHAR(100)` in the database but cast to `BIGINT` in queries
2. **Hex Encoding**: Many fields (hashes, addresses, base_fee) are stored as hex-encoded strings with `0x` prefix
3. **Additional Fields**: The database schema contains many fields beyond what the `BlockHeader` model exposes (miner, difficulty, etc.)
4. **Read-Only Access**: This crate queries the indexer database which is populated by a separate indexing service

## Migrations

**Note**: The proving service's `db` crate does not manage database migrations. The database schema is owned and managed by the **fossil indexer** service, which populates the `blockheaders` table.

The proving service has **read-only access** to this database via the `INDEXER_DATABASE_URL` environment variable.

For testing purposes, the schema is created in test containers using the SQL script at `/home/ametel/source/fossil-monorepo/proving-service/tests/init.sql`.

## Usage Examples

### Basic Connection and Query

```rust
use db::{DbConnection, models::{get_block_headers_by_time_range, BlockHeader}};
use std::sync::Arc;

#[tokio::main]
async fn main() -> eyre::Result<()> {
    // Initialize tracing for debugging
    tracing_subscriber::fmt::init();

    // Connect to database
    let database_url = std::env::var("INDEXER_DATABASE_URL")
        .expect("INDEXER_DATABASE_URL must be set");
    let db = DbConnection::new(&database_url).await?;

    // Query blocks from the last hour
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)?
        .as_secs() as i64;
    let one_hour_ago = now - 3600;

    let headers = get_block_headers_by_time_range(
        db.clone(),
        one_hour_ago,
        now
    ).await?;

    tracing::info!("Found {} blocks in the last hour", headers.len());

    Ok(())
}
```

### Calculating Average Gas Fee

```rust
use db::{DbConnection, models::get_block_base_fee_by_time_range};

async fn calculate_average_base_fee(
    db: Arc<DbConnection>,
    start: i64,
    end: i64,
) -> eyre::Result<f64> {
    let base_fees = get_block_base_fee_by_time_range(db, start, end).await?;

    let total: u128 = base_fees
        .iter()
        .filter_map(|fee| {
            // Remove "0x" prefix and parse hex
            fee.strip_prefix("0x")
                .and_then(|s| u128::from_str_radix(s, 16).ok())
        })
        .sum();

    let average = if base_fees.is_empty() {
        0.0
    } else {
        total as f64 / base_fees.len() as f64
    };

    tracing::info!(
        "Average base fee over {} blocks: {:.2} wei",
        base_fees.len(),
        average
    );

    Ok(average)
}
```

### Sharing Connection Across Tasks

```rust
use db::DbConnection;
use std::sync::Arc;
use tokio::task;

#[tokio::main]
async fn main() -> eyre::Result<()> {
    let db = DbConnection::new("postgresql://...").await?;

    // Spawn multiple tasks sharing the same connection pool
    let mut handles = vec![];

    for i in 0..3 {
        let db_clone = db.clone();
        let handle = task::spawn(async move {
            let start = 1743249000 + (i * 100);
            let end = start + 100;

            get_block_base_fee_by_time_range(db_clone, start, end).await
        });
        handles.push(handle);
    }

    // Wait for all tasks to complete
    for handle in handles {
        let fees = handle.await??;
        println!("Retrieved {} fees", fees.len());
    }

    Ok(())
}
```

## Testing

### Test Infrastructure

The `db` crate uses **testcontainers** to spin up ephemeral PostgreSQL databases for each test. This ensures:
- Tests are isolated and don't interfere with each other
- No manual database setup required
- Tests can run in CI/CD environments
- Each test gets a clean database state

### Test Setup

**File**: `/home/ametel/source/fossil-monorepo/proving-service/crates/db/src/models.rs`

```rust
#[cfg(test)]
mod tests {
    use super::*;
    use lazy_static::lazy_static;
    use testcontainers::{Container, clients::Cli, images::postgres::Postgres};

    lazy_static! {
        static ref DOCKER: Cli = Cli::default();
    }

    struct TestDb {
        db: Arc<DbConnection>,
        _container: Container<'static, Postgres>,
    }

    async fn setup_db() -> TestDb {
        let container = DOCKER.run(Postgres::default());
        let port = container.get_host_port_ipv4(5432);
        let connection_string = format!(
            "postgres://postgres:postgres@localhost:{}/postgres",
            port
        );

        let pool = sqlx::postgres::PgPoolOptions::new()
            .max_connections(5)
            .connect(&connection_string)
            .await
            .expect("Failed to create database pool");

        // Create table schema
        sqlx::query(
            r#"
            CREATE TABLE IF NOT EXISTS blockheaders (
                block_hash TEXT,
                number BIGINT,
                gas_limit BIGINT,
                gas_used BIGINT,
                nonce TEXT,
                transaction_root TEXT,
                base_fee_per_gas TEXT,
                receipts_root TEXT,
                state_root TEXT,
                timestamp BIGINT
            )
            "#,
        )
        .execute(&pool)
        .await
        .expect("Failed to create block_headers table");

        // Insert test data
        sqlx::query(
            r#"
            INSERT INTO blockheaders
            (block_hash, number, gas_limit, gas_used, nonce, transaction_root,
             base_fee_per_gas, receipts_root, state_root, timestamp)
            VALUES
            ('0x1', 8006481, 100, 50, '0xnonce1', '0xtx1', '0xa0ba15',
             '0xreceipt1', '0xstate1', 1743249000),
            ('0x2', 8006482, 100, 50, '0xnonce2', '0xtx2', '0x9ed346',
             '0xreceipt2', '0xstate2', 1743249060),
            ('0x3', 8006483, 100, 50, '0xnonce3', '0xtx3', '0xa85f1d',
             '0xreceipt3', '0xstate3', 1743249090),
            ('0x4', 8006484, 100, 50, '0xnonce4', '0xtx4', '0x9aeae1',
             '0xreceipt4', '0xstate4', 1743249110),
            ('0x5', 8006485, 100, 50, '0xnonce5', '0xtx5', '0x9fda11',
             '0xreceipt5', '0xstate5', 1743249120)
            "#,
        )
        .execute(&pool)
        .await
        .expect("Failed to insert sample data");

        let db = Arc::new(DbConnection { pool });

        TestDb {
            db,
            _container: container,
        }
    }
}
```

### Test Examples

#### Test 1: Retrieve All Headers in Range

```rust
#[tokio::test]
async fn test_should_get_all_block_headers_by_time_range() {
    let test_db = setup_db().await;

    let headers = get_block_headers_by_time_range(
        test_db.db,
        1743249000,
        1743249120
    )
    .await
    .unwrap();

    assert_eq!(headers.len(), 5);
    assert_eq!(headers[0].number, 8006481);
    assert_eq!(headers[1].number, 8006482);
    assert_eq!(headers[2].number, 8006483);
    assert_eq!(headers[3].number, 8006484);
    assert_eq!(headers[4].number, 8006485);
}
```

#### Test 2: Partial Range Query

```rust
#[tokio::test]
async fn test_should_only_get_partial_block_headers_by_time_range() {
    let test_db = setup_db().await;

    let headers = get_block_headers_by_time_range(
        test_db.db,
        1743249000,
        1743249100
    )
    .await
    .unwrap();

    assert_eq!(headers.len(), 3);
    assert_eq!(headers[0].number, 8006481);
    assert_eq!(headers[1].number, 8006482);
    assert_eq!(headers[2].number, 8006483);
}
```

#### Test 3: No Results

```rust
#[tokio::test]
async fn test_should_get_block_headers_by_time_range_with_no_results() {
    let test_db = setup_db().await;

    let headers = get_block_headers_by_time_range(
        test_db.db,
        1643249000,
        1643249100
    )
    .await
    .unwrap();

    assert_eq!(headers.len(), 0);
}
```

#### Test 4: Base Fee Retrieval

```rust
#[tokio::test]
async fn test_should_get_all_block_base_fee_by_time_range() {
    let test_db = setup_db().await;

    let base_fees = get_block_base_fee_by_time_range(
        test_db.db,
        1743249000,
        1743249120
    )
    .await
    .unwrap();

    assert_eq!(base_fees.len(), 5);
    assert_eq!(base_fees[0], "0xa0ba15");
    assert_eq!(base_fees[1], "0x9ed346");
    assert_eq!(base_fees[2], "0xa85f1d");
    assert_eq!(base_fees[3], "0x9aeae1");
    assert_eq!(base_fees[4], "0x9fda11");
}
```

### Running Tests

From the proving service directory:

```bash
# Run all tests (includes Docker setup)
cd proving-service && make test

# Run only db crate tests
cd proving-service/crates/db && cargo test

# Run tests with output
cd proving-service/crates/db && cargo test -- --nocapture

# Run a specific test
cd proving-service/crates/db && cargo test test_should_get_all_block_headers_by_time_range
```

**Prerequisites**: Docker must be running for testcontainers to work.

## Configuration

### Environment Variables

The `db` crate requires a PostgreSQL connection string. In the proving service context, this is configured via:

```bash
# Read-only access to the indexer database (populated by fossil indexer)
INDEXER_DATABASE_URL=postgresql://postgres:postgres@localhost:5433/postgres
```

### Connection String Format

```
postgresql://[user]:[password]@[host]:[port]/[database]
```

**Example connection strings**:

```bash
# Local development (default)
INDEXER_DATABASE_URL=postgresql://postgres:postgres@localhost:5433/postgres

# Docker environment
INDEXER_DATABASE_URL=postgresql://postgres:postgres@indexer_db:5432/postgres

# Production (Sepolia testnet)
INDEXER_DATABASE_URL=postgresql://user:password@sepolia-indexer.example.com:5432/fossil
```

### Environment File Locations

As per the project's centralized environment management:

1. **Root level**: `/home/ametel/source/fossil-monorepo/.env.local`
   - Contains `INDEXER_DATABASE_URL` and other shared variables

2. **Service level**: `/home/ametel/source/fossil-monorepo/proving-service/.env.local`
   - Service-specific overrides only

### Configuration Example

Create `.env.local` in the proving service directory:

```bash
# Database URLs
INDEXER_DATABASE_URL=postgresql://postgres:postgres@localhost:5433/postgres
PROVING_SERVICE_DATABASE_URL=postgresql://postgres:postgres@localhost:5435/postgres

# Logging
RUST_LOG=info,db=debug
```

Load in your application:

```rust
use db::DbConnection;

#[tokio::main]
async fn main() -> eyre::Result<()> {
    // Load environment variables
    dotenv::dotenv().ok();

    // Initialize logging
    tracing_subscriber::fmt::init();

    // Connect to database
    let database_url = std::env::var("INDEXER_DATABASE_URL")?;
    let db = DbConnection::new(&database_url).await?;

    tracing::info!("Successfully connected to database");

    Ok(())
}
```

## Architecture Context

### Database Separation

The proving service uses **two separate databases**:

1. **Indexer Database** (via `db` crate)
   - **Access**: Read-only
   - **Managed by**: Fossil Indexer service
   - **Contains**: Block headers populated by indexer
   - **Environment Variable**: `INDEXER_DATABASE_URL`
   - **Purpose**: Query historical blockchain data

2. **Proving Service Database** (not in this crate)
   - **Access**: Read/write
   - **Managed by**: Proving service
   - **Contains**: Job tracking, proof status, etc.
   - **Environment Variable**: `PROVING_SERVICE_DATABASE_URL`
   - **Purpose**: Track proof generation jobs

### Integration with Other Crates

The `db` crate is used by:

- **message-handler**: Retrieves block data for proof composition
- **proving-service**: Provides historical gas fee data for API endpoints
- **starknet-handler**: Accesses blockchain data for Starknet proof generation

### Data Flow

```
Fossil Indexer
     |
     v
blockheaders table (INDEXER_DATABASE_URL)
     |
     v
db::DbConnection
     |
     v
get_block_headers_by_time_range()
get_block_base_fee_by_time_range()
     |
     v
Message Handler / Proving Service
```

## Performance Considerations

### Connection Pooling

- Maximum 5 concurrent connections per `DbConnection` instance
- Connections are reused efficiently via SQLx's pool management
- Safe to clone `Arc<DbConnection>` across multiple tasks/threads

### Query Optimization

1. **Use `get_block_base_fee_by_time_range()` when possible**
   - Retrieves only the needed data (base fee)
   - Reduces network transfer and parsing overhead
   - Faster than `get_block_headers_by_time_range()`

2. **Time range queries are indexed**
   - Queries use timestamp range filters
   - Results ordered by block number (primary key)
   - Efficient for sequential block access

3. **Batch queries when possible**
   - Retrieve larger time ranges in single queries
   - Avoid N+1 query patterns
   - Use async concurrency for independent queries

### Example: Efficient Batch Query

```rust
// Good: Single query for range
let fees = get_block_base_fee_by_time_range(db, start, end).await?;

// Bad: Multiple queries in loop
for timestamp in start..end {
    let fees = get_block_base_fee_by_time_range(db, timestamp, timestamp).await?;
}
```

## Error Handling

### Error Types

- **Connection Errors**: `eyre::Error` wrapping SQLx connection failures
- **Query Errors**: `sqlx::Error` from database operations
- **Type Conversion Errors**: Implicit through SQLx's `FromRow` derive

### Error Handling Patterns

```rust
use db::models::get_block_headers_by_time_range;

// Pattern 1: Propagate with ?
async fn my_function(db: Arc<DbConnection>) -> eyre::Result<()> {
    let headers = get_block_headers_by_time_range(db, start, end).await?;
    Ok(())
}

// Pattern 2: Handle specific errors
async fn my_function_with_handling(db: Arc<DbConnection>) -> eyre::Result<()> {
    match get_block_headers_by_time_range(db, start, end).await {
        Ok(headers) => {
            tracing::info!("Found {} headers", headers.len());
            Ok(())
        }
        Err(e) => {
            tracing::error!("Database query failed: {}", e);
            Err(eyre::eyre!("Failed to retrieve block headers: {}", e))
        }
    }
}

// Pattern 3: Provide default on error
async fn my_function_with_default(db: Arc<DbConnection>) -> Vec<BlockHeader> {
    get_block_headers_by_time_range(db, start, end)
        .await
        .unwrap_or_else(|e| {
            tracing::warn!("Query failed, returning empty: {}", e);
            vec![]
        })
}
```

## Logging and Tracing

The `db` crate uses the `tracing` library for structured logging.

### Query Logging

Each query function logs at the `debug` level:

```rust
tracing::debug!(
    "Getting block headers by time range: {} to {}",
    start_timestamp,
    end_timestamp
);
```

### Enabling Debug Logs

```bash
# Enable all db logs
RUST_LOG=db=debug cargo run

# Enable query-level logs
RUST_LOG=db=trace,sqlx=debug cargo run

# Production: Info level only
RUST_LOG=info cargo run
```

### Example Log Output

```
2025-01-15T10:30:45.123Z DEBUG db::models: Getting block headers by time range: 1743249000 to 1743249120
2025-01-15T10:30:45.234Z DEBUG sqlx::query: SELECT block_hash, number, ... FROM blockheaders WHERE ...
2025-01-15T10:30:45.456Z INFO  my_app: Retrieved 5 block headers
```

## Best Practices

### 1. Reuse Connection Pools

```rust
// Good: Create once, share via Arc
let db = DbConnection::new(&url).await?;
let db_clone1 = db.clone();
let db_clone2 = db.clone();

// Bad: Creating multiple connections
let db1 = DbConnection::new(&url).await?;
let db2 = DbConnection::new(&url).await?;
```

### 2. Use Appropriate Query Functions

```rust
// Good: Use base fee function when only fees needed
let fees = get_block_base_fee_by_time_range(db, start, end).await?;

// Bad: Fetching full headers when only fees needed
let headers = get_block_headers_by_time_range(db, start, end).await?;
let fees: Vec<_> = headers.iter()
    .filter_map(|h| h.base_fee_per_gas.clone())
    .collect();
```

### 3. Handle Optional Fields

```rust
let headers = get_block_headers_by_time_range(db, start, end).await?;

for header in headers {
    // Good: Handle Option properly
    if let Some(base_fee) = header.base_fee_per_gas {
        process_base_fee(base_fee);
    }

    // Bad: Unwrapping without checking
    // let fee = header.base_fee_per_gas.unwrap(); // May panic!
}
```

### 4. Use Structured Logging

```rust
// Good: Structured logging with context
tracing::info!(
    block_count = headers.len(),
    start_timestamp = start,
    end_timestamp = end,
    "Retrieved block headers"
);

// Bad: String formatting
println!("Got {} headers from {} to {}", headers.len(), start, end);
```

### 5. Test with Realistic Data

```rust
#[tokio::test]
async fn test_with_realistic_base_fees() {
    let test_db = setup_db().await;

    // Insert realistic base fee values (actual Ethereum values)
    // instead of simple test values

    let fees = get_block_base_fee_by_time_range(db, start, end).await?;

    // Verify realistic properties
    assert!(fees.iter().all(|f| f.starts_with("0x")));
    assert!(fees.len() > 0);
}
```

## Troubleshooting

### Common Issues

#### 1. Connection Refused

```
Error: Failed to connect to database: Connection refused (os error 111)
```

**Solutions**:
- Ensure PostgreSQL is running: `docker ps`
- Check port configuration in `INDEXER_DATABASE_URL`
- Verify firewall/network settings
- For tests: Ensure Docker daemon is running

#### 2. Authentication Failed

```
Error: Failed to connect to database: password authentication failed
```

**Solutions**:
- Verify credentials in connection string
- Check PostgreSQL user permissions
- Ensure database exists

#### 3. Table Not Found

```
Error: relation "blockheaders" does not exist
```

**Solutions**:
- Verify database schema is initialized
- For production: Ensure indexer has run and populated data
- For tests: Check `setup_db()` creates table correctly

#### 4. Timestamp Parsing Issues

```
Error: Invalid timestamp value
```

**Solutions**:
- Ensure timestamps are Unix timestamps (seconds since epoch)
- Check timestamp values are within valid range
- Verify timestamp column in database is correctly populated

#### 5. Connection Pool Exhausted

```
Error: Timed out waiting for connection from pool
```

**Solutions**:
- Increase `max_connections` in `DbConnection::new()`
- Review application for connection leaks
- Check if queries are taking too long

### Debug Strategies

#### Enable SQL Query Logging

```bash
# See all SQL queries and their execution time
RUST_LOG=sqlx::query=debug cargo run
```

#### Test Database Connection

```rust
#[tokio::test]
async fn test_connection() {
    let db = DbConnection::new("postgresql://...").await.unwrap();

    // Test basic query
    let result: (i64,) = sqlx::query_as("SELECT COUNT(*) FROM blockheaders")
        .fetch_one(&db.pool)
        .await
        .unwrap();

    println!("Total blocks in database: {}", result.0);
}
```

#### Inspect Test Data

```rust
#[tokio::test]
async fn inspect_test_data() {
    let test_db = setup_db().await;

    let all_headers = get_block_headers_by_time_range(
        test_db.db,
        0,
        i64::MAX
    ).await.unwrap();

    for header in all_headers {
        println!("{:?}", header);
    }
}
```

## Next Steps

### Related Documentation

- [Message Handler Crate](./message-handler.md) - Uses db crate to fetch block data for proof generation
- [Proving Service Crate](./proving-service.md) - HTTP API that exposes block data endpoints
- [Starknet Handler Crate](./starknet-handler.md) - Uses db crate for Starknet proof generation

### Related Services

- **Fossil Indexer**: Populates the blockheaders table that this crate queries
- **PostgreSQL Documentation**: [SQLx documentation](https://docs.rs/sqlx/)
- **Testcontainers**: [Testcontainers Rust](https://docs.rs/testcontainers/)

### Further Reading

- [Workspace Architecture](../../architecture/workspace.md) - Overall monorepo structure
- [Database Strategy](../../architecture/database-strategy.md) - Multi-database architecture
- [Testing Guide](../../guides/testing.md) - Testing patterns and best practices
- [Environment Configuration](../../guides/environment.md) - Environment variable management

### Extending the Crate

To add new query functions:

1. Add the function to `/home/ametel/source/fossil-monorepo/proving-service/crates/db/src/models.rs`
2. Follow the pattern of existing functions (use `Arc<DbConnection>`, return `Result`)
3. Add comprehensive tests using `setup_db()`
4. Document the function with examples
5. Update this documentation with the new function

Example:

```rust
pub async fn get_block_by_number(
    db: Arc<DbConnection>,
    block_number: i64,
) -> Result<Option<BlockHeader>, Error> {
    let header = sqlx::query_as(
        r#"
        SELECT block_hash, number, gas_limit, gas_used,
               base_fee_per_gas, nonce, transaction_root,
               receipts_root, state_root, timestamp
        FROM blockheaders
        WHERE number = $1
        "#,
    )
    .bind(block_number)
    .fetch_optional(&db.pool)
    .await?;

    Ok(header)
}

#[cfg(test)]
mod tests {
    #[tokio::test]
    async fn test_get_block_by_number() {
        let test_db = setup_db().await;

        let header = get_block_by_number(test_db.db, 8006481)
            .await
            .unwrap();

        assert!(header.is_some());
        assert_eq!(header.unwrap().number, 8006481);
    }
}
```

---

**Documentation Version**: 1.0
**Last Updated**: 2025-10-06
**Crate Version**: 0.1.0
**Maintainer**: Fossil Team
