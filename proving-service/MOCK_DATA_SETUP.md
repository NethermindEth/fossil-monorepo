# Mock Data Setup for End-to-End Testing

This document explains how to set up and use mock data for testing the proving service when onchain data is not yet available.

## Overview

The proving service requires exactly **5760 onchain fee values** (representing 8 months of hourly data) to generate proofs. When onchain data is unavailable, you can use mock data that generates realistic fee values matching the expected format.

## Setup Instructions

### 1. Environment Configuration

Add the following environment variable to enable mock data mode:

```bash
export USE_MOCK_STARKNET_DATA=true
```

You can add this to your `.env.local` file:

```env
# Mock Data Configuration
USE_MOCK_STARKNET_DATA=true

# Required but can be dummy values when using mock data
FOSSIL_STORE_ADDRESS=0x027a6e498bfad98a5145ac507f90fac6268cce6c0373c6eecd38de46be655b55
HASH_STORE_ADDRESS=0x027a6e498bfad98a5145ac507f90fac6268cce6c0373c6eecd38de46be655b55
STARKNET_RPC_URL=https://starknet-mainnet.public.blastapi.io
```

### 2. Mock Data Features

When `USE_MOCK_STARKNET_DATA=true` is set, the system will:

- Generate exactly **5760 mock fee values** 
- Each value follows the format similar to: `954945341986763873307577006398851026606748675`
- Values include realistic variation simulating daily/weekly patterns
- Mock verification hashes are generated deterministically
- All data is consistent across multiple runs

### 3. Data Characteristics

The mock data generator creates:

- **Base fee**: ~950,000,000,000,000,000,000,000,000,000,000,000,000,000 (similar to your example)
- **Variation**: ±50,000,000,000,000,000,000,000,000,000,000,000,000,000 with cyclical patterns
- **Trends**: Small linear increase over time to simulate market conditions
- **Format**: Hex-encoded strings starting with "0x"
- **Validation**: All values are valid Felt objects for RISC0 processing

## Usage Examples

### 3.1 Testing with Mock Data

```bash
# Set environment variable
export USE_MOCK_STARKNET_DATA=true

# Run the proving service tests
cd proving-service
make test

# Or test individual components
cd crates/starknet-handler
cargo run --bin mock-data-example --features binary
```

### 3.2 Programmatic Usage

```rust
use starknet_handler::{config::load_starknet_config, provider::StarknetProvider};

#[tokio::main]
async fn main() -> eyre::Result<()> {
    // Load config (will use mock data if USE_MOCK_STARKNET_DATA=true)
    let config = load_starknet_config()?;
    let provider = StarknetProvider::new(config)?;
    
    // These calls will return mock data when enabled
    let fee_data = provider.get_avg_fees_in_range(start_time, end_time).await?;
    let raw_fees = provider.get_raw_fees_in_range(start_time, end_time).await?;
    let hash = provider.get_verification_hash(start_time).await?;
    
    println!("Got {} fee values", fee_data.block_hashes.len()); // Should print 5760
    Ok(())
}
```

### 3.3 Running Integration Tests

Test the complete mock data system:

```bash
# Test mock data generation
cd proving-service/crates/starknet-handler
cargo test mock_data --lib

# Test mock data integration with StarknetProvider  
cargo test test_mock_data_integration --lib -- --nocapture

# Test all starknet-handler functionality
cargo test --lib
```

### 3.3 Proof Generation Testing

```bash
# Set mock data mode
export USE_MOCK_STARKNET_DATA=true

# Enable mock proof generation (if needed)
export USE_RISC0_INTEGRATION=true

# Run proof generation test
cd proving-service/crates/message-handler
cargo test test_proof_composition_with_mock_data
```

## Integration Points

### 4.1 StarknetProvider Methods

The following methods support mock data when enabled:

- `get_avg_fees_in_range()` - Returns 5760 mock fee values as hex strings
- `get_raw_fees_in_range()` - Returns 5760 mock fee values as Felt objects
- `get_verification_hash()` - Returns deterministic mock hash [u32; 8]
- `get_fees_with_verification()` - Combines raw fees and verification hash

### 4.2 Proof Composition Integration

The mock data integrates seamlessly with the existing proof composition system:

```rust
// This works with both real and mock data
let fee_data = provider.get_avg_fees_in_range(start, end).await?;

// Validates 5760 requirement (passes with mock data)
if fee_data.block_hashes.len() < 5760 {
    return Err(eyre!("Insufficient fee data"));
}

// Converts to Felts for RISC0 processing
let raw_input: Vec<String> = fee_data.block_hashes
    .iter()
    .take(5760)
    .cloned()
    .collect();
```

## Development Workflow

### 5.1 End-to-End Testing

1. **Setup**: Export `USE_MOCK_STARKNET_DATA=true`
2. **Run Tests**: Execute full test suite with mock data
3. **Validate**: Ensure 5760 values are generated and processed
4. **Integration**: Test proof composition with mock data
5. **Switch Back**: Remove env var to test real onchain integration

### 5.2 Debugging

When debugging, you can verify mock data is being used by checking logs:

```
INFO Using mock data for fee range request
INFO Generated mock average fees and block hashes num_hashes=5760
INFO Using mock data for raw fees request  
INFO Generated mock raw fee data for RISC0 processing num_fees=5760
INFO Using mock data for verification hash request
INFO Generated mock verification hash hash=[1234567890, ...]
```

## Production Considerations

### 6.1 Safety Guards

- Mock data mode requires explicit environment variable setting
- Logs clearly indicate when mock data is being used
- Mock data is never enabled by default
- Production deployments should never have `USE_MOCK_STARKNET_DATA=true`

### 6.2 Transition to Real Data

When onchain data becomes available:

1. Remove or set `USE_MOCK_STARKNET_DATA=false`
2. Ensure fossil_store contract has sufficient historical data (5760+ hourly records)
3. Verify hash_store contract is properly configured
4. Test with small timestamp ranges first
5. Gradually increase to full 8-month periods

## Troubleshooting

### Common Issues

1. **Mock data not being used**: Check environment variable is set correctly
2. **Wrong data count**: Mock generator always produces exactly 5760 values
3. **Invalid hex values**: Mock generator produces valid hex strings with "0x" prefix
4. **RISC0 conversion errors**: Mock Felt objects are properly formed for RISC0

### Verification Commands

```bash
# Check if mock data env var is set
echo $USE_MOCK_STARKNET_DATA

# Test mock data generation
cd proving-service/crates/starknet-handler  
cargo test test_mock_generator_creates_correct_amount_of_data

# Run mock example
cargo run --bin mock-data-example --features binary
```

This mock data system enables complete end-to-end testing of the proving service while waiting for onchain data availability.