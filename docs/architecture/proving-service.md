# Proving Service Architecture

This document provides a comprehensive overview of the Proving Service architecture, including its crate organization, component relationships, data flow, and integration with RISC Zero and StarkNet.

## Table of Contents
- [Overview](#overview)
- [Crate Organization](#crate-organization)
- [Architecture Diagram](#architecture-diagram)
- [Database Layer (db crate)](#database-layer-db-crate)
- [Message Handler (message-handler crate)](#message-handler-message-handler-crate)
- [Proving Service API (proving-service crate)](#proving-service-api-proving-service-crate)
- [StarkNet Handler (starknet-handler crate)](#starknet-handler-starknet-handler-crate)
- [Proof Generation Flow](#proof-generation-flow)
- [Queue Management](#queue-management)
- [Configuration](#configuration)
- [Next Steps](#next-steps)

## Overview

The Proving Service is the backend system responsible for RISC Zero proof generation and StarkNet verification. It handles the complete lifecycle of cryptographic proof generation, from receiving job requests to submitting verified proofs onchain.

### Purpose and Responsibilities

The Proving Service is responsible for:

1. **Job Management**: Receiving proof generation requests via HTTP API
2. **Async Processing**: Queue-based asynchronous job processing via AWS SQS
3. **Proof Generation**: Creating RISC Zero proofs via Bonsai API for financial calculations
4. **StarkNet Integration**: Submitting proofs to StarkNet for onchain verification
5. **Data Fetching**: Querying historical blockchain data from indexer database
6. **Status Tracking**: Maintaining job state and providing status updates

### Technology Stack

| Component | Technology | Version | Purpose |
|-----------|-----------|---------|---------|
| Language | Rust | Edition 2024 | Core implementation |
| Runtime | Tokio | 1.39+ | Async runtime |
| HTTP Framework | Axum | 0.8+ | REST API server |
| Database | PostgreSQL | 14+ | Job tracking and metadata |
| ORM | SQLx | 0.8+ | Database queries |
| Message Queue | AWS SQS | - | Async job processing |
| Proof System | RISC Zero | 2.3.1 | Zero-knowledge proofs |
| Proof API | Bonsai | - | Remote proof generation |
| StarkNet RPC | starknet-rs | 0.16+ | Blockchain integration |

### Key Features

- **Asynchronous Processing**: Decouples job submission from proof generation
- **Retry Logic**: Automatic retries with exponential backoff for transient failures
- **Concurrent Proof Limiting**: Prevents Bonsai API contention with semaphore-based rate limiting
- **Graceful Shutdown**: Clean termination of running jobs on shutdown signal
- **Comprehensive Testing**: Unit and integration tests with Docker-based test infrastructure

### ⚠️ Important: Bonsai Prover Transition

**Current State:** The Pitchlake Coprocessor currently relies on **Bonsai**, a managed proving service operated by the RISC0 team.

**Upcoming Changes:** RISC0 has announced plans to deprecate Bonsai in favor of **Boundless**, a decentralized and trustless proving marketplace.

**Migration Considerations:**

1. **Boundless Migration (RISC0 Continuation):**
   - Requires modifications to proof submission and verification workflows
   - Changes to proof batching, verification latency, and cost structures
   - Security guarantees remain equivalent but require additional protocol-level coordination
   - Integration complexity: Non-trivial architectural changes

2. **Alternative: SP1 Evaluation (Performance-Focused):**
   - **SP1 (Succinct)** offers lower proof generation latency and improved scalability
   - Better suited for large, data-heavy computations like Pitchlake's pricing models
   - Would require adapting proof format and verification contracts
   - Trade-off: Migration effort vs. long-term performance gains

**Recommendation for Future Maintainers:**

Given the computational complexity of Pitchlake's pricing models (TWAP, reserve price with Monte Carlo simulation, max return calculations) and the size of Fossil's aggregated datasets, the development team should carefully assess:

- Migrating to **Boundless** if maintaining RISC0 compatibility is a priority
- Evaluating **SP1** as a potential replacement for better performance characteristics
- Impact on proof generation costs, latency, and operational complexity

**Current Configuration References:**
- Bonsai API configuration: `proving-service/crates/message-handler/src/main.rs:714` (BONSAI_API_KEY)
- Proof generation timeout: `proving-service/crates/message-handler/src/main.rs:764` (1 hour default)
- Concurrent proof limits: `proving-service/crates/message-handler/src/services/proof_job_handler.rs:489` (MAX_CONCURRENT_PROOFS)
- Retry configuration: Environment variables RISC0_MAX_RETRIES, RISC0_INITIAL_RETRY_DELAY_MS

## Crate Organization

The Proving Service is organized as a Rust workspace with four crates:

```
proving-service/
├── Cargo.toml                    # Workspace configuration
└── crates/
    ├── db/                       # Database layer
    ├── message-handler/          # SQS message processing and proof generation
    ├── proving-service/          # HTTP API for job submission
    └── starknet-handler/         # StarkNet integration
```

### Workspace Dependencies

The workspace defines shared dependencies in `proving-service/Cargo.toml`:

```toml
[workspace.dependencies]
# Core dependencies
eyre = "0.6.12"
tokio = { version = "1.39.0", features = ["full"] }
serde = { version = "1.0.219", features = ["derive"] }
async-trait = "0.1"
tracing = "0.1.41"

# AWS
aws-config = "1.6.0"
aws-sdk-sqs = "1.62.0"

# Database
sqlx = { version = "0.8", features = ["postgres", "runtime-tokio-native-tls"] }

# StarkNet
starknet = "0.16.0"
starknet-crypto = "0.7.4"
```

### Crate Dependencies

**Dependency Graph:**
```
proving-service (HTTP API)
    └── message-handler (job processing)
        ├── db (database access)
        └── starknet-handler (blockchain integration)
```

## Architecture Diagram

### Component Architecture

```
┌─────────────────────────────────────────────────────────────────────┐
│                         Proving Service                             │
│                                                                     │
│  ┌──────────────────────────────────────────────────────────────┐  │
│  │                   Proving Service API                         │  │
│  │                    (Axum HTTP Server)                         │  │
│  │                                                               │  │
│  │  Endpoints:                                                   │  │
│  │    POST /api/job      - Submit proof job                     │  │
│  │    GET  /health       - Health check                         │  │
│  └────────────────────────┬─────────────────────────────────────┘  │
│                           │                                         │
│                           │ Dispatch Job                            │
│                           ▼                                         │
│  ┌──────────────────────────────────────────────────────────────┐  │
│  │                     Job Dispatcher                            │  │
│  │              (Serializes & queues jobs)                       │  │
│  └────────────────────────┬─────────────────────────────────────┘  │
│                           │                                         │
│                           │ Send to SQS                             │
│                           ▼                                         │
│  ┌──────────────────────────────────────────────────────────────┐  │
│  │                      AWS SQS Queue                            │  │
│  │               (fossilQueue - async buffer)                    │  │
│  └────────────────────────┬─────────────────────────────────────┘  │
│                           │                                         │
│                           │ Long Polling (20s)                      │
│                           ▼                                         │
│  ┌──────────────────────────────────────────────────────────────┐  │
│  │                   Message Handler                             │  │
│  │               (Background worker process)                     │  │
│  │                                                               │  │
│  │  Components:                                                  │  │
│  │    • ProofJobHandler   - Main processing loop                │  │
│  │    • ProofProvider     - Proof generation interface          │  │
│  │    • BonsaiProofProvider - RISC Zero proof generation        │  │
│  │    • Queue Management  - SQS message lifecycle               │  │
│  └────┬──────────────┬──────────────┬─────────────────────────┬─┘  │
│       │              │              │                         │    │
│       │ Fetch Data   │ Generate     │ Submit                  │    │
│       ▼              │ Proof        │ Proof                   │    │
│  ┌─────────┐         ▼              ▼                         ▼    │
│  │Database │    ┌─────────┐    ┌──────────┐         ┌─────────────┐│
│  │ Layer   │    │ Bonsai  │    │StarkNet  │         │  Status     ││
│  │         │    │   API   │    │ Handler  │         │  Updates    ││
│  └─────────┘    └─────────┘    └──────────┘         └─────────────┘│
└─────────────────────────────────────────────────────────────────────┘
         │                  │              │
         │                  │              │
         ▼                  ▼              ▼
   ┌──────────┐      ┌──────────┐    ┌────────────┐
   │Indexer DB│      │ RISC Zero│    │  StarkNet  │
   │(read-only│      │  Bonsai  │    │  Network   │
   │)         │      │   Cloud  │    │            │
   └──────────┘      └──────────┘    └────────────┘
```

## Database Layer (db crate)

### Purpose

The `db` crate provides database access for querying historical blockchain data needed for proof generation.

**Note:** In the current implementation, the Pitchlake Coprocessor primarily fetches validated base fee data from the **Fossil Store Contract** on Starknet (via `starknet-handler` crate). The `db` crate was originally designed for direct database access but has been largely superseded by querying the on-chain Fossil Store.

**Location:** `proving-service/crates/db/`

### Architecture

```rust
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

### Data Models

**BlockHeader Model:**

```rust
#[derive(sqlx::FromRow, Debug)]
pub struct BlockHeader {
    pub block_hash: Option<String>,
    pub number: i64,
    pub gas_limit: Option<i64>,
    pub gas_used: Option<i64>,
    pub nonce: Option<String>,
    pub transaction_root: Option<String>,
    pub base_fee_per_gas: Option<String>,  // Primary field used
    pub receipts_root: Option<String>,
    pub state_root: Option<String>,
    pub timestamp: Option<i64>,
}
```

### Key Queries

**1. Get Block Headers by Time Range:**

```rust
pub async fn get_block_headers_by_time_range(
    db: Arc<DbConnection>,
    start_timestamp: i64,
    end_timestamp: i64,
) -> Result<Vec<BlockHeader>, Error>
```

Retrieves all block header information within a timestamp range. Used for debugging and comprehensive data analysis.

**SQL Query:**
```sql
SELECT
    block_hash, number, gas_limit, gas_used, base_fee_per_gas,
    nonce, transaction_root, receipts_root, state_root, timestamp
FROM blockheaders
WHERE CAST(timestamp AS BIGINT) BETWEEN $1 AND $2
ORDER BY number ASC
```

**2. Get Block Base Fees (Optimized):**

```rust
pub async fn get_block_base_fee_by_time_range(
    db: Arc<DbConnection>,
    start_timestamp: i64,
    end_timestamp: i64,
) -> Result<Vec<String>, Error>
```

Optimized query that retrieves only `base_fee_per_gas` data. This is the production query used for proof generation.

**SQL Query:**
```sql
SELECT base_fee_per_gas
FROM blockheaders
WHERE CAST(timestamp AS BIGINT) BETWEEN $1 AND $2
ORDER BY number ASC
```

### Database Schema

The `db` crate expects a `blockheaders` table with the following schema:

```sql
CREATE TABLE blockheaders (
    block_hash TEXT,
    number BIGINT PRIMARY KEY,
    gas_limit BIGINT,
    gas_used BIGINT,
    nonce TEXT,
    transaction_root TEXT,
    base_fee_per_gas TEXT,
    receipts_root TEXT,
    state_root TEXT,
    timestamp BIGINT
);

-- Index for timestamp queries (critical for performance)
CREATE INDEX idx_blockheaders_timestamp ON blockheaders(timestamp);
```

### Connection Configuration

The database connection is configured via environment variable:

```bash
PROVING_SERVICE_DATABASE_URL=postgresql://postgres:postgres@localhost:5435/postgres
```

### Usage Example

```rust
use db::{DbConnection, models::get_block_base_fee_by_time_range};

// Initialize connection
let db_url = std::env::var("PROVING_SERVICE_DATABASE_URL")?;
let db = DbConnection::new(&db_url).await?;

// Query fee data
let start_timestamp = 1672531200;
let end_timestamp = 1672617600;
let base_fees = get_block_base_fee_by_time_range(
    db.clone(),
    start_timestamp,
    end_timestamp
).await?;
```

### Testing

The `db` crate includes comprehensive tests using `testcontainers` for isolated database testing:

```rust
#[tokio::test]
async fn test_should_get_all_block_headers_by_time_range() {
    let test_db = setup_db().await;

    let headers = get_block_headers_by_time_range(
        test_db.db,
        1743249000,
        1743249120
    ).await.unwrap();

    assert_eq!(headers.len(), 5);
}
```

## Message Handler (message-handler crate)

### Purpose

The `message-handler` crate is the core processing engine that:
- Polls AWS SQS for proof generation jobs
- Generates RISC Zero proofs via Bonsai API
- Composes complex proofs from multiple sub-proofs
- Submits proofs to StarkNet for verification
- Manages job lifecycle and error handling

**Location:** `proving-service/crates/message-handler/`

### Module Organization

```
message-handler/
├── src/
│   ├── lib.rs                          # Module exports
│   ├── main.rs                         # Binary entry point
│   ├── bonsai_test.rs                  # Bonsai API testing
│   ├── example_service_main.rs         # Example implementation
│   ├── queue/
│   │   ├── mod.rs                      # Queue module
│   │   ├── message_queue.rs            # Queue trait definition
│   │   ├── sqs_message_queue.rs        # AWS SQS implementation
│   │   └── local_message_queue.rs      # In-memory queue (testing)
│   ├── services/
│   │   ├── mod.rs                      # Service exports
│   │   ├── proof_job_handler.rs        # Main job processing logic
│   │   ├── job_dispatcher.rs           # Job dispatching
│   │   ├── jobs.rs                     # Job data models
│   │   ├── hashing_service.rs          # Hash preparation service
│   │   └── example_message_handler.rs  # Example handler
│   ├── proof_composition/
│   │   └── mod.rs                      # Proof composition logic
│   ├── response_handler/
│   │   └── mod.rs                      # Response handling
│   ├── hashing/
│   │   └── mod.rs                      # Hashing utilities
│   ├── proof_verifier.rs               # Proof verification
│   └── risc0_generator.rs              # RISC Zero proof generation
└── Cargo.toml
```

### Feature Flags

The crate supports multiple feature flags for different proof modes:

```toml
[features]
default = []
proof-composition = [
    "coprocessor_common",
    "coprocessor_core",
    # ... full proof composition dependencies
]
mock-proof = [
    "mock-proof-composition-methods",
    "risc0-ethereum-contracts",
    "starknet-handler"
]
```

**Usage:**
- `proof-composition`: Full production proof generation with real coprocessor methods
- `mock-proof`: Lightweight mock proofs for testing and development

### Job Models

**RequestProof Job:**

```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RequestProof {
    pub job_id: String,
    pub job_group_id: Option<String>,
    pub start_timestamp: i64,
    pub end_timestamp: i64,

    // Component-specific timestamp ranges
    pub twap_start_timestamp: Option<i64>,
    pub twap_end_timestamp: Option<i64>,
    pub reserve_price_start_timestamp: Option<i64>,
    pub reserve_price_end_timestamp: Option<i64>,
    pub max_return_start_timestamp: Option<i64>,
    pub max_return_end_timestamp: Option<i64>,

    // StarkNet verification context
    pub vault_address: Option<String>,
    pub vault_timestamp: Option<i64>,
}
```

**ProofGenerated Job:**

```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProofGenerated {
    pub job_id: String,
    pub receipt: Receipt,  // RISC Zero Receipt
}
```

**Job Enum:**

```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(untagged)]
pub enum Job {
    RequestProof(RequestProof),
    ProofGenerated(Box<ProofGenerated>),
}
```

### Queue Abstraction

**Queue Trait:**

```rust
#[async_trait]
pub trait Queue {
    async fn send_message(&self, message: String) -> Result<(), QueueError>;
    async fn receive_messages(&self) -> Result<Vec<QueueMessage>, QueueError>;
    async fn delete_message(&self, message: &QueueMessage) -> Result<(), QueueError>;
}
```

**SQS Implementation:**

```rust
pub struct SqsMessageQueue {
    client: aws_sdk_sqs::Client,
    queue_url: String,
}

impl SqsMessageQueue {
    pub fn new(queue_url: String, config: SdkConfig) -> Self {
        let client = aws_sdk_sqs::Client::new(&config);
        Self { client, queue_url }
    }
}
```

**Local Queue (Testing):**

```rust
pub struct LocalMessageQueue {
    messages: Arc<Mutex<VecDeque<QueueMessage>>>,
}
```

### ProofJobHandler - Core Processing

**Architecture:**

```rust
pub struct ProofJobHandler<Q: Queue + Send + Sync + 'static> {
    queue: Arc<Q>,
    terminator: Arc<AtomicBool>,
    proof_provider: Arc<dyn ProofProvider + Send + Sync>,
    proof_generation_timeout: Duration,
    processing_jobs: Arc<Mutex<HashSet<String>>>,
    job_failures: Arc<Mutex<HashMap<String, JobProcessingState>>>,
    max_failures: u32,
    proof_generation_semaphore: Arc<Semaphore>,
}
```

**Key Features:**

1. **Concurrent Proof Limiting**: Uses semaphore to limit concurrent Bonsai API requests
2. **Duplicate Detection**: Tracks processing jobs to prevent duplicate processing
3. **Failure Tracking**: Counts failures and forcibly deletes after max retries
4. **Graceful Shutdown**: Respects terminator flag for clean shutdown

**Main Processing Loop:**

```rust
pub async fn receive_job(&self) -> Result<()> {
    while !self.terminator.load(Ordering::Relaxed) {
        // 1. Poll SQS queue (long polling, 20 second wait)
        let messages = self.queue.receive_messages().await?;

        for message in messages {
            // 2. Parse job from message body
            let job: Job = serde_json::from_str(&message.body)?;

            // 3. Validate job type (only RequestProof)
            let request_proof = match job {
                Job::RequestProof(job) => job,
                _ => continue, // Delete non-RequestProof messages
            };

            // 4. Check for duplicate processing
            if !processing_jobs.insert(job.job_id.clone()) {
                continue; // Skip if already processing
            }

            // 5. Spawn async task for proof generation
            join_set.spawn(async move {
                // Acquire semaphore permit (limits concurrent proofs)
                let _permit = proof_semaphore.acquire().await;

                // Create timestamp ranges
                let timestamp_ranges = create_timestamp_ranges(&job);

                // Generate proof with timeout
                let proof_result = tokio::time::timeout(
                    timeout_duration,
                    proof_provider.generate_proofs_from_data(
                        timestamp_ranges,
                        Some(&job)
                    ),
                ).await;

                // Handle result and update status
                match proof_result {
                    Ok(Ok(receipt)) => {
                        // Success: Send ProofGenerated message
                        send_job_to_queue(&queue, &ProofGenerated { ... }).await;
                        queue.delete_message(&message).await;
                    }
                    Ok(Err(e)) => {
                        // Failure: Increment failure count
                        // Requeue or delete based on failure count
                    }
                    Err(_) => {
                        // Timeout: Handle as failure
                    }
                }

                // Remove from processing set
                processing_jobs.remove(&job.job_id);
            });
        }
    }

    // Wait for all spawned tasks to complete
    join_set.join_all().await;
    Ok(())
}
```

### Proof Composition

**Overview:**

The Pitchlake Coprocessor generates cryptographic proofs for three key pricing calculations used in the Pitchlake options market:

- **TWAP** (Time-Weighted Average Price) - Volatility measurement over specified time ranges
- **Max Return** - Maximum return calculation for risk assessment across full dataset
- **Reserve Price** - Monte Carlo simulation for option pricing based on statistical models

Each calculation is performed in the RISC0 zkVM with sub-proofs that are composed into a final proof:

1. **Data Hashing** - Integrity verification of input fee data
2. **Statistical Calculations** - TWAP and max return computations
3. **Reserve Price Sub-Models**:
   - Deseasonalization (remove trends)
   - Markov transition matrices (PT/PT1 calculations)
   - 7-day moving average (TWAP 7D)
   - Monte Carlo price simulation

**Why Proof Composition?**

Pitchlake's pricing models are computationally intensive, especially the reserve price calculation which involves complex statistical operations and Monte Carlo simulation. Breaking computations into sub-proofs provides:

- **Parallel proof generation** - Sub-proofs can be generated concurrently
- **Reduced complexity** - Each sub-proof has a smaller circuit size
- **Independent verification** - Sub-components can be verified separately
- **Better error isolation** - Failures can be traced to specific sub-computations

**ProofProvider Trait:**

```rust
#[async_trait]
pub trait ProofProvider {
    async fn generate_proofs_from_data(
        &self,
        timestamp_ranges: ProofTimestampRanges,
        job_context: Option<&RequestProof>,
    ) -> Result<Receipt>;

    fn is_disabled(&self) -> bool {
        false
    }
}
```

**BonsaiProofProvider - Production Implementation:**

The `BonsaiProofProvider` orchestrates complex proof composition with 7 sub-proofs:

```rust
impl ProofProvider for BonsaiProofProvider {
    async fn generate_proofs_from_data(
        &self,
        timestamp_ranges: ProofTimestampRanges,
        job_context: Option<&RequestProof>,
    ) -> Result<Receipt> {
        // 1. Fetch fee data from StarkNet Fossil Store
        let fee_data = provider.get_avg_fees_in_range(
            overall_start,
            overall_end
        ).await?;

        // 2. Generate basic sub-proofs (parallel)
        let (hashing_receipt, hashing_result) = hash_felts(...);
        let (max_return_receipt, max_return_value) = max_return(...);
        let (twap_receipt, twap_value) = calculate_twap(...);

        // 3. Generate reserve price sub-proofs (parallel)
        let (
            reserve_price_result,
            remove_seasonality_receipt,
            calculate_pt_pt1_receipt,
            add_twap_7d_receipt,
            simulate_price_receipt,
        ) = generate_reserve_price_sub_proofs(...).await?;

        // 4. Build ProofCompositionInput
        let composition_input = build_proof_composition_input(...);

        // 5. Compose final proof with all sub-proof assumptions
        let receipt = compose_final_proof(
            composition_input,
            hashing_receipt,
            max_return_receipt,
            twap_receipt,
            remove_seasonality_receipt,
            calculate_pt_pt1_receipt,
            add_twap_7d_receipt,
            simulate_price_receipt,
        ).await?;

        Ok(receipt)
    }
}
```

**Sub-Proof Architecture:**

```
Final Proof (Composition)
├── Assumption 1: Hashing Receipt
├── Assumption 2: Max Return Receipt
├── Assumption 3: TWAP Receipt
├── Assumption 4: Remove Seasonality Receipt
├── Assumption 5: Calculate PT/PT1 Receipt
├── Assumption 6: Add TWAP 7D Receipt
└── Assumption 7: Simulate Price Receipt
```

Each sub-proof is generated independently and then composed into the final proof using RISC Zero's assumption mechanism.

### Timestamp Range Management

**ProofTimestampRanges:**

```rust
#[derive(Debug, Clone)]
pub struct ProofTimestampRanges {
    pub twap: (i64, i64),
    pub reserve_price: (i64, i64),
    pub max_return: (i64, i64),
}

impl ProofTimestampRanges {
    pub fn overall_range(&self) -> (i64, i64) {
        // Find highest end timestamp
        let end = max(
            max(self.twap.1, self.reserve_price.1),
            self.max_return.1
        );

        // Subtract safety buffer for Fossil store lag
        const FOSSIL_STORE_LAG_HOURS: i64 = 12;
        let safe_end = end - (FOSSIL_STORE_LAG_HOURS * 3600);

        // Normalize to hour boundary
        let safe_end_normalized = (safe_end / 3600) * 3600;

        // Calculate start (POC: 1440 hours = 2 months)
        const REQUIRED_HOURS: i64 = 1440;
        const FETCH_BUFFER_HOURS: i64 = 120;

        let start = safe_end_normalized -
            ((REQUIRED_HOURS + FETCH_BUFFER_HOURS) * 3600);

        (start, safe_end_normalized)
    }
}
```

### Configuration

**Environment Variables:**

```bash
# Queue Configuration
SQS_QUEUE_URL=http://localhost:4567/000000000000/fossilQueue
AWS_REGION=us-east-1
AWS_ACCESS_KEY_ID=test
AWS_SECRET_ACCESS_KEY=test
AWS_ENDPOINT_URL=http://localhost:4567

# Database Configuration
PROVING_SERVICE_DATABASE_URL=postgresql://postgres:postgres@localhost:5435/postgres

# Proof Generation
ENABLE_PROOF=true
USE_SIMPLE_MOCK=false
USE_RISC0_INTEGRATION=true
MAX_CONCURRENT_PROOFS=1

# Bonsai API
BONSAI_API_KEY=your_api_key_here
BONSAI_API_URL=https://api.bonsai.xyz/
RISC0_MAX_RETRIES=7
RISC0_INITIAL_RETRY_DELAY_MS=3000

# StarkNet Configuration (via starknet-handler)
STARKNET_RPC_URL=http://localhost:5050
VERIFY_PROOFS_ONCHAIN=true
```

### Binary Entry Point

**Main Function (`src/main.rs`):**

```rust
#[tokio::main]
async fn main() -> Result<()> {
    // 1. Initialize tracing
    tracing_subscriber::FmtSubscriber::builder()
        .with_env_filter(EnvFilter::from_default_env())
        .init();

    // 2. Load environment variables
    dotenv::dotenv().ok();

    // 3. Configure AWS SQS
    let queue_url = std::env::var("SQS_QUEUE_URL")?;
    let config = aws_config::defaults(BehaviorVersion::latest())
        .load()
        .await;
    let queue = Arc::new(SqsMessageQueue::new(queue_url, config));

    // 4. Create proof provider based on environment
    let enable_proof = std::env::var("ENABLE_PROOF")
        .map(|v| v.to_lowercase() == "true")
        .unwrap_or(false);

    let proof_provider: Arc<dyn ProofProvider + Send + Sync> =
        if enable_proof {
            Arc::new(BonsaiProofProvider::new())
        } else {
            Arc::new(NoOpProofProvider::new())
        };

    // 5. Create job processor
    let terminator = Arc::new(AtomicBool::new(false));
    let processor = ProofJobHandler::new(
        queue.clone(),
        terminator.clone(),
        proof_provider,
        Duration::from_secs(3600), // 1 hour timeout
    );

    // 6. Start processing
    let handle = tokio::spawn(async move {
        processor.receive_job().await
    });

    // 7. Wait for Ctrl+C
    signal::ctrl_c().await?;

    // 8. Graceful shutdown
    terminator.store(true, Ordering::Relaxed);
    handle.await??;

    Ok(())
}
```

## Proving Service API (proving-service crate)

### Purpose

The `proving-service` crate provides an HTTP API for receiving proof generation requests from the Fossil API and dispatching them to the SQS queue.

**Location:** `proving-service/crates/proving-service/`

### HTTP Endpoints

**1. Health Check**

```
GET /health

Response:
{
  "status": "healthy",
  "service": "proving-service"
}
```

**2. Submit Proof Job**

```
POST /api/job

Request Body:
{
  "job_group_id": "job_abc123",
  "twap": {
    "start_timestamp": 1672531200,
    "end_timestamp": 1672617600
  },
  "reserve_price": {
    "start_timestamp": 1672531200,
    "end_timestamp": 1672617600
  },
  "max_return": {
    "start_timestamp": 1672531200,
    "end_timestamp": 1672617600
  },
  "vault_address": "0x004018ae...",
  "vault_timestamp": 1672531200
}

Response (200 OK):
{
  "status": "success",
  "message": "Job dispatched successfully",
  "job_group_id": "job_abc123"
}

Response (500 Internal Server Error):
{
  "status": "error",
  "message": "Failed to dispatch job: <error details>",
  "job_group_id": "job_abc123"
}
```

### Router Configuration

```rust
pub async fn create_router(queue: Arc<SqsMessageQueue>) -> Router {
    let dispatcher = Arc::new(JobDispatcher::new(queue));

    Router::new()
        .route("/health", get(health_check))
        .route("/api/job", post(handle_job_request))
        .with_state(dispatcher)
}
```

### Job Request Handler

**Flow:**

```rust
pub async fn handle_job_request(
    State(dispatcher): State<Arc<JobDispatcher<SqsMessageQueue>>>,
    Json(request): Json<JobRequest>,
) -> impl IntoResponse {
    // 1. Log incoming request
    info!("Received job request for group: {}", request.job_group_id);

    // 2. Create combined job with all timestamp ranges
    let combined_job = Job::RequestProof(RequestProof {
        job_id: request.job_group_id.clone(),
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

    // 3. Dispatch to queue
    match dispatcher.dispatch_job(combined_job).await {
        Ok(_) => (
            StatusCode::OK,
            Json(Response {
                status: "success".to_string(),
                message: "Job dispatched successfully".to_string(),
                job_group_id: request.job_group_id,
            })
        ),
        Err(e) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(Response {
                status: "error".to_string(),
                message: format!("Failed to dispatch job: {e}"),
                job_group_id: request.job_group_id,
            })
        ),
    }
}
```

### Job Dispatcher

**Implementation:**

```rust
pub struct JobDispatcher<Q: Queue> {
    queue: Arc<Q>,
}

impl<Q: Queue> JobDispatcher<Q> {
    pub const fn new(queue: Arc<Q>) -> Self {
        Self { queue }
    }

    pub async fn dispatch_job(&self, job: Job) -> Result<()> {
        // Serialize job to JSON
        let message_body = serde_json::to_string(&job)?;

        // Send to queue
        self.queue
            .send_message(message_body)
            .await
            .map_err(|e| eyre::eyre!(e))?;

        Ok(())
    }
}
```

### Server Startup

**Main Function:**

```rust
use axum::Router;
use message_handler::queue::sqs_message_queue::SqsMessageQueue;
use proving_service::create_router;
use std::sync::Arc;

#[tokio::main]
async fn main() -> Result<()> {
    // 1. Initialize tracing
    tracing_subscriber::fmt::init();

    // 2. Load environment
    dotenv::dotenv().ok();

    // 3. Configure AWS SQS
    let queue_url = std::env::var("SQS_QUEUE_URL")?;
    let config = aws_config::defaults(BehaviorVersion::latest())
        .load()
        .await;
    let queue = Arc::new(SqsMessageQueue::new(queue_url, config));

    // 4. Create router
    let app = create_router(queue).await;

    // 5. Start server
    let listener = tokio::net::TcpListener::bind("0.0.0.0:3001")
        .await?;

    info!("Proving Service API listening on 0.0.0.0:3001");

    axum::serve(listener, app).await?;

    Ok(())
}
```

### Testing

**Unit Tests:**

```rust
#[tokio::test]
async fn test_handle_job_request_success() {
    let mock_queue = MockQueue::new(false);
    let dispatcher = Arc::new(JobDispatcher::new(Arc::new(mock_queue)));

    let request = JobRequest {
        job_group_id: "test-group-123".to_string(),
        twap: TimeRange {
            start_timestamp: 1000,
            end_timestamp: 2000
        },
        // ...
    };

    let response = handle_job_request(
        State(dispatcher),
        Json(request)
    ).await;

    assert_eq!(response.0, StatusCode::OK);
    assert_eq!(response.1.status, "success");
}
```

## StarkNet Handler (starknet-handler crate)

### Purpose

The `starknet-handler` crate provides StarkNet blockchain integration for:
- Fetching historical fee data from Fossil Store contract
- Creating batch hashes for fee data
- Submitting proofs to verifier contracts
- Managing StarkNet account and transactions

**Location:** `proving-service/crates/starknet-handler/`

### Module Organization

```
starknet-handler/
├── src/
│   ├── lib.rs                # Module exports and types
│   ├── main.rs               # Binary example
│   ├── account.rs            # StarkNet account management
│   ├── provider.rs           # RPC provider and contract calls
│   ├── config.rs             # Configuration loading
│   ├── example.rs            # Usage examples
│   ├── mock_example.rs       # Mock examples
│   └── mock_data.rs          # Test data
└── Cargo.toml
```

### Core Types

**FeeData:**

```rust
#[derive(Clone, Debug)]
pub struct FeeData {
    pub first_timestamp: u64,
    pub last_timestamp: u64,
    pub fees: Vec<Felt>,
}
```

**FeeDataWithHash:**

```rust
#[derive(Clone, Debug)]
pub struct FeeDataWithHash {
    pub raw_fees: Vec<Felt>,
    pub verification_hash: [u32; 8],
    pub avg_l1_gas_fee: u64,
    pub avg_l2_gas_fee: u64,
}
```

**PitchLakeJobRequest:**

```rust
#[derive(Debug, Clone, Encode)]
pub struct PitchLakeJobRequest {
    pub vault_address: Felt,
    pub timestamp: u64,
    pub program_id: Felt,  // 'PITCH_LAKE_V1'
}
```

### StarkNet Account

**Account Management:**

```rust
pub struct StarknetAccount {
    account: SingleOwnerAccount<
        Arc<JsonRpcClient<HttpTransport>>,
        LocalWallet
    >,
}

impl StarknetAccount {
    pub fn new(
        provider: Arc<JsonRpcClient<HttpTransport>>,
        account_private_key: &str,
        account_address: &str,
    ) -> Result<Self> {
        let private_key = Felt::from_hex(account_private_key)?;
        let signer = LocalWallet::from(
            SigningKey::from_secret_scalar(private_key)
        );
        let address = Felt::from_hex(account_address)?;

        let account = SingleOwnerAccount::new(
            provider,
            signer,
            address,
            KATANA_CHAIN_ID,
            ExecutionEncoding::New,
        );

        Ok(Self { account })
    }
}
```

### Hash Creation

**1. Create Batch Hash (180 fees):**

```rust
pub async fn create_batch_hash(
    &self,
    hash_store_address: &str,
    start_timestamp: u64,
) -> Result<Felt> {
    // Normalize timestamp to hour boundary
    let (start_timestamp, was_normalized) =
        normalize_timestamp(start_timestamp);

    // Call hash_avg_fees_and_store
    let selector = selector!("hash_avg_fees_and_store");
    let call = Call {
        selector,
        calldata: vec![Felt::from(start_timestamp)],
        to: Felt::from_hex(hash_store_address)?,
    };

    // Execute with retry logic
    let mut attempt = 0;
    loop {
        match self.account.execute_v3(vec![call.clone()]).send().await {
            Ok(tx) => return Ok(tx.transaction_hash),
            Err(e) => {
                if attempt >= MAX_RETRIES {
                    return Err(e.into());
                }
                let backoff = INITIAL_BACKOFF * 2u32.pow(attempt);
                tokio::time::sleep(backoff).await;
                attempt += 1;
            }
        }
    }
}
```

**2. Create Batched Hash (32 batches):**

```rust
pub async fn create_batched_hash(
    &self,
    hash_store_address: &str,
    start_timestamp: u64,
) -> Result<Felt> {
    let selector = selector!("hash_batched_avg_fees");
    let call = Call {
        selector,
        calldata: vec![Felt::from(start_timestamp)],
        to: Felt::from_hex(hash_store_address)?,
    };

    // Execute with retry logic (similar to above)
}
```

### Proof Verification

**Verify Groth16 Proof Onchain:**

```rust
pub async fn verify_proof(
    &self,
    verifier_address: &str,
    proof: Vec<Felt>,
    pitchlake_job_request: PitchLakeJobRequest,
) -> Result<Felt> {
    // Encode calldata
    let mut calldata = vec![];
    proof.encode(&mut calldata)?;
    pitchlake_job_request.encode(&mut calldata)?;

    // Call verify_proof_for_pitch_lake
    let selector = Felt::from_hex(
        "0x821b8b00fd9e4b2b57538b4571c0227e80f5dbdbfef0628722b3f06f3188"
    )?;
    let call = Call {
        selector,
        calldata,
        to: Felt::from_hex(verifier_address)?,
    };

    // Execute with retry logic
    let mut attempt = 0;
    loop {
        match self.account.execute_v3(vec![call.clone()]).send().await {
            Ok(tx) => {
                info!("Proof verified onchain: {:?}", tx.transaction_hash);
                return Ok(tx.transaction_hash);
            }
            Err(e) => {
                if attempt >= MAX_RETRIES {
                    return Err(e.into());
                }
                let backoff = INITIAL_BACKOFF * 2u32.pow(attempt);
                tokio::time::sleep(backoff).await;
                attempt += 1;
            }
        }
    }
}
```

### StarkNet Provider

**Provider for Contract Calls:**

```rust
pub struct StarknetProvider {
    provider: Arc<JsonRpcClient<HttpTransport>>,
    fossil_store_address: Felt,
}

impl StarknetProvider {
    pub fn new(config: StarknetConfig) -> Result<Self> {
        let url = url::Url::parse(&config.rpc_url)?;
        let provider = Arc::new(JsonRpcClient::new(
            HttpTransport::new(url)
        ));

        Ok(Self {
            provider,
            fossil_store_address: config.fossil_store_address,
        })
    }

    pub async fn get_avg_fees_in_range(
        &self,
        start_timestamp: u64,
        end_timestamp: u64,
    ) -> Result<FeeData> {
        // Call get_avg_fees_in_range on Fossil Store
        let selector = selector!("get_avg_fees_in_range");
        let calldata = vec![
            Felt::from(start_timestamp),
            Felt::from(end_timestamp),
        ];

        let result = self.provider.call(
            Call {
                to: self.fossil_store_address,
                selector,
                calldata,
            },
            BlockId::Tag(BlockTag::Latest),
        ).await?;

        // Parse result: [first_ts, last_ts, array_len, ...fees]
        let first_timestamp = result[0].to_u64();
        let last_timestamp = result[1].to_u64();
        let array_len = result[2].to_usize();
        let fees = result[3..3+array_len].to_vec();

        Ok(FeeData {
            first_timestamp,
            last_timestamp,
            fees,
        })
    }
}
```

### Configuration

**StarknetConfig:**

```rust
pub struct StarknetConfig {
    pub rpc_url: String,
    pub account_address: String,
    pub private_key: String,
    pub fossil_store_address: Felt,
    pub hash_storage_address: Felt,
    pub verifier_contract_address: Option<Felt>,
}

pub fn load_starknet_config() -> Result<StarknetConfig> {
    Ok(StarknetConfig {
        rpc_url: std::env::var("STARKNET_RPC_URL")?,
        account_address: std::env::var("STARKNET_ACCOUNT_ADDRESS")?,
        private_key: std::env::var("STARKNET_PRIVATE_KEY")?,
        fossil_store_address: Felt::from_hex(
            &std::env::var("FOSSIL_STORE_ADDRESS")?
        )?,
        hash_storage_address: Felt::from_hex(
            &std::env::var("HASH_STORAGE_ADDRESS")?
        )?,
        verifier_contract_address: std::env::var("PITCHLAKE_VERIFIER_CONTRACT")
            .ok()
            .and_then(|s| Felt::from_hex(&s).ok()),
    })
}
```

**Environment Variables:**

```bash
# StarkNet RPC
STARKNET_RPC_URL=http://localhost:5050
STARKNET_ACCOUNT_ADDRESS=0x127fd5f1fe78a71f8bcd1fec63e3fe2f0486b6ecd5c86a0466c3a21fa5cfcec
STARKNET_PRIVATE_KEY=0xc5b2fcab997346f3ea1c00b002ecf6f382c5f9c9659a3894eb783c5320f912

# Contract Addresses
FOSSIL_STORE_ADDRESS=0x00e581139553c8666f60b6646f277a336f99f108f8e5fa7cb300b6a6ce7c3b8c
HASH_STORAGE_ADDRESS=0x...
PITCHLAKE_VERIFIER_CONTRACT=0x...
PITCHLAKE_VAULT=0x...
```

### Retry Configuration

**Retry Parameters:**

```rust
const MAX_RETRIES: u32 = 3;
const INITIAL_BACKOFF: Duration = Duration::from_secs(1);

// Exponential backoff: INITIAL_BACKOFF * 2^attempt
// Retry 0: 1 second
// Retry 1: 2 seconds
// Retry 2: 4 seconds
```

**Environment Override:**

```bash
STARKNET_MAX_RETRIES=3
STARKNET_INITIAL_BACKOFF_MS=100
STARKNET_MAX_BACKOFF_MS=1000
```

### Usage Example

```rust
use starknet_handler::{
    account::StarknetAccount,
    config::load_starknet_config,
    provider::StarknetProvider,
};

// Initialize provider
let config = load_starknet_config()?;
let provider = StarknetProvider::new(config.clone())?;

// Fetch fee data
let fee_data = provider.get_avg_fees_in_range(
    1672531200,
    1672617600,
).await?;

// Initialize account
let account = StarknetAccount::new(
    provider.provider.clone(),
    &config.private_key,
    &config.account_address,
)?;

// Verify proof onchain
let job_request = PitchLakeJobRequest {
    vault_address: Felt::from_hex("0x...")?,
    timestamp: 1672531200,
    program_id: Felt::from_hex("0x504954434c4c414b455f5631")?,
};

let tx_hash = account.verify_proof(
    &config.verifier_contract_address.unwrap().to_hex_string(),
    proof_data,
    job_request,
).await?;
```

## Proof Generation Flow

### End-to-End Flow

```
┌─────────────────────────────────────────────────────────────────────┐
│                    Proof Generation Lifecycle                        │
└─────────────────────────────────────────────────────────────────────┘

1. Job Submission
   ┌──────────────┐
   │ Fossil API   │ POST /api/job
   │              ├────────────────────┐
   └──────────────┘                    │
                                       ▼
                              ┌────────────────┐
                              │ Proving Service│
                              │      API       │
                              └────────┬───────┘
                                       │
                                       │ Serialize & Queue
                                       ▼
2. Queue Dispatch            ┌────────────────┐
                             │   AWS SQS      │
                             │  fossilQueue   │
                             └────────┬───────┘
                                      │
                                      │ Long Poll (20s)
                                      ▼
3. Job Processing            ┌────────────────┐
                             │ Message Handler│
                             │  (Background)  │
                             └────────┬───────┘
                                      │
            ┌─────────────────────────┼─────────────────────────┐
            │                         │                         │
            ▼                         ▼                         ▼
4. Data     ┌──────────┐   5. Proof  ┌──────────┐   6. Verify ┌──────────┐
   Fetch    │ Indexer  │      Gen    │  Bonsai  │      Onchain│ StarkNet │
            │    DB    │             │   API    │             │ Network  │
            └──────────┘             └──────────┘             └──────────┘
                 │                        │                         │
                 │                        │                         │
                 ▼                        ▼                         ▼
            Block Headers           RISC Zero Receipt        Transaction Hash
            (base_fee_per_gas)      (Groth16 Proof)         (Proof Verified)
```

### Detailed Steps

**Step 1: Job Submission (Proving Service API)**

```rust
// Client submits job
POST /api/job
{
  "job_group_id": "job_abc123",
  "twap": { "start_timestamp": 1672531200, "end_timestamp": 1672617600 },
  // ...
}

// API handler creates RequestProof job
let job = Job::RequestProof(RequestProof {
    job_id: "job_abc123",
    twap_start_timestamp: Some(1672531200),
    twap_end_timestamp: Some(1672617600),
    // ...
});

// Dispatcher queues job
dispatcher.dispatch_job(job).await?;
```

**Step 2: Queue Storage (SQS)**

```rust
// Job serialized to JSON and sent to SQS
let message_body = serde_json::to_string(&job)?;
sqs_client.send_message()
    .queue_url(&queue_url)
    .message_body(message_body)
    .send()
    .await?;
```

**Step 3: Job Retrieval (Message Handler)**

```rust
// Message handler polls queue
let messages = sqs_client.receive_message()
    .queue_url(&queue_url)
    .max_number_of_messages(10)
    .wait_time_seconds(20)  // Long polling
    .send()
    .await?;

// Parse job
let job: Job = serde_json::from_str(&message.body)?;
let request_proof = match job {
    Job::RequestProof(job) => job,
    _ => continue,
};
```

**Step 4: Data Fetching (Database & StarkNet)**

```rust
// Calculate timestamp ranges
let timestamp_ranges = ProofTimestampRanges::new(
    request_proof.twap_start_timestamp.unwrap(),
    request_proof.twap_end_timestamp.unwrap(),
    request_proof.reserve_price_start_timestamp.unwrap(),
    request_proof.reserve_price_end_timestamp.unwrap(),
    request_proof.max_return_start_timestamp.unwrap(),
    request_proof.max_return_end_timestamp.unwrap(),
);

let (overall_start, overall_end) = timestamp_ranges.overall_range();

// Fetch fee data from StarkNet Fossil Store
let fee_data = provider.get_avg_fees_in_range(
    overall_start as u64,
    overall_end as u64,
).await?;

// Validate data points (POC: 1440 hours, Production: 5760 hours)
if fee_data.fees.len() < REQUIRED_DATA_POINTS {
    return Err(eyre!("Insufficient onchain fee data"));
}
```

**Step 5: Proof Generation (Bonsai API)**

```rust
// Generate 7 sub-proofs in parallel

// Basic sub-proofs
let (hashing_receipt, hashing_res) = hash_felts(raw_input);
let (max_return_receipt, max_return_value) = max_return(data_8_months);
let (twap_receipt, twap_value) = calculate_twap(data_3_months);

// Reserve price sub-proofs (parallel)
let (
    reserve_price_result,
    remove_seasonality_receipt,
    calculate_pt_pt1_receipt,
    add_twap_7d_receipt,
    simulate_price_receipt,
) = tokio::try_join!(
    remove_seasonality_task,
    calculate_pt_pt1_task,
    add_twap_7d_task,
    simulate_price_task,
)?;

// Compose final proof with assumptions
let composition_input = ProofCompositionInput {
    data_8_months,
    data_8_months_hash: hashing_res.hash,
    twap_result: twap_value,
    max_return: max_return_value,
    reserve_price: reserve_price_result.reserve_price,
    // ... all other fields
};

let receipt = tokio::task::spawn_blocking(move || {
    let env = ExecutorEnv::builder()
        .add_assumption(hashing_receipt)
        .add_assumption(max_return_receipt)
        .add_assumption(twap_receipt)
        .add_assumption(remove_seasonality_receipt)
        .add_assumption(calculate_pt_pt1_receipt)
        .add_assumption(add_twap_7d_receipt)
        .add_assumption(simulate_price_receipt)
        .write(&composition_input)?
        .build()?;

    // Call Bonsai API for Groth16 proof
    default_prover().prove(env, PROOF_COMPOSITION_ELF)?
}).await??;
```

**Step 6: StarkNet Verification**

```rust
// Extract proof from receipt
let proof_data: Vec<Felt> = extract_groth16_proof(&receipt)?;

// Create job request for verifier
let job_request = PitchLakeJobRequest {
    vault_address: Felt::from_hex(&request_proof.vault_address.unwrap())?,
    timestamp: request_proof.vault_timestamp.unwrap() as u64,
    program_id: Felt::from_hex("0x504954434c4c414b455f5631")?, // 'PITCH_LAKE_V1'
};

// Submit proof to StarkNet verifier contract
let tx_hash = account.verify_proof(
    &verifier_contract_address,
    proof_data,
    job_request,
).await?;

info!("Proof verified onchain: {:?}", tx_hash);
```

**Step 7: Status Update**

```rust
// Create ProofGenerated job
let proof_generated = Job::ProofGenerated(Box::new(ProofGenerated {
    job_id: request_proof.job_id.clone(),
    receipt,
}));

// Send to result queue (optional)
queue.send_message(serde_json::to_string(&proof_generated)?).await?;

// Delete original message from queue
queue.delete_message(&message).await?;

info!("Job completed successfully: {}", request_proof.job_id);
```

### Proof Composition Details

**Sub-Proof Dependencies:**

```
┌──────────────────────────────────────────────────────────────┐
│              Proof Composition Architecture                   │
└──────────────────────────────────────────────────────────────┘

Input Data: 1440 hourly fee values (POC: 2 months, Production: 8 months)
     │
     ├──► Hashing Sub-Proof
     │    └── Output: SHA-256 hash of fee data
     │
     ├──► Max Return Sub-Proof (uses full 1440 values)
     │    └── Output: Maximum return (volatility measure)
     │
     ├──► TWAP Sub-Proof (uses last 720 values = 1 month)
     │    └── Output: Time-weighted average price
     │
     └──► Reserve Price Sub-Proofs (uses last 720 values = 1 month)
          ├── Remove Seasonality Sub-Proof
          │   └── Output: Detrended, deseasoned data
          ├── Calculate PT/PT1 Sub-Proof
          │   └── Output: Markov transition matrices
          ├── Add TWAP 7D Sub-Proof
          │   └── Output: 7-day moving average
          └── Simulate Price Sub-Proof
              └── Output: Monte Carlo price simulation
                  └── Final Output: Reserve price

┌──────────────────────────────────────────────────────────────┐
│                Final Proof Composition                        │
└──────────────────────────────────────────────────────────────┘

Inputs:
  • ProofCompositionInput (all parameters and results)
  • 7 Sub-Proof Receipts (as assumptions)

Execution:
  1. Verify all 7 sub-proof assumptions
  2. Re-compute calculations to validate consistency
  3. Generate final Groth16 proof via Bonsai

Output:
  • RISC Zero Receipt with Groth16 proof
  • Journal containing:
    - TWAP result
    - Max return result
    - Reserve price result
    - Data hash
    - Timestamp metadata
```

### Error Handling

**Timeout Handling:**

```rust
// Proof generation with 1-hour timeout
let proof_result = tokio::time::timeout(
    Duration::from_secs(3600),
    proof_provider.generate_proofs_from_data(timestamp_ranges, Some(&job)),
).await;

match proof_result {
    Ok(Ok(receipt)) => {
        // Success
    }
    Ok(Err(e)) => {
        // Proof generation error
        error!("Proof generation failed: {}", e);
        // Increment failure count
        // Requeue if under max failures
    }
    Err(_) => {
        // Timeout
        error!("Proof generation timed out after 1 hour");
        // Increment failure count
        // Requeue if under max failures
    }
}
```

**Failure Tracking:**

```rust
struct JobProcessingState {
    failure_count: u32,
    last_failure_time: Instant,
}

// Track failures per job
let job_failures: Arc<Mutex<HashMap<String, JobProcessingState>>>;

// On failure
{
    let mut failures = job_failures.lock().await;
    let entry = failures.entry(job.job_id.clone())
        .or_insert_with(|| JobProcessingState {
            failure_count: 0,
            last_failure_time: Instant::now(),
        });
    entry.failure_count += 1;
    entry.last_failure_time = Instant::now();
}

// Check if max failures reached
if failure_count >= MAX_FAILURES {
    // Force delete from queue
    queue.delete_message(&message).await?;
    failures.remove(&job.job_id);
} else {
    // Message will reappear after visibility timeout
}
```

**Retry Configuration:**

```bash
# SQS Visibility Timeout (must be >= proof timeout)
SQS_VISIBILITY_TIMEOUT=3600  # 1 hour

# Proof generation timeout
PROOF_GENERATION_TIMEOUT=3600  # 1 hour

# Max failures before force delete
MAX_FAILURES=3

# Bonsai API retry configuration
RISC0_MAX_RETRIES=7
RISC0_INITIAL_RETRY_DELAY_MS=3000

# StarkNet retry configuration
STARKNET_MAX_RETRIES=3
STARKNET_INITIAL_BACKOFF_MS=100
```

## Queue Management

### SQS Configuration

**Queue Creation:**

```bash
# Using AWS CLI (or LocalStack)
aws sqs create-queue \
    --queue-name fossilQueue \
    --attributes VisibilityTimeout=3600,MessageRetentionPeriod=86400
```

**Queue Attributes:**

| Attribute | Value | Purpose |
|-----------|-------|---------|
| VisibilityTimeout | 3600 seconds (1 hour) | Prevents duplicate processing during proof generation |
| MessageRetentionPeriod | 86400 seconds (24 hours) | How long messages stay in queue |
| ReceiveMessageWaitTimeSeconds | 20 seconds | Long polling to reduce empty responses |
| MaximumMessageSize | 262144 bytes (256 KB) | Maximum message size |

### Message Lifecycle

```
┌──────────────────────────────────────────────────────────────┐
│                   SQS Message Lifecycle                       │
└──────────────────────────────────────────────────────────────┘

1. Message Sent
   ┌─────────┐
   │ Visible │  Message is visible in queue
   └────┬────┘
        │
        │ receive_messages()
        ▼
2. Message Received
   ┌─────────┐
   │ Hidden  │  Message is hidden for VisibilityTimeout
   └────┬────┘  (prevents other workers from processing)
        │
        ├─► Processing Success
        │   └─► delete_message()
        │       └─► Message removed from queue
        │
        ├─► Processing Failure (< 3 failures)
        │   └─► Message NOT deleted
        │       └─► After VisibilityTimeout expires:
        │           Message becomes visible again (retry)
        │
        └─► Processing Failure (>= 3 failures)
            └─► delete_message()
                └─► Message removed from queue (prevent infinite loop)

3. Message Timeout
   If VisibilityTimeout expires before delete:
   ┌─────────┐
   │ Visible │  Message reappears in queue for retry
   └─────────┘
```

### Long Polling

**Configuration:**

```rust
let messages = sqs_client.receive_message()
    .queue_url(&queue_url)
    .max_number_of_messages(10)
    .wait_time_seconds(20)  // Long polling: wait up to 20 seconds
    .send()
    .await?;
```

**Benefits:**
- Reduces empty responses (fewer API calls)
- Lower costs
- More responsive to new messages
- Reduces CPU usage from tight polling loops

### Visibility Timeout

**Purpose:**

Prevents multiple workers from processing the same message simultaneously.

**Calculation:**

```
VisibilityTimeout >= ProofGenerationTimeout + SafetyBuffer

3600 seconds >= 3600 seconds + 0 seconds
```

**Behavior:**

1. Worker A receives message → message becomes invisible for 3600 seconds
2. Worker A processes job (takes 30 minutes)
3. Worker A deletes message → message removed from queue
4. If Worker A crashes or times out:
   - After 3600 seconds, message becomes visible again
   - Worker B can pick up the message and retry

### Dead Letter Queue (Optional)

For production, configure a Dead Letter Queue to capture messages that fail repeatedly:

```bash
aws sqs create-queue --queue-name fossilQueue-DLQ

aws sqs set-queue-attributes \
    --queue-url $QUEUE_URL \
    --attributes '{
        "RedrivePolicy": "{
            \"deadLetterTargetArn\":\"'$DLQ_ARN'\",
            \"maxReceiveCount\":\"3\"
        }"
    }'
```

## Configuration

### Environment Variables

**Complete Configuration Reference:**

```bash
# =============================================================================
# STARKNET CONFIGURATION
# =============================================================================
STARKNET_RPC_URL=http://localhost:5050
STARKNET_ACCOUNT_ADDRESS=0x127fd5f1fe78a71f8bcd1fec63e3fe2f0486b6ecd5c86a0466c3a21fa5cfcec
STARKNET_PRIVATE_KEY=0xc5b2fcab997346f3ea1c00b002ecf6f382c5f9c9659a3894eb783c5320f912

# Contract Addresses
FOSSIL_STORE_ADDRESS=0x00e581139553c8666f60b6646f277a336f99f108f8e5fa7cb300b6a6ce7c3b8c
HASH_STORAGE_ADDRESS=0x...
PITCHLAKE_VERIFIER_CONTRACT=0x...
PITCHLAKE_VAULT=0x...

# =============================================================================
# DATABASE CONFIGURATION
# =============================================================================
PROVING_SERVICE_DATABASE_URL=postgresql://postgres:postgres@localhost:5435/postgres
INDEXER_DATABASE_URL=postgresql://postgres:postgres@localhost:5433/postgres

# =============================================================================
# AWS CONFIGURATION (LocalStack for development)
# =============================================================================
AWS_REGION=us-east-1
AWS_ACCESS_KEY_ID=test
AWS_SECRET_ACCESS_KEY=test
AWS_ENDPOINT_URL=http://localhost:4567
SQS_QUEUE_URL=http://localhost:4567/000000000000/fossilQueue

# =============================================================================
# BONSAI CONFIGURATION
# =============================================================================
BONSAI_API_KEY=your_api_key_here
BONSAI_API_URL=https://api.bonsai.xyz/

# RISC0 Retry Configuration
RISC0_MAX_RETRIES=7
RISC0_INITIAL_RETRY_DELAY_MS=3000

# =============================================================================
# MESSAGE HANDLER CONFIGURATION
# =============================================================================
ENABLE_PROOF=true
USE_SIMPLE_MOCK=false
USE_RISC0_INTEGRATION=true
MAX_CONCURRENT_PROOFS=1

# =============================================================================
# STARKNET RETRY CONFIGURATION
# =============================================================================
STARKNET_MAX_RETRIES=3
STARKNET_INITIAL_BACKOFF_MS=100
STARKNET_MAX_BACKOFF_MS=1000

# =============================================================================
# VERIFICATION CONFIGURATION
# =============================================================================
VERIFY_PROOFS_ONCHAIN=true

# =============================================================================
# LOGGING CONFIGURATION
# =============================================================================
RUST_LOG=info,proving_service=debug,message_handler=debug
```

### Docker vs Native Development

**Docker Compose (.env.docker):**

```bash
# Use Docker service names
STARKNET_RPC_URL=http://katana:5050
AWS_ENDPOINT_URL=http://localstack:4566
PROVING_SERVICE_DATABASE_URL=postgresql://postgres:postgres@proving_service_db:5432/postgres
```

**Native Development (.env.local):**

```bash
# Use localhost
STARKNET_RPC_URL=http://localhost:5050
AWS_ENDPOINT_URL=http://localhost:4567
PROVING_SERVICE_DATABASE_URL=postgresql://postgres:postgres@localhost:5435/postgres
```

### Feature Flag Configuration

**Compile-Time Features:**

```bash
# Build with proof composition (production)
cargo build --release --features proof-composition

# Build with mock proofs (testing)
cargo build --release --features mock-proof

# Build without proof features (API only)
cargo build --release
```

**Runtime Features:**

```bash
# Enable proof generation at runtime
ENABLE_PROOF=true

# Use simple mock provider (no external dependencies)
USE_SIMPLE_MOCK=true

# Use RISC0 integration with mock proofs
USE_RISC0_INTEGRATION=true
```

### Port Assignments

| Service | Port | Purpose |
|---------|------|---------|
| Proving Service API | 3001 | HTTP API for job submission |
| StarkNet (Katana) | 5050 | Local StarkNet devnet |
| Proving Service DB | 5435 | PostgreSQL (external access) |
| Indexer DB | 5433 | PostgreSQL (external access) |
| LocalStack | 4567 | AWS SQS emulation |

## Next Steps

### Related Documentation

- [Architecture Overview](overview.md) - High-level system architecture
- [Data Flow Documentation](data-flow.md) - Complete request/response flows
- [Fossil API Architecture](fossil-api.md) - API service details
- [StarkNet Contracts](starknet-contracts.md) - Smart contract architecture

### Operational Guides

- [Local Development](../getting-started/local-development.md) - Setting up local environment
- [Deployment Guide](../guides/deployment.md) - Production deployment
- [Debugging Guide](../guides/debugging.md) - Troubleshooting common issues

### Advanced Topics

- **Proof Composition**: Understanding RISC Zero assumption mechanism
- **Bonsai Integration**: Optimizing remote proof generation
- **Queue Scaling**: Horizontal scaling with multiple workers
- **Monitoring**: Setting up observability and alerting

### Development Workflow

**1. Start Development Services:**

```bash
# From repository root
make dev-services

# This starts:
# - PostgreSQL (proving service DB)
# - PostgreSQL (indexer DB)
# - LocalStack (SQS)
# - Katana (StarkNet devnet)
```

**2. Run Proving Service API:**

```bash
cd proving-service/crates/proving-service
cargo run --release
```

**3. Run Message Handler:**

```bash
cd proving-service/crates/message-handler
cargo run --release --features mock-proof
```

**4. Submit Test Job:**

```bash
curl -X POST http://localhost:3001/api/job \
  -H "Content-Type: application/json" \
  -d '{
    "job_group_id": "test-job-1",
    "twap": {"start_timestamp": 1672531200, "end_timestamp": 1672617600},
    "reserve_price": {"start_timestamp": 1672531200, "end_timestamp": 1672617600},
    "max_return": {"start_timestamp": 1672531200, "end_timestamp": 1672617600},
    "vault_address": "0x004018ae0157b10d08cb1f70d34e32fd8b25e4ad1d70afc89616c3b300257fd9",
    "vault_timestamp": 1672531200
  }'
```

**5. Monitor Logs:**

```bash
# Proving Service API logs
tail -f proving-service-api.log

# Message Handler logs
tail -f message-handler.log
```

**6. Run Tests:**

```bash
# Test all crates
cd proving-service
make test

# Test individual crate
cd crates/db
cargo test

cd crates/message-handler
cargo test --features mock-proof
```

### Contributing

When contributing to the Proving Service:

1. **Code Style**: Follow Rust standard formatting (`cargo fmt`)
2. **Linting**: Run `cargo clippy` and fix all warnings
3. **Testing**: Add tests for new functionality
4. **Documentation**: Update architecture docs for significant changes
5. **PR Checklist**: Run `make pr` before submitting pull requests

### Performance Tuning

**Concurrent Proof Limit:**

Adjust based on Bonsai API rate limits:

```bash
# Conservative (avoid rate limiting)
MAX_CONCURRENT_PROOFS=1

# Aggressive (faster processing, may hit rate limits)
MAX_CONCURRENT_PROOFS=3
```

**Database Connection Pool:**

Adjust in `db/src/lib.rs`:

```rust
let pool = PgPoolOptions::new()
    .max_connections(5)  // Increase for high concurrency
    .connect(database_url)
    .await?;
```

**SQS Batch Size:**

Adjust in message handler:

```rust
let messages = sqs_client.receive_message()
    .queue_url(&queue_url)
    .max_number_of_messages(10)  // 1-10 messages per poll
    .send()
    .await?;
```
