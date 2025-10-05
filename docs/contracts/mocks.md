# Mock Contracts

This document provides comprehensive information about the mock contract implementations used for testing and local development in the Fossil monorepo.

## Table of Contents
- [Overview](#overview)
- [Available Mocks](#available-mocks)
- [Mock Contract Details](#mock-contract-details)
- [Usage in Local Development](#usage-in-local-development)
- [Usage in Testing](#usage-in-testing)
- [Configuration](#configuration)
- [Deployment](#deployment)
- [Test Examples](#test-examples)
- [Limitations](#limitations)
- [Migration to Production](#migration-to-production)
- [Next Steps](#next-steps)

## Overview

Mock contracts provide lightweight, simplified implementations of external dependencies for testing and local development. They enable:

1. **Fast Local Testing** - Test contract interactions without deploying full contract ecosystems
2. **Isolated Development** - Develop and test features without external dependencies
3. **Predictable Test Data** - Control test data for deterministic testing
4. **CI/CD Integration** - Fast contract testing in continuous integration pipelines
5. **Development Iteration** - Rapid development cycles without network latency

### Purpose in the System

Mock contracts serve as simplified stand-ins for production contracts during development:

- **Testing Infrastructure** - Enable comprehensive unit and integration tests
- **Local Development** - Support development on local Katana devnet without full dependency stack
- **Contract Validation** - Verify contract interfaces and integration points
- **Cost Reduction** - Eliminate gas costs during development and testing

### Technology Stack

- **Language**: Cairo 2.x (StarkNet smart contract language)
- **Build Tool**: Scarb 2.9.2 (Cairo package manager)
- **Testing Framework**: Starknet Foundry (snforge)
- **Test Utilities**: snforge_std, assert_macros

## Available Mocks

The Fossil monorepo includes the following mock contracts:

| Mock Contract | Purpose | Production Equivalent | Location |
|---------------|---------|----------------------|----------|
| **MockFossilStore** | Provides test data for average fee queries | Fossil Store (Fossil Light Client) | `starknet-contracts/fossil-hash-store/src/mock/fossil_store.cairo` |
| **TestUpgrade** | Tests contract upgradeability | N/A (Testing utility) | `starknet-contracts/fossil-hash-store/src/mock/test_upgrade.cairo` |

### Mock Contract Package

A dedicated mock contracts package exists at `/starknet-contracts/mocks/` for standalone mock deployments:

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

This package can contain additional mock contracts for local development deployments.

## Mock Contract Details

### MockFossilStore

**Purpose**: Provides a simplified in-memory implementation of the Fossil Store contract interface for testing hash generation workflows.

**Location**: `/starknet-contracts/fossil-hash-store/src/mock/fossil_store.cairo`

#### Interface Implementation

The mock implements the `IFossilMinimalAvgFeeStore` interface required by the Fossil Hash Store contract:

```cairo
#[starknet::interface]
pub trait IFossilMinimalAvgFeeStore<TContractState> {
    fn get_avg_fee(self: @TContractState, timestamp: u64) -> felt252;
    fn get_avg_fees_in_range(
        self: @TContractState,
        start_timestamp: u64,
        end_timestamp: u64
    ) -> Array<felt252>;
}
```

#### Simplified vs Real Implementation

**Real Fossil Store** (Production):
- Indexes blockchain data from Ethereum/StarkNet
- Computes weighted average fees from multiple data sources
- Requires fossil-light-client indexer service
- Has ~12 hour indexing lag for recent data
- Stores historical data in optimized format

**MockFossilStore** (Testing):
- Stores fee data in simple in-memory map
- Provides manual data insertion via `store_avg_fees()`
- No external dependencies or indexing
- Instant data availability
- Minimal storage overhead

#### Implementation Details

```cairo
#[starknet::contract]
mod MockFossilStore {
    use starknet::storage::{
        Map, StoragePathEntry, StoragePointerReadAccess, StoragePointerWriteAccess,
    };
    use crate::interface::fossil_store::IFossilMinimalAvgFeeStore;
    use super::IMockFossilWriteStore;

    #[starknet::storage_node]
    pub struct AvgFees {
        data_points: u64,
        avg_fee: felt252,
    }

    #[storage]
    struct Storage {
        avg_fees: Map<u64, AvgFees>,
    }

    const HOUR_IN_SECONDS: u64 = 3600;

    // Read operations (matches production interface)
    #[abi(embed_v0)]
    impl MockFossilStoreImpl of IFossilMinimalAvgFeeStore<ContractState> {
        fn get_avg_fee(self: @ContractState, timestamp: u64) -> felt252 {
            assert!(timestamp % HOUR_IN_SECONDS == 0, "Timestamp must be a multiple of 3600");
            let curr_state = self.avg_fees.entry(timestamp);
            curr_state.avg_fee.read()
        }

        fn get_avg_fees_in_range(
            self: @ContractState,
            start_timestamp: u64,
            end_timestamp: u64
        ) -> Array<felt252> {
            assert!(
                start_timestamp <= end_timestamp,
                "Start timestamp must be less than or equal to end timestamp"
            );
            assert!(
                start_timestamp % HOUR_IN_SECONDS == 0,
                "Start timestamp must be a multiple of 3600"
            );
            assert!(
                end_timestamp % HOUR_IN_SECONDS == 0,
                "End timestamp must be a multiple of 3600"
            );

            let mut fees = array![];
            let mut i = start_timestamp;
            while i <= end_timestamp {
                fees.append(self.get_avg_fee(i));
                i += HOUR_IN_SECONDS;
            }
            fees
        }
    }

    // Write operations (testing utility, not in production interface)
    #[abi(embed_v0)]
    impl MockFossilWriteStoreImpl of IMockFossilWriteStore<ContractState> {
        fn store_avg_fees(
            ref self: ContractState,
            timestamp: u64,
            avg_fee: felt252,
            data_points: u64
        ) {
            let mut avg_fees = self.avg_fees.entry(timestamp);
            avg_fees.data_points.write(data_points);
            avg_fees.avg_fee.write(avg_fee);
        }
    }
}
```

#### When to Use MockFossilStore

Use MockFossilStore when:
- ✅ Testing Fossil Hash Store contract hash generation
- ✅ Validating hash computation algorithms
- ✅ Unit testing without external dependencies
- ✅ CI/CD pipeline testing
- ✅ Local development without fossil-light-client indexer

Do NOT use MockFossilStore when:
- ❌ Testing production data pipelines
- ❌ Validating real blockchain data indexing
- ❌ Testing historical data accuracy
- ❌ Production deployments
- ❌ Integration testing with real data sources

#### How It Differs from Production

| Feature | MockFossilStore | Production Fossil Store |
|---------|-----------------|------------------------|
| **Data Source** | Manual insertion | Blockchain indexing |
| **Data Validation** | Timestamp format only | Full data validation |
| **Performance** | O(1) reads | Optimized database queries |
| **Historical Data** | Must be manually populated | Automatically indexed |
| **Data Lag** | None | ~12 hour indexing lag |
| **Dependencies** | None | fossil-light-client, PostgreSQL |
| **Deployment** | Inline with tests | Separate service deployment |

### TestUpgrade

**Purpose**: Tests the upgradeability functionality of the Fossil Hash Store contract.

**Location**: `/starknet-contracts/fossil-hash-store/src/mock/test_upgrade.cairo`

#### Implementation

```cairo
#[starknet::interface]
pub trait ITestUpgrade<TContractState> {
    fn store_name(ref self: TContractState, name: felt252);
    fn get_name(self: @TContractState, address: ContractAddress) -> felt252;
}

#[starknet::contract]
mod TestUpgrade {
    use starknet::storage::{
        Map, StoragePathEntry, StoragePointerReadAccess, StoragePointerWriteAccess,
    };
    use starknet::{ContractAddress, get_caller_address};

    #[storage]
    struct Storage {
        names: Map<ContractAddress, felt252>,
        total_names: u128,
    }

    #[derive(Drop, Serde, starknet::Store)]
    pub struct Person {
        address: ContractAddress,
        name: felt252,
    }

    #[constructor]
    fn constructor(ref self: ContractState, owner: Person) {
        self.names.entry(owner.address).write(owner.name);
        self.total_names.write(1);
    }

    #[abi(embed_v0)]
    impl TestUpgrade of super::ITestUpgrade<ContractState> {
        fn store_name(ref self: ContractState, name: felt252) {
            let caller = get_caller_address();
            self._store_name(caller, name);
        }

        fn get_name(self: @ContractState, address: ContractAddress) -> felt252 {
            self.names.entry(address).read()
        }
    }

    // Standalone public function
    #[external(v0)]
    fn get_contract_name(self: @ContractState) -> felt252 {
        'Test Upgrade'
    }

    // Internal functions
    #[generate_trait]
    impl InternalFunctions of InternalFunctionsTrait {
        fn _store_name(ref self: ContractState, user: ContractAddress, name: felt252) {
            let total_names = self.total_names.read();
            self.names.entry(user).write(name);
            self.total_names.write(total_names + 1);
        }
    }
}
```

#### When to Use TestUpgrade

Use TestUpgrade when:
- ✅ Testing contract upgrade functionality
- ✅ Validating OpenZeppelin Upgradeable component
- ✅ Testing storage layout preservation after upgrade
- ✅ Verifying class hash replacement
- ✅ Testing access control on upgrade functions

#### Integration with Test Suite

TestUpgrade is used in the Fossil Hash Store test suite to validate upgradeability:

```cairo
#[test]
fn test_should_be_able_to_upgrade_by_owner() {
    // 1. Declare the TestUpgrade contract
    let new_class_hash = *declare("TestUpgrade").unwrap().contract_class().class_hash;

    // 2. Deploy original Sha2Input contract
    let contract_address = deploy_contract_sha2_input(owner(), fossil_store());

    // 3. Create upgrade dispatcher
    let dispatcher = IUpgradeableDispatcher { contract_address };

    // 4. Upgrade contract (owner only)
    start_cheat_caller_address(contract_address, owner());
    dispatcher.upgrade(new_class_hash);

    // 5. Verify upgrade succeeded
    assert(get_class_hash(contract_address) == new_class_hash, 'did not upgrade correctly');
}
```

This test validates:
- Owner can upgrade contract to new implementation
- Class hash is correctly updated
- Access control prevents non-owners from upgrading

## Usage in Local Development

### Starting Local Development Environment

The `make dev-up` command orchestrates local development services, including contract deployment with mocks:

```bash
# Start all local development services
make dev-up
```

This command:
1. Starts Katana devnet on `http://localhost:5050`
2. Starts PostgreSQL databases (Proving Service, Fossil API)
3. Starts LocalStack (AWS service mocks)
4. Deploys contracts to Katana
5. Starts application services

### Contract Deployment Process

The deployment script (`scripts/deploy-starknet.sh`) handles contract deployment:

```bash
# Deploy contracts to local Katana via Docker
./scripts/deploy-starknet.sh docker

# Deploy to local Katana directly
./scripts/deploy-starknet.sh local
```

#### Deployment Workflow

```
1. Environment Setup
   ↓
2. Scarb Build (compiles all contracts)
   ↓
3. Contract Declaration
   ├── Declare Sha2Input (Fossil Hash Store)
   ├── Declare Groth16Verifier
   ├── Declare PitchLakeVerifier
   └── Declare Universal ECIP
   ↓
4. Contract Deployment
   ├── Deploy Universal ECIP
   ├── Deploy Groth16Verifier (with ECIP address)
   ├── Deploy PitchLakeVerifier (with Groth16 address)
   └── Deploy Fossil Hash Store (with Fossil Store address)
   ↓
5. Update Environment Variables
   └── Write contract addresses to .env.docker and .env.local
```

### Mock Data Population

When using MockFossilStore in local development:

#### Option 1: snforge Mock Calls

Use snforge's `start_mock_call` in tests:

```cairo
// Mock all get_avg_fee calls to return specific value
start_mock_call(
    fossil_store_address,
    selector!("get_avg_fee"),
    565966358523639806057303238395575262125865566208
);
```

#### Option 2: Direct Storage Manipulation

For integration tests, directly populate mock storage:

```cairo
use snforge_std::{store, map_entry_address};

// Store fee data for specific timestamp
let timestamp = 1714636800_u64;
let fee_value = 565966358523639806057303238395575262125865566208;

store(
    mock_fossil_store_address,
    map_entry_address(
        selector!("avg_fees"),
        array![timestamp.into()].span()
    ),
    array![
        100_u64,      // data_points
        fee_value     // avg_fee
    ].span()
);
```

### Development Services Configuration

Local development uses `.env.docker` for service-to-service communication and `.env.local` for external access:

```bash
# .env.docker (internal service communication)
STARKNET_RPC_URL=http://katana:5050
FOSSIL_STORE_ADDRESS=0x00e581139553c8666f60b6646f277a336f99f108f8e5fa7cb300b6a6ce7c3b8c

# .env.local (external access)
STARKNET_RPC_URL=http://localhost:5050
FOSSIL_STORE_ADDRESS=0x00e581139553c8666f60b6646f277a336f99f108f8e5fa7cb300b6a6ce7c3b8c
```

## Usage in Testing

### Unit Testing with Mocks

Mock contracts enable isolated unit testing of contract functionality:

#### Example: Testing Hash Generation

```cairo
use openzeppelin_upgrades::interface::{IUpgradeableDispatcher, IUpgradeableDispatcherTrait};
use sha2_input::{ISha2InputDispatcher, ISha2InputDispatcherTrait};
use snforge_std::{
    ContractClassTrait, DeclareResultTrait, declare, start_mock_call,
};
use starknet::ContractAddress;

fn fossil_store() -> starknet::ContractAddress {
    'FOSSIL_STORE_ADDRESS'.try_into().unwrap()
}

fn owner() -> starknet::ContractAddress {
    'OWNER_ADDRESS'.try_into().unwrap()
}

#[test]
fn test_hash_avg_fees_and_store() {
    let start_timestamp = 1714636800_u64;

    // Mock Fossil Store to return constant fee value
    start_mock_call(
        fossil_store(),
        selector!("get_avg_fee"),
        565966358523639806057303238395575262125865566208
    );

    // Deploy Fossil Hash Store contract
    let contract_address = deploy_contract_sha2_input(owner(), fossil_store());
    let dispatcher = ISha2InputDispatcher { contract_address };

    // Generate Level 1 hash
    dispatcher.hash_avg_fees_and_store(start_timestamp);

    // Verify hash was computed correctly
    let hash_res = dispatcher.get_hash_stored_avg_fees(start_timestamp);
    assert(
        hash_res == [
            0x71316d72, 0x99f3b0a0, 0xf67978f3, 0x2f96f5de,
            0x0a358b38, 0x65b4a286, 0x568c4b8a, 0x833531ef
        ],
        'invalid hash result'
    );
}
```

#### Example: Testing Error Cases

```cairo
#[test]
#[should_panic(expected: 'Avg fees is 0')]
fn test_hash_avg_fees_and_store_fail_with_avg_fees_is_0() {
    let start_timestamp = 1714636800_u64;

    // Mock zero fee (invalid data)
    start_mock_call(fossil_store(), selector!("get_avg_fee"), 0);

    let contract_address = deploy_contract_sha2_input(owner(), fossil_store());
    let dispatcher = ISha2InputDispatcher { contract_address };

    // This should panic due to zero fee validation
    dispatcher.hash_avg_fees_and_store(start_timestamp);
}
```

### Integration Testing

For integration tests that require multiple batches of data:

```cairo
#[test]
fn test_hash_batched_avg_fees() {
    let start_timestamp = 1714636800_u64;
    let contract_address = deploy_contract_sha2_input(owner(), fossil_store());
    let dispatcher = ISha2InputDispatcher { contract_address };

    let num_in_a_batch = 32_u64;  // 5760/180

    // Populate Level 1 hashes directly in storage (simulates pre-existing hashes)
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
                171281208, 1706336902, 1452034954, 2201301487
            ].span(),
        );
    }

    // Generate Level 2 composite hash
    dispatcher.hash_batched_avg_fees(start_timestamp);

    // Verify composite hash
    let hash_res = dispatcher.get_hash_stored_batched_avg_fees(start_timestamp);
    assert(
        hash_res == [
            0xa375b797, 0x375dca1e, 0x2eb59bf5, 0xe5743db6,
            0x343351e6, 0xa16ea306, 0xb40c72b8, 0xa68e3133
        ],
        'invalid hash result'
    );
}
```

### Test Helpers

Common test utilities for working with mocks:

```cairo
// Deploy Sha2Input contract with mock Fossil Store
fn deploy_contract_sha2_input(
    owner: starknet::ContractAddress,
    fossil_store: starknet::ContractAddress
) -> ContractAddress {
    let contract = declare("Sha2Input").unwrap().contract_class();
    let (contract_address, _) = contract
        .deploy(@array![owner.into(), fossil_store.into()])
        .unwrap();
    contract_address
}

// Helper: Create mock addresses
fn fossil_store() -> starknet::ContractAddress {
    'FOSSIL_STORE_ADDRESS'.try_into().unwrap()
}

fn owner() -> starknet::ContractAddress {
    'OWNER_ADDRESS'.try_into().unwrap()
}
```

### Running Tests

```bash
# Run all contract tests
cd /home/ametel/source/fossil-monorepo/starknet-contracts
scarb test

# Run Fossil Hash Store tests specifically
cd /home/ametel/source/fossil-monorepo/starknet-contracts/fossil-hash-store
scarb test

# Run specific test
scarb test test_hash_avg_fees_and_store

# Run with verbose output
scarb test -- --exact test_hash_avg_fees_and_store --show-output
```

## Configuration

### Environment Variables

Mock contracts don't require special environment variables, but the contracts that use them do:

```bash
# Fossil Store address (can be mock or real contract)
FOSSIL_STORE_ADDRESS=0x00e581139553c8666f60b6646f277a336f99f108f8e5fa7cb300b6a6ce7c3b8c

# Hash storage contract address
HASH_STORAGE_ADDRESS=0x01929d8c867c2a261669ccb0e90cec2c81329164e581ad095edd0cce11cc617a

# StarkNet network configuration
STARKNET_RPC_URL=http://localhost:5050
NETWORK=DEVNET_KATANA
```

### Feature Flags

Control mock usage via environment feature flags:

```bash
# Enable mock pricing data (Fossil API)
USE_MOCK_PRICING_DATA=true

# Use simple mock for testing (Proving Service)
USE_SIMPLE_MOCK=false

# Enable proof generation (affects mock behavior)
ENABLE_PROOF=true
```

### Scarb Configuration

Mock contracts are configured in the Scarb workspace:

```toml
# /starknet-contracts/mocks/Scarb.toml
[package]
name = "mocks"
version = "0.1.0"
edition = "2024_07"

[dependencies]
starknet = "2.12.1"

[dev-dependencies]
snforge_std = "0.49.0"
assert_macros = "2.12.1"

[[target.starknet-contract]]
sierra = true

[scripts]
test = "snforge test"

[tool.scarb]
allow-prebuilt-plugins = ["snforge_std"]
```

## Deployment

### Local Development Deployment

Mock contracts are typically embedded in test files and don't require separate deployment. However, for standalone mock deployments:

#### Option 1: Inline Mock (Recommended)

Include mocks directly in the contract's `src/mock/` directory:

```
starknet-contracts/fossil-hash-store/
├── src/
│   ├── lib.cairo
│   ├── mock/
│   │   ├── fossil_store.cairo   # MockFossilStore
│   │   └── test_upgrade.cairo   # TestUpgrade
│   └── mock.cairo               # Module exports
└── tests/
    └── test_contract.cairo      # Tests using mocks
```

#### Option 2: Standalone Mock Package

For mocks used across multiple contracts, deploy from `/starknet-contracts/mocks/`:

```bash
cd /home/ametel/source/fossil-monorepo/starknet-contracts/mocks

# Build mock contracts
scarb build

# Deploy using starkli
starkli declare target/dev/mocks_MockFossilStore.contract_class.json \
    --account katana-0 \
    --rpc http://localhost:5050

starkli deploy <CLASS_HASH> \
    --account katana-0 \
    --rpc http://localhost:5050
```

### Docker Deployment

Mock contracts are automatically available when running the full development stack:

```bash
# Start complete development environment
make dev-up

# This starts:
# - Katana devnet (contract deployment target)
# - PostgreSQL databases
# - LocalStack (AWS mocks)
# - Deploys all contracts (including mocks if configured)
# - Starts application services
```

### Test Environment Deployment

For CI/CD testing, mocks are declared and deployed automatically by snforge:

```cairo
// snforge automatically declares and deploys mocks in tests
let contract = declare("MockFossilStore").unwrap().contract_class();
let (contract_address, _) = contract.deploy(@constructor_calldata).unwrap();
```

## Test Examples

### Example 1: Basic Hash Generation Test

```cairo
use sha2_input::{ISha2InputDispatcher, ISha2InputDispatcherTrait};
use snforge_std::{declare, ContractClassTrait, start_mock_call};

#[test]
fn test_basic_hash_generation() {
    // Setup
    let start_timestamp = 1714636800_u64;
    let test_fee = 565966358523639806057303238395575262125865566208;

    // Mock Fossil Store
    let fossil_store = 'FOSSIL_STORE'.try_into().unwrap();
    start_mock_call(fossil_store, selector!("get_avg_fee"), test_fee);

    // Deploy contract
    let owner = 'OWNER'.try_into().unwrap();
    let contract_class = declare("Sha2Input").unwrap().contract_class();
    let (contract_address, _) = contract_class
        .deploy(@array![owner.into(), fossil_store.into()])
        .unwrap();

    // Execute
    let dispatcher = ISha2InputDispatcher { contract_address };
    dispatcher.hash_avg_fees_and_store(start_timestamp);

    // Verify
    let hash = dispatcher.get_hash_stored_avg_fees(start_timestamp);
    assert(hash.len() == 8, 'Hash should be 8 u32 values');

    // Verify hash is not empty
    let mut is_empty = true;
    for i in 0..8 {
        if *hash.at(i) != 0 {
            is_empty = false;
            break;
        }
    }
    assert(!is_empty, 'Hash should not be empty');
}
```

### Example 2: Testing Access Control

```cairo
use openzeppelin_upgrades::interface::{IUpgradeableDispatcher, IUpgradeableDispatcherTrait};
use snforge_std::{declare, start_cheat_caller_address};

#[test]
fn test_owner_can_set_fossil_store() {
    let owner = 'OWNER'.try_into().unwrap();
    let fossil_store = 'FOSSIL_STORE'.try_into().unwrap();

    // Deploy contract
    let contract_class = declare("Sha2Input").unwrap().contract_class();
    let (contract_address, _) = contract_class
        .deploy(@array![owner.into(), fossil_store.into()])
        .unwrap();

    let dispatcher = ISha2InputDispatcher { contract_address };

    // Verify initial fossil store
    assert(dispatcher.get_fossil_store() == fossil_store, 'Initial store incorrect');

    // Set new fossil store as owner
    let new_store = 'NEW_STORE'.try_into().unwrap();
    start_cheat_caller_address(contract_address, owner);
    dispatcher.set_fossil_store(new_store);

    // Verify update
    assert(dispatcher.get_fossil_store() == new_store, 'Store not updated');
}

#[test]
#[should_panic(expected: 'Caller is not the owner')]
fn test_non_owner_cannot_set_fossil_store() {
    let owner = 'OWNER'.try_into().unwrap();
    let fossil_store = 'FOSSIL_STORE'.try_into().unwrap();
    let non_owner = 'NON_OWNER'.try_into().unwrap();

    // Deploy contract
    let contract_class = declare("Sha2Input").unwrap().contract_class();
    let (contract_address, _) = contract_class
        .deploy(@array![owner.into(), fossil_store.into()])
        .unwrap();

    let dispatcher = ISha2InputDispatcher { contract_address };

    // Try to set fossil store as non-owner (should panic)
    start_cheat_caller_address(contract_address, non_owner);
    dispatcher.set_fossil_store('NEW_STORE'.try_into().unwrap());
}
```

### Example 3: Testing Upgrade Functionality

```cairo
use openzeppelin_upgrades::interface::{IUpgradeableDispatcher, IUpgradeableDispatcherTrait};
use snforge_std::{declare, get_class_hash, start_cheat_caller_address};

#[test]
fn test_contract_upgrade() {
    // Setup
    let owner = 'OWNER'.try_into().unwrap();
    let fossil_store = 'FOSSIL_STORE'.try_into().unwrap();

    // Deploy original contract
    let original_class = declare("Sha2Input").unwrap().contract_class();
    let (contract_address, _) = original_class
        .deploy(@array![owner.into(), fossil_store.into()])
        .unwrap();

    let original_class_hash = *original_class.class_hash;

    // Verify original class hash
    assert(
        get_class_hash(contract_address) == original_class_hash,
        'Original class hash mismatch'
    );

    // Declare upgrade contract
    let upgrade_class = declare("TestUpgrade").unwrap().contract_class();
    let new_class_hash = *upgrade_class.class_hash;

    // Perform upgrade
    let upgrade_dispatcher = IUpgradeableDispatcher { contract_address };
    start_cheat_caller_address(contract_address, owner);
    upgrade_dispatcher.upgrade(new_class_hash);

    // Verify upgrade
    assert(
        get_class_hash(contract_address) == new_class_hash,
        'Upgrade did not succeed'
    );
}
```

### Example 4: Testing Batch Hash Generation

```cairo
use sha2_input::{ISha2InputDispatcher, ISha2InputDispatcherTrait};
use snforge_std::{declare, store, map_entry_address};

#[test]
fn test_batch_hash_generation() {
    // Setup
    let owner = 'OWNER'.try_into().unwrap();
    let fossil_store = 'FOSSIL_STORE'.try_into().unwrap();
    let start_timestamp = 1714636800_u64;

    // Deploy contract
    let contract_class = declare("Sha2Input").unwrap().contract_class();
    let (contract_address, _) = contract_class
        .deploy(@array![owner.into(), fossil_store.into()])
        .unwrap();

    let dispatcher = ISha2InputDispatcher { contract_address };

    // Populate Level 1 hashes
    let num_batches = 8_u64;  // Simplified test (production uses 32)
    for i in 0..num_batches {
        let timestamp = start_timestamp + (i * 3600_u64 * 180_u64);

        // Inject Level 1 hash into storage
        store(
            contract_address,
            map_entry_address(
                selector!("hash_stored_avg_fees"),
                array![timestamp.into()].span()
            ),
            array![
                1899064690, 2582884512, 4135155955, 798422494,
                171281208, 1706336902, 1452034954, 2201301487
            ].span(),
        );
    }

    // Generate Level 2 composite hash
    dispatcher.hash_batched_avg_fees(start_timestamp);

    // Verify composite hash exists
    let batch_hash = dispatcher.get_hash_stored_batched_avg_fees(start_timestamp);

    let mut is_empty = true;
    for i in 0..8 {
        if *batch_hash.at(i) != 0 {
            is_empty = false;
            break;
        }
    }
    assert(!is_empty, 'Batch hash should not be empty');
}
```

## Limitations

Mock contracts have important limitations compared to production implementations:

### Data Accuracy

- **Mock**: Returns pre-programmed or manually inserted test data
- **Production**: Indexes real blockchain data from multiple sources
- **Impact**: Mocks cannot validate data accuracy or historical correctness

### Performance Characteristics

- **Mock**: O(1) lookups from in-memory map
- **Production**: Database queries with indexing overhead
- **Impact**: Performance testing with mocks may not reflect production behavior

### Data Availability

- **Mock**: No automatic data population
- **Production**: Continuous indexing with ~12 hour lag
- **Impact**: Integration tests must manually populate all required data

### Edge Cases

- **Mock**: Simplified validation logic
- **Production**: Comprehensive validation (data sources, timestamps, gas limits)
- **Impact**: Some edge cases may not be caught in mock testing

### State Management

- **Mock**: Simple storage map
- **Production**: Optimized storage layout, batch processing, caching
- **Impact**: State management issues may not appear in mock testing

### External Dependencies

- **Mock**: No external dependencies
- **Production**: Requires fossil-light-client indexer, PostgreSQL, blockchain RPC
- **Impact**: Integration issues with external services not tested with mocks

### What Mocks Don't Test

Mocks do NOT validate:
- ❌ Real blockchain data indexing accuracy
- ❌ Network latency and RPC failures
- ❌ Gas costs and optimization
- ❌ Concurrent access patterns
- ❌ Database performance under load
- ❌ Indexing lag handling
- ❌ Data source failover and redundancy
- ❌ Historical data migration
- ❌ Long-running process stability

## Migration to Production

When transitioning from mock contracts to production:

### Step 1: Deploy Production Contracts

Replace mock Fossil Store with real Fossil Store contract:

```bash
# Sepolia testnet deployment
./scripts/deploy-starknet.sh sepolia

# Update environment with production addresses
# .env.sepolia
FOSSIL_STORE_ADDRESS=0x<PRODUCTION_FOSSIL_STORE_ADDRESS>
HASH_STORAGE_ADDRESS=0x<PRODUCTION_HASH_STORAGE_ADDRESS>
```

### Step 2: Update Service Configuration

Update Rust services to use production contract addresses:

```rust
// proving-service/.env.sepolia
STARKNET_RPC_URL=https://starknet-sepolia.infura.io/v3/<API_KEY>
FOSSIL_STORE_ADDRESS=0x<PRODUCTION_ADDRESS>
HASH_STORAGE_ADDRESS=0x<PRODUCTION_ADDRESS>
NETWORK=SEPOLIA
```

### Step 3: Verify Production Integration

Test production integration before full deployment:

```bash
# Run integration tests against Sepolia testnet
cd proving-service
cargo test --test integration_sepolia -- --nocapture

# Verify contract interactions
starkli call $FOSSIL_STORE_ADDRESS get_avg_fee 1714636800
```

### Step 4: Update Feature Flags

Disable mock-related features in production:

```bash
# .env.production
USE_MOCK_PRICING_DATA=false
USE_SIMPLE_MOCK=false
ENABLE_PROOF=true
VERIFY_PROOFS_ONCHAIN=true
```

### Step 5: Monitor Production Behavior

After migration, monitor for differences from mock behavior:

```bash
# Monitor contract events
starkli events $HASH_STORAGE_ADDRESS

# Check transaction success rates
# Review gas costs
# Validate data accuracy
```

### Migration Checklist

- [ ] Production contracts deployed and verified
- [ ] Environment variables updated with production addresses
- [ ] Feature flags updated (mocks disabled)
- [ ] Integration tests pass against production contracts
- [ ] Data availability verified (fossil-light-client running)
- [ ] Gas costs analyzed and acceptable
- [ ] Performance benchmarks meet requirements
- [ ] Error handling tested with real network conditions
- [ ] Monitoring and alerting configured
- [ ] Rollback plan documented

### Common Migration Issues

| Issue | Cause | Solution |
|-------|-------|----------|
| **Missing historical data** | Production Fossil Store not fully indexed | Wait for indexer to catch up or populate manually |
| **Transaction failures** | Gas estimation incorrect | Update gas limits based on production testing |
| **RPC timeouts** | Network latency vs mock instant response | Add retry logic and timeout handling |
| **Unexpected data formats** | Production data differs from mock test data | Update data parsing and validation |
| **Performance degradation** | Mock O(1) vs production database queries | Implement caching and optimize query patterns |

## Next Steps

### Related Documentation

- **[Fossil Hash Store Contract](fossil-hash-store.md)** - Production contract implementation
- **[StarkNet Contracts Architecture](../architecture/starknet-contracts.md)** - Complete contract ecosystem overview
- **[Testing Guide](../guides/testing.md)** - Comprehensive testing strategies
- **[Deployment Guide](../guides/deployment.md)** - Production deployment procedures

### Development Resources

- **Cairo Documentation**: https://book.cairo-lang.org/
- **Starknet Foundry**: https://foundry-rs.github.io/starknet-foundry/
- **snforge Testing**: https://foundry-rs.github.io/starknet-foundry/testing/testing.html
- **OpenZeppelin Cairo**: https://docs.openzeppelin.com/contracts-cairo/

### Advanced Topics

- **Custom Mock Development** - Creating new mocks for additional dependencies
- **Mock Data Generation** - Automating test data population
- **Performance Testing** - Bridging gap between mock and production performance
- **Continuous Integration** - Integrating mock tests into CI/CD pipelines

### Best Practices

1. **Always use mocks for unit tests** - Fast, deterministic, isolated
2. **Use production contracts for integration tests** - Validate real behavior
3. **Document mock limitations** - Make limitations explicit in test documentation
4. **Keep mocks synchronized** - Update mocks when production interfaces change
5. **Test migration path** - Regularly test against production on testnets
6. **Monitor divergence** - Track when mock behavior differs from production

---

## Summary

Mock contracts provide essential testing infrastructure for the Fossil monorepo:

- **MockFossilStore** enables testing of hash generation workflows without external dependencies
- **TestUpgrade** validates contract upgradeability functionality
- **snforge integration** provides powerful testing utilities (mock calls, storage manipulation)
- **Local development** uses mocks for fast iteration without full dependency stack
- **Migration path** is well-defined for transitioning to production contracts

Mock contracts are a critical tool for development and testing, but must be complemented with production integration testing to ensure system correctness and performance.
