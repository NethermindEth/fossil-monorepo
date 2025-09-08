# Fossil Hash Store Contract Architecture

## Overview

The Fossil Hash Store contract (`Sha2Input`) is a StarkNet smart contract that provides hierarchical hashing functionality for average fee data from the Fossil store. It creates a two-tiered hash system for efficient storage and verification of large datasets used in the Fossil proof generation pipeline.

## Contract Details

**Contract Name**: `Sha2Input`
**Interface**: `ISha2Input`
**Package**: `sha2_input`
**Version**: `0.1.0`

## Architecture & Design

### Core Functionality

The contract implements a **hierarchical hashing system** with two levels:

1. **Level 1**: Hashes batches of 180 consecutive hourly average fees
2. **Level 2**: Hashes collections of 32 Level 1 hashes to create batch summaries

This design enables efficient proof generation for large datasets (5760 data points) while maintaining reasonable gas costs and storage requirements.

### Data Flow

```
Fossil Store (Raw Data)
    ↓
Level 1: hash_avg_fees_and_store()
    ↓ (180 hourly fees → SHA256 hash)
Storage: hash_stored_avg_fees[timestamp] = [u32; 8]
    ↓
Level 2: hash_batched_avg_fees()
    ↓ (32 hashes → SHA256 hash)
Storage: hash_batched_avg_fees[timestamp] = [u32; 8]
```

## Contract Interface

### Core Functions

#### `hash_avg_fees_and_store(start_timestamp: u64)`
- **Purpose**: Hashes 180 consecutive hourly average fees starting from `start_timestamp`
- **Process**:
  1. Fetches 180 hourly fees from Fossil store (3600-second intervals)
  2. Converts each fee to u32 array representation
  3. Computes SHA256 hash of concatenated data
  4. Stores hash in `hash_stored_avg_fees` mapping
- **Storage**: Maps `start_timestamp` → `[u32; 8]` hash
- **Events**: Emits `HashStoredAvgFees`

#### `get_hash_stored_avg_fees(timestamp: u64) -> [u32; 8]`
- **Purpose**: Retrieves stored Level 1 hash for given timestamp
- **Returns**: SHA256 hash as 8 x u32 array

#### `hash_batched_avg_fees(start_timestamp: u64)`
- **Purpose**: Hashes 32 Level 1 hashes into a single batch hash
- **Process**:
  1. Collects 32 Level 1 hashes (each covering 180 hours = 648,000 seconds)
  2. Validates all hashes are non-empty
  3. Computes SHA256 hash of concatenated hash data
  4. Stores result in `hash_batched_avg_fees` mapping
- **Storage**: Maps `start_timestamp` → `[u32; 8]` batch hash
- **Events**: Emits `HashStoredBatchedAvgFees`

#### `get_hash_stored_batched_avg_fees(timestamp: u64) -> [u32; 8]`
- **Purpose**: Retrieves stored Level 2 batch hash
- **Returns**: SHA256 hash as 8 x u32 array

### Administrative Functions

#### `set_fossil_store(fossil_store: ContractAddress)`
- **Purpose**: Updates the Fossil store contract address
- **Access**: Owner only
- **Security**: Protected by OpenZeppelin Ownable component

#### `get_fossil_store() -> ContractAddress`
- **Purpose**: Returns current Fossil store contract address

## Mathematical Constants

- **Batch Size (Level 1)**: 180 hours per hash
- **Batch Collection (Level 2)**: 32 hashes per batch
- **Total Coverage**: 180 × 32 = 5760 hours of data per Level 2 hash
- **Time Intervals**: 3600 seconds (1 hour) between data points
- **Hash Format**: SHA256 represented as `[u32; 8]` array

## Dependencies & Integrations

### External Contracts
- **Fossil Store**: Source of average fee data via `IFossilMinimalAvgFeeStore` interface
  - `get_avg_fee(timestamp: u64) -> felt252`
  - `get_avg_fees_in_range(start_timestamp: u64, end_timestamp: u64) -> Array<felt252>`

### OpenZeppelin Components
- **OwnableComponent**: Access control for administrative functions
- **UpgradeableComponent**: Contract upgradeability support

### Core StarkNet Libraries
- **SHA256**: `core::sha256::compute_sha256_u32_array`
- **Storage**: Maps for persistent hash storage

## Integration with Proving Service

### HashingProvider Integration

The contract integrates with the proving service through the `HashingProvider` struct located in:
`proving-service/crates/message-handler/src/hashing/mod.rs`

#### Key Integration Points:

1. **Environment Configuration**:
   ```rust
   HASH_STORAGE_ADDRESS=0x01929d8c867c2a261669ccb0e90cec2c81329164e581ad095edd0cce11cc617a
   ```

2. **HashingService Workflow**:
   ```rust
   // 1. Check data availability
   check_avg_fees_availability(start_timestamp, end_timestamp)

   // 2. Find missing Level 1 hashes
   get_unavailable_batch_timestamp_hashes()

   // 3. Generate missing hashes on-chain
   hash_and_store_avg_fees_onchain()

   // 4. Generate Level 2 batch hash
   hash_batch_avg_fees_onchain()
   ```

3. **Provider Functions**:
   - `get_hash_stored_avg_fees()`: Reads Level 1 hashes
   - `get_hash_batched_avg_fees()`: Reads Level 2 hashes
   - `hash_avg_fees_and_store()`: Triggers Level 1 hash generation
   - `hash_batched_avg_fees()`: Triggers Level 2 hash generation

### E2E Testing Integration

The contract is essential for the end-to-end testing workflow described in `TEST_E2E_GUIDE.md`:

#### Test Environment Setup:
```bash
# Contract addresses in .env.local
HASH_STORAGE_ADDRESS=0x01929d8c867c2a261669ccb0e90cec2c81329164e581ad095edd0cce11cc617a
FOSSIL_STORE_ADDRESS=0x00e581139553c8666f60b6646f277a336f99f108f8e5fa7cb300b6a6ce7c3b8c
```

#### E2E Test Flow:
1. **Service Startup**: HashingService initializes with contract addresses
2. **Data Processing**: Services request hash generation for proof composition
3. **Hash Generation**: Contract generates and stores hierarchical hashes
4. **Proof Generation**: RISC0 proof pipeline uses stored hashes
5. **Verification**: On-chain verification of proofs using hash data

## Storage Layout

### State Variables
```cairo
#[storage]
struct Storage {
    fossil_store: IFossilMinimalAvgFeeStoreDispatcher,
    hash_stored_avg_fees: Map<u64, [u32; 8]>,        // Level 1: timestamp → hash
    hash_batched_avg_fees: Map<u64, [u32; 8]>,       // Level 2: timestamp → batch hash
    // OpenZeppelin components
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

## Helper Functions

### Data Conversion (`helper.cairo`)

#### `convert_avg_fees_to_u32_array(avg_fees: felt252) -> Array<u32>`
- Converts felt252 values to u32 arrays for SHA256 input
- Handles big-endian to little-endian conversion
- Splits 252-bit values into 8 × 32-bit words

#### `hash_of_avg_fees(avg_fees: Array<felt252>) -> [u32; 8]`
- Utility function for hashing arrays of average fees
- Used for testing and validation

## Mock Implementation

### MockHashStorage
Located in: `proving-service/mock_contracts/src/mock_hash_storage.cairo`

Provides testing implementation with additional functions:
- `set_hash_stored_avg_fees()`: Direct hash injection for testing
- `set_hash_stored_batched_avg_fees()`: Direct batch hash injection
- Simplified logic without actual computation

## Deployment & Configuration

### Deployment Script
The contract is deployed via `scripts/deploy-starknet.sh`:

```bash
# Deploy Sha2Input contract
SHA2INPUT_ADDRESS=$(starkli deploy $SHA2INPUT_HASH $STARKNET_ACCOUNT_ADDRESS 0x0 ...)

# Update environment with new address
update_env_var "HASH_STORAGE_ADDRESS" "$SHA2INPUT_ADDRESS"
```

### Constructor Parameters
- `owner`: Contract owner address (typically deployer account)
- `fossil_store`: Address of the Fossil store contract

## Security Considerations

### Access Control
- **Owner-only functions**: `set_fossil_store()`, `upgrade()`
- **OpenZeppelin Ownable**: Standardized access control implementation
- **Upgradeability**: Contract can be upgraded by owner

### Data Validation
- **Non-zero fee check**: Prevents hashing of invalid/missing data
- **Hash emptiness check**: Ensures Level 1 hashes exist before Level 2 processing
- **Timestamp validation**: Enforces hourly intervals in Fossil store queries

### Gas Optimization
- **Batch processing**: Reduces transaction costs through hierarchical hashing
- **Storage efficiency**: Fixed-size hash storage vs. raw data arrays
- **Concurrent execution**: Level 1 hashing can be parallelized

## Testing

### Test Coverage
Located in: `starknet-contracts/fossil-hash-store/tests/`

#### `test_contract.cairo`
- ✅ Level 1 hash generation and storage
- ✅ Level 2 batch hash generation
- ✅ Owner access control validation
- ✅ Contract upgradeability
- ✅ Error cases (zero fees, missing hashes)

#### `test_helper.cairo`
- ✅ Data conversion utilities
- ✅ Hash computation functions

### Running Tests
```bash
cd starknet-contracts/fossil-hash-store
scarb test
```

## Performance Characteristics

### Gas Costs
- **Level 1 Hash**: ~180 Fossil store calls + SHA256 computation
- **Level 2 Hash**: ~32 storage reads + SHA256 computation
- **Batch optimization**: Reduces per-data-point cost through aggregation

### Storage Requirements
- **Level 1**: 1 hash per 180 hours of data
- **Level 2**: 1 hash per 5760 hours of data
- **Hash size**: 256 bits (8 × u32) per stored hash

### Time Complexity
- **Level 1 generation**: O(n) where n = 180 data points
- **Level 2 generation**: O(m) where m = 32 hashes
- **Hash retrieval**: O(1) for both levels

## Future Considerations

### Potential Improvements
1. **Dynamic batch sizes**: Configurable batch parameters
2. **Incremental hashing**: Support for partial hash updates
3. **Multi-level hierarchy**: Additional hash levels for larger datasets
4. **Cross-chain compatibility**: Integration with other blockchain networks

### Monitoring & Observability
- **Event emission**: Track hash generation activity
- **Hash validation**: Verify hash integrity across service restarts
- **Performance metrics**: Monitor gas usage and transaction success rates

## HashingService Integration Roadmap

### Current Status: Integration Gap Identified

The HashingService exists but is **not integrated** into the current proof generation workflow. The proof pipeline currently bypasses hash preparation, leading to potential inefficiencies and reliability issues.

### Integration TODO List

#### Phase 1: Foundation Setup
- [x] **Integrate HashingService into BonsaiProofProvider workflow**
  - Location: `proving-service/crates/message-handler/src/proof_composition/mod.rs`
  - Action: Add HashingService instantiation and execution before proof generation
  - Impact: Ensures required hashes are available before expensive proof operations

- [x] **Add HashingProvider initialization to proof composition**
  - Location: `BonsaiProofProvider.generate_proofs_from_data()` method
  - Action: Create HashingProvider instance alongside StarknetProvider
  - Dependencies: Environment variables `HASH_STORAGE_ADDRESS`, `FOSSIL_STORE_ADDRESS`

- [x] **Implement hash availability validation**
  - Location: Before fee data processing in proof composition
  - Action: Call `HashingService.run()` with computed timestamp ranges
  - Benefit: Pre-validates and generates missing hash data

#### Phase 2: Workflow Integration
- [x] **Modify ProofJobHandler to include hashing step**
  - Location: `proving-service/crates/message-handler/src/services/proof_job_handler.rs`
  - Action: Add HashingService call before proof generation (line ~220)
  - Integration point: Between timestamp range creation and proof provider call
  - Implementation: Hash preparation integrated within BonsaiProofProvider.generate_proofs_from_data()

- [x] **Add hash service dependency injection**
  - Location: `ProofJobHandler.new()` constructor
  - Action: Accept HashingProvider parameter and pass to proof generation
  - Dependencies: Update main.rs to instantiate HashingProvider
  - Implementation: HashingProvider created within BonsaiProofProvider (simpler architecture)

- [x] **Implement error handling for hash preparation failures**
  - Location: Proof generation error handling block
  - Action: Distinguish hash preparation failures from proof generation failures
  - Benefit: Better error reporting and retry logic
  - Implementation: Hash errors propagate through existing ProofJobHandler failure handling

#### Phase 3: Data Flow Optimization
- [x] **Hash-verified data retrieval with dual hashing system**
  - Location: `BonsaiProofProvider.generate_proofs_from_data()`
  - Implementation: Uses **two complementary hashing systems**:
    1. **On-chain hash store validation** (lines 140-150): HashingService ensures data availability
    2. **In-memory proof composition hashing** (line 202): `hash_felts()` creates integrity hash
  - Result: Proof composition gets both `data_8_months` (raw data) and `data_8_months_hash` (integrity hash)

- [x] **Add hash validation to fee data processing**
  - Location: Fee data validation block in proof composition  
  - Implementation: Hash preparation runs before data fetching (lines 146-150)
  - Security: Prevents proof generation with corrupted or incomplete data
  - Note: Hash store validation ensures data integrity; proof composition uses separate hash

- [x] **Implement batch hash optimization**
  - Location: Large dataset processing (5760 data points)
  - Implementation: HashingService uses hierarchical Level 1 & Level 2 batch hashes
  - Performance: Reduces validation overhead through pre-computed hash infrastructure

#### Phase 4: Configuration & Environment
- [ ] **Update environment configuration**
  - Files: `.env.local`, `.env.docker`, `.env.sepolia`
  - Action: Ensure all environments have correct `HASH_STORAGE_ADDRESS`
  - Validation: Deploy script automatically updates addresses

- [ ] **Add hash service configuration validation**
  - Location: Service startup validation
  - Action: Verify hash storage contract is deployed and accessible
  - Reliability: Fail fast if hash infrastructure is unavailable

- [ ] **Update E2E test integration**
  - Location: `test-e2e.sh` script and E2E test flow
  - Action: Add hash preparation validation to test suite
  - Coverage: Ensure hash generation is tested in full pipeline

#### Phase 5: Performance & Monitoring
- [ ] **Add hash service performance metrics**
  - Location: HashingService execution
  - Metrics: Hash generation time, success rate, gas costs
  - Monitoring: Track hash preparation impact on overall proof time

- [ ] **Implement hash caching strategy**
  - Location: HashingService implementation
  - Action: Avoid redundant hash generation for overlapping time ranges
  - Optimization: Cache recently generated hashes in memory

- [ ] **Add hash service health checks**
  - Location: Service monitoring endpoints
  - Action: Expose hash availability status via health endpoint
  - Operations: Enable monitoring of hash infrastructure health

### Implementation Strategy

#### Immediate Actions (Week 1)
1. **Integrate HashingService into BonsaiProofProvider**
2. **Add hash availability validation before proof generation**
3. **Update ProofJobHandler to include hashing step**

#### Short-term Actions (Week 2-3)
4. **Replace direct fee fetching with hash-verified data retrieval**
5. **Implement comprehensive error handling**
6. **Update environment configuration and validation**

#### Medium-term Actions (Month 1)
7. **Add performance monitoring and optimization**
8. **Implement hash caching strategy**
9. **Update E2E testing to include hash validation**

### Integration Benefits

1. **Data Integrity**: Hash verification ensures proof generation uses correct data
2. **Performance**: Pre-computed hashes reduce proof generation time
3. **Reliability**: Early hash preparation prevents proof failures due to missing data
4. **Cost Optimization**: Batch hash processing reduces overall gas costs
5. **Monitoring**: Hash service provides additional observability into data pipeline

### Risk Mitigation

- **Backwards Compatibility**: Keep existing proof generation path as fallback
- **Gradual Rollout**: Enable hash integration via feature flag
- **Error Handling**: Graceful degradation if hash service unavailable
- **Testing**: Comprehensive integration testing before production deployment

## Conclusion

The Fossil Hash Store contract provides a critical infrastructure component for the Fossil ecosystem, enabling efficient storage and verification of large average fee datasets. Its hierarchical hashing design balances computational efficiency, storage costs, and proof generation requirements.

**The integration roadmap above addresses the current gap where HashingService exists but is not utilized, providing a clear path to full integration with the proof generation pipeline.** Once integrated, the hash store will become essential for the end-to-end proof pipeline described in the broader Fossil monorepo architecture.
