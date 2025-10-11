# Message Handler Crate

**Location:** `/proving-service/crates/message-handler/`

## Overview

The `message-handler` crate is the core proof generation engine for the Fossil proving service. It processes job requests from AWS SQS queues, generates zero-knowledge proofs using RISC0 and Bonsai API, and manages the complete proof lifecycle including composition, verification, and on-chain submission to StarkNet.

### Key Responsibilities

- **Queue Processing**: Receives and processes proof generation requests from AWS SQS
- **Proof Generation**: Orchestrates ZK proof generation via RISC0/Bonsai API
- **Proof Composition**: Combines multiple sub-proofs (TWAP, max return, reserve price) into a single verifiable proof
- **Hash Management**: Prepares and stores cryptographic hashes of fee data on StarkNet
- **On-chain Verification**: Submits proofs to StarkNet verifier contracts
- **Job Management**: Handles retries, timeouts, duplicate detection, and failure recovery

## Crate Structure

```
message-handler/
├── src/
│   ├── lib.rs                          # Crate root, module exports
│   ├── main.rs                         # Primary message-handler binary
│   ├── example_service_main.rs         # Example binary for testing
│   ├── bonsai_test.rs                  # Bonsai API integration test binary
│   │
│   ├── queue/                          # Message queue abstractions
│   │   ├── mod.rs
│   │   ├── message_queue.rs            # Queue trait definition
│   │   ├── sqs_message_queue.rs        # AWS SQS implementation
│   │   └── local_message_queue.rs      # In-memory queue for testing
│   │
│   ├── services/                       # Core business logic
│   │   ├── mod.rs
│   │   ├── jobs.rs                     # Job data models (RequestProof, ProofGenerated)
│   │   ├── proof_job_handler.rs        # Main proof processing orchestrator
│   │   ├── job_dispatcher.rs           # Job submission utility
│   │   ├── hashing_service.rs          # Fee data hashing orchestration
│   │   └── example_message_handler.rs  # Simple example handler
│   │
│   ├── proof_composition/              # Proof composition logic
│   │   └── mod.rs                      # ProofProvider trait, BonsaiProofProvider
│   │
│   ├── hashing/                        # StarkNet hash operations
│   │   └── mod.rs                      # HashingProvider for fee data
│   │
│   ├── risc0_generator.rs              # RISC0 proof generation via Bonsai
│   ├── proof_verifier.rs               # Integrated proof verification
│   └── response_handler/               # StarkNet transaction handling
│       └── mod.rs                      # Account management, proof submission
│
└── Cargo.toml                          # Dependencies and feature flags
```

## Feature Flags

The crate uses Cargo feature flags to control proof generation modes:

### `proof-composition` (Production)

Enables full proof composition with real coprocessor methods.

**Dependencies:**
- `coprocessor_common`, `coprocessor_core` - Base coprocessor functionality
- Computation methods:
  - `add_twap_7d_error_bound_floating`
  - `max_return_floating`
  - `remove_seasonality_error_bound_floating`
  - `simulate_price_verify_position_floating`
  - `calculate_pt_pt1_error_bound_floating`
  - `twap_error_bound_floating`
  - `hashing_felts`
- `proof_composition_twap_maxreturn_reserveprice_floating_hashing_methods` - Composition guest program
- `starknet-handler` - StarkNet integration

**Use Case:** Production proof generation with actual financial calculations.

### `mock-proof` (Testing/Development)

Enables mock proof generation for testing without heavy computation.

**Dependencies:**
- `mock-proof-composition-methods` - Simplified guest program
- `coprocessor_core` - Core types only
- `nalgebra` - Linear algebra for mock data
- `garaga_rs` - Groth16 proof formatting for StarkNet
- `risc0-ethereum-contracts` - Seal encoding
- `starknet-handler` - StarkNet integration

**Use Case:** Development, testing, integration tests without Bonsai API dependency.

### Default (No Features)

When no features are enabled, the service runs in disabled mode - acknowledging jobs without processing them.

**Use Case:** Deployment scenarios where proof generation should be disabled.

## Job Models

### `RequestProof`

Represents a request to generate a proof.

```rust
pub struct RequestProof {
    pub job_id: String,                           // Unique job identifier
    pub job_group_id: Option<String>,             // Optional grouping
    pub start_timestamp: i64,                     // Overall start (fallback)
    pub end_timestamp: i64,                       // Overall end (fallback)

    // Specific timestamp ranges for each calculation
    pub twap_start_timestamp: Option<i64>,        // TWAP calculation range
    pub twap_end_timestamp: Option<i64>,
    pub reserve_price_start_timestamp: Option<i64>, // Reserve price range
    pub reserve_price_end_timestamp: Option<i64>,
    pub max_return_start_timestamp: Option<i64>,  // Max return range
    pub max_return_end_timestamp: Option<i64>,

    // On-chain verification metadata
    pub vault_address: Option<String>,            // Vault contract address
    pub vault_timestamp: Option<i64>,             // Vault timestamp for verification
}
```

**Timestamp Fallback Logic:**
- If specific timestamps (e.g., `twap_start_timestamp`) are `None`, falls back to general `start_timestamp`/`end_timestamp`
- Allows flexible configuration per calculation type

### `ProofGenerated`

Contains the completed proof and job metadata.

```rust
pub struct ProofGenerated {
    pub job_id: String,              // Matches original RequestProof.job_id
    pub receipt: Receipt,            // RISC0 Receipt with proof data
}
```

### `Job` Enum

Tagged union for different job types.

```rust
pub enum Job {
    RequestProof(RequestProof),
    ProofGenerated(Box<ProofGenerated>),  // Boxed due to large Receipt
}
```

**Serialization:** Uses `serde` with `#[serde(untagged)]` for JSON message parsing.

## Queue System

### `Queue` Trait

Abstract interface for message queues.

```rust
#[async_trait]
pub trait Queue {
    async fn send_message(&self, message: String) -> Result<(), QueueError>;
    async fn receive_messages(&self) -> Result<Vec<QueueMessage>, QueueError>;
    async fn delete_message(&self, message: &QueueMessage) -> Result<(), QueueError>;
}
```

### `QueueMessage`

```rust
pub struct QueueMessage {
    pub body: String,           // JSON-serialized Job
    pub id: Option<String>,     // Receipt handle for deletion (SQS-specific)
}
```

### `QueueError`

```rust
pub enum QueueError {
    SendError(String),
    ReceiveError(String),
    DeleteError(String),
}
```

### Implementations

#### `SqsMessageQueue` (Production)

AWS SQS integration for production workloads.

```rust
pub struct SqsMessageQueue {
    queue_url: String,
    client: Client,  // AWS SDK Client
}
```

**Configuration:**
- **Wait Time:** 20 seconds (long polling)
- **Max Messages:** 10 per receive
- **Visibility Timeout:** 3600 seconds (1 hour) - prevents duplicate processing during long proof generation

**Usage:**
```rust
let config = aws_config::defaults(BehaviorVersion::latest()).load().await;
let queue = Arc::new(SqsMessageQueue::new(queue_url, config));
```

#### `LocalMessageQueue` (Testing)

In-memory queue implementation for unit tests.

```rust
pub struct LocalMessageQueue {
    messages: Arc<Mutex<Vec<QueueMessage>>>,
}
```

**Features:**
- Thread-safe via `Arc<Mutex<>>`
- Generates UUID message IDs
- Suitable for integration tests

## Proof Job Handler

**File:** `services/proof_job_handler.rs`

The `ProofJobHandler` is the core orchestrator that:
1. Polls the queue for `RequestProof` jobs
2. Manages concurrent proof generation with semaphore limiting
3. Handles duplicate detection and failure tracking
4. Dispatches `ProofGenerated` results back to the queue

### Structure

```rust
pub struct ProofJobHandler<Q: Queue + Send + Sync + 'static> {
    queue: Arc<Q>,
    terminator: Arc<AtomicBool>,                          // Graceful shutdown flag
    proof_provider: Arc<dyn ProofProvider + Send + Sync>, // Proof generation strategy
    proof_generation_timeout: Duration,                    // Max proof generation time
    processing_jobs: Arc<Mutex<HashSet<String>>>,         // Prevents duplicate processing
    job_failures: Arc<Mutex<HashMap<String, JobProcessingState>>>, // Failure tracking
    max_failures: u32,                                     // Max retries before delete (3)
    proof_generation_semaphore: Arc<Semaphore>,           // Concurrency limiter
}

struct JobProcessingState {
    failure_count: u32,
    last_failure_time: Instant,
}
```

### Key Features

#### 1. Semaphore-Based Concurrency Control

Prevents Bonsai API contention by limiting concurrent proof generations.

```rust
// Environment variable: MAX_CONCURRENT_PROOFS (default: 1)
let concurrent_proofs = std::env::var("MAX_CONCURRENT_PROOFS")
    .ok()
    .and_then(|s| s.parse().ok())
    .unwrap_or(1);

proof_generation_semaphore: Arc::new(Semaphore::new(concurrent_proofs))
```

**Per-Job Flow:**
```rust
let _permit = proof_semaphore.acquire().await?;
// Proof generation happens here (only N concurrent)
// Permit automatically released when dropped
```

#### 2. Duplicate Detection

Prevents the same job from being processed multiple times.

```rust
let mut processing_jobs = self.processing_jobs.lock().await;
if processing_jobs.contains(&job.job_id) {
    info!("Job ID {} is already being processed, skipping", job.job_id);
    continue;
}
processing_jobs.insert(job.job_id.clone());
```

#### 3. Failure Tracking with Auto-Delete

Jobs that fail repeatedly (3 times by default) are automatically deleted from the queue.

```rust
let mut failures = job_failures.lock().await;
let failure_entry = failures.entry(job.job_id.clone())
    .or_insert_with(|| JobProcessingState {
        failure_count: 0,
        last_failure_time: Instant::now(),
    });

if failure_entry.failure_count >= max_failures {
    // Forcibly delete after max failures to prevent infinite loops
    queue.delete_message(&message).await?;
}
```

#### 4. Timeout Protection

Each proof generation has a timeout (default 3600 seconds / 1 hour).

```rust
let proof_result = tokio::time::timeout(
    timeout_duration,
    proof_provider.generate_proofs_from_data(timestamp_ranges, Some(&job)),
)
.await;
```

### Processing Loop

```rust
pub async fn receive_job(&self) -> Result<()> {
    while !self.terminator.load(Ordering::Relaxed) {
        // 1. Receive messages from queue
        let messages = self.queue.receive_messages().await?;

        for message in messages {
            // 2. Parse Job from JSON
            let job: Job = serde_json::from_str(&message.body)?;

            // 3. Extract RequestProof (skip other job types)
            let job = match job {
                Job::RequestProof(job) => job,
                _ => { queue.delete_message(&message).await?; continue; }
            };

            // 4. Check duplicate processing
            if processing_jobs.contains(&job.job_id) { continue; }

            // 5. Check if proof provider is disabled
            if proof_provider.is_disabled() {
                queue.delete_message(&message).await?;
                continue;
            }

            // 6. Spawn async task for proof generation
            join_set.spawn(async move {
                // Acquire semaphore permit
                let _permit = proof_semaphore.acquire().await?;

                // Create timestamp ranges
                let timestamp_ranges = create_timestamp_ranges(&job);

                // Generate proof with timeout
                let proof_result = tokio::time::timeout(
                    timeout_duration,
                    proof_provider.generate_proofs_from_data(timestamp_ranges, Some(&job))
                ).await;

                match proof_result {
                    Ok(Ok(receipt)) => {
                        // Success: Send ProofGenerated to queue
                        let proof_generated = Job::ProofGenerated(Box::new(ProofGenerated {
                            job_id: job.job_id.clone(),
                            receipt,
                        }));
                        send_job_to_queue(&queue, &proof_generated).await?;
                        queue.delete_message(&message).await?;
                    }
                    Ok(Err(e)) | Err(_) => {
                        // Failure: Increment failure count
                        // Delete if max failures reached, else requeue
                    }
                }
            });
        }
    }

    // Wait for all spawned tasks to complete
    join_set.join_all().await;
    Ok(())
}
```

## Proof Composition

**File:** `proof_composition/mod.rs`

### `ProofProvider` Trait

Abstract interface for proof generation strategies.

```rust
#[async_trait]
pub trait ProofProvider {
    async fn generate_proofs_from_data(
        &self,
        timestamp_ranges: ProofTimestampRanges,
        job_context: Option<&RequestProof>,
    ) -> Result<Receipt>;

    fn is_disabled(&self) -> bool { false }
}
```

### `ProofTimestampRanges`

Encapsulates different timestamp ranges for each calculation.

```rust
pub struct ProofTimestampRanges {
    pub twap: (i64, i64),           // TWAP calculation range
    pub reserve_price: (i64, i64),  // Reserve price calculation range
    pub max_return: (i64, i64),     // Max return calculation range
}

impl ProofTimestampRanges {
    pub fn overall_range(&self) -> (i64, i64) {
        let end = max(max(self.twap.1, self.reserve_price.1), self.max_return.1);

        // Safety buffer for Fossil store indexing lag (12 hours)
        let safe_end = end - (12 * 3600);
        let safe_end_normalized = (safe_end / 3600) * 3600;

        // POC: 1440 hours (2 months) + 120 hour buffer
        // Production: 5760 hours (8 months) + 120 hour buffer
        const REQUIRED_HOURS: i64 = 1440;
        const FETCH_BUFFER_HOURS: i64 = 120;

        let start = safe_end_normalized - ((REQUIRED_HOURS + FETCH_BUFFER_HOURS) * 3600);
        (start, safe_end_normalized)
    }
}
```

**Key Insights:**
- **Fossil Store Lag:** Subtracts 12-hour buffer to ensure data availability
- **Fetch Buffer:** Adds 120-hour buffer to account for data gaps
- **POC vs Production:** Currently uses 1440 hours (2 months), production needs 5760 hours (8 months)

### `BonsaiProofProvider`

Production implementation using RISC0 and Bonsai API.

#### Proof Generation Pipeline

**Feature: `proof-composition`**

```rust
async fn generate_proofs_from_data(
    &self,
    timestamp_ranges: ProofTimestampRanges,
    job_context: Option<&RequestProof>,
) -> Result<Receipt> {
    // 1. Initialize StarkNet provider and hashing service
    let config = load_starknet_config()?;
    let provider = StarknetProvider::new(config)?;
    let hashing_provider = HashingProvider::from_env()?;
    let hashing_service = HashingService::new(hashing_provider, 1440, 180);

    // 2. Run hash preparation (stores hashes on-chain if needed)
    let (overall_start, overall_end) = timestamp_ranges.overall_range();
    hashing_service.run(overall_start as u64).await?;

    // 3. Fetch and validate fee data from StarkNet
    let raw_input = Self::fetch_and_validate_fee_data(&provider, overall_start, overall_end).await?;

    // 4. Generate basic sub-proofs (hashing, max return, TWAP)
    let (hashing_receipt, hashing_res, max_return_receipt, max_return_value,
         calculate_twap_receipt, twap_original) =
        Self::generate_basic_sub_proofs(raw_input, &data_8_months, &data_3_months)?;

    // 5. Generate reserve price sub-proofs in parallel
    let (res, remove_seasonality_receipt, calculate_pt_pt1_receipt,
         add_twap_7d_receipt, simulate_price_receipt) =
        Self::generate_reserve_price_sub_proofs(data_3_months, reserve_price_start,
                                                 overall_start, overall_end).await?;

    // 6. Build ProofCompositionInput
    let composition_input = Self::build_proof_composition_input(
        data_8_months, hashing_res.hash, overall_start, overall_end,
        &timestamp_ranges, &res, twap_original, max_return_value
    );

    // 7. Compose final proof with 7 sub-proof assumptions
    let receipt = Self::compose_final_proof(
        composition_input,
        hashing_receipt,
        max_return_receipt,
        calculate_twap_receipt,
        remove_seasonality_receipt,
        calculate_pt_pt1_receipt,
        add_twap_7d_receipt,
        simulate_price_receipt
    ).await?;

    Ok(receipt)
}
```

#### Sub-Proof Generation

**Basic Sub-Proofs (Sequential):**
```rust
fn generate_basic_sub_proofs(
    raw_input: Vec<Felt>,
    data_full: &[f64],      // 1440 hours (POC) or 5760 hours (Production)
    data_3_months: &[f64],  // 720 hours (POC) or 2160 hours (Production)
) -> Result<(Receipt, HashingFeltOutput, Receipt, f64, Receipt, f64)> {
    // 1. Hash felts (convert Felt to f64, compute hash)
    let (hashing_receipt, hashing_res) = hash_felts(HashingFeltInput { inputs: raw_input });

    // 2. Max return (volatility calculation on full dataset)
    let (max_return_receipt, max_return_res) = max_return(MaxReturnInput {
        data: data_full.to_vec(),
    });

    // 3. TWAP (time-weighted average price on 3-month subset)
    let twap_original = floating_point::calculate_twap(&data_3_months.to_vec());
    let (calculate_twap_receipt, _) = calculate_twap(TwapErrorBoundInput {
        avg_hourly_gas_fee: data_3_months.to_vec(),
        twap_tolerance: 1.0,
        twap_result: twap_original,
    });

    Ok((hashing_receipt, hashing_res, max_return_receipt,
         max_return_res.1, calculate_twap_receipt, twap_original))
}
```

**Reserve Price Sub-Proofs (Parallel):**
```rust
async fn generate_reserve_price_sub_proofs(
    data_3_months: Vec<f64>,
    reserve_price_start: i64,
    overall_start: i64,
    overall_end: i64,
) -> Result<(AllInputsToReservePrice, Receipt, Receipt, Receipt, Receipt)> {
    // Calculate reserve price using original method
    let data_with_timestamps = convert_data_to_vec_of_tuples(data_3_months.clone(), reserve_price_start);
    let res = original::calculate_reserve_price(&data_with_timestamps, 15000, 720);

    // Spawn parallel tasks (4 sub-proofs)
    let remove_seasonality_task = tokio::spawn(async move {
        remove_seasonality_error_bound(RemoveSeasonalityErrorBoundFloatingInput { /* ... */ })
    });

    let calculate_pt_pt1_task = tokio::spawn(async move {
        calculate_pt_pt1_error_bound_floating(CalculatePtPt1ErrorBoundFloatingInput { /* ... */ })
    });

    let add_twap_7d_task = tokio::spawn(async move {
        add_twap_7d_error_bound(AddTwap7dErrorBoundFloatingInput { /* ... */ })
    });

    let simulate_price_task = tokio::spawn(async move {
        simulate_price_verify_position(SimulatePriceVerifyPositionInput { /* ... */ })
    });

    // Join all parallel tasks
    let receipts = try_join!(remove_seasonality_task, calculate_pt_pt1_task,
                             add_twap_7d_task, simulate_price_task)?;

    Ok((res, receipts.0.0, receipts.1.0, receipts.2.0, receipts.3.0))
}
```

#### Final Proof Composition

```rust
async fn compose_final_proof(
    composition_input: ProofCompositionInput,
    hashing_receipt: Receipt,
    max_return_receipt: Receipt,
    calculate_twap_receipt: Receipt,
    remove_seasonality_receipt: Receipt,
    calculate_pt_pt1_receipt: Receipt,
    add_twap_7d_receipt: Receipt,
    simulate_price_receipt: Receipt,
) -> Result<Receipt> {
    let receipt = task::spawn_blocking(move || {
        let env = ExecutorEnv::builder()
            .add_assumption(hashing_receipt)            // Sub-proof 1: Data hashing
            .add_assumption(max_return_receipt)         // Sub-proof 2: Max return
            .add_assumption(calculate_twap_receipt)     // Sub-proof 3: TWAP
            .add_assumption(remove_seasonality_receipt) // Sub-proof 4: Seasonality removal
            .add_assumption(calculate_pt_pt1_receipt)   // Sub-proof 5: Markov matrices
            .add_assumption(add_twap_7d_receipt)        // Sub-proof 6: 7-day TWAP
            .add_assumption(simulate_price_receipt)     // Sub-proof 7: Price simulation
            .write(&composition_input)
            .unwrap()
            .build()
            .unwrap();

        default_prover().prove(
            env,
            PROOF_COMPOSITION_TWAP_MAXRETURN_RESERVEPRICE_FLOATING_HASHING_GUEST_ELF
        )
    })
    .await
    .unwrap()
    .unwrap()
    .receipt;

    Ok(receipt)
}
```

**Feature: `mock-proof`**

Simplified mock proof for testing:

```rust
async fn generate_proofs_from_data(
    &self,
    timestamp_ranges: ProofTimestampRanges,
    job_context: Option<&RequestProof>,
) -> Result<Receipt> {
    let mock_input = ProofCompositionInput {
        data_8_months: vec![0.1, 0.2, 0.3, 0.4, 0.5],
        data_8_months_hash: [0x12345678, /* ... */],
        start_timestamp: timestamp_ranges.overall_range().0,
        end_timestamp: timestamp_ranges.overall_range().1,
        // ... other mock fields
    };

    let result = tokio::task::spawn_blocking(move || -> Result<Receipt> {
        let env = ExecutorEnv::builder()
            .write(&mock_input)?
            .build()?;

        let prover_result = default_prover().prove_with_ctx(
            env,
            &VerifierContext::default(),
            MOCK_PROOF_COMPOSITION_GUEST_ELF,
            &ProverOpts::groth16(),
        )?;

        Ok(prover_result.receipt)
    })
    .await??;

    Ok(result)
}
```

## RISC0 Integration

**File:** `risc0_generator.rs`

### `Risc0Config`

Configuration for RISC0 proof generation.

```rust
pub struct Risc0Config {
    pub max_retries: u32,                  // Max retry attempts (default: 5)
    pub initial_retry_delay_ms: u64,       // Initial backoff delay (default: 2000ms)
    pub verifier_contract_address: String, // StarkNet verifier contract
}

impl Default for Risc0Config {
    fn default() -> Self {
        Self {
            max_retries: std::env::var("RISC0_MAX_RETRIES")
                .ok().and_then(|s| s.parse().ok()).unwrap_or(5),
            initial_retry_delay_ms: std::env::var("RISC0_INITIAL_RETRY_DELAY_MS")
                .ok().and_then(|s| s.parse().ok()).unwrap_or(2000),
            verifier_contract_address: std::env::var("PITCHLAKE_VERIFIER_CONTRACT")
                .unwrap_or_else(|_| "0x0".to_string()),
        }
    }
}
```

### `Risc0ProofResult`

```rust
pub struct Risc0ProofResult {
    pub receipt: Receipt,                          // RISC0 proof receipt
    pub calldata: Vec<Felt>,                       // StarkNet calldata for verification
    #[cfg(feature = "mock-proof")]
    pub journal_output: ProofCompositionOutput,    // Decoded journal (reserve_price, twap, max_return)
    pub generation_time_ms: u64,                   // Time taken to generate proof
}
```

### `Risc0ProofGenerator` Trait

```rust
#[async_trait]
pub trait Risc0ProofGenerator {
    async fn generate_proof_with_data(
        &self,
        timestamp_ranges: ProofTimestampRanges,
        config: Risc0Config,
    ) -> Result<Risc0ProofResult>;

    #[cfg(feature = "mock-proof")]
    async fn generate_mock_proof(
        &self,
        timestamp_ranges: ProofTimestampRanges,
        config: Risc0Config,
    ) -> Result<Risc0ProofResult>;
}
```

### `Risc0Generator` Implementation

**Proof Generation with Retries:**

```rust
async fn generate_proof_internal(
    &self,
    input: ProofCompositionInput,
    config: &Risc0Config,
) -> Result<Risc0ProofResult> {
    let start_time = Instant::now();
    let mut last_error = None;

    for attempt in 1..=config.max_retries {
        info!("RISC0 proof generation attempt {}/{}", attempt, config.max_retries);

        match self.generate_proof_attempt(input.clone()).await {
            Ok(result) => {
                let generation_time_ms = start_time.elapsed().as_millis() as u64;
                return Ok(Risc0ProofResult {
                    receipt: result.0,
                    calldata: result.1,
                    journal_output: result.2,
                    generation_time_ms,
                });
            }
            Err(e) => {
                last_error = Some(e);
                if attempt < config.max_retries {
                    // Exponential backoff: 2s, 4s, 8s, 16s, 32s
                    let delay = config.initial_retry_delay_ms * (2_u64.pow(attempt - 1));
                    tokio::time::sleep(Duration::from_millis(delay)).await;
                }
            }
        }
    }

    Err(last_error.unwrap())
}
```

**Single Proof Attempt (with Bonsai API):**

```rust
async fn generate_proof_attempt(
    &self,
    input: ProofCompositionInput,
) -> Result<(Receipt, Vec<Felt>, ProofCompositionOutput)> {
    // Use spawn_blocking to avoid Tokio runtime conflicts
    let result = tokio::task::spawn_blocking({
        let input = input.clone();

        move || -> Result<ProveInfo> {
            let env = ExecutorEnv::builder()
                .write(&input)?
                .build()?;

            // Call Bonsai prover with Groth16
            default_prover().prove_with_ctx(
                env,
                &VerifierContext::default(),
                MOCK_PROOF_COMPOSITION_GUEST_ELF,
                &ProverOpts::groth16(),
            )
        }
    })
    .await??;

    let receipt = result.receipt;

    // Decode journal output
    let journal_output: ProofCompositionOutput = receipt.journal.decode()?;

    // Encode seal for StarkNet
    let encoded_seal = encode_seal(&receipt)?;
    let image_id = compute_image_id(MOCK_PROOF_COMPOSITION_GUEST_ELF)?;
    let journal = receipt.journal.bytes.clone();

    // Create Groth16 proof for StarkNet
    let groth16_proof = Groth16Proof::from_risc0(
        encoded_seal,
        image_id.as_bytes().to_vec(),
        journal
    );

    // Generate StarkNet calldata
    let calldata = get_groth16_calldata_felt(
        &groth16_proof,
        &get_risc0_vk(),
        CurveID::BN254
    )?;

    Ok((receipt, calldata, journal_output))
}
```

## Hashing Service

**File:** `services/hashing_service.rs`

The `HashingService` orchestrates hash preparation for fee data before proof generation. It ensures that all required hash batches are available on-chain.

### Structure

```rust
pub struct HashingService<T: HashingProviderTrait + Sync + Send + 'static> {
    hashing_provider: Arc<T>,
    required_avg_fees_length: usize,  // Total hours needed (1440 for POC)
    hash_batch_size: usize,            // Batch size per hash (180 hours)
}
```

### Pipeline

```rust
pub async fn run(&self, start_timestamp: u64) -> Result<(), String> {
    // 1. Normalize timestamp to hour boundary
    let normalized_start = (start_timestamp / 3600) * 3600;
    let end_timestamp = normalized_start + 3600 * (self.required_avg_fees_length as u64 - 1);

    // 2. Check fee data availability
    self.check_avg_fees_availability(normalized_start, end_timestamp).await?;

    // 3. Get list of missing hash batches
    let unavailable_batch_timestamp_hashes = self
        .get_unavailable_batch_timestamp_hashes(normalized_start, end_timestamp)
        .await?;

    // 4. Store missing hashes on-chain (sequential to avoid nonce conflicts)
    if !unavailable_batch_timestamp_hashes.is_empty() {
        self.hash_and_store_avg_fees_onchain(unavailable_batch_timestamp_hashes).await?;
    }

    // 5. Create batch hash (hash of hashes)
    if !self.is_batch_hash_avg_fees_available(normalized_start).await? {
        self.hash_batch_avg_fees_onchain(normalized_start).await?;
    }

    Ok(())
}
```

### Fee Data Validation

```rust
async fn check_avg_fees_availability(
    &self,
    start_timestamp: u64,
    end_timestamp: u64,
) -> Result<(), String> {
    let avg_fees = self.hashing_provider
        .get_avg_fees_in_range(start_timestamp, end_timestamp)
        .await?;

    // Check for zero fees (will cause Cairo contract to fail)
    let zero_count = avg_fees.iter().filter(|&&fee| fee == 0.0).count();
    if zero_count > 0 {
        warn!("Found {} zero fees out of {} total", zero_count, avg_fees.len());
    }

    Ok(())
}
```

### Hash Storage (Sequential Transactions)

```rust
async fn hash_and_store_avg_fees_onchain(
    &self,
    unavailable_batch_timestamp_hashes: Vec<u64>,
) -> Result<(), String> {
    // Submit transactions SEQUENTIALLY to avoid nonce conflicts
    for timestamp in unavailable_batch_timestamp_hashes {
        // Submit transaction
        let tx_result = self.hashing_provider
            .hash_avg_fees_and_store(timestamp)
            .await?;

        // Wait for receipt (60 retries × 3s = 3 minutes max)
        let receipt = self.wait_for_transaction_receipt(
            tx_result.transaction_hash,
            60,
            3000
        ).await?;

        // Check for revert
        if let TransactionReceipt::Invoke(invoke_receipt) = &receipt.receipt {
            if invoke_receipt.execution_result.status() == TransactionExecutionStatus::Reverted {
                return Err(format!("Transaction reverted for timestamp {}", timestamp));
            }
        }
    }

    Ok(())
}
```

### `HashingProvider` Trait

**File:** `hashing/mod.rs`

Interface for StarkNet fee data and hash operations.

```rust
#[async_trait]
pub trait HashingProviderTrait {
    fn get_provider(&self) -> &JsonRpcClient<HttpTransport>;
    fn get_fossil_store_address(&self) -> &Felt;
    fn get_hash_storage_address(&self) -> &Felt;

    // Fetch average fees from Fossil store
    async fn get_avg_fees_in_range(
        &self,
        start_timestamp: u64,
        end_timestamp: u64,
    ) -> Result<Vec<f64>, ProviderError>;

    async fn get_avg_fees_in_range_as_felt(
        &self,
        start_timestamp: u64,
        end_timestamp: u64,
    ) -> Result<Vec<Felt>, ProviderError>;

    // Retrieve stored hashes
    async fn get_hash_stored_avg_fees(&self, timestamp: u64) -> Result<[u32; 8], ProviderError>;
    async fn get_hash_batched_avg_fees(&self, start_timestamp: u64) -> Result<[u32; 8], ProviderError>;

    // Store hashes on-chain
    async fn hash_avg_fees_and_store(&self, start_timestamp: u64) -> Result<InvokeTransactionResult>;
    async fn hash_batched_avg_fees(&self, start_timestamp: u64) -> Result<InvokeTransactionResult>;
}
```

**Implementation:**

```rust
pub struct HashingProvider {
    provider: JsonRpcClient<HttpTransport>,
    fossil_store_address: Felt,     // Read fee data
    hash_storage_address: Felt,      // Read/write hashes
    account: SingleOwnerAccount<JsonRpcClient<HttpTransport>, LocalWallet>,
}

impl HashingProvider {
    pub fn from_env() -> Result<Self> {
        // Load from environment:
        // - STARKNET_RPC_URL
        // - FOSSIL_STORE_ADDRESS
        // - HASH_STORAGE_ADDRESS
        // - STARKNET_PRIVATE_KEY
        // - STARKNET_ACCOUNT_ADDRESS
    }
}
```

## Proof Verifier

**File:** `proof_verifier.rs`

Integrates RISC0 proof generation with on-chain verification.

### `ProofVerifierConfig`

```rust
pub struct ProofVerifierConfig {
    pub risc0_config: Risc0Config,
    pub verifier_contract_address: String,
    pub verify_onchain: bool,
    pub job_request: Option<PitchLakeJobRequest>,  // Required for on-chain verification
}
```

### `ProofVerificationResult`

```rust
pub struct ProofVerificationResult {
    pub risc0_result: Risc0ProofResult,
    pub onchain_tx_hash: Option<Felt>,  // Transaction hash if verified on-chain
    pub total_time_ms: u64,
}
```

### `IntegratedProofVerifier`

```rust
pub struct IntegratedProofVerifier<G: Risc0ProofGenerator> {
    generator: G,
}

#[async_trait]
impl<G: Risc0ProofGenerator + Send + Sync> ProofVerifier for IntegratedProofVerifier<G> {
    async fn generate_and_verify_proof(
        &self,
        timestamp_ranges: ProofTimestampRanges,
        config: ProofVerifierConfig,
    ) -> Result<ProofVerificationResult> {
        // Step 1: Generate RISC0 proof
        let risc0_result = self.generator
            .generate_proof_with_data(timestamp_ranges, config.risc0_config)
            .await?;

        // Step 2: Verify on-chain (if configured)
        let onchain_tx_hash = if config.verify_onchain {
            let job_request = config.job_request.ok_or_else(||
                eyre!("job_request is required for onchain verification")
            )?;

            Some(self.verify_proof_onchain(
                risc0_result.calldata.clone(),
                &config.verifier_contract_address,
                job_request,
            ).await?)
        } else {
            None
        };

        Ok(ProofVerificationResult {
            risc0_result,
            onchain_tx_hash,
            total_time_ms: total_start_time.elapsed().as_millis() as u64,
        })
    }
}
```

### On-Chain Verification

```rust
async fn verify_proof_onchain(
    &self,
    calldata: Vec<Felt>,
    verifier_address: &str,
    job_request: PitchLakeJobRequest,
) -> Result<Felt> {
    // Load StarkNet configuration
    let config = load_starknet_config()?;
    let provider = StarknetProvider::new(config)?;

    // Convert to starknet-handler job request
    let starknet_job_request = starknet_handler::account::PitchLakeJobRequest {
        vault_address: job_request.vault_address,
        timestamp: job_request.timestamp,
        program_id: job_request.program_id,
    };

    // Verify proof on-chain
    let tx_hash = provider
        .verify_proof_onchain(verifier_address, calldata, starknet_job_request)
        .await?;

    Ok(tx_hash)
}
```

## Response Handler

**File:** `response_handler/mod.rs`

Manages StarkNet account operations and proof submission.

### `PitchLakeJobRequest`

```rust
#[derive(Clone, Default, Debug, Encode, Decode)]
pub struct PitchLakeJobRequest {
    pub vault_address: Felt,  // Vault contract address
    pub timestamp: u64,       // Proof timestamp
    pub program_id: Felt,     // 'PITCH_LAKE_V1' or program hash
}
```

### `StarknetAccount`

```rust
pub struct StarknetAccount {
    account: SingleOwnerAccount<Arc<JsonRpcClient<HttpTransport>>, LocalWallet>,
}

impl StarknetAccount {
    pub fn from_env() -> Result<Self> {
        // Load from:
        // - STARKNET_RPC_URL
        // - STARKNET_PRIVATE_KEY
        // - STARKNET_ACCOUNT_ADDRESS
    }

    pub async fn verify_proof(
        &self,
        verifier_address: &str,
        proof: Vec<Felt>,
        request: PitchLakeJobRequest,
    ) -> Result<Felt> {
        const MAX_RETRIES: u32 = 3;
        const INITIAL_BACKOFF: Duration = Duration::from_secs(1);

        // Encode calldata
        let mut calldata: Vec<Felt> = vec![];
        proof.encode(&mut calldata)?;
        request.encode(&mut calldata)?;

        // Submit with retries and exponential backoff
        let call = Call {
            to: Self::felt(verifier_address)?,
            selector: selector!("verify_proof"),
            calldata,
        };

        for attempt in 0..MAX_RETRIES {
            match self.account.execute_v3(vec![call.clone()]).send().await {
                Ok(tx) => return Ok(tx.transaction_hash),
                Err(e) if attempt < MAX_RETRIES - 1 => {
                    let backoff = INITIAL_BACKOFF * 2u32.pow(attempt);
                    tokio::time::sleep(backoff).await;
                }
                Err(e) => return Err(e.into()),
            }
        }
    }
}
```

## Configuration

All environment variables referenced in the crate:

### Queue Configuration

- **`SQS_QUEUE_URL`** (required): AWS SQS queue URL for job processing
  - Example: `https://sqs.us-east-1.amazonaws.com/123456789012/fossil-proving-queue`

- **`AWS_ENDPOINT_URL`** (optional): Override AWS endpoint (for LocalStack)
  - Example: `http://localhost:4566`

### Database

- **`PROVING_SERVICE_DATABASE_URL`** (required): PostgreSQL connection string
  - Example: `postgresql://user:pass@localhost:5432/proving_service`

### Proof Generation Control

- **`ENABLE_PROOF`** (default: `false`): Enable/disable proof generation
  - Values: `true`, `false`
  - When `false`, jobs are acknowledged without processing

- **`USE_SIMPLE_MOCK`** (default: `false`): Use simplified mock proofs (no external deps)
  - Values: `true`, `false`
  - Useful for testing without Bonsai API

- **`USE_RISC0_INTEGRATION`** (default: `false`): Use RISC0 integration path
  - Values: `true`, `false`
  - Only effective with `mock-proof` feature

- **`MAX_CONCURRENT_PROOFS`** (default: `1`): Concurrent proof generation limit
  - Example: `1`, `2`, `5`
  - Prevents Bonsai API contention

- **`SAVE_PROOF_COMPOSITION_INPUT`** (default: `false`): Save proof input to JSON
  - Values: `true`, `false`
  - Saves to `proof_composition_input.json` for debugging

### RISC0 / Bonsai API

- **`BONSAI_API_KEY`** (required for Bonsai): Bonsai API authentication key
  - Example: `YOUR_API_KEY_HERE`

- **`BONSAI_API_URL`** (default: `https://api.bonsai.xyz/`): Bonsai API endpoint
  - Example: `https://api.bonsai.xyz/`

- **`RISC0_MAX_RETRIES`** (default: `5`): Max retry attempts for proof generation
  - Example: `5`, `10`

- **`RISC0_INITIAL_RETRY_DELAY_MS`** (default: `2000`): Initial retry backoff (ms)
  - Example: `2000`, `5000`
  - Exponential backoff: 2s, 4s, 8s, 16s, 32s

### StarkNet Configuration

- **`STARKNET_RPC_URL`** (required): StarkNet RPC endpoint
  - Example: `http://localhost:5050`, `https://rpc.starknet.sepolia.io`

- **`STARKNET_ACCOUNT_ADDRESS`** (required): Account contract address
  - Example: `0x127fd5f1fe78a71f8bcd1fec63e3fe2f0486b6ecd5c86a0466c3a21fa5cfcec`

- **`STARKNET_PRIVATE_KEY`** (required): Account private key
  - Example: `0xc5b2fcab997346f3ea1c00b002ecf6f382c5f9c9659a3894eb783c5320f912`

- **`FOSSIL_STORE_ADDRESS`** (required): Fossil store contract address
  - Example: `0x00e581139553c8666f60b6646f277a336f99f108f8e5fa7cb300b6a6ce7c3b8c`

- **`HASH_STORAGE_ADDRESS`** (required): Hash storage contract address
  - Example: `0x...`

- **`PITCHLAKE_VERIFIER_CONTRACT`** (optional): Verifier contract address
  - Example: `0x...`
  - Set to `0x0` or empty to disable on-chain verification

- **`PITCHLAKE_VAULT`** (optional): Vault contract address for verification
  - Example: `0x...`

- **`VERIFY_PROOFS_ONCHAIN`** (default: `false`): Enable on-chain verification
  - Values: `true`, `false`

- **`PROVING_DELAY`** (default: `120`): Proving delay in seconds
  - Example: `120`, `300`
  - Used for timestamp calculation in verification

### StarkNet Retry Configuration

- **`STARKNET_MAX_RETRIES`** (default: `3`): Max retry attempts for StarkNet transactions
  - Example: `3`, `5`

- **`STARKNET_INITIAL_BACKOFF_MS`** (default: `100`): Initial backoff delay (ms)
  - Example: `100`, `500`

- **`STARKNET_MAX_BACKOFF_MS`** (default: `1000`): Maximum backoff delay (ms)
  - Example: `1000`, `5000`

### Mock Data (Development Only)

- **`USE_MOCK_STARKNET_DATA`** (default: `false`): Use mock StarkNet data
  - Values: `true`, `false`
  - For testing without on-chain data

### Logging

- **`RUST_LOG`** (optional): Tracing log level
  - Values: `trace`, `debug`, `info`, `warn`, `error`
  - Example: `RUST_LOG=message_handler=debug,risc0_zkvm=info`

## Binaries

### 1. `message-handler` (Primary Service)

**Path:** `src/main.rs`

Production message handler that polls SQS and processes proof jobs.

**Features:**
- AWS SQS integration
- Graceful shutdown (Ctrl+C)
- Proof provider selection based on features and environment
- Database integration

**Run:**
```bash
# Production with proof-composition
cargo run --bin message-handler --features proof-composition

# Mock proof for testing
cargo run --bin message-handler --features mock-proof

# Disabled mode (acknowledge jobs without processing)
cargo run --bin message-handler
```

**Environment Setup:**
```bash
SQS_QUEUE_URL=https://sqs.us-east-1.amazonaws.com/.../queue
PROVING_SERVICE_DATABASE_URL=postgresql://user:pass@localhost/db
ENABLE_PROOF=true
BONSAI_API_KEY=your_key_here
```

### 2. `example-message-handler` (Testing Service)

**Path:** `src/example_service_main.rs`

Example service demonstrating job dispatching and processing.

**Features:**
- Simple message dispatcher that sends jobs every second
- Example message handler (non-proof processing)
- Useful for testing queue integration

**Run:**
```bash
cargo run --bin example-message-handler
```

**Use Case:** Testing SQS integration without proof generation overhead.

### 3. `bonsai-test` (Bonsai API Test)

**Path:** `src/bonsai_test.rs`

Standalone test for Bonsai API integration.

**Features:**
- Direct Bonsai API call with mock data
- Enhanced error diagnostics
- No queue or database dependencies
- Optional on-chain verification test

**Run:**
```bash
# Basic Bonsai test
cargo run --bin bonsai-test --features mock-proof

# With on-chain verification
VERIFY_PROOFS_ONCHAIN=true cargo run --bin bonsai-test --features mock-proof,starknet-handler
```

**Environment:**
```bash
BONSAI_API_KEY=your_key_here
BONSAI_API_URL=https://api.bonsai.xyz/
VERIFY_PROOFS_ONCHAIN=true  # Optional
PITCHLAKE_VERIFIER_CONTRACT=0x...  # If verifying on-chain
```

## Usage Examples

### Example 1: Process Proof Job (Production)

```rust
use message_handler::{
    queue::sqs_message_queue::SqsMessageQueue,
    proof_composition::BonsaiProofProvider,
    services::proof_job_handler::ProofJobHandler,
};
use aws_config::{BehaviorVersion, defaults};
use std::sync::{Arc, atomic::AtomicBool};

#[tokio::main]
async fn main() -> eyre::Result<()> {
    // Load AWS config
    let config = defaults(BehaviorVersion::latest()).load().await;
    let queue = Arc::new(SqsMessageQueue::new(queue_url, config));

    // Create terminator for graceful shutdown
    let terminator = Arc::new(AtomicBool::new(false));

    // Create proof provider
    let proof_provider = Arc::new(BonsaiProofProvider::new());

    // Create proof job handler
    let handler = ProofJobHandler::new(
        queue,
        terminator.clone(),
        proof_provider,
        std::time::Duration::from_secs(3600),
    );

    // Start processing
    handler.receive_job().await?;

    Ok(())
}
```

### Example 2: Dispatch Proof Request

```rust
use message_handler::{
    queue::sqs_message_queue::SqsMessageQueue,
    services::{job_dispatcher::JobDispatcher, jobs::{Job, RequestProof}},
};

#[tokio::main]
async fn main() -> eyre::Result<()> {
    let queue = Arc::new(SqsMessageQueue::new(queue_url, config));
    let dispatcher = JobDispatcher::new(queue);

    // Create proof request
    let job = Job::RequestProof(RequestProof {
        job_id: "job-123".to_string(),
        job_group_id: Some("group-1".to_string()),
        start_timestamp: 1672531200,
        end_timestamp: 1704067200,
        twap_start_timestamp: Some(1672531200),
        twap_end_timestamp: Some(1704067200),
        reserve_price_start_timestamp: Some(1672531200),
        reserve_price_end_timestamp: Some(1704067200),
        max_return_start_timestamp: Some(1672531200),
        max_return_end_timestamp: Some(1704067200),
        vault_address: Some("0x123...".to_string()),
        vault_timestamp: Some(1704067200),
    });

    // Dispatch job
    dispatcher.dispatch_job(job).await?;

    Ok(())
}
```

### Example 3: Generate Mock Proof

```rust
use message_handler::{
    risc0_generator::{Risc0Generator, Risc0ProofGenerator, Risc0Config},
    proof_composition::ProofTimestampRanges,
};

#[tokio::main]
async fn main() -> eyre::Result<()> {
    let generator = Risc0Generator::new();
    let config = Risc0Config::default();

    let ranges = ProofTimestampRanges::new(
        1672531200, 1704067200,  // TWAP
        1672531200, 1704067200,  // Reserve price
        1672531200, 1704067200,  // Max return
    );

    let result = generator.generate_mock_proof(ranges, config).await?;

    println!("Proof generated in {}ms", result.generation_time_ms);
    println!("Calldata length: {}", result.calldata.len());

    Ok(())
}
```

### Example 4: Integrated Proof Verification

```rust
use message_handler::{
    risc0_generator::Risc0Generator,
    proof_verifier::{IntegratedProofVerifier, ProofVerifier, ProofVerifierConfig},
    proof_composition::ProofTimestampRanges,
    response_handler::PitchLakeJobRequest,
};
use starknet_crypto::Felt;

#[tokio::main]
async fn main() -> eyre::Result<()> {
    let generator = Risc0Generator::new();
    let verifier = IntegratedProofVerifier::new(generator);

    let ranges = ProofTimestampRanges::new(
        1672531200, 1704067200,
        1672531200, 1704067200,
        1672531200, 1704067200,
    );

    let job_request = PitchLakeJobRequest {
        vault_address: Felt::from_hex("0x123...")?,
        timestamp: 1704067200,
        program_id: Felt::from_hex("0x504954434c4c414b455f5631")?, // 'PITCH_LAKE_V1'
    };

    let mut config = ProofVerifierConfig::from_env()?;
    config.verify_onchain = true;
    config.job_request = Some(job_request);

    let result = verifier.generate_and_verify_proof(ranges, config).await?;

    println!("Total time: {}ms", result.total_time_ms);
    if let Some(tx_hash) = result.onchain_tx_hash {
        println!("On-chain verification tx: {:?}", tx_hash);
    }

    Ok(())
}
```

### Example 5: Hash Service (Fee Data Preparation)

```rust
use message_handler::{
    hashing::HashingProvider,
    services::hashing_service::HashingService,
};

#[tokio::main]
async fn main() -> eyre::Result<()> {
    // Initialize hashing provider from environment
    let hashing_provider = HashingProvider::from_env()?;

    // Create hashing service
    // - 1440 hours (2 months) of fee data required
    // - 180 hours per hash batch
    let hashing_service = HashingService::new(hashing_provider, 1440, 180);

    // Prepare hashes for timestamp (normalized to hour boundary)
    let start_timestamp = 1672531200_u64;
    hashing_service.run(start_timestamp).await.map_err(|e| eyre::eyre!(e))?;

    println!("Hash preparation completed successfully");

    Ok(())
}
```

## Flow Diagrams

### High-Level Proof Generation Flow

```
┌─────────────┐
│ SQS Queue   │
│ (Input)     │
└──────┬──────┘
       │ RequestProof Job
       ▼
┌────────────────────────────────────────┐
│ ProofJobHandler                        │
│ - Poll queue                           │
│ - Parse job                            │
│ - Check duplicate                      │
│ - Acquire semaphore                    │
└────────┬───────────────────────────────┘
         │
         ▼
┌────────────────────────────────────────┐
│ BonsaiProofProvider                    │
│ 1. Hash preparation (StarkNet)         │
│ 2. Fetch fee data                      │
│ 3. Generate 7 sub-proofs (parallel)    │
│ 4. Compose final proof                 │
└────────┬───────────────────────────────┘
         │ Receipt
         ▼
┌────────────────────────────────────────┐
│ ProofJobHandler (continued)            │
│ - Create ProofGenerated job            │
│ - Send to queue                        │
│ - Delete original RequestProof         │
└────────┬───────────────────────────────┘
         │
         ▼
┌─────────────┐
│ SQS Queue   │
│ (Output)    │
└─────────────┘
```

### Sub-Proof Composition Flow

```
┌──────────────────┐
│ Fee Data (Felt)  │
│ from StarkNet    │
└────────┬─────────┘
         │
         ▼
┌────────────────────────────────┐
│ Basic Sub-Proofs (Sequential)  │
│ 1. Hash Felts → f64 + hash     │
│ 2. Max Return (volatility)     │
│ 3. TWAP calculation            │
└────────┬───────────────────────┘
         │
         ├─────────────────────────────┐
         │                             │
         ▼                             ▼
┌──────────────────────┐   ┌──────────────────────┐
│ Reserve Price Calcs  │   │ Data Preparation     │
│ (Parallel)           │   │ - Extract subsets    │
│ 4. Remove Seasonality│   │ - Create ranges      │
│ 5. Calc Pt/Pt1       │   └──────────┬───────────┘
│ 6. Add TWAP 7d       │              │
│ 7. Simulate Price    │              │
└──────────┬───────────┘              │
           │                          │
           └──────────┬───────────────┘
                      │
                      ▼
         ┌────────────────────────────┐
         │ ProofCompositionInput      │
         │ - All timestamp ranges     │
         │ - All calculated values    │
         │ - Statistical parameters   │
         └────────┬───────────────────┘
                  │
                  ▼
         ┌────────────────────────────┐
         │ Compose Final Proof        │
         │ ExecutorEnv::builder()     │
         │   .add_assumption(sub1)    │
         │   .add_assumption(sub2)    │
         │   ...                      │
         │   .add_assumption(sub7)    │
         │   .write(input)            │
         │ default_prover().prove()   │
         └────────┬───────────────────┘
                  │
                  ▼
         ┌────────────────────────────┐
         │ RISC0 Receipt              │
         │ - Proof                    │
         │ - Journal output           │
         │ - StarkNet calldata        │
         └────────────────────────────┘
```

### Hash Preparation Flow

```
┌──────────────────┐
│ Start Timestamp  │
└────────┬─────────┘
         │
         ▼
┌────────────────────────────────┐
│ 1. Check Fee Data Availability │
│ - Fetch fees from Fossil Store │
│ - Validate count               │
│ - Check for zero fees          │
└────────┬───────────────────────┘
         │
         ▼
┌────────────────────────────────┐
│ 2. Find Missing Hash Batches   │
│ - For each 180-hour batch:     │
│   - Check if hash exists       │
│   - Collect missing batches    │
└────────┬───────────────────────┘
         │
         ▼
┌────────────────────────────────┐
│ 3. Store Missing Hashes        │
│ (Sequential - avoid nonce)     │
│ - For each missing:            │
│   - Submit tx                  │
│   - Wait for receipt (3 min)   │
│   - Check for revert           │
└────────┬───────────────────────┘
         │
         ▼
┌────────────────────────────────┐
│ 4. Create Batch Hash           │
│ - Hash of all hash batches     │
│ - Submit tx                    │
│ - Wait for receipt             │
└────────┬───────────────────────┘
         │
         ▼
┌────────────────────────────────┐
│ Hash Preparation Complete      │
│ Ready for proof generation     │
└────────────────────────────────┘
```

## Next Steps

### Related Documentation

- **[Proving Service Overview](docs/crates/proving-service/)** - Parent service architecture
- **[DB Crate](docs/crates/proving-service/db.md)** - Database models and queries
- **[StarkNet Handler Crate](proving-service/crates/starknet-handler/)** - StarkNet integration
- **[Fossil API](docs/crates/fossil-api/)** - API service that dispatches proof jobs

### Development Resources

- **[RISC0 Documentation](https://dev.risczero.com/)** - RISC0 zkVM and Bonsai API
- **[StarkNet Documentation](https://docs.starknet.io/)** - StarkNet smart contracts and integration
- **[AWS SQS Documentation](https://docs.aws.amazon.com/sqs/)** - AWS SQS queue management
- **[Pitchlake Coprocessor](https://github.com/NethermindEth/pitchlake-coprocessor)** - Computation methods and proof composition

### Testing

Run tests:
```bash
# Unit tests
cd proving-service/crates/message-handler
cargo test

# Integration tests with specific features
cargo test --features mock-proof

# Test with coverage
cargo llvm-cov --html --features mock-proof
```

### Debugging

Enable detailed logging:
```bash
RUST_LOG=message_handler=debug,risc0_zkvm=info cargo run --bin message-handler --features mock-proof
```

Inspect proof composition input:
```bash
SAVE_PROOF_COMPOSITION_INPUT=true cargo run --bin message-handler --features proof-composition
# Saves to: proof_composition_input.json
```

Test Bonsai API directly:
```bash
cargo run --bin bonsai-test --features mock-proof
```
