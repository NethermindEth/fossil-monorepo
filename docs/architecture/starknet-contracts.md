# StarkNet Contracts Architecture

This document provides a comprehensive overview of the StarkNet smart contract infrastructure used in the Fossil monorepo for proof verification and data integrity validation.

## Table of Contents
- [Overview](#overview)
- [Contract Organization](#contract-organization)
- [Fossil Store Contract (Upstream Infrastructure)](#fossil-store-contract-upstream-infrastructure)
- [Fossil Hash Store](#fossil-hash-store)
- [PitchLake Verifier](#pitchlake-verifier)
- [Mock Contracts](#mock-contracts)
- [Deployment](#deployment)
- [Integration with Rust Services](#integration-with-rust-services)
- [Testing](#testing)
- [Configuration](#configuration)
- [Next Steps](#next-steps)

## Overview

The StarkNet contracts provide the onchain infrastructure for the Fossil proof generation and verification system. They enable:

1. **Data Integrity Verification** - Hash-based validation of historical fee data
2. **Zero-Knowledge Proof Verification** - RISC Zero Groth16 proof verification on StarkNet
3. **Callback Integration** - Direct integration with PitchLake vault contracts for automated settlement

### Purpose in the System

The StarkNet contracts serve as the trust anchor for the entire Fossil ecosystem:

- **Fossil Hash Store** generates and stores cryptographic hashes of large fee datasets, enabling efficient data integrity validation without storing raw data onchain
- **PitchLake Verifier** validates RISC Zero proofs onchain and triggers callbacks to vault contracts with verified computation results
- **Mock Contracts** provide testing infrastructure for local development without deploying to live networks

### Technology Stack

- **Language**: Cairo 2.x (Starknet smart contract language)
- **Build Tool**: Scarb 2.9.2 (Cairo package manager)
- **Testing**: Starknet Foundry (snforge)
- **Network**: StarkNet (Sepolia testnet, Mainnet, and Katana devnet)
- **Dependencies**: OpenZeppelin contracts, Garaga (zk-SNARK verification), PitchLake vault

## Contract Organization

The contracts are organized in a Scarb workspace at `/starknet-contracts/`:

```
starknet-contracts/
├── Scarb.toml                    # Workspace configuration
├── Scarb.lock                    # Dependency lock file
├── fossil-hash-store/            # Hash storage and batch verification
│   ├── src/
│   │   ├── lib.cairo             # Main contract implementation
│   │   ├── interface.cairo       # Contract interfaces
│   │   ├── helper.cairo          # Hash computation utilities
│   │   └── mock/                 # Mock implementations
│   ├── tests/
│   │   ├── test_contract.cairo   # Contract integration tests
│   │   └── test_helper.cairo     # Helper function tests
│   ├── Scarb.toml
│   ├── ARCHITECTURE.md           # Detailed contract documentation
│   └── README.md
├── pitchlake-verifier/           # RISC Zero proof verification
│   ├── src/
│   │   ├── lib.cairo             # Common types and utilities
│   │   ├── pitchlake_verifier.cairo    # Main verifier contract
│   │   ├── groth16_verifier.cairo      # Groth16 verification logic
│   │   ├── groth16_verifier_constants.cairo  # Verification parameters
│   │   ├── universal_ecip.cairo        # Elliptic curve operations
│   │   └── fixtures.cairo              # Test fixtures
│   ├── tests/
│   │   └── integration.cairo     # Integration tests
│   └── Scarb.toml
└── mocks/                        # Local development mocks
    ├── Scarb.toml
    └── (mock contract implementations)
```

### Workspace Configuration

The workspace uses a centralized dependency management system defined in the root `Scarb.toml`:

```toml
[workspace]
members = ["fossil-hash-store", "pitchlake-verifier"]

[workspace.dependencies]
starknet = "2.12.1"
assert_macros = "2.12.1"
openzeppelin_upgrades = "2.0.0"
openzeppelin_access = "2.0.0"
snforge_std = "0.49.0"
pitch_lake = {git = "https://github.com/ametel01/pitchlake_starknet.git", branch = "scarb-updates"}
```

## Fossil Store Contract (Upstream Infrastructure)

### Purpose

**Important:** The **Fossil Store Contract** is part of the broader Fossil infrastructure and is **not managed in this repository**. It is maintained by the upstream Fossil system (MMR Builder and Light Client components).

The Fossil Store Contract serves as the **primary data source** for the Pitchlake Coprocessor, providing validated hourly average base fee data from Ethereum L1.

### Role in the Fossil Ecosystem

```
Ethereum L1 → Indexer → MMR Builder → Fossil Store (Starknet) → Pitchlake Coprocessor (this repo)
```

The Fossil Store Contract:
- Stores hourly average base fee data computed by the MMR Builder
- Maintains MMR root hashes for data integrity verification
- Provides IPFS references to complete MMR snapshots
- Stores proof journals from zkVM executions in the upstream Fossil system

### Integration with Pitchlake Coprocessor

The Pitchlake Coprocessor queries the Fossil Store Contract for validated base fee data:

**Location:** `proving-service/crates/starknet-handler/src/provider.rs:1238-1268`

**Key Function:**
```rust
pub async fn get_avg_fees_in_range(
    &self,
    start_timestamp: u64,
    end_timestamp: u64,
) -> Result<FeeData> {
    // Calls Fossil Store contract's get_avg_fees_in_range method
    // Returns: FeeData with hourly average base fees
}
```

**Data Flow:**
1. Message Handler requests fee data for specific timestamp range
2. StarkNet Provider queries Fossil Store contract on Starknet
3. Fossil Store returns validated hourly average base fees
4. Data is used for TWAP, max return, and reserve price calculations in RISC0

### Configuration

The Fossil Store contract address is configured via environment variable:

```bash
FOSSIL_STORE_ADDRESS=0x00e581139553c8666f60b6646f277a336f99f108f8e5fa7cb300b6a6ce7c3b8c
```

This address points to the upstream Fossil Store contract deployed and maintained by the Fossil infrastructure team.

## Fossil Hash Store

### Purpose

The Fossil Hash Store contract (`Sha2Input`) implements a hierarchical hashing system for efficient validation of large historical fee datasets. It solves the problem of onchain data verification by pre-computing and storing cryptographic hashes instead of raw data.

### Contract Interface

```cairo
#[starknet::interface]
pub trait ISha2Input<TContractState> {
    // Administrative functions
    fn set_fossil_store(ref self: TContractState, fossil_store: starknet::ContractAddress);
    fn get_fossil_store(self: @TContractState) -> starknet::ContractAddress;

    // Level 1: Batch hash generation (180 hourly fees)
    fn hash_avg_fees_and_store(ref self: TContractState, start_timestamp: u64);
    fn get_hash_stored_avg_fees(self: @TContractState, timestamp: u64) -> [u32; 8];

    // Level 2: Composite hash generation (32 batch hashes)
    fn hash_batched_avg_fees(ref self: TContractState, start_timestamp: u64);
    fn get_hash_stored_batched_avg_fees(self: @TContractState, timestamp: u64) -> [u32; 8];
}
```

### Hierarchical Hashing Architecture

The contract implements a two-tier hashing system:

#### Tier 1: Batch Hashes (`hash_stored_avg_fees`)

Each Level 1 hash represents **180 consecutive hourly fee values** (7.5 days of data):

```cairo
fn hash_avg_fees_and_store(ref self: TContractState, start_timestamp: u64) {
    // 1. Fetch 180 hourly fees from Fossil Store (3600-second intervals)
    let avg_fees = self.fossil_store.read()
        .get_avg_fees_in_range(start_timestamp, start_timestamp + (180 * 3600));

    // 2. Convert felt252 values to u32 arrays for SHA256 input
    let mut data_to_hash: Array<u32> = array![];
    for fee in avg_fees {
        let fee_as_u32_array = convert_avg_fees_to_u32_array(fee);
        data_to_hash.append_span(fee_as_u32_array.span());
    }

    // 3. Compute SHA256 hash
    let hash: [u32; 8] = compute_sha256_u32_array(data_to_hash);

    // 4. Store in mapping
    self.hash_stored_avg_fees.write(start_timestamp, hash);

    // 5. Emit event
    self.emit(HashStoredAvgFees { timestamp: start_timestamp });
}
```

**Storage**: `Map<u64, [u32; 8]>` - Maps start timestamp to SHA256 hash (256 bits as 8x u32)

#### Tier 2: Composite Hashes (`hash_batched_avg_fees`)

Level 2 combines **32 Level 1 hashes** into a single composite hash:

```cairo
fn hash_batched_avg_fees(ref self: TContractState, start_timestamp: u64) {
    let num_in_a_batch = 32_u64;  // Production: 32 batches = 5760 hours
    let batch_interval = 180_u64 * 3600_u64;  // 180 hours per batch

    let mut batch_data: Array<u32> = array![];

    // Collect all 32 batch hashes
    for i in 0..num_in_a_batch {
        let timestamp = start_timestamp + (i * batch_interval);
        let hash = self.hash_stored_avg_fees.read(timestamp);

        // Validate hash exists
        assert(hash != [0; 8], 'Hash is empty');

        // Append to composite data
        for value in hash {
            batch_data.append(value);
        }
    }

    // Compute composite hash
    let composite_hash = compute_sha256_u32_array(batch_data);

    // Store composite hash
    self.hash_batched_avg_fees.write(start_timestamp, composite_hash);

    self.emit(HashStoredBatchedAvgFees { timestamp: start_timestamp });
}
```

**Coverage**: 180 hours × 32 batches = **5,760 hours (8 months) of historical data**

### Data Flow

```
Fossil Store (Raw Hourly Fee Data)
         ↓
Level 1: hash_avg_fees_and_store(timestamp)
         ↓ (Fetches 180 fees, computes SHA256)
Storage: hash_stored_avg_fees[timestamp] = [u32; 8]
         ↓
Level 2: hash_batched_avg_fees(timestamp)
         ↓ (Collects 32 hashes, computes composite SHA256)
Storage: hash_batched_avg_fees[timestamp] = [u32; 8]
         ↓
RISC Zero Proof Generation
         ↓ (Uses composite hash for data integrity validation)
Proof Verification on StarkNet
```

### Storage Layout

```cairo
#[storage]
struct Storage {
    fossil_store: IFossilMinimalAvgFeeStoreDispatcher,
    hash_stored_avg_fees: Map<u64, [u32; 8]>,        // Level 1 hashes
    hash_batched_avg_fees: Map<u64, [u32; 8]>,       // Level 2 composite hashes
    ownable: OwnableComponent::Storage,
    upgradeable: UpgradeableComponent::Storage,
}
```

### Events

```cairo
#[derive(Drop, starknet::Event)]
struct HashStoredAvgFees {
    timestamp: u64,
}

#[derive(Drop, starknet::Event)]
struct HashStoredBatchedAvgFees {
    timestamp: u64,
}
```

### Helper Functions

Located in `src/helper.cairo`:

```cairo
// Converts felt252 fee values to u32 arrays for SHA256 input
fn convert_avg_fees_to_u32_array(avg_fees: felt252) -> Array<u32> {
    // Handles big-endian to little-endian conversion
    // Splits 252-bit values into 8 × 32-bit words
}

// Utility function for hashing arrays of average fees
fn hash_of_avg_fees(avg_fees: Array<felt252>) -> [u32; 8] {
    // Used for testing and validation
}
```

### Security Features

1. **Access Control**: Owner-only functions protected by OpenZeppelin Ownable component
2. **Data Validation**:
   - Non-zero fee checks prevent hashing invalid/missing data
   - Hash emptiness checks ensure Level 1 hashes exist before Level 2 processing
3. **Upgradeability**: Contract can be upgraded by owner using OpenZeppelin Upgradeable component

## PitchLake Verifier

### Purpose

The PitchLake Verifier contract validates RISC Zero Groth16 proofs onchain and triggers callbacks to PitchLake vault contracts with verified computation results.

### Contract Interface

```cairo
#[starknet::interface]
pub trait IPitchLakeVerifier<TContractState> {
    fn verify_proof(
        ref self: TContractState,
        proof: Span<felt252>,
        pitchlake_job_request: PitchLakeJobRequest,
    );
    fn update_verifier_address(
        ref self: TContractState,
        new_verifier_address: starknet::ContractAddress,
    );
    fn get_groth16_verifier_address(self: @TContractState) -> starknet::ContractAddress;
    fn get_pitchlake_client_address(self: @TContractState) -> starknet::ContractAddress;
    fn upgrade(ref self: TContractState, new_class_hash: starknet::ClassHash);
}
```

### Core Data Structures

#### PitchLake Job Request

```cairo
#[derive(Copy, Debug, Drop, Serde)]
pub struct PitchLakeJobRequest {
    pub vault_address: starknet::ContractAddress,  // Target vault for callback
    pub timestamp: u64,                             // Round timestamp
    pub program_id: felt252,                        // 'PITCH_LAKE_V1'
}
```

#### Journal Structure

The journal contains all verified computation results:

```cairo
#[derive(Drop, Debug, Copy, PartialEq, Serde)]
pub struct Journal {
    data_8_months_hash: [u32; 8],              // 32 bytes - Data integrity hash
    start_timestamp: u64,                       // 8 bytes - Data range start
    end_timestamp: u64,                         // 8 bytes - Data range end
    reserve_price_start_timestamp: u64,         // 8 bytes - Reserve price calc start
    reserve_price_end_timestamp: u64,           // 8 bytes - Reserve price calc end
    reserve_price: felt252,                     // 32 bytes - Calculated reserve price
    twap_start_timestamp: u64,                  // 8 bytes - TWAP calc start
    twap_end_timestamp: u64,                    // 8 bytes - TWAP calc end
    twap_result: felt252,                       // 32 bytes - Time-weighted average price
    max_return_start_timestamp: u64,            // 8 bytes - Max return calc start
    max_return_end_timestamp: u64,              // 8 bytes - Max return calc end
    max_return: felt252,                        // 32 bytes - Maximum return value
    floating_point_tolerance: felt252,          // 32 bytes - Computation tolerance
    reserve_price_tolerance: felt252,           // 32 bytes - Reserve price tolerance
    twap_tolerance: felt252,                    // 32 bytes - TWAP tolerance
    gradient_tolerance: felt252,                // 32 bytes - Gradient tolerance
}
```

### Proof Verification Flow

```cairo
fn verify_proof(
    ref self: ContractState,
    mut proof: Span<felt252>,
    pitchlake_job_request: PitchLakeJobRequest,
) {
    // 1. Remove first element (proof format compatibility)
    let _ = proof.pop_front();

    // 2. Verify Groth16 proof using BN254 verifier
    let journal = self.bn254_verifier.read()
        .verify_r0_groth16_proof_bn254(proof)
        .expect('Failed to verify proof');

    // 3. Decode journal from bytes
    let journal = decode_journal(journal);

    // 4. Serialize job request data
    let mut job_request_data: Array<felt252> = array![];
    pitchlake_job_request.vault_address.serialize(ref job_request_data);
    pitchlake_job_request.timestamp.serialize(ref job_request_data);
    pitchlake_job_request.program_id.serialize(ref job_request_data);

    // 5. Serialize computation results
    let mut job_result_data: Array<felt252> = array![];
    journal.reserve_price_start_timestamp.serialize(ref job_result_data);
    journal.reserve_price_end_timestamp.serialize(ref job_result_data);
    journal.reserve_price.serialize(ref job_result_data);
    journal.twap_start_timestamp.serialize(ref job_result_data);
    journal.twap_end_timestamp.serialize(ref job_result_data);
    journal.twap_result.serialize(ref job_result_data);
    journal.max_return_start_timestamp.serialize(ref job_result_data);
    journal.max_return_end_timestamp.serialize(ref job_result_data);
    journal.max_return.serialize(ref job_result_data);

    // 6. Call vault contract with verified results
    let pitchlake_vault = IVaultDispatcher {
        contract_address: pitchlake_job_request.vault_address,
    };
    pitchlake_vault.fossil_callback(job_request_data.span(), job_result_data.span());

    // 7. Emit verification event
    self.emit(PitchlakeProofVerified { /* all journal fields */ });
}
```

### Groth16 Verification

The contract uses the Garaga library for efficient BN254 pairing-based verification:

```cairo
#[starknet::interface]
pub trait IRisc0Groth16VerifierBN254<TContractState> {
    fn verify_r0_groth16_proof_bn254(
        self: @TContractState,
        full_proof_with_hints: Span<felt252>,
    ) -> Option<Span<u8>>;
}
```

**Key Operations**:
1. Deserializes proof and hints
2. Validates elliptic curve points (a, b, c on BN254 curve)
3. Computes receipt claim digest (SHA256 of journal + image_id)
4. Performs multi-scalar multiplication (MSM) for public inputs
5. Executes pairing check (verifies proof validity)
6. Returns journal bytes if verification succeeds

### Journal Decoding

The `decode_journal` function converts raw bytes to structured data:

```cairo
pub fn decode_journal(journal_bytes: Span<u8>) -> Journal {
    // Parse data_8_months_hash (32 bytes as 8 u32 values)
    let val0: u32 = parse_u32_le(journal_bytes, 0);
    let val1: u32 = parse_u32_le(journal_bytes, 4);
    // ... (8 total u32 values)

    // Parse timestamps (u64 values)
    let (start_timestamp, offset) = safe_parse_u64(journal_bytes, 32);
    let (end_timestamp, offset) = safe_parse_u64(journal_bytes, offset);

    // Parse fixed-point values (as hex strings)
    let (reserve_price, offset) = safe_parse_packed_fixed_point(journal_bytes, offset);
    let (twap_result, offset) = safe_parse_packed_fixed_point(journal_bytes, offset);
    // ... (more financial values)

    Journal { /* all fields */ }
}
```

### Events

```cairo
#[derive(Drop, starknet::Event)]
struct PitchlakeProofVerified {
    data_8_months_hash: [u32; 8],
    start_timestamp: u64,
    end_timestamp: u64,
    reserve_price_start_timestamp: u64,
    reserve_price_end_timestamp: u64,
    reserve_price: felt252,
    twap_start_timestamp: u64,
    twap_end_timestamp: u64,
    twap_result: felt252,
    max_return_start_timestamp: u64,
    max_return_end_timestamp: u64,
    max_return: felt252,
    floating_point_tolerance: felt252,
    reserve_price_tolerance: felt252,
    twap_tolerance: felt252,
    gradient_tolerance: felt252,
}
```

### Dependencies

- **Garaga**: BN254 pairing-based cryptography for Groth16 verification
- **OpenZeppelin**: Access control (Ownable) and upgradeability
- **PitchLake Vault**: Callback interface for verified results

## Mock Contracts

### Purpose

Mock contracts provide lightweight testing infrastructure for local development without requiring full contract deployments.

### Location

`/starknet-contracts/mocks/`

### Configuration

```toml
[package]
name = "mocks"
version = "0.1.0"
edition = "2024_07"

[dependencies]
starknet = "2.12.1"

[dev-dependencies]
snforge_std = "0.49.0"
assert_macros = "2.12.1"
```

### Use Cases

1. **Local Development**: Test contract interactions without deploying to Katana
2. **Unit Testing**: Isolated testing of individual contract functions
3. **CI/CD**: Fast contract testing in continuous integration pipelines
4. **Integration Testing**: Mock external dependencies (Fossil Store, etc.)

## Deployment

### Deployment Architecture

Contracts are deployed using a Docker-based deployment pipeline:

```yaml
# docker-compose.deploy.yml
services:
  contract-deployer:
    build:
      context: .
      dockerfile: docker/Dockerfile.deploy
    networks:
      - fossil-monorepo_local-network
    volumes:
      - ./starknet-contracts:/app/starknet-contracts
      - ./scripts:/app/scripts
      - ./.env.docker:/app/.env.docker
    environment:
      - STARKNET_RPC_URL=http://katana:5050
    command: ["sh", "-c", "chmod +x scripts/deploy-starknet.sh && ./scripts/deploy-starknet.sh docker"]
```

### Deployment Script

The deployment is orchestrated by `/scripts/deploy-starknet.sh`:

```bash
#!/bin/bash
# Supports multiple environments: local, sepolia, mainnet, docker

# Usage:
./scripts/deploy-starknet.sh docker        # Deploy to local Katana via Docker
./scripts/deploy-starknet.sh sepolia       # Deploy to Sepolia testnet
./scripts/deploy-starknet.sh mainnet       # Deploy to StarkNet mainnet
./scripts/deploy-starknet.sh local         # Deploy to local Katana directly

# Options:
./scripts/deploy-starknet.sh --no-build docker  # Skip build step
```

### Deployment Process

1. **Environment Setup**
   - Loads environment variables from `.env.{environment}`
   - Validates required variables (RPC URL, account address, private key)
   - Sets up Scarb build environment

2. **Contract Compilation**
   ```bash
   cd starknet-contracts
   scarb build
   ```

3. **Contract Declaration**
   - Declares contract class hashes on StarkNet
   - Stores class hashes for deployment

4. **Contract Deployment**
   - Deploys Universal ECIP (elliptic curve operations)
   - Deploys Groth16 Verifier (with ECIP dependency)
   - Deploys PitchLake Verifier (with Groth16 dependency)
   - Deploys Fossil Hash Store

5. **Environment Variable Updates**
   - Updates `.env.{environment}` with deployed contract addresses
   - For docker environment, syncs to `.env.local`

### Environment Variables

Required variables in `.env.{environment}`:

```bash
# StarkNet Network Configuration
STARKNET_RPC_URL=http://katana:5050
STARKNET_ACCOUNT=katana-0
STARKNET_ACCOUNT_ADDRESS=0x127fd5f1fe78a71f8bcd1fec63e3fe2f0486b6ecd5c86a0466c3a21fa5cfcec
STARKNET_PRIVATE_KEY=0xc5b2fcab997346f3ea1c00b002ecf6f382c5f9c9659a3894eb783c5320f912

# Contract Addresses (populated by deploy script)
UNIVERSAL_ECIP_CONTRACT=
GROTH16_VERIFIER_CONTRACT=
PITCHLAKE_VERIFIER_CONTRACT=
HASH_STORAGE_ADDRESS=
FOSSIL_STORE_ADDRESS=0x00e581139553c8666f60b6646f277a336f99f108f8e5fa7cb300b6a6ce7c3b8c
```

### Deployment to Different Networks

#### Local Katana (Docker)
```bash
# 1. Start Katana devnet
docker-compose -f docker-compose.local.yml up -d katana

# 2. Deploy contracts
docker-compose -f docker-compose.deploy.yml up

# Contracts deployed to http://katana:5050
```

#### Sepolia Testnet
```bash
# 1. Configure .env.sepolia with Sepolia RPC and account
# 2. Fund account with Sepolia ETH
# 3. Deploy
./scripts/deploy-starknet.sh sepolia
```

#### Mainnet
```bash
# 1. Configure .env.mainnet with Mainnet RPC and account
# 2. Ensure sufficient ETH for deployment gas
# 3. Deploy (use with caution)
./scripts/deploy-starknet.sh mainnet
```

## Integration with Rust Services

### HashingProvider Integration

The `HashingProvider` struct in the proving service provides Rust integration with the Fossil Hash Store contract:

**Location**: `/proving-service/crates/message-handler/src/hashing/mod.rs`

```rust
pub struct HashingProvider {
    provider: JsonRpcClient<HttpTransport>,
    fossil_store_address: Felt,
    hash_storage_address: Felt,
    account: SingleOwnerAccount<JsonRpcClient<HttpTransport>, LocalWallet>,
}

#[async_trait]
pub trait HashingProviderTrait {
    // Read operations
    async fn get_avg_fees_in_range(
        &self,
        start_timestamp: u64,
        end_timestamp: u64,
    ) -> Result<Vec<f64>, ProviderError>;

    async fn get_hash_stored_avg_fees(
        &self,
        timestamp: u64
    ) -> Result<[u32; 8], ProviderError>;

    async fn get_hash_batched_avg_fees(
        &self,
        start_timestamp: u64,
    ) -> Result<[u32; 8], ProviderError>;

    // Write operations (transaction invocations)
    async fn hash_avg_fees_and_store(
        &self,
        start_timestamp: u64,
    ) -> Result<InvokeTransactionResult>;

    async fn hash_batched_avg_fees(
        &self,
        start_timestamp: u64
    ) -> Result<InvokeTransactionResult>;
}
```

### HashingService Workflow

The `HashingService` orchestrates hash preparation before proof generation:

```rust
pub struct HashingService {
    provider: HashingProvider,
}

impl HashingService {
    // Main execution flow
    pub async fn run(&self, start_timestamp: u64, end_timestamp: u64) -> Result<()> {
        // 1. Check data availability
        self.check_avg_fees_availability(start_timestamp, end_timestamp).await?;

        // 2. Find missing Level 1 hashes
        let missing_hashes = self.get_unavailable_batch_timestamp_hashes(
            start_timestamp,
            end_timestamp
        ).await?;

        // 3. Generate missing Level 1 hashes onchain
        for timestamp in missing_hashes {
            self.hash_and_store_avg_fees_onchain(timestamp).await?;
        }

        // 4. Generate Level 2 composite hash
        self.hash_batch_avg_fees_onchain(start_timestamp).await?;

        Ok(())
    }
}
```

### Proof Verification Integration

The `ProofVerifier` integrates with the PitchLake Verifier contract:

**Location**: `/proving-service/crates/message-handler/src/proof_verifier.rs`

```rust
pub struct ProofVerifierConfig {
    pub risc0_config: Risc0Config,
    pub verifier_contract_address: String,
    pub verify_onchain: bool,
    pub job_request: Option<PitchLakeJobRequest>,
}

#[async_trait]
pub trait ProofVerifier {
    async fn generate_and_verify_proof(
        &self,
        timestamp_ranges: ProofTimestampRanges,
        config: ProofVerifierConfig,
    ) -> Result<ProofVerificationResult>;
}

impl IntegratedProofVerifier {
    async fn verify_proof_onchain(
        &self,
        calldata: Vec<Felt>,
        verifier_address: &str,
        job_request: PitchLakeJobRequest,
    ) -> Result<Felt> {
        // 1. Load StarkNet configuration
        let starknet_config = load_starknet_config()?;
        let provider = StarknetProvider::new(starknet_config).await?;

        // 2. Prepare contract call
        let call = Call {
            to: verifier_address.parse()?,
            selector: selector!("verify_proof"),
            calldata,
        };

        // 3. Execute transaction
        let result = provider.account.execute(vec![call]).send().await?;

        Ok(result.transaction_hash)
    }
}
```

### Contract Call Examples

#### Reading Hash Data

```rust
// Get Level 1 hash for a specific timestamp
let timestamp = 1714636800_u64;
let hash = hashing_provider
    .get_hash_stored_avg_fees(timestamp)
    .await?;

println!("Level 1 hash: {:x?}", hash);
```

#### Generating Hashes

```rust
// Generate Level 1 hash (invokes contract transaction)
let start_timestamp = 1714636800_u64;
let tx_result = hashing_provider
    .hash_avg_fees_and_store(start_timestamp)
    .await?;

println!("Transaction hash: {:x}", tx_result.transaction_hash);

// Wait for transaction confirmation
// ... (transaction receipt polling)

// Retrieve generated hash
let hash = hashing_provider
    .get_hash_stored_avg_fees(start_timestamp)
    .await?;
```

#### Verifying Proofs

```rust
// Prepare proof calldata
let proof_calldata: Vec<Felt> = prepare_proof_calldata(risc0_result)?;

// Create job request
let job_request = PitchLakeJobRequest {
    vault_address: vault_addr,
    timestamp: round_timestamp,
    program_id: Felt::from_bytes_be_slice(b"PITCH_LAKE_V1"),
};

// Verify proof onchain
let tx_hash = verifier
    .verify_proof_onchain(proof_calldata, verifier_address, job_request)
    .await?;

println!("Proof verified! Transaction: {:x}", tx_hash);
```

## Testing

### Testing Framework

The contracts use **Starknet Foundry (snforge)** for comprehensive testing:

```toml
[dev-dependencies]
snforge_std = "0.49.0"
assert_macros = "2.12.1"
cairo_test = "2.12.1"
```

### Fossil Hash Store Tests

**Location**: `/starknet-contracts/fossil-hash-store/tests/test_contract.cairo`

#### Test: Level 1 Hash Generation

```cairo
#[test]
fn test_hash_avg_fees_and_store() {
    let start_timestamp = 1714636800_u64;

    // Mock Fossil Store response
    start_mock_call(
        fossil_store(),
        selector!("get_avg_fee"),
        565966358523639806057303238395575262125865566208,
    );

    // Deploy contract
    let contract_address = deploy_contract_sha2_input(owner(), fossil_store());
    let dispatcher = ISha2InputDispatcher { contract_address };

    // Generate hash
    dispatcher.hash_avg_fees_and_store(start_timestamp);

    // Verify hash
    let hash_res = dispatcher.get_hash_stored_avg_fees(start_timestamp);
    assert(
        hash_res == [
            0x71316d72, 0x99f3b0a0, 0xf67978f3, 0x2f96f5de,
            0x0a358b38, 0x65b4a286, 0x568c4b8a, 0x833531ef,
        ],
        'invalid hash result',
    );
}
```

#### Test: Error Handling

```cairo
#[test]
#[should_panic(expected: 'Avg fees is 0')]
fn test_hash_avg_fees_and_store_fail_with_avg_fees_is_0() {
    let start_timestamp = 1714636800_u64;

    // Mock zero fee (should fail)
    start_mock_call(fossil_store(), selector!("get_avg_fee"), 0);

    let contract_address = deploy_contract_sha2_input(owner(), fossil_store());
    let dispatcher = ISha2InputDispatcher { contract_address };

    // This should panic
    dispatcher.hash_avg_fees_and_store(start_timestamp);
}
```

#### Test: Level 2 Composite Hash

```cairo
#[test]
fn test_hash_batched_avg_fees() {
    let start_timestamp = 1714636800_u64;
    let contract_address = deploy_contract_sha2_input(owner(), fossil_store());
    let dispatcher = ISha2InputDispatcher { contract_address };

    let num_in_a_batch = 32_u64;  // 5760/180

    // Manually inject Level 1 hashes into storage (for testing)
    for i in 0..num_in_a_batch {
        let timestamp = start_timestamp + (i * 3600_u64 * 180_u64);
        store(
            contract_address,
            map_entry_address(
                selector!("hash_stored_avg_fees"),
                array![timestamp.into()].span()
            ),
            array![
                1899064690, 2582884512, 4135155955, 798422494,
                171281208, 1706336902, 1452034954, 2201850415,
            ].span(),
        );
    }

    // Generate composite hash
    dispatcher.hash_batched_avg_fees(start_timestamp);

    // Verify composite hash
    let batch_hash = dispatcher.get_hash_stored_batched_avg_fees(start_timestamp);
    assert(
        batch_hash == [
            3776352627, 613591289, 3835625742, 4221307373,
            1854134067, 2029419697, 1476454056, 1330734629,
        ],
        'invalid batch hash result',
    );
}
```

### PitchLake Verifier Tests

**Location**: `/starknet-contracts/pitchlake-verifier/tests/integration.cairo`

Tests include:
- Journal decoding validation
- Proof verification with test fixtures
- Callback execution to vault contracts
- Access control enforcement

### Running Tests

```bash
# Test all contracts
cd starknet-contracts
scarb test

# Test specific contract
cd starknet-contracts/fossil-hash-store
scarb test

# Test with coverage
cd starknet-contracts/pitchlake-verifier
scarb test --coverage
```

### Test Utilities

#### Mock Contract Deployment

```cairo
fn deploy_contract_sha2_input(
    owner: starknet::ContractAddress,
    fossil_store: starknet::ContractAddress,
) -> ContractAddress {
    let contract = declare("Sha2Input").unwrap().contract_class();
    let (contract_address, _) = contract
        .deploy(@array![owner.into(), fossil_store.into()])
        .unwrap();
    contract_address
}
```

#### Storage Manipulation

```cairo
// Directly inject values into contract storage (testing only)
store(
    contract_address,
    map_entry_address(
        selector!("hash_stored_avg_fees"),
        array![timestamp.into()].span()
    ),
    array![val0, val1, val2, val3, val4, val5, val6, val7].span(),
);
```

#### Mock External Calls

```cairo
// Mock Fossil Store responses
start_mock_call(
    fossil_store_address,
    selector!("get_avg_fee"),
    expected_fee_value,
);
```

## Configuration

### Environment Variables

Contract addresses and network configuration are managed through environment files:

```bash
# .env.example (template)
STARKNET_RPC_URL=http://localhost:5050
STARKNET_ACCOUNT=katana-0
STARKNET_ACCOUNT_ADDRESS=0x127fd5f1fe78a71f8bcd1fec63e3fe2f0486b6ecd5c86a0466c3a21fa5cfcec
STARKNET_PRIVATE_KEY=0xc5b2fcab997346f3ea1c00b002ecf6f382c5f9c9659a3894eb783c5320f912

# Contract Addresses (populated by deploy script)
UNIVERSAL_ECIP_CONTRACT=
GROTH16_VERIFIER_CONTRACT=
PITCHLAKE_VERIFIER_CONTRACT=
HASH_STORAGE_ADDRESS=
FOSSIL_STORE_ADDRESS=0x00e581139553c8666f60b6646f277a336f99f108f8e5fa7cb300b6a6ce7c3b8c

# Vault Addresses (PitchLake integration)
PITCHLAKE_VAULT_12MIN=
PITCHLAKE_VAULT_3H=
PITCHLAKE_VAULT_1M=

NETWORK=DEVNET_KATANA
```

### Network Configuration

Different environments use different RPC endpoints:

| Environment | RPC URL | Network |
|-------------|---------|---------|
| Docker (internal) | `http://katana:5050` | Katana devnet |
| Local (external) | `http://localhost:5050` | Katana devnet |
| Sepolia | `https://starknet-sepolia.infura.io/v3/...` | Sepolia testnet |
| Mainnet | `https://starknet-mainnet.infura.io/v3/...` | StarkNet mainnet |

### Contract Addresses by Environment

#### Local Development (Katana)
Addresses are dynamically assigned during deployment. Example addresses:

```bash
HASH_STORAGE_ADDRESS=0x01929d8c867c2a261669ccb0e90cec2c81329164e581ad095edd0cce11cc617a
PITCHLAKE_VERIFIER_CONTRACT=0x123...
GROTH16_VERIFIER_CONTRACT=0x456...
```

#### Sepolia Testnet
Production testnet addresses (update after deployment):

```bash
HASH_STORAGE_ADDRESS=0x...
PITCHLAKE_VERIFIER_CONTRACT=0x...
GROTH16_VERIFIER_CONTRACT=0x...
```

### Scarb Configuration

#### Workspace Configuration

```toml
# /starknet-contracts/Scarb.toml
[workspace]
members = ["fossil-hash-store", "pitchlake-verifier"]

[[target.starknet-contract]]
casm = true
casm-add-pythonic-hints = true

[workspace.tool.fmt]
sort-module-level-items = true
```

#### Contract-Specific Configuration

```toml
# /starknet-contracts/fossil-hash-store/Scarb.toml
[package]
name = "sha2_input"
version = "0.1.0"
edition = "2023_11"

[dependencies]
openzeppelin_upgrades.workspace = true
openzeppelin_access.workspace = true
starknet.workspace = true

[dev-dependencies]
snforge_std.workspace = true
assert_macros.workspace = true
```

### Feature Flags

Rust services use feature flags to control contract integration:

```bash
# Enable onchain proof verification
VERIFY_PROOFS_ONCHAIN=true

# Enable proof generation
ENABLE_PROOF=true

# Use RISC0 integration
USE_RISC0_INTEGRATION=true
```

## Next Steps

### Contract-Specific Documentation

For detailed implementation documentation, see:

- **Fossil Hash Store**: [`/starknet-contracts/fossil-hash-store/ARCHITECTURE.md`](starknet-contracts/fossil-hash-store/ARCHITECTURE.md)
  - Detailed hashing algorithms
  - HashingService integration roadmap
  - Performance characteristics
  - Future improvements

- **Fossil Hash Store README**: [`/starknet-contracts/fossil-hash-store/README.md`](starknet-contracts/fossil-hash-store/README.md)
  - Build requirements
  - Deployment instructions
  - Version compatibility notes

### Related Architecture Documents

- [Architecture Overview](docs/architecture/overview.md) - System-wide architecture
- [Data Flow](docs/architecture/data-flow.md) - End-to-end data processing
- [Proving Service Architecture](docs/architecture/proving-service.md) - RISC Zero proof generation
- [Fossil API Architecture](docs/architecture/fossil-api.md) - API design and endpoints

### Development Resources

- **Cairo Documentation**: https://book.cairo-lang.org/
- **Starknet Foundry**: https://foundry-rs.github.io/starknet-foundry/
- **Scarb Package Manager**: https://docs.swmansion.com/scarb/
- **OpenZeppelin Cairo Contracts**: https://docs.openzeppelin.com/contracts-cairo/
- **Garaga (ZK-SNARK verification)**: https://github.com/keep-starknet-strange/garaga
- **PitchLake Vault**: https://github.com/OilerNetwork/pitchlake_starknet

### Local Development Setup

```bash
# 1. Install dependencies
# See: docs/getting-started/installation.md

# 2. Start local Katana devnet
docker-compose -f docker-compose.local.yml up -d katana

# 3. Deploy contracts
docker-compose -f docker-compose.deploy.yml up

# 4. Verify deployment
source .env.local
echo "Hash Store: $HASH_STORAGE_ADDRESS"
echo "Verifier: $PITCHLAKE_VERIFIER_CONTRACT"

# 5. Run contract tests
cd starknet-contracts
scarb test
```

### Integration Testing

For end-to-end integration testing including contract interactions:

```bash
# Run full E2E test suite (includes contract deployment)
# See: docs/getting-started/testing.md

# Test hash generation workflow
cd proving-service
cargo test --test hashing_integration -- --nocapture

# Test proof verification workflow
cargo test --test proof_verification_e2e -- --nocapture
```

### Monitoring and Debugging

#### Contract Events

Monitor contract events using StarkNet tools:

```bash
# Watch for HashStoredAvgFees events
starkli events $HASH_STORAGE_ADDRESS

# Watch for PitchlakeProofVerified events
starkli events $PITCHLAKE_VERIFIER_CONTRACT
```

#### Transaction Tracking

```bash
# Check transaction status
starkli transaction <tx_hash>

# View transaction receipt
starkli receipt <tx_hash>
```

#### Contract State Inspection

```bash
# Read Level 1 hash
starkli call $HASH_STORAGE_ADDRESS get_hash_stored_avg_fees <timestamp>

# Read Level 2 composite hash
starkli call $HASH_STORAGE_ADDRESS get_hash_stored_batched_avg_fees <timestamp>
```

### Troubleshooting

#### Common Issues

1. **Contract Not Found**
   - Verify contract is deployed: Check `.env.{environment}` for addresses
   - Confirm RPC URL is correct for environment
   - Ensure network is running (Katana for local development)

2. **Hash Generation Failures**
   - Verify Fossil Store has data for requested timestamp range
   - Check account has sufficient ETH for transaction fees
   - Confirm Level 1 hashes exist before generating Level 2 hash

3. **Proof Verification Failures**
   - Validate proof format matches expected Groth16 structure
   - Verify verifier contract address is correct
   - Check journal decoding matches proof output format
   - Ensure vault address exists and implements callback interface

4. **Deployment Issues**
   - Confirm Scarb version matches project requirements (2.9.2)
   - Verify all dependencies are available
   - Check starkli and starknet-devnet versions
   - Ensure account has deployment permissions

### Future Enhancements

**Planned Improvements** (from Fossil Hash Store ARCHITECTURE.md):

1. **Dynamic Batch Sizes**: Configurable batch parameters for different use cases
2. **Incremental Hashing**: Support for partial hash updates without full regeneration
3. **Multi-Level Hierarchy**: Additional hash levels for larger datasets (>8 months)
4. **Cross-Chain Compatibility**: Integration with other blockchain networks
5. **Hash Caching**: In-memory caching to avoid redundant hash generation
6. **Performance Metrics**: Monitoring of gas usage and transaction success rates

**Integration Roadmap** (HashingService):

- Phase 1: Foundation Setup (COMPLETED)
  - ✅ Integrate HashingService into BonsaiProofProvider
  - ✅ Add hash availability validation
  - ✅ Implement error handling

- Phase 2: Workflow Integration (COMPLETED)
  - ✅ Modify ProofJobHandler to include hashing step
  - ✅ Add hash service dependency injection
  - ✅ Implement dual hashing system (onchain + in-memory)

- Phase 3: Configuration & Environment (IN PROGRESS)
  - [ ] Update environment configuration for all deployments
  - [ ] Add hash service configuration validation
  - [ ] Update E2E test integration

- Phase 4: Performance & Monitoring (PLANNED)
  - [ ] Add hash service performance metrics
  - [ ] Implement hash caching strategy
  - [ ] Add health checks and observability

---

## Summary

The StarkNet contracts provide critical infrastructure for the Fossil ecosystem:

1. **Fossil Hash Store** enables efficient data integrity validation through hierarchical hashing (Level 1: 180-hour batches, Level 2: 32-batch composites covering 8 months)

2. **PitchLake Verifier** validates RISC Zero Groth16 proofs onchain using BN254 pairing-based cryptography and triggers automated callbacks to vault contracts

3. **Mock Contracts** accelerate development with lightweight testing infrastructure

4. **Docker-based deployment** supports multiple environments (local Katana, Sepolia, Mainnet) with automated address management

5. **Rust integration** via `HashingProvider` and `ProofVerifier` enables seamless contract interaction from proof generation services

This architecture balances onchain security with computational efficiency, using cryptographic hashes to validate large datasets without storing raw data onchain, and leveraging zero-knowledge proofs to verify complex financial calculations with mathematical certainty.
