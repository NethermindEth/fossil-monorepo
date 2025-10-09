# StarkNet Handler Crate

## Overview

The `starknet-handler` crate provides a comprehensive interface for interacting with StarkNet blockchain, specifically designed for the Fossil proving service. It handles:

- **StarkNet Integration**: Reading fee data from the `fossil_store` contract deployed on StarkNet
- **Proof Submission**: Submitting Groth16 proofs to the verification contract onchain
- **Hash Management**: Creating and managing cryptographic hashes for data verification in RISC0 proof generation
- **Account Management**: Managing StarkNet accounts for write operations (transaction signing)
- **Contract Interactions**: Making RPC calls to StarkNet contracts with automatic retry logic

This crate serves as the bridge between the Fossil proving service and the StarkNet blockchain, enabling the service to fetch onchain data, prepare it for proof generation, and submit verified proofs back to the network.

## Crate Structure

### Files and Modules

```
proving-service/crates/starknet-handler/
├── Cargo.toml
├── src/
│   ├── lib.rs              # Core types and utilities
│   ├── config.rs           # Configuration loading from environment
│   ├── provider.rs         # StarkNet provider for contract interactions
│   ├── account.rs          # Account management and transaction signing
│   ├── example.rs          # Usage examples
│   ├── mock_data.rs        # Mock data generation for testing
│   ├── mock_example.rs     # Mock usage examples
│   └── main.rs             # Binary entry point (optional)
├── tests/
│   └── integration_tests.rs # Integration tests with Sepolia testnet
└── examples/
    └── test_local_connection.rs
```

### Module Breakdown

- **`lib.rs`**: Defines core data structures (`FeeData`, `FeeDataWithHash`) and utility functions
- **`config.rs`**: Environment-based configuration loading
- **`provider.rs`**: Main provider implementation with retry logic and contract calls
- **`account.rs`**: StarkNet account wrapper for signing and submitting transactions
- **`mock_data.rs`**: Mock data generators for testing without live blockchain connection
- **Integration tests**: Real-world tests against Sepolia testnet

## Core Types

### FeeData

Represents raw fee data returned from the `fossil_store` contract.

```rust
#[derive(Clone, Debug)]
pub struct FeeData {
    pub first_timestamp: u64,
    pub last_timestamp: u64,
    pub fees: Vec<Felt>,
}
```

**Usage:**
```rust
let fee_data = FeeData::new(
    1755457200,  // first_timestamp
    1755464400,  // last_timestamp
    vec![Felt::from(123u64), Felt::from(456u64)],  // fees
);
```

### FeeDataWithHash

Represents fee data combined with a cryptographic verification hash for RISC0 proof generation.

```rust
#[derive(Clone, Debug)]
pub struct FeeDataWithHash {
    pub raw_fees: Vec<Felt>,           // 5760 fee values (8 months hourly)
    pub verification_hash: [u32; 8],   // Cryptographic hash for verification
    pub avg_l1_gas_fee: u64,           // Average L1 gas fee (placeholder)
    pub avg_l2_gas_fee: u64,           // Average L2 gas fee (placeholder)
}
```

**Usage:**
```rust
let fee_data_with_hash = provider
    .get_fees_with_verification(start_timestamp, end_timestamp)
    .await?;

// Use in RISC0 proof generation
let raw_fees = fee_data_with_hash.raw_fees;
let hash = fee_data_with_hash.verification_hash;
```

### PitchLakeJobRequest

Represents a job request for PitchLake proof verification. This struct is encoded and passed to the verification contract.

```rust
#[derive(Debug, Clone, Encode)]
pub struct PitchLakeJobRequest {
    pub vault_address: Felt,
    pub timestamp: u64,
    pub program_id: Felt,  // 'PITCH_LAKE_V1'
}
```

**Usage:**
```rust
let job_request = PitchLakeJobRequest {
    vault_address: Felt::from_hex("0x123...")?,
    timestamp: 1672531200u64,
    program_id: Felt::from_hex("0x504954434c4c414b455f5631")?, // 'PITCH_LAKE_V1'
};

// Submit proof with job request
let tx_hash = account
    .verify_proof(verifier_address, proof, job_request)
    .await?;
```

## StarkNet Configuration

### StarkNetConfig

Configuration for StarkNet RPC connection and contract addresses.

```rust
#[derive(Clone, Debug)]
pub struct StarkNetConfig {
    pub rpc_url: String,
    pub fossil_store_address: String,
    pub hash_store_address: Option<String>,
    pub account_address: Option<String>,
    pub account_private_key: Option<String>,
    pub max_retries: Option<u32>,
    pub initial_backoff_ms: Option<u64>,
    pub max_backoff_ms: Option<u64>,
    pub use_mock_data: bool,
}
```

**Builder Pattern:**
```rust
let config = StarkNetConfig::new(
    "https://starknet-mainnet.public.blastapi.io".to_string(),
    "0x05f80abda60bd853551f43cc00539a3c884384ec01f7a9f5709296c74a3e490b".to_string()
)
.with_hash_store("0x123...".to_string())
.with_account("0xabc...".to_string(), "0xdef...".to_string())
.with_retry_config(5, 100, 10000)
.with_mock_data(false);
```

### Loading Configuration from Environment

The crate provides automatic configuration loading from environment variables:

```rust
use starknet_handler::config::load_starknet_config;

let config = load_starknet_config()?;
let provider = StarknetProvider::new(config)?;
```

**Environment Variables:**

| Variable | Required | Default | Description |
|----------|----------|---------|-------------|
| `STARKNET_RPC_URL` | No | `https://starknet-mainnet.public.blastapi.io` | StarkNet RPC endpoint |
| `FOSSIL_STORE_ADDRESS` | Yes | - | Address of fossil_store contract |
| `HASH_STORE_ADDRESS` | No | - | Address of hash_store contract |
| `STARKNET_ACCOUNT_ADDRESS` | No | - | Account address for write operations |
| `STARKNET_PRIVATE_KEY` | No | - | Private key for account |
| `STARKNET_MAX_RETRIES` | No | 5 | Maximum retry attempts |
| `STARKNET_INITIAL_BACKOFF_MS` | No | 100 | Initial backoff in milliseconds |
| `STARKNET_MAX_BACKOFF_MS` | No | 10000 | Maximum backoff in milliseconds |
| `USE_MOCK_STARKNET_DATA` | No | false | Use mock data for testing |

**Example .env:**
```bash
STARKNET_RPC_URL=https://starknet-sepolia.public.blastapi.io
FOSSIL_STORE_ADDRESS=0x05f80abda60bd853551f43cc00539a3c884384ec01f7a9f5709296c74a3e490b
HASH_STORE_ADDRESS=0x027a6e498bfad98a5145ac507f90fac6268cce6c0373c6eecd38de46be655b55
STARKNET_ACCOUNT_ADDRESS=0x1234567890abcdef
STARKNET_PRIVATE_KEY=0xfedcba0987654321
STARKNET_MAX_RETRIES=3
STARKNET_INITIAL_BACKOFF_MS=200
STARKNET_MAX_BACKOFF_MS=5000
```

## StarkNet Provider

The `StarknetProvider` is the main interface for reading data from StarkNet contracts.

### Creating a Provider

```rust
use starknet_handler::provider::{StarkNetConfig, StarknetProvider};

let config = StarkNetConfig::new(
    "http://localhost:5050".to_string(),
    "0x1234567890abcdef".to_string(),
);

let provider = StarknetProvider::new(config)?;
```

### Reading Fee Data

#### get_avg_fees_in_range

Fetches average fee data for a specified time range from the `fossil_store` contract.

```rust
let start_timestamp = 1755390640; // Must be multiple of 3600 (1 hour)
let end_timestamp = 1755398352;   // Must be multiple of 3600

let fee_data = provider
    .get_avg_fees_in_range(start_timestamp, end_timestamp)
    .await?;

println!("First timestamp: {}", fee_data.first_timestamp);
println!("Last timestamp: {}", fee_data.last_timestamp);
println!("Number of fees: {}", fee_data.fees.len());
```

**Contract Call:**
- Function: `get_avg_fees_in_range(start_timestamp: u64, end_timestamp: u64)`
- Returns: `(u64, u64, Array<felt252>)`
- Automatically retries on failure with exponential backoff

#### get_raw_fees_in_range

Fetches raw fee data suitable for RISC0 proof generation.

```rust
let raw_fees = provider
    .get_raw_fees_in_range(start_timestamp, end_timestamp)
    .await?;

// raw_fees is Vec<Felt> ready for RISC0 processing
assert_eq!(raw_fees.len(), 5760); // 8 months of hourly data
```

#### get_verification_hash

Fetches the cryptographic verification hash from the `hash_store` contract.

```rust
let verification_hash = provider
    .get_verification_hash(start_timestamp)
    .await?;

// verification_hash is [u32; 8]
println!("Hash: {:?}", verification_hash);
```

**Contract Call:**
- Function: `get_hash_stored_batched_avg_fees(start_timestamp: u64)`
- Returns: `[u32; 8]` - Array of 8 u32 values representing the hash

#### get_fees_with_verification

Combined function that fetches both raw fees and verification hash in parallel.

```rust
let fee_data_with_hash = provider
    .get_fees_with_verification(start_timestamp, end_timestamp)
    .await?;

// Use for RISC0 proof generation
let raw_fees = fee_data_with_hash.raw_fees;
let hash = fee_data_with_hash.verification_hash;
```

This is the **primary function** for RISC0 proof generation as it provides all necessary data in one call.

## Hash Operations

The crate provides functions for creating and managing cryptographic hashes used in RISC0 proof verification.

### Hash Creation Workflow

The hash creation follows a two-level batching approach:

1. **Batch Hashes**: Create 32 individual hashes, each covering 180 hours of fee data
2. **Batched Hash**: Combine the 32 batch hashes into a single final hash

### ensure_hashes_exist

Main function that ensures all required hashes exist before proof generation.

```rust
// Ensure all hashes exist for the 8-month period
provider.ensure_hashes_exist(start_timestamp, end_timestamp).await?;

// This function:
// 1. Checks if final batched hash exists
// 2. If not, identifies missing batch hashes (out of 32)
// 3. Creates missing batch hashes via create_batch_hash
// 4. Creates final batched hash via create_batched_hash
```

### create_batch_hash

Creates a hash for 180 average fees (via StarkNet account).

```rust
let account = StarknetAccount::new(provider, private_key, account_address)?;

let tx_hash = account
    .create_batch_hash(hash_store_address, start_timestamp)
    .await?;

println!("Batch hash created: {:#x}", tx_hash);
```

**Contract Call:**
- Function: `hash_avg_fees_and_store(start_timestamp: u64)`
- Creates onchain transaction to store hash
- Uses automatic retry with exponential backoff (max 3 retries)

### create_batched_hash

Creates final batched hash from 32 individual batch hashes.

```rust
let tx_hash = account
    .create_batched_hash(hash_store_address, start_timestamp)
    .await?;

println!("Final batched hash created: {:#x}", tx_hash);
```

**Contract Call:**
- Function: `hash_batched_avg_fees(start_timestamp: u64)`
- Combines 32 batch hashes into single verification hash
- Required before RISC0 proof generation can proceed

## StarkNet Account

The `StarknetAccount` manages account credentials and transaction signing for write operations.

### Creating an Account

```rust
use starknet_handler::account::StarknetAccount;
use std::sync::Arc;

let provider = Arc::new(JsonRpcClient::new(HttpTransport::new(url)));

let account = StarknetAccount::new(
    provider,
    "0x1234567890abcdef",  // private_key
    "0xfedcba0987654321",  // account_address
)?;
```

**Technical Details:**
- Uses `SingleOwnerAccount` with `LocalWallet` signer
- Configured for Katana devnet chain ID by default
- Uses `ExecutionEncoding::New` for transaction encoding

### Timestamp Normalization

All timestamp-based operations automatically normalize to hour boundaries:

```rust
// Timestamp normalization (internal)
fn normalize_timestamp(timestamp: u64) -> (u64, bool) {
    let normalized = (timestamp / 3600) * 3600;  // Round down to nearest hour
    let was_normalized = normalized != timestamp;
    (normalized, was_normalized)
}

// Example:
// Input: 1755391840 (arbitrary timestamp)
// Output: 1755390000 (rounded to nearest hour)
```

This ensures all operations work with hour-aligned timestamps as required by the contracts.

## Proof Verification

### verify_proof_onchain

Submits a Groth16 proof to the verification contract onchain.

```rust
use starknet_handler::account::PitchLakeJobRequest;

let verifier_address = "0x123...";
let proof = vec![Felt::from(1), Felt::from(2), /* ... */];

let job_request = PitchLakeJobRequest {
    vault_address: Felt::from_hex("0xabc...")?,
    timestamp: 1672531200u64,
    program_id: Felt::from_hex("0x504954434c4c414b455f5631")?, // 'PITCH_LAKE_V1'
};

let tx_hash = provider
    .verify_proof_onchain(verifier_address, proof, job_request)
    .await?;

println!("Proof verified, tx hash: {:#x}", tx_hash);
```

**Contract Call Details:**
- Function selector: `0x821b8b00fd9e4b2b57538b4571c0227e80f5dbdbfef0628722b3f06f3188`
- Parameters:
  - `proof: Span<felt252>` - Groth16 proof elements
  - `job_request: PitchLakeJobRequest` - Job metadata
- Returns: Transaction hash of verification transaction
- Uses automatic retry with exponential backoff (max 3 retries)

**Calldata Encoding:**

The function uses starknet-rs encoding to properly format the calldata:

```rust
let mut calldata = vec![];

// Encode proof Vec<Felt> -> [length, ...elements] for Span<felt252>
proof.encode(&mut calldata)?;

// Encode PitchLakeJobRequest struct -> [field1, field2, field3]
pitchlake_job_request.encode(&mut calldata)?;
```

## Retry Logic

All contract interactions use exponential backoff retry logic to handle transient network failures.

### Default Configuration

```rust
const DEFAULT_MAX_RETRIES: u32 = 5;
const DEFAULT_INITIAL_BACKOFF_MS: u64 = 100;
const DEFAULT_MAX_BACKOFF_MS: u64 = 10000; // 10 seconds
```

### Retry Implementation

```rust
async fn with_retry<F, Fut, T>(&self, operation_name: &str, f: F) -> Result<T>
where
    F: Fn() -> Fut,
    Fut: std::future::Future<Output = Result<T>>,
{
    let max_retries = self.config.max_retries.unwrap_or(DEFAULT_MAX_RETRIES);
    let mut backoff_ms = self.config.initial_backoff_ms
        .unwrap_or(DEFAULT_INITIAL_BACKOFF_MS);
    let max_backoff_ms = self.config.max_backoff_ms
        .unwrap_or(DEFAULT_MAX_BACKOFF_MS);

    let mut attempt = 0;

    loop {
        attempt += 1;

        match f().await {
            Ok(result) => {
                if attempt > 1 {
                    info!("Operation succeeded after retry: {}", operation_name);
                }
                return Ok(result);
            }
            Err(err) => {
                if attempt >= max_retries {
                    error!("Operation failed after maximum retries: {}", operation_name);
                    return Err(err);
                }

                // Exponential backoff, capped at max_backoff_ms
                backoff_ms = std::cmp::min(backoff_ms * 2, max_backoff_ms);

                warn!(
                    "Operation failed, retrying after {}ms: {}",
                    backoff_ms, operation_name
                );

                tokio::time::sleep(Duration::from_millis(backoff_ms)).await;
            }
        }
    }
}
```

**Backoff Progression Example:**
- Attempt 1 fails: Wait 100ms
- Attempt 2 fails: Wait 200ms
- Attempt 3 fails: Wait 400ms
- Attempt 4 fails: Wait 800ms
- Attempt 5 fails: Wait 1600ms
- Continues until max_backoff_ms (10000ms) is reached

### HTTP Timeout Configuration

For slow network connections (e.g., Katana fork), extended HTTP timeouts are configured:

```rust
const DEFAULT_HTTP_TIMEOUT_SECS: u64 = 300; // 5 minutes for slow fork fetches

let http_client = Client::builder()
    .timeout(Duration::from_secs(DEFAULT_HTTP_TIMEOUT_SECS))
    .build()?;
```

## Integration Tests

The crate includes comprehensive integration tests that verify functionality against the Sepolia testnet.

### Running Integration Tests

```bash
# Run all tests (including Sepolia integration)
cd proving-service/crates/starknet-handler
cargo test

# Run only unit tests (skip integration tests)
cargo test --lib

# Run specific integration test
cargo test test_sepolia_get_avg_fees_in_range

# Run ignored tests (require local node)
cargo test -- --ignored
```

### Sepolia Integration Test

The main integration test verifies end-to-end functionality with real contracts:

```rust
#[tokio::test]
async fn test_sepolia_get_avg_fees_in_range() -> Result<()> {
    const SEPOLIA_RPC_URL: &str = "https://starknet-sepolia.public.blastapi.io";
    const SEPOLIA_CONTRACT_ADDRESS: &str =
        "0x05f80abda60bd853551f43cc00539a3c884384ec01f7a9f5709296c74a3e490b";

    let config = StarkNetConfig::new(
        SEPOLIA_RPC_URL.to_string(),
        SEPOLIA_CONTRACT_ADDRESS.to_string(),
    ).with_retry_config(5, 200, 5000);

    let provider = StarknetProvider::new(config)?;

    // Test connectivity
    let client = provider.provider();
    let chain_id = client.chain_id().await?;
    assert_eq!(
        format!("{:#x}", chain_id),
        "0x534e5f5345504f4c4941",  // SN_SEPOLIA
        "Should be connected to Sepolia network"
    );

    // Test contract call
    let fee_data = provider
        .get_avg_fees_in_range(0x68a226b0, 0x68a242d0)
        .await?;

    // Verify expected data
    assert_eq!(fee_data.first_timestamp, 0x68a226b0);
    assert_eq!(fee_data.last_timestamp, 0x68a242d0);
    assert_eq!(fee_data.fees.len(), 3);

    Ok(())
}
```

**Test Coverage:**
1. Provider creation and configuration
2. RPC connectivity to Sepolia
3. Chain ID verification
4. Contract function calls
5. Response parsing and validation
6. Retry mechanism under failure conditions

## Usage Examples

### Example 1: Reading Fee Data

```rust
use starknet_handler::{
    config::load_starknet_config,
    provider::StarknetProvider,
};
use eyre::Result;

#[tokio::main]
async fn main() -> Result<()> {
    // Load configuration from environment
    let config = load_starknet_config()?;

    // Create provider
    let provider = StarknetProvider::new(config)?;

    // Calculate time range (last 24 hours)
    let end_timestamp = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)?
        .as_secs();
    let start_timestamp = end_timestamp - (24 * 60 * 60);

    // Fetch fee data
    let fee_data = provider
        .get_avg_fees_in_range(start_timestamp, end_timestamp)
        .await?;

    println!("Retrieved {} fees", fee_data.fees.len());
    println!("Time range: {} to {}",
        fee_data.first_timestamp,
        fee_data.last_timestamp
    );

    // Display first few fees
    for (i, fee) in fee_data.fees.iter().take(5).enumerate() {
        println!("Fee[{}]: {:#x}", i, fee);
    }

    Ok(())
}
```

### Example 2: RISC0 Proof Preparation

```rust
use starknet_handler::{
    config::load_starknet_config,
    provider::StarknetProvider,
};
use eyre::Result;

#[tokio::main]
async fn main() -> Result<()> {
    let config = load_starknet_config()?;
    let provider = StarknetProvider::new(config)?;

    // 8-month period starting timestamp (must be hour-aligned)
    let start_timestamp = 1672531200; // 2023-01-01 00:00:00 UTC
    let end_timestamp = start_timestamp + (8 * 30 * 24 * 3600); // ~8 months

    // Ensure all required hashes exist onchain
    provider.ensure_hashes_exist(start_timestamp, end_timestamp).await?;
    println!("All hashes verified/created");

    // Fetch data for RISC0 proof generation
    let fee_data = provider
        .get_fees_with_verification(start_timestamp, end_timestamp)
        .await?;

    println!("Raw fees count: {}", fee_data.raw_fees.len());
    println!("Verification hash: {:?}", fee_data.verification_hash);

    // Pass to RISC0 prover
    // let proof = generate_risc0_proof(fee_data.raw_fees, fee_data.verification_hash)?;

    Ok(())
}
```

### Example 3: Submitting Proof

```rust
use starknet_handler::{
    config::load_starknet_config,
    provider::StarknetProvider,
    account::PitchLakeJobRequest,
};
use starknet_crypto::Felt;
use eyre::Result;

#[tokio::main]
async fn main() -> Result<()> {
    let config = load_starknet_config()?;
    let provider = StarknetProvider::new(config)?;

    // Proof data (from RISC0 prover)
    let proof = vec![
        Felt::from(1),
        Felt::from(2),
        // ... additional proof elements
    ];

    // Job request metadata
    let job_request = PitchLakeJobRequest {
        vault_address: Felt::from_hex("0x123...")?,
        timestamp: 1672531200,
        program_id: Felt::from_hex("0x504954434c4c414b455f5631")?, // 'PITCH_LAKE_V1'
    };

    // Submit proof to verifier contract
    let verifier_address = "0x456...";
    let tx_hash = provider
        .verify_proof_onchain(verifier_address, proof, job_request)
        .await?;

    println!("Proof submitted successfully!");
    println!("Transaction hash: {:#x}", tx_hash);

    Ok(())
}
```

### Example 4: Using Mock Data for Testing

```rust
use starknet_handler::{
    provider::{StarkNetConfig, StarknetProvider},
    mock_data::MockFeeDataGenerator,
};
use eyre::Result;

#[tokio::main]
async fn main() -> Result<()> {
    // Create config with mock data enabled
    let config = StarkNetConfig::new(
        "http://localhost:5050".to_string(),
        "0x123...".to_string(),
    )
    .with_mock_data(true);  // Enable mock data

    let provider = StarknetProvider::new(config)?;

    // All operations now return mock data (no network calls)
    let fee_data = provider
        .get_avg_fees_in_range(1672531200, 1693699200)
        .await?;

    println!("Mock fees count: {}", fee_data.fees.len());
    assert_eq!(fee_data.fees.len(), 5760); // Always returns 5760 mock fees

    // Generate custom mock data
    let mock_generator = MockFeeDataGenerator::new(
        95494534198676387330u128,  // base_fee
        5000000000000000000u128,   // variance
    );

    let mock_fees = mock_generator.generate_mock_fee_data_as_felts()?;
    println!("Generated {} mock fees", mock_fees.len());

    Ok(())
}
```

### Example 5: Custom Retry Configuration

```rust
use starknet_handler::{
    provider::{StarkNetConfig, StarknetProvider},
};
use eyre::Result;

#[tokio::main]
async fn main() -> Result<()> {
    // Configure aggressive retry for unreliable networks
    let config = StarkNetConfig::new(
        "https://starknet-sepolia.public.blastapi.io".to_string(),
        "0x05f80abda60bd853551f43cc00539a3c884384ec01f7a9f5709296c74a3e490b".to_string(),
    )
    .with_retry_config(
        10,      // max_retries: Try up to 10 times
        500,     // initial_backoff_ms: Start with 500ms wait
        30000,   // max_backoff_ms: Cap at 30 seconds
    );

    let provider = StarknetProvider::new(config)?;

    // Operations will retry up to 10 times with exponential backoff
    let fee_data = provider
        .get_avg_fees_in_range(1755390640, 1755398352)
        .await?;

    println!("Retrieved {} fees with custom retry config", fee_data.fees.len());

    Ok(())
}
```

## Next Steps

### Related Documentation

- **[Message Handler Crate](./message-handler.md)**: Learn how the message handler uses starknet-handler to process SQS messages and orchestrate proof generation
- **[Database Crate](./db.md)**: Understand how StarkNet data is persisted in PostgreSQL
- **[Proving Service](./proving-service.md)**: See how the HTTP service exposes StarkNet functionality via REST API

### Integration Points

1. **Message Handler Integration**: The message-handler crate uses starknet-handler to:
   - Fetch fee data for proof generation requests
   - Ensure hashes exist before proof generation
   - Submit verified proofs back to StarkNet

2. **Database Integration**: StarkNet transaction hashes and verification statuses are stored in PostgreSQL via the db crate

3. **HTTP Service Integration**: The proving-service HTTP API exposes StarkNet operations as REST endpoints

### Development Workflow

```bash
# Navigate to crate
cd proving-service/crates/starknet-handler

# Run tests (including Sepolia integration)
cargo test

# Run ignored tests (require local Katana node)
cargo test -- --ignored

# Build the crate
cargo build --release

# Run example binary
cargo run --features binary --bin starknet-handler-example

# Format code
cargo +nightly fmt

# Run lints
cargo +nightly clippy -- -D warnings

# Generate documentation
cargo doc --open
```

### Common Patterns

#### Pattern 1: Fetch and Verify

```rust
// 1. Ensure hashes exist
provider.ensure_hashes_exist(start_ts, end_ts).await?;

// 2. Fetch data for proof
let data = provider.get_fees_with_verification(start_ts, end_ts).await?;

// 3. Generate proof
let proof = generate_proof(data.raw_fees, data.verification_hash)?;

// 4. Submit proof
let tx_hash = provider.verify_proof_onchain(verifier, proof, job_req).await?;
```

#### Pattern 2: Graceful Degradation

```rust
// Try real data first, fall back to mock on failure
let fee_data = match provider.get_avg_fees_in_range(start_ts, end_ts).await {
    Ok(data) => data,
    Err(e) => {
        warn!("Failed to fetch real data: {}, using mock", e);
        let mock_gen = MockFeeDataGenerator::default();
        let fees = mock_gen.generate_mock_fee_data_as_felts()?;
        FeeData::new(start_ts, end_ts, fees)
    }
};
```

#### Pattern 3: Batch Processing

```rust
// Process multiple time ranges in parallel
let ranges = vec![
    (start_ts1, end_ts1),
    (start_ts2, end_ts2),
    (start_ts3, end_ts3),
];

let results = futures::future::try_join_all(
    ranges.into_iter().map(|(start, end)| {
        provider.get_avg_fees_in_range(start, end)
    })
).await?;

for (i, data) in results.iter().enumerate() {
    println!("Range {}: {} fees", i, data.fees.len());
}
```

### Troubleshooting

**Issue: "Contract not found" error**
- Verify `FOSSIL_STORE_ADDRESS` is correct for the target network
- Ensure you're connected to the correct network (check chain ID)
- Confirm the contract is deployed at the specified address

**Issue: "Entry point not found" error**
- Verify the contract has the expected functions deployed
- Check function selector matches the contract implementation
- Ensure you're using the correct contract version

**Issue: Timeout errors**
- Increase `DEFAULT_HTTP_TIMEOUT_SECS` for slow networks
- Adjust retry configuration with more retries or longer backoff
- Check network connectivity to the RPC endpoint

**Issue: "Invalid hex string" error**
- Ensure all addresses are properly formatted with "0x" prefix
- Verify hex strings contain valid hexadecimal characters only
- Check that addresses are not truncated or malformed

**Issue: Timestamp normalization warnings**
- All timestamps must be multiples of 3600 (1 hour boundaries)
- The crate automatically normalizes timestamps (rounds down)
- Use pre-normalized timestamps to avoid warnings

### Additional Resources

- [StarkNet Documentation](https://docs.starknet.io/)
- [starknet-rs Library](https://github.com/xJonathanLEI/starknet-rs)
- [Cairo Smart Contracts](https://book.cairo-lang.org/)
- [Fossil Store Contract ABI](../../contracts/fossil-store.md) (if available)
