# Fossil Hash Store Contract

This document provides an overview of the Fossil Hash Store contract. For complete architectural details, see the [contract's ARCHITECTURE.md](../../starknet-contracts/fossil-hash-store/ARCHITECTURE.md).

## Overview

The Fossil Hash Store contract (`Sha2Input`) is a StarkNet smart contract that provides hierarchical hashing functionality for average fee data. It creates a two-tiered hash system for efficient storage and verification of large datasets used in the Fossil proof generation pipeline.

**Contract Name**: `Sha2Input`
**Interface**: `ISha2Input`
**Location**: `starknet-contracts/fossil-hash-store/`

## Key Features

- **Hierarchical Hashing**: Two-level hash system for efficient data management
- **Level 1**: Hashes batches of 180 consecutive hourly average fees
- **Level 2**: Hashes collections of 32 Level 1 hashes
- **Total Coverage**: 5760 hours of data per Level 2 hash
- **SHA256**: Cryptographic hashing for data integrity
- **Access Control**: OpenZeppelin Ownable for administrative functions
- **Upgradeability**: Contract can be upgraded by owner

## Architecture Summary

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

### Core Functions

| Function | Purpose | Parameters |
|----------|---------|------------|
| `hash_avg_fees_and_store` | Generate Level 1 hash | `start_timestamp: u64` |
| `get_hash_stored_avg_fees` | Retrieve Level 1 hash | `timestamp: u64` |
| `hash_batched_avg_fees` | Generate Level 2 hash | `start_timestamp: u64` |
| `get_hash_stored_batched_avg_fees` | Retrieve Level 2 hash | `timestamp: u64` |
| `set_fossil_store` | Update Fossil store address | `fossil_store: ContractAddress` |
| `get_fossil_store` | Get Fossil store address | - |

## Integration with Proving Service

The contract integrates with the Rust proving service through the `HashingProvider`:

```rust
// Location: proving-service/crates/message-handler/src/hashing/mod.rs

// 1. Check data availability
check_avg_fees_availability(start_timestamp, end_timestamp)

// 2. Find missing Level 1 hashes
get_unavailable_batch_timestamp_hashes()

// 3. Generate missing hashes on-chain
hash_and_store_avg_fees_onchain()

// 4. Generate Level 2 batch hash
hash_batch_avg_fees_onchain()
```

## Configuration

### Environment Variables

```bash
# Hash storage contract address
HASH_STORAGE_ADDRESS=0x01929d8c867c2a261669ccb0e90cec2c81329164e581ad095edd0cce11cc617a

# Fossil store contract address
FOSSIL_STORE_ADDRESS=0x00e581139553c8666f60b6646f277a336f99f108f8e5fa7cb300b6a6ce7c3b8c
```

### Deployment

The contract is deployed via the deployment script:

```bash
# Deploy script location
scripts/deploy-starknet.sh

# Constructor parameters
- owner: Contract owner address
- fossil_store: Fossil store contract address
```

## Testing

### Running Tests

```bash
cd starknet-contracts/fossil-hash-store
scarb test
```

### Test Coverage

Located in `tests/`:
- ✅ Level 1 hash generation and storage
- ✅ Level 2 batch hash generation
- ✅ Owner access control validation
- ✅ Contract upgradeability
- ✅ Error cases (zero fees, missing hashes)

## Performance Characteristics

### Storage Requirements
- **Level 1**: 1 hash per 180 hours of data
- **Level 2**: 1 hash per 5760 hours of data
- **Hash size**: 256 bits (8 × u32) per stored hash

### Time Complexity
- **Level 1 generation**: O(180) - 180 data points
- **Level 2 generation**: O(32) - 32 hashes
- **Hash retrieval**: O(1) for both levels

### Gas Optimization
- Batch processing reduces transaction costs
- Fixed-size hash storage vs. raw data arrays
- Level 1 hashing can be parallelized

## Security

### Access Control
- Owner-only functions: `set_fossil_store()`, `upgrade()`
- OpenZeppelin Ownable implementation
- Upgradeability restricted to owner

### Data Validation
- Non-zero fee check prevents invalid data hashing
- Hash emptiness check ensures Level 1 hashes exist
- Timestamp validation enforces hourly intervals

## Integration Status

### Current Integration (✅ Completed)

The HashingService is **fully integrated** into the proof generation workflow:

1. **BonsaiProofProvider Integration**
   - HashingService runs before proof generation
   - Ensures all required hashes are available
   - Location: `proof_composition/mod.rs`

2. **Dual Hashing System**
   - On-chain hash store validation via HashingService
   - In-memory proof composition hashing via `hash_felts()`
   - Both provide data integrity at different stages

3. **Error Handling**
   - Hash preparation failures handled gracefully
   - Propagates through existing failure tracking

### Future Enhancements (Planned)

- Environment configuration validation on startup
- E2E test integration with hash validation
- Performance metrics and monitoring
- Hash caching strategy for overlapping ranges
- Health check endpoints for hash infrastructure

## Complete Documentation

For comprehensive architectural details, integration roadmap, and implementation guides, see:

📄 **[Complete Architecture Documentation](../../starknet-contracts/fossil-hash-store/ARCHITECTURE.md)**

This includes:
- Detailed contract interface
- Complete integration roadmap
- HashingService workflow
- Storage layout and events
- Helper functions
- Mock implementation
- Performance analysis
- Security considerations

## Next Steps

- [StarkNet Contracts Architecture](../architecture/starknet-contracts.md) - Overview of all contracts
- [Message Handler Crate](../crates/proving-service/message-handler.md) - Hashing service implementation
- [Deployment Guide](../guides/deployment.md) - Deploying contracts
