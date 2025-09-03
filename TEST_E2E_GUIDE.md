# End-to-End Test Guide: test-e2e.sh

This comprehensive guide explains how to use the `test-e2e.sh` script for complete end-to-end testing of the fossil monorepo services, including all prerequisites, running options, and troubleshooting.

## Overview

The `test-e2e.sh` script orchestrates a complete integration test by:

1. 🚀 Starting the **proving-service** HTTP API 
2. 🔧 Starting the **message-handler** background service with RISC0 integration
3. 📊 Starting the **offchain-processor** API
4. 🧪 Executing test scenarios including RISC0 proof generation
5. 📝 Monitoring job status and proof completion
6. 🔍 Analyzing logs and providing detailed results

## Prerequisites

### Required Docker Services

Before running the E2E test, you **must** start the integration services:

```bash
# Start all integration services
docker compose -f docker-compose.integration.yml up -d

# Verify services are running
docker ps
```

#### Required Containers

The script expects these Docker containers to be running:

| Container | Purpose | Health Check |
|-----------|---------|-------------|
| `offchain-processor-offchain_processor_db-1` | PostgreSQL database for offchain processor | `docker ps \| grep offchain_processor_db` |
| `sqs-localstack` | LocalStack SQS for message queuing | `docker ps \| grep sqs-localstack` |

### Optional Containers

Additional containers that may be useful:

| Container | Purpose | Command |
|-----------|---------|---------|
| `proving-service-db` | PostgreSQL for proving service | Part of integration compose |
| `indexer-db` | PostgreSQL for indexer data | Part of integration compose |

## Environment Configuration

### Available Environments

The script supports three environment modes:

```bash
./test-e2e.sh [environment]
```

| Environment | File | Purpose | Use Case |
|-------------|------|---------|----------|
| `local` (default) | `.env.local` | Local development | Development and testing with mock data |
| `docker` | `.env.docker` | Docker environment | Containerized testing |
| `sepolia` | `.env.sepolia` | Sepolia testnet | Real network testing |

### Environment File Requirements

Each environment requires a corresponding `.env.{environment}` file in the root directory:

#### .env.local (Recommended for Testing)
```env
# StarkNet Configuration
STARKNET_RPC_URL=http://localhost:5050
STARKNET_ACCOUNT_ADDRESS=0x127fd5f1fe78a71f8bcd1fec63e3fe2f0486b6ecd5c86a0466c3a21fa5cfcec
STARKNET_PRIVATE_KEY=0xc5b2fcab997346f3ea1c00b002ecf6f382c5f9c9659a3894eb783c5320f912

# Contract Addresses
PITCHLAKE_VERIFIER_CONTRACT=0x04d691258aa7311f00f6f42bf382b56b90a49edb3561cc798a9bafcf3e29a9d5
FOSSIL_STORE_ADDRESS=0x00e581139553c8666f60b6646f277a336f99f108f8e5fa7cb300b6a6ce7c3b8c
HASH_STORAGE_ADDRESS=0x01929d8c867c2a261669ccb0e90cec2c81329164e581ad095edd0cce11cc617a

# Database URLs
OFFCHAIN_PROCESSOR_DATABASE_URL=postgresql://postgres:postgres@localhost:5434/postgres
PROVING_SERVICE_DATABASE_URL=postgresql://postgres:postgres@localhost:5434/postgres
INDEXER_DATABASE_URL=postgresql://postgres:postgres@localhost:5433/postgres

# AWS/SQS Configuration (LocalStack)
AWS_REGION=us-east-1
AWS_ACCESS_KEY_ID=test
AWS_SECRET_ACCESS_KEY=test
AWS_ENDPOINT_URL=http://localhost:4567
SQS_QUEUE_URL=http://localhost:4567/000000000000/fossilQueue

# Bonsai Configuration
BONSAI_API_KEY="your_bonsai_api_key"
BONSAI_API_URL="https://api.bonsai.xyz/"

# Mock Data Configuration
USE_MOCK_STARKNET_DATA=true
SAVE_PROOF_COMPOSITION_INPUT=true

# Feature Flags
USE_MOCK_PRICING_DATA=true
VERIFY_PROOFS_ONCHAIN=true
```

## Running the E2E Test

### Basic Usage

```bash
# Run with default local environment
./test-e2e.sh

# Run with specific environment
./test-e2e.sh local
./test-e2e.sh docker
./test-e2e.sh sepolia
```

### Complete Setup and Run

```bash
# 1. Start integration services
docker compose -f docker-compose.integration.yml up -d

# 2. Wait for services to be ready (optional)
sleep 10

# 3. Run the E2E test
./test-e2e.sh local

# 4. Keep services running for inspection or stop them
# Press Enter when prompted, or Ctrl+C to stop immediately
```

## Test Flow and Phases

### Phase 1: Service Startup

The script starts three main services:

#### 1. Proving Service HTTP API (Port 3001)
```bash
# Command executed:
cargo run --bin proving-service --features mock-proof

# Purpose:
- HTTP API for job submission
- Health endpoint: http://localhost:3001/health
- Logs: proving-service.log
```

#### 2. Message Handler Background Service
```bash
# Command executed:
cargo run --bin message-handler --features mock-proof,starknet-handler

# Environment:
USE_SIMPLE_MOCK=false
ENABLE_PROOF=true 
VERIFY_PROOFS_ONCHAIN=true
USE_RISC0_INTEGRATION=true

# Purpose:
- SQS message processing
- RISC0 proof generation with Bonsai
- On-chain verification
- Logs: message-handler.log
```

#### 3. Offchain Processor API (Port 3000)
```bash
# Command executed:
cargo run --bin server

# Purpose:
- Job management and status tracking
- API key generation
- Client request processing
- Logs: offchain-processor.log
```

### Phase 2: Health Checks

The script performs comprehensive health checks:

```bash
# Proving Service Health Check
curl -s http://localhost:3001/health

# Offchain Processor Health Check  
curl -s http://localhost:3000/health

# Service readiness validation
# Retries: Up to 20 attempts for proving service
# Retries: Up to 10 attempts for offchain processor
```

### Phase 3: Test Execution

#### Basic Test Request
```json
{
  "identifiers": ["0x50495443485f4c414b455f5631"],
  "params": {
    "twap": [1672531200, 1672574400],
    "volatility": [1672531200, 1672574400], 
    "reserve_price": [1672531200, 1672574400]
  },
  "client_info": {
    "client_address": "0x018df581fe0ee497a4a3595cf62aea0bafa7ba1a54a7dcbafca37bfada67c718",
    "vault_address": "0x07b0110e7230a20881e57804d68e640777f4b55b487321556682e550f93fec7c",
    "timestamp": 1741243059
  }
}
```

#### RISC0 Proof Generation Test
```json
{
  "identifiers": ["RISC0_MOCK_PROOF_TEST"],
  "params": {
    "twap": [1672531200, 1672617600],
    "volatility": [1672531200, 1672617600],
    "reserve_price": [1672531200, 1672617600]
  },
  "client_info": {
    "client_address": "0x018df581fe0ee497a4a3595cf62aea0bafa7ba1a54a7dcbafca37bfada67c718",
    "vault_address": "0x07b0110e7230a20881e57804d68e640777f4b55b487321556682e550f93fec7c", 
    "timestamp": 1741243059
  }
}
```

### Phase 4: Monitoring and Results

The script provides detailed monitoring with different retry strategies:

#### Job Status Monitoring
- **Basic jobs**: 20 retries, 3-second intervals
- **RISC0 jobs**: 60 retries, backoff timing (5s → 30s max)
- **Real-time log analysis** for proof generation markers

#### Log Analysis
The script analyzes multiple log sources:

1. **Proof Calldata Generation**:
   ```bash
   grep -i "Generated.*proof calldata\|🔍.*calldata" message-handler.log
   ```

2. **Bonsai/RISC0 Activity**:
   ```bash
   grep -i "risc0\|proof.*generation\|bonsai\|groth16" message-handler.log
   ```

3. **On-chain Verification**:
   ```bash
   grep -i "starknet_handler.*successful\|tx_hash=" message-handler.log
   ```

4. **StarkNet Handler Activity** (saved to `starknet-handler.log`):
   ```bash
   grep -i "starknet_handler\|tx_hash=" message-handler.log
   ```

## Command Line Options and Features

### Service Control

The script automatically manages service lifecycles:

```bash
# Automatic cleanup on exit
trap cleanup EXIT

# Manual cleanup function
cleanup() {
    kill $PROVING_SERVICE_PID 2>/dev/null || true
    kill $MESSAGE_HANDLER_PID 2>/dev/null || true  
    kill $OFFCHAIN_PROCESSOR_PID 2>/dev/null || true
}
```

### Logging and Output

#### Color-coded Output
- 🟢 **Green**: Success messages and info
- 🟡 **Yellow**: Warnings
- 🔴 **Red**: Errors

#### Log Files Generated
| File | Content | Purpose |
|------|---------|---------|
| `proving-service.log` | HTTP API logs | Service requests and responses |
| `message-handler.log` | Background processing | Proof generation, SQS processing |
| `offchain-processor.log` | Job management | Database operations, API calls |
| `starknet-handler.log` | On-chain activity | Transaction hashes, verification results |

### Environment-Specific Configurations

#### Local Environment
```bash
# Override for local testing
export PROVING_SERVICE_URL="http://127.0.0.1:3001"
```

#### Docker Environment
```bash
# Docker-specific configurations
# (Add docker-specific overrides here)
```

#### Sepolia Environment  
```bash
# Sepolia-specific configurations
# (Add sepolia testnet overrides here)
```

## Docker Container Management

### Starting Integration Services

```bash
# Start all required services
docker compose -f docker-compose.integration.yml up -d

# Check service status
docker compose -f docker-compose.integration.yml ps

# View service logs
docker compose -f docker-compose.integration.yml logs -f [service_name]
```

### Individual Container Commands

#### PostgreSQL Database (Offchain Processor)
```bash
# Connect to database
docker exec -it offchain-processor-offchain_processor_db-1 psql -U postgres -d postgres

# View database logs
docker logs offchain-processor-offchain_processor_db-1 -f
```

#### LocalStack SQS
```bash
# Access LocalStack container
docker exec -it sqs-localstack bash

# List SQS queues
docker exec sqs-localstack awslocal sqs list-queues

# Create SQS queue (done automatically by script)
docker exec sqs-localstack awslocal sqs create-queue --queue-name fossilQueue

# Purge SQS queue
docker exec sqs-localstack awslocal sqs purge-queue --queue-url http://localhost:4567/000000000000/fossilQueue
```

### Container Health Checks

```bash
# Check if required containers are running
if ! docker ps | grep -q "offchain-processor-offchain_processor_db-1"; then
    echo "❌ Database not running"
    exit 1
fi

if ! docker ps | grep -q "sqs-localstack"; then
    echo "❌ SQS not running" 
    exit 1
fi
```

## Advanced Usage

### Custom Environment Variables

You can override specific variables for testing:

```bash
# Override environment variables for single run
RUST_LOG=trace BONSAI_API_KEY="test_key" ./test-e2e.sh local

# Test with different contract addresses
PITCHLAKE_VERIFIER_CONTRACT="0x123..." ./test-e2e.sh local

# Test with real StarkNet data
USE_MOCK_STARKNET_DATA=false ./test-e2e.sh local
```

### Debugging Options

#### Verbose Logging
```bash
# Set verbose logging
export RUST_LOG=debug
./test-e2e.sh local
```

#### Service-Specific Debugging
```bash
# Debug specific components
export RUST_LOG="message_handler=trace,proving_service=debug"
./test-e2e.sh local
```

#### Save Proof Input
```bash
# Save ProofCompositionInput for inspection
export SAVE_PROOF_COMPOSITION_INPUT=true
./test-e2e.sh local

# Inspect generated input
cat proving-service/proof_composition_input.json | jq '.'
```

## Expected Results and Success Criteria

### Successful Test Run Indicators

1. ✅ **Service Startup**:
   - All three services start without errors
   - Health checks pass
   - No immediate crashes

2. ✅ **API Key Generation**:
   ```
   Generated API key: [api_key_string]
   ```

3. ✅ **Job Creation**:
   ```
   Job created with ID: [job_id]
   ```

4. ✅ **Job Completion**:
   ```
   Job status: Completed
   Job completed successfully! ✓
   ```

5. ✅ **RISC0 Proof Generation**:
   ```
   ✅ RISC0 proof generation job completed successfully!
   ✅ Proof calldata successfully generated (Bonsai integration working)
   ✅ On-chain verification completed
   💎 Transaction Hash: 0x[hash]
   ```

6. ✅ **Mock Data Integration** (when enabled):
   ```
   📊 ProofCompositionInput prepared and ready for real proof method:
      • data_8_months: 5760 values (need 5760) ✓
   🔧 Using mock StarkNet data - ProofCompositionInput contains mock-derived data
   💾 Saving ProofCompositionInput to proof_composition_input.json
   ✅ ProofCompositionInput saved successfully
   ```

### Final Summary

The script provides a comprehensive summary:

```
=== RISC0 Integration Test Summary ===
✅ RISC0 mock proof generation test: PASSED
   - Job ID: [job_id]
   - Status: Completed
   - RISC0 proof pipeline successfully integrated
   - ✅ Proof calldata successfully generated (Bonsai integration working)
   - ✅ On-chain verification completed
   - 💎 Transaction Hash: 0x[hash]

End-to-end test completed! ✓
```

## Troubleshooting

### Common Issues

#### 1. Docker Services Not Running
```
❌ Offchain Processor DB not running. Please start with: 
   docker compose -f docker-compose.integration.yml up -d
```
**Solution**: Start integration services before running the test.

#### 2. Service Health Check Failures
```
❌ Proving-service health check endpoint not available after 20 attempts
```
**Solutions**:
- Check compilation errors in `proving-service.log`
- Verify environment variables are correctly loaded
- Ensure no port conflicts (ports 3000, 3001)

#### 3. Job Creation Failures
```
❌ Failed to generate API key
Response: {"error": "Database connection failed"}
```
**Solutions**:
- Verify database container is running
- Check database connection string in environment file
- Ensure database is initialized and accessible

#### 4. RISC0 Proof Generation Timeouts
```
⚠️ RISC0 proof generation monitoring timed out after 60 attempts
```
**Solutions**:
- Check Bonsai API key is valid
- Verify network connectivity to Bonsai API
- Review message-handler logs for specific errors
- Consider increasing timeout for complex proofs

#### 5. Mock Data Not Working
```
⚠️ No mock data detected, may be using real onchain data
```
**Solutions**:
- Verify `USE_MOCK_STARKNET_DATA=true` in environment file
- Check environment file is being loaded correctly
- Look for "Using mock data for fee range request" in logs

### Debug Commands

#### Check Service Status
```bash
# Check if processes are running
ps aux | grep -E "(proving-service|message-handler|server)"

# Check port usage
netstat -tlnp | grep -E ":300[01]"
lsof -i :3000
lsof -i :3001
```

#### Analyze Logs
```bash
# Real-time log monitoring
tail -f proving-service.log message-handler.log offchain-processor.log

# Search for errors
grep -i error *.log
grep -i panic *.log
grep -i failed *.log

# Search for successful operations
grep -i "completed successfully\|✅" *.log
```

#### Database Debugging
```bash
# Connect to offchain processor database
docker exec -it offchain-processor-offchain_processor_db-1 psql -U postgres -d postgres

# Check job status table
SELECT * FROM job_requests ORDER BY created_at DESC LIMIT 10;
SELECT * FROM api_keys ORDER BY created_at DESC LIMIT 5;
```

#### SQS Debugging
```bash
# Check SQS messages
docker exec sqs-localstack awslocal sqs receive-message --queue-url http://localhost:4567/000000000000/fossilQueue

# Check queue attributes
docker exec sqs-localstack awslocal sqs get-queue-attributes --queue-url http://localhost:4567/000000000000/fossilQueue --attribute-names All
```

### Environment Validation

Before running tests, validate your environment:

```bash
# Check required files exist
ls -la .env.local .env.docker .env.sepolia

# Check Docker services
docker compose -f docker-compose.integration.yml ps

# Check environment variables are loaded
source .env.local
echo $STARKNET_RPC_URL
echo $FOSSIL_STORE_ADDRESS  
echo $BONSAI_API_KEY
```

## Integration with CI/CD

### Automated Testing

The script can be integrated into CI/CD pipelines:

```yaml
# Example GitHub Actions workflow
- name: Setup Integration Services
  run: docker compose -f docker-compose.integration.yml up -d

- name: Wait for Services
  run: sleep 30

- name: Run E2E Tests
  run: ./test-e2e.sh local
  timeout-minutes: 30

- name: Upload Logs
  if: always()
  uses: actions/upload-artifact@v3
  with:
    name: e2e-logs
    path: "*.log"
```

### Performance Considerations

- **Compilation Time**: First run takes longer due to Rust compilation
- **Proof Generation**: RISC0 proofs can take 5-10 minutes with Bonsai
- **Network Dependencies**: Requires stable internet for Bonsai API calls
- **Resource Usage**: Services require ~2GB RAM and moderate CPU

This comprehensive guide covers all aspects of using the `test-e2e.sh` script for thorough integration testing of the fossil monorepo services.