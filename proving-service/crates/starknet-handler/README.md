# StarkNet Handler

A Rust crate for reading fees and block hashes from the StarkNet blockchain through the `fossil_store` contract.

## Features

- **Provider**: StarkNet RPC client with retry mechanisms and exponential backoff
- **Contract Integration**: Direct integration with `fossil_store` contract's `get_avg_fees_in_range` function
- **Configuration**: Environment-based configuration with sensible defaults
- **Error Handling**: Comprehensive error handling with detailed logging

## Usage

### Environment Variables

Set the following environment variables:

```bash
# Required
FOSSIL_STORE_ADDRESS=0x1234567890abcdef...  # Address of the fossil_store contract

# Optional (with defaults)
STARKNET_RPC=https://starknet-mainnet.public.blastapi.io  # StarkNet RPC endpoint
STARKNET_MAX_RETRIES=5                      # Maximum retry attempts
STARKNET_INITIAL_BACKOFF_MS=100            # Initial backoff in milliseconds
STARKNET_MAX_BACKOFF_MS=10000              # Maximum backoff in milliseconds
```

### Basic Usage

```rust
use starknet_handler::{config::load_starknet_config, provider::StarknetProvider};

#[tokio::main]
async fn main() -> eyre::Result<()> {
    // Load configuration from environment
    let config = load_starknet_config()?;
    
    // Create provider
    let provider = StarknetProvider::new(config)?;
    
    // Get fees for the last 24 hours
    let end_timestamp = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)?
        .as_secs();
    let start_timestamp = end_timestamp - (24 * 60 * 60);
    
    // Call the contract
    let fee_data = provider
        .get_avg_fees_in_range(start_timestamp, end_timestamp)
        .await?;
    
    println!("L1 Gas Fee: {}", fee_data.avg_l1_gas_fee);
    println!("L2 Gas Fee: {}", fee_data.avg_l2_gas_fee);
    println!("Block Hashes: {:?}", fee_data.block_hashes);
    
    Ok(())
}
```

### Running the Example

```bash
# Set environment variables
export FOSSIL_STORE_ADDRESS="0x1234567890abcdef..."

# Run the example binary
cargo run --bin starknet-handler-example
```

## Contract Interface

The crate integrates with the `fossil_store` contract's `get_avg_fees_in_range` function:

```cairo
fn get_avg_fees_in_range(
    self: @TContractState, 
    start_timestamp: u64, 
    end_timestamp: u64,
) -> (u64, u64, Array<felt252>);
```

**Returns:**
- `u64`: Average L1 gas fee
- `u64`: Average L2 gas fee  
- `Array<felt252>`: Array of block hashes in the time range

## Error Handling

The provider includes:
- **Retry Logic**: Exponential backoff for failed requests
- **Validation**: Input validation for timestamps and contract responses
- **Logging**: Comprehensive tracing for debugging

## Testing

### Unit Tests

Run unit tests (no external dependencies required):

```bash
cargo test --package starknet-handler --lib
```

### Integration Tests

Integration tests require a local StarkNet node running on `localhost:5050`.

#### Prerequisites for Integration Tests

1. **Start Local StarkNet Node:**
   ```bash
   # Using starknet-devnet (example)
   starknet-devnet --host 0.0.0.0 --port 5050
   ```

2. **Deploy fossil_store Contract:**
   The tests expect a contract with the `get_avg_fees_in_range` function deployed at the address specified in `FOSSIL_STORE_ADDRESS`.

3. **Set Environment Variables:**
   ```bash
   cp .env.test .env
   # Edit .env with your actual contract address
   ```

#### Running Integration Tests

```bash
# Test with provided timestamps (1755457200 to 1755464400)
cargo test --package starknet-handler -- --ignored

# Or use the test script
./test.sh
```

### Test Coverage

The test suite includes:

- **Unit Tests:**
  - Configuration creation and validation
  - Fee data structure operations
  - Provider initialization

- **Integration Tests:**
  - RPC connectivity to local node
  - Contract function calls with real timestamps
  - Error handling and retry mechanisms
  - Environment variable configuration

### Test Configuration

Key test parameters:
- **RPC URL:** `http://localhost:5050`
- **Test Timestamps:** 
  - Start: `1755457200` 
  - End: `1755464400`
- **Retry Config:** 3 retries, 100ms initial backoff, 1000ms max backoff

### Troubleshooting Tests

Common issues:

1. **"Connection refused"** - Local StarkNet node not running
2. **"Contract not found"** - Wrong contract address in `FOSSIL_STORE_ADDRESS`
3. **"Entry point not found"** - Contract missing `get_avg_fees_in_range` function

## Dependencies

- `starknet`: StarkNet Rust SDK for RPC communication
- `eyre`: Error handling
- `tracing`: Structured logging
- `tokio`: Async runtime