# E2E Integration Guide: Mock Data + Real Proof Composition Structure

This guide explains how to run end-to-end tests with mock StarkNet data that produces **real ProofCompositionInput** structures ready for the actual proof method, while still using mock proof generation for testing.

## Overview

The hybrid approach allows you to:

1. ✅ **Use mock StarkNet data** (5760 fee values) when onchain data isn't available
2. ✅ **Generate real ProofCompositionInput** with proper structure and format  
3. ✅ **Validate data preparation** for the real proof method
4. ✅ **Still use mock proof generation** for testing without Bonsai costs
5. ✅ **Save input data** for inspection and validation

## Configuration

### Environment Variables

The following variables in `.env.local` control the hybrid mode:

```env
# Enable mock StarkNet data (5760 values, realistic fee structure)
USE_MOCK_STARKNET_DATA=true

# Save ProofCompositionInput to JSON file for inspection
SAVE_PROOF_COMPOSITION_INPUT=true

# Use RISC0 integration with mock proof generation
USE_RISC0_INTEGRATION=true

# Keep mock proof generation (instead of real Bonsai calls)
# This is controlled by the --features mock-proof flag in test-e2e.sh
```

### Key Components

1. **Mock Data Generator**: Produces 5760 realistic fee values
2. **Real Processing Pipeline**: All statistical calculations use real algorithms
3. **ProofCompositionInput**: Perfect replica of real proof method input
4. **Mock Proof Generation**: Avoids Bonsai costs while testing data flow

## Running E2E Tests

### Basic E2E Test with Mock Data

```bash
# Start integration services
docker compose -f docker-compose.integration.yml up -d

# Run E2E test with mock data
./test-e2e.sh local

# Check logs for ProofCompositionInput details
tail -f message-handler.log | grep "ProofCompositionInput"
```

### Expected Log Output

When running with mock data, you'll see detailed logging:

```
📊 ProofCompositionInput prepared and ready for real proof method:
   • data_8_months: 5760 values (need 5760) ✓
   • data_8_months_hash: [1234567, 2345678, ...]
   • timestamps: 1672531200 to 1693737600
   • positions: 720 values
   • Statistical data: pt=720, pt_1=720, season_param=720
   • Parameters: reserve_price=2.5, max_return=0.3, twap_result=1.25
   • Tolerances: gradient=0.05, floating_point=0.0001, reserve_price=0.01, twap=1.0
🔧 Using mock StarkNet data - ProofCompositionInput contains mock-derived data
💡 This input is ready to be sent to the real proof method at:
   /home/ametel/source/pitchlake-coprocessor/methods/proof-composition-twap-maxreturn-reserveprice-floating-hashing-methods
💾 Saving ProofCompositionInput to proof_composition_input.json
✅ ProofCompositionInput saved successfully
```

### Inspecting Generated Data

The ProofCompositionInput is saved to `proof_composition_input.json`:

```bash
# View the generated input structure
cd proving-service
cat proof_composition_input.json | jq '.data_8_months | length'  # Should show 5760
cat proof_composition_input.json | jq '.data_8_months_hash'       # Shows hash array
cat proof_composition_input.json | jq '.reserve_price'            # Shows calculated reserve price
```

## Data Flow Validation

### 1. Mock Data Generation
- **Input**: `USE_MOCK_STARKNET_DATA=true`
- **Process**: StarknetProvider generates 5760 realistic fee values
- **Output**: Mock fee data in exact onchain format

### 2. Real Processing Pipeline  
- **Input**: Mock fee data (5760 values)
- **Process**: All real statistical calculations (TWAP, seasonality removal, etc.)
- **Output**: Complete ProofCompositionInput structure

### 3. ProofCompositionInput Structure
The generated input matches exactly what the real proof method expects:

```rust
ProofCompositionInput {
    data_8_months: Vec<f64>,           // 5760 mock fee values as f64
    data_8_months_hash: [u32; 8],     // Hash of fee data
    start_timestamp: i64,             // Time range start
    end_timestamp: i64,               // Time range end
    positions: Vec<f64>,              // Calculated positions (720 values)
    pt: DVector<f64>,                 // Statistical calculations
    pt_1: DVector<f64>,               // Statistical calculations  
    // ... all other required fields
}
```

### 4. Proof Generation
- **Current**: Mock proof generation (fast, no cost)
- **Ready For**: Real proof method with prepared data
- **Switch**: Remove `--features mock-proof` when ready for real proofs

## Integration with Real Proof Method

### Data is Ready For

The generated ProofCompositionInput can be directly used with:
```
/home/ametel/source/pitchlake-coprocessor/methods/proof-composition-twap-maxreturn-reserveprice-floating-hashing-methods
```

### Manual Testing

You can test the real proof method with the generated data:

```bash
# Generate ProofCompositionInput with mock data
./test-e2e.sh local

# Copy the generated input to the proof method directory
cp proving-service/proof_composition_input.json /path/to/real/proof/method/

# Test the real proof method (when ready)
# cd /home/ametel/source/pitchlake-coprocessor
# cargo run --bin proof-composition-test < proof_composition_input.json
```

## Transition to Real Data

When onchain data becomes available:

### 1. Disable Mock Data
```env
# In .env.local
USE_MOCK_STARKNET_DATA=false
```

### 2. Ensure Onchain Data Availability
- Fossil store contract has 5760+ hourly fee records
- Hash store contract is properly configured
- StarkNet RPC connectivity is working

### 3. Switch to Real Proof Generation
```bash
# Remove mock-proof feature to use real Bonsai calls
# Edit test-e2e.sh line 150:
# Change: --features mock-proof,starknet-handler
# To:     --features starknet-handler
```

## Verification Steps

### Mock Data Validation
1. **Count**: Exactly 5760 fee values ✓
2. **Format**: Valid hex strings convertible to Felt ✓  
3. **Range**: Realistic fee value magnitudes ✓
4. **Hash**: Valid cryptographic hash of data ✓

### Processing Pipeline Validation
1. **Statistical Calculations**: All real algorithms ✓
2. **Data Structure**: Matches ProofCompositionInput exactly ✓
3. **Field Types**: Correct types (Vec<f64>, DVector<f64>, etc.) ✓
4. **Completeness**: All required fields populated ✓

### Integration Readiness
1. **Serialization**: ProofCompositionInput serializes correctly ✓
2. **File Output**: Input saved to JSON for inspection ✓
3. **Real Method**: Structure matches real proof method interface ✓
4. **Mock Proof**: Test flow works end-to-end ✓

## Troubleshooting

### Common Issues

**Mock data not being used:**
- Check `USE_MOCK_STARKNET_DATA=true` in environment
- Look for "Using mock data for fee range request" in logs

**ProofCompositionInput not saved:**  
- Check `SAVE_PROOF_COMPOSITION_INPUT=true` in environment
- Look for "💾 Saving ProofCompositionInput" in logs

**Wrong data count:**
- Mock generator always produces exactly 5760 values
- If seeing different count, check if mock mode is actually enabled

**E2E test failures:**
- Ensure integration services are running
- Check that both mock data AND mock proof features are enabled
- Verify environment variables are loaded correctly

## Summary

This hybrid approach gives you:

✅ **End-to-end testing** with complete data flow validation  
✅ **Real proof input preparation** without requiring onchain data  
✅ **Cost-effective testing** using mock proof generation  
✅ **Seamless transition** to real proofs when data is available  
✅ **Comprehensive logging** for debugging and validation  

The ProofCompositionInput generated with mock data is **identical in structure** to what you'll use with real onchain data, ensuring your integration is fully tested and ready.