# Debugging Guide

This guide provides comprehensive debugging techniques, common issues, and solutions for the Fossil monorepo. Whether you're troubleshooting development issues or investigating production problems, this guide will help you diagnose and resolve issues efficiently.

## Table of Contents
- [Overview](#overview)
- [Logging and Tracing](#logging-and-tracing)
- [Common Issues](#common-issues)
- [Debugging Techniques](#debugging-techniques)
- [Service-Specific Debugging](#service-specific-debugging)
- [Database Debugging](#database-debugging)
- [Network Debugging](#network-debugging)
- [Proof Generation Issues](#proof-generation-issues)
- [StarkNet Issues](#starknet-issues)
- [Performance Profiling](#performance-profiling)
- [Useful Commands](#useful-commands)

## Overview

### Debugging Philosophy

The Fossil monorepo uses structured logging and tracing to provide visibility into system behavior. When debugging:

1. **Start broad, narrow down**: Check service health → review logs → identify the failing component
2. **Follow the data flow**: Client → Fossil API → Proving Service → Message Handler → StarkNet
3. **Reproduce consistently**: Understand the exact conditions that trigger the issue
4. **Isolate components**: Test each service independently when possible
5. **Check the basics first**: Network connectivity, database access, environment configuration

### Available Tools

| Tool | Purpose | Use Case |
|------|---------|----------|
| `tracing` | Structured logging | Production and development logging |
| `docker logs` | Container output | Service logs in Docker environment |
| `psql` | Database queries | Direct database inspection |
| `curl` | HTTP testing | API endpoint testing |
| `aws` CLI | Queue inspection | SQS message debugging (LocalStack) |
| `starkli` | StarkNet CLI | Contract interaction and debugging |
| `cargo test` | Unit/integration tests | Isolated component testing |

## Logging and Tracing

### Log Levels

The project uses the `tracing` crate with standard log levels:

```rust
use tracing::{trace, debug, info, warn, error};

trace!("Very detailed information, usually disabled");
debug!("Detailed diagnostic information");
info!("General informational messages");
warn!("Warning messages for potentially problematic situations");
error!("Error messages for failures");
```

### Configuring Log Levels

**Environment Variable:**

```bash
# Set global log level
export RUST_LOG=info

# Set per-module log levels
export RUST_LOG=info,server=debug,message_handler=debug

# Show all logs (very verbose)
export RUST_LOG=trace

# Show only errors
export RUST_LOG=error

# Complex filtering
export RUST_LOG=info,proving_service=debug,message_handler=trace,sqlx=error
```

**Common Configurations:**

```bash
# Development (recommended)
RUST_LOG=info,server=debug,message_handler=debug,sqlx=warn

# Debugging specific service
RUST_LOG=debug,sqlx=warn  # See debug logs, hide SQL queries

# Production (recommended)
RUST_LOG=info,sqlx=error

# Testing with verbose output
RUST_LOG=debug cargo test -- --nocapture
```

### Structured Logging Examples

**Basic Logging:**

```rust
// Information
info!("Received pricing data request");

// With context
info!(
    "Generated job_id: {}. program_id={}, vault_address={}",
    job_id, program_id, vault_address
);

// Warnings
warn!("Invalid request: {:?}", error);

// Errors
error!("Database error: {}. job_id={}", error, job_id);
```

**Contextual Logging:**

```rust
// Build context string for consistent logging
let context = format!(
    "job_id={}, vault_address={}, timestamp={}",
    job_id, vault_address, timestamp
);

info!("Processing job. {}", context);
debug!("Fetching fee data. {}", context);
error!("Proof generation failed: {}. {}", error, context);
```

**Debugging with Emojis (development only):**

```rust
info!("🔍 Raw message received from queue: {}", message.body);
info!("✅ Job completed successfully: {}", job_id);
info!("❌ Proof generation failed: {}", error);
info!("📤 Submitting pricing data request...");
```

### Log Output Samples

**Successful Job Processing:**

```
[2024-01-15T10:30:00Z INFO  message_handler] Starting to poll for messages from queue
[2024-01-15T10:30:05Z INFO  message_handler] Received 1 messages from queue
[2024-01-15T10:30:05Z INFO  message_handler] 🔍 Raw message received from queue: {"RequestProof":{"job_id":"job_abc123",...}}
[2024-01-15T10:30:05Z INFO  message_handler] 🔍 Received RequestProof job - vault_address: Some("0x004018ae..."), vault_timestamp: Some(1672531200)
[2024-01-15T10:30:05Z DEBUG message_handler] Fetching fee data from StarkNet Fossil Store
[2024-01-15T10:30:10Z INFO  message_handler] Generating proof with 1440 fee data points
[2024-01-15T10:45:00Z INFO  starknet_handler] Proof verified onchain: 0x05a7f8b...
[2024-01-15T10:45:00Z INFO  message_handler] ✅ Job completed successfully: job_abc123
```

**Failed Job Processing:**

```
[2024-01-15T10:30:00Z INFO  message_handler] Starting to poll for messages from queue
[2024-01-15T10:30:05Z INFO  message_handler] Received 1 messages from queue
[2024-01-15T10:30:05Z WARN  message_handler] Error parsing job: EOF while parsing. Message body: {invalid json}
[2024-01-15T10:30:05Z INFO  message_handler] Deleted invalid message from queue
```

**Database Connection Error:**

```
[2024-01-15T10:30:00Z ERROR server] Database error: connection refused. job_id=job_abc123
[2024-01-15T10:30:00Z ERROR server] Failed to connect to database at postgresql://postgres:postgres@localhost:5434/postgres
```

## Common Issues

### 1. Database Connection Refused

**Symptoms:**

```
Error: connection refused
Error: FATAL: password authentication failed
```

**Diagnosis:**

```bash
# Check if database containers are running
docker ps | grep postgres

# Test connection directly
psql postgresql://postgres:postgres@localhost:5434/postgres

# Check Docker network
docker network inspect fossil-monorepo_default

# Check database logs
docker logs fossil-monorepo-fossil_api_db-1
```

**Solutions:**

```bash
# Restart database containers
docker restart fossil-monorepo-fossil_api_db-1
docker restart fossil-monorepo-proving_service_db-1

# Check environment variables
echo $OFFCHAIN_PROCESSOR_DATABASE_URL
echo $PROVING_SERVICE_DATABASE_URL

# Verify .env.local vs .env.docker configuration
# .env.local should use localhost:5434, localhost:5435
# .env.docker should use fossil_api_db:5432, proving_service_db:5432

# Reset databases completely
make dev-down
docker volume rm fossil-monorepo_fossil_api_db_data
docker volume rm fossil-monorepo_proving_service_db_data
make dev-up
```

### 2. Contract Addresses Not Set

**Symptoms:**

```
Error: Invalid contract address: ""
Environment variable PITCHLAKE_VAULT is empty
```

**Diagnosis:**

```bash
# Check if contract addresses are populated
grep PITCHLAKE_VERIFIER_CONTRACT .env.local
grep PITCHLAKE_VAULT_12MIN .env.local

# Check deployment logs
docker logs fossil-monorepo-contract-deployer
```

**Solutions:**

```bash
# Redeploy contracts
make dev-down
make dev-up

# Manually trigger contract deployment
docker compose -f docker-compose.deploy.yml run --rm contract-deployer

# Verify .env.local was updated
cat .env.local | grep PITCHLAKE
```

### 3. SQS Queue Not Found

**Symptoms:**

```
Error: The specified queue does not exist
QueueDoesNotExist: AWS.SimpleQueueService.NonExistentQueue
```

**Diagnosis:**

```bash
# Check LocalStack is running
docker logs fossil-monorepo-localstack-1

# List existing queues
aws --endpoint-url=http://localhost:4567 sqs list-queues

# Verify queue URL
echo $SQS_QUEUE_URL
```

**Solutions:**

```bash
# Restart LocalStack
docker restart fossil-monorepo-localstack-1

# Recreate queue manually
aws --endpoint-url=http://localhost:4567 sqs create-queue --queue-name fossilQueue

# Check queue attributes
aws --endpoint-url=http://localhost:4567 sqs get-queue-attributes \
  --queue-url http://localhost:4567/000000000000/fossilQueue \
  --attribute-names All
```

### 4. Katana Not Responding

**Symptoms:**

```
Error: node not reachable
Connection timeout
RPC error: -32603
```

**Diagnosis:**

```bash
# Check Katana is running
docker logs fossil-monorepo-katana-1

# Test RPC connection
curl -X POST http://localhost:5050 \
  -H "Content-Type: application/json" \
  -d '{"jsonrpc":"2.0","method":"starknet_chainId","params":[],"id":1}'

# Check Katana health
docker ps | grep katana
```

**Solutions:**

```bash
# Restart Katana
docker restart fossil-monorepo-katana-1

# Check Katana logs for errors
docker logs fossil-monorepo-katana-1 --tail 100

# Redeploy full stack
make dev-down && make dev-up

# Test with starkli
starkli chain-id --rpc http://localhost:5050
```

### 5. Bonsai API Errors

**Symptoms:**

```
Error: 401 Unauthorized
Error: Bonsai API key invalid
Error: Rate limit exceeded
```

**Diagnosis:**

```bash
# Verify API key is set
echo $BONSAI_API_KEY

# Check if key is in correct format
# Should be non-empty string

# Check message handler logs
docker logs fossil-monorepo-message-handler-1 | grep -i bonsai
```

**Solutions:**

```bash
# Set API key in environment
export BONSAI_API_KEY="your_api_key_here"

# Update .env.local
echo 'BONSAI_API_KEY="your_api_key_here"' >> .env.local

# Update .env.docker
echo 'BONSAI_API_KEY="your_api_key_here"' >> .env.docker

# Regenerate API key from Bonsai dashboard at https://bonsai.xyz

# Use mock proofs for development (no API key needed)
export ENABLE_PROOF=false
# or
export USE_SIMPLE_MOCK=true
```

### 6. Port Conflicts

**Symptoms:**

```
Error: Address already in use
Error: port 3000 is already allocated
```

**Diagnosis:**

```bash
# Find process using port
lsof -i :3000
lsof -i :3001
lsof -i :5050
lsof -i :5434

# Check all Fossil-related ports
for port in 3000 3001 5050 5433 5434 5435 4567; do
  echo "Port $port:"; lsof -i :$port
done
```

**Solutions:**

```bash
# Kill process using port
kill -9 $(lsof -t -i:3000)

# Stop all Docker containers
make dev-down

# Change port in configuration (if needed)
# Update docker-compose.local.yml port mappings

# Restart services
make dev-up
```

### 7. Proof Generation Timeout

**Symptoms:**

```
Error: Proof generation timed out after 1 hour
Task timeout expired
```

**Diagnosis:**

```bash
# Check message handler logs
docker logs fossil-monorepo-message-handler-1 | grep -i timeout

# Check Bonsai API status
# (no direct API for this, check logs)

# Verify proof generation is enabled
echo $ENABLE_PROOF
echo $USE_RISC0_INTEGRATION
```

**Solutions:**

```bash
# Increase timeout (in message handler configuration)
# Default: 3600 seconds (1 hour)
# Edit proving-service/crates/message-handler/src/main.rs
# Change: Duration::from_secs(3600)

# Use mock proofs for development
export USE_SIMPLE_MOCK=true

# Check system resources
docker stats

# Reduce concurrent proofs
export MAX_CONCURRENT_PROOFS=1
```

### 8. Timestamp Validation Errors

**Symptoms:**

```
Error: Invalid timestamp range
Error: End timestamp must be after start timestamp
Error: Insufficient onchain fee data
```

**Diagnosis:**

```bash
# Check current block timestamp
starkli block-number --rpc http://localhost:5050

# Check vault contract timestamp
# Use test-local-request.sh script which handles this automatically

# Verify timestamp ranges in request
curl http://localhost:3000/pricing_data \
  -H "Content-Type: application/json" \
  -H "X-API-Key: $API_KEY" \
  -d '{"params":{"twap":[1672531200,1672617600],...}}'
```

**Solutions:**

```bash
# Use automated test script (recommended)
./test-local-request.sh

# Calculate valid timestamp manually
# 1. Get current block timestamp
# 2. Subtract required data period (e.g., 60 days)
# 3. Ensure timestamps are on hour boundaries

# Example with date commands
END_TS=$(date -u +%s)
START_TS=$((END_TS - 60*24*3600))  # 60 days ago
echo "Start: $START_TS, End: $END_TS"
```

## Debugging Techniques

### 1. End-to-End Request Tracing

**Workflow:**

```bash
# Step 1: Enable debug logging
export RUST_LOG=debug

# Step 2: Generate API key
API_KEY=$(curl -s -X POST "http://localhost:3000/api_key" \
  -H "Content-Type: application/json" \
  -d '{"name":"debug_key"}' | jq -r '.api_key')
echo "API Key: $API_KEY"

# Step 3: Submit job with verbose logging
curl -v -X POST "http://localhost:3000/pricing_data" \
  -H "Content-Type: application/json" \
  -H "X-API-Key: $API_KEY" \
  -d '{
    "program_id": "RISC0_MOCK_PROOF_TEST",
    "vault_address": "0x004018ae0157b10d08cb1f70d34e32fd8b25e4ad1d70afc89616c3b300257fd9",
    "params": {
      "twap": [1672531200, 1672617600],
      "max_return": [1672531200, 1672617600],
      "reserve_price": [1672531200, 1672617600]
    }
  }' | jq

# Step 4: Track job through each service
JOB_ID="job_abc123"  # From response

# Watch Fossil API logs
docker logs -f fossil-monorepo-fossil-api-1 | grep $JOB_ID

# Watch Proving Service API logs
docker logs -f fossil-monorepo-proving-service-api-1 | grep $JOB_ID

# Watch Message Handler logs
docker logs -f fossil-monorepo-message-handler-1 | grep $JOB_ID

# Step 5: Check job status
watch -n 5 "curl -s http://localhost:3000/job_status/$JOB_ID | jq"

# Step 6: Verify in database
psql postgresql://postgres:postgres@localhost:5434/postgres \
  -c "SELECT * FROM job_requests WHERE job_id = '$JOB_ID';"
```

### 2. Isolating Component Failures

**Test Each Service Independently:**

```bash
# Test 1: Fossil API health
curl http://localhost:3000/health
# Expected: {"status":"healthy","service":"fossil-api"}

# Test 2: Proving Service health
curl http://localhost:3001/health
# Expected: {"status":"healthy","service":"proving-service"}

# Test 3: Database connectivity
psql postgresql://postgres:postgres@localhost:5434/postgres -c "SELECT 1;"
# Expected: 1

# Test 4: StarkNet RPC
curl -X POST http://localhost:5050 \
  -H "Content-Type: application/json" \
  -d '{"jsonrpc":"2.0","method":"starknet_chainId","params":[],"id":1}'
# Expected: {"jsonrpc":"2.0","result":"0x4b4154414e41","id":1}

# Test 5: LocalStack SQS
aws --endpoint-url=http://localhost:4567 sqs list-queues
# Expected: Queue URLs

# Test 6: StarkNet contract
starkli call \
  --rpc http://localhost:5050 \
  $FOSSIL_STORE_ADDRESS \
  get_avg_fees_in_range 1672531200 1672617600
```

### 3. Binary Search Debugging

**When System Works Sometimes:**

```bash
# Narrow down the failure point systematically

# 1. Does the request reach the API?
docker logs fossil-monorepo-fossil-api-1 | grep "Received pricing data request"

# 2. Is the job created in database?
psql postgresql://postgres:postgres@localhost:5434/postgres \
  -c "SELECT job_id, status, created_at FROM job_requests ORDER BY created_at DESC LIMIT 10;"

# 3. Is the job dispatched to SQS?
aws --endpoint-url=http://localhost:4567 sqs receive-message \
  --queue-url http://localhost:4567/000000000000/fossilQueue \
  --max-number-of-messages 1

# 4. Does message handler pick up the job?
docker logs fossil-monorepo-message-handler-1 | grep "Received RequestProof job"

# 5. Does proof generation start?
docker logs fossil-monorepo-message-handler-1 | grep "Generating proof"

# 6. Does proof generation complete?
docker logs fossil-monorepo-message-handler-1 | grep "Job completed successfully"

# 7. Is proof verified onchain?
docker logs fossil-monorepo-message-handler-1 | grep "Proof verified onchain"
```

### 4. Bisecting Configuration Issues

**When Configuration Seems Wrong:**

```bash
# Print all environment variables
docker exec fossil-monorepo-fossil-api-1 env | grep -E "(DATABASE|STARKNET|PROVING|AWS)"

# Compare .env.local vs .env.docker
diff .env.local .env.docker

# Verify environment is loaded correctly
docker exec fossil-monorepo-fossil-api-1 printenv STARKNET_RPC_URL
docker exec fossil-monorepo-proving-service-api-1 printenv SQS_QUEUE_URL

# Check which env file Docker Compose is using
docker compose -f docker-compose.local.yml config | grep -A 5 "environment:"
```

### 5. Reproduction with Minimal Example

**Create Minimal Reproducible Case:**

```bash
# Minimal API call test
cat > test_minimal.sh << 'EOF'
#!/bin/bash
set -e

# Health check
echo "1. Testing health endpoint..."
curl -f http://localhost:3000/health

# Generate API key
echo "2. Generating API key..."
API_KEY=$(curl -s -X POST "http://localhost:3000/api_key" \
  -H "Content-Type: application/json" \
  -d '{"name":"test"}' | jq -r '.api_key')

# Submit minimal job
echo "3. Submitting job..."
JOB_ID=$(curl -s -X POST "http://localhost:3000/pricing_data" \
  -H "Content-Type: application/json" \
  -H "X-API-Key: $API_KEY" \
  -d '{
    "program_id": "TEST",
    "vault_address": "'$PITCHLAKE_VAULT_12MIN'",
    "params": {
      "twap": [1672531200, 1672617600],
      "max_return": [1672531200, 1672617600],
      "reserve_price": [1672531200, 1672617600]
    }
  }' | jq -r '.job_id')

echo "Job ID: $JOB_ID"

# Check status
echo "4. Checking status..."
curl -s "http://localhost:3000/job_status/$JOB_ID" | jq
EOF

chmod +x test_minimal.sh
./test_minimal.sh
```

## Service-Specific Debugging

### Fossil API Debugging

**Common Issues:**

1. **API Key Validation Failures**

```bash
# Check API key in database
psql postgresql://postgres:postgres@localhost:5434/postgres << EOF
SELECT api_key, name, created_at, is_active
FROM api_keys
ORDER BY created_at DESC
LIMIT 10;
EOF

# Test with valid API key
curl -X POST "http://localhost:3000/pricing_data" \
  -H "X-API-Key: sk_test_valid_key" \
  -d '{...}'

# Check auth middleware logs
docker logs fossil-monorepo-fossil-api-1 | grep -i "auth"
```

2. **Job Creation Failures**

```bash
# Enable detailed SQL logging
export RUST_LOG=debug,sqlx=debug

# Check job_requests table schema
psql postgresql://postgres:postgres@localhost:5434/postgres << EOF
\d job_requests
EOF

# Manually insert test job
psql postgresql://postgres:postgres@localhost:5434/postgres << EOF
INSERT INTO job_requests (job_id, program_id, vault_address, status, created_at)
VALUES ('test_job_123', 'TEST', '0x123', 'pending', NOW());
EOF
```

3. **Proving Service Communication**

```bash
# Test direct communication to Proving Service
curl -X POST "http://localhost:3001/api/job" \
  -H "Content-Type: application/json" \
  -d '{
    "job_group_id": "test_123",
    "twap": {"start_timestamp": 1672531200, "end_timestamp": 1672617600},
    "reserve_price": {"start_timestamp": 1672531200, "end_timestamp": 1672617600},
    "max_return": {"start_timestamp": 1672531200, "end_timestamp": 1672617600},
    "vault_address": "'$PITCHLAKE_VAULT_12MIN'",
    "vault_timestamp": 1672531200
  }'

# Check network connectivity between containers
docker exec fossil-monorepo-fossil-api-1 ping -c 3 proving-service-api
docker exec fossil-monorepo-fossil-api-1 curl http://proving-service-api:3001/health
```

**Fossil API Debug Mode:**

```bash
# Run with maximum verbosity
cd fossil-api
RUST_LOG=trace cargo run --bin server 2>&1 | tee fossil-api.log

# Filter for specific request
cat fossil-api.log | grep "job_abc123"
```

### Proving Service Debugging

**Common Issues:**

1. **Job Dispatcher Failures**

```bash
# Check SQS message format
aws --endpoint-url=http://localhost:4567 sqs receive-message \
  --queue-url http://localhost:4567/000000000000/fossilQueue \
  --max-number-of-messages 1 \
  --wait-time-seconds 5 | jq

# Manually send test message
aws --endpoint-url=http://localhost:4567 sqs send-message \
  --queue-url http://localhost:4567/000000000000/fossilQueue \
  --message-body '{
    "RequestProof": {
      "job_id": "manual_test_123",
      "job_group_id": "manual_test_123",
      "start_timestamp": 1672531200,
      "end_timestamp": 1672617600,
      "vault_address": "'$PITCHLAKE_VAULT_12MIN'",
      "vault_timestamp": 1672531200
    }
  }'

# Check queue attributes
aws --endpoint-url=http://localhost:4567 sqs get-queue-attributes \
  --queue-url http://localhost:4567/000000000000/fossilQueue \
  --attribute-names All | jq
```

2. **API Endpoint Issues**

```bash
# Test health endpoint
curl -v http://localhost:3001/health

# Test job submission with verbose output
curl -v -X POST "http://localhost:3001/api/job" \
  -H "Content-Type: application/json" \
  -d '{...}' 2>&1 | tee proving-service-request.log

# Check router configuration
docker logs fossil-monorepo-proving-service-api-1 | grep -i "route"
```

**Proving Service Debug Mode:**

```bash
# Run with debug logging
cd proving-service/crates/proving-service
RUST_LOG=debug cargo run 2>&1 | tee proving-service-api.log
```

### Message Handler Debugging

**Common Issues:**

1. **Queue Polling Problems**

```bash
# Check long polling configuration
docker logs fossil-monorepo-message-handler-1 | grep "poll"

# Verify queue visibility timeout
aws --endpoint-url=http://localhost:4567 sqs get-queue-attributes \
  --queue-url http://localhost:4567/000000000000/fossilQueue \
  --attribute-names VisibilityTimeout | jq

# Monitor queue depth
watch -n 5 "aws --endpoint-url=http://localhost:4567 sqs get-queue-attributes \
  --queue-url http://localhost:4567/000000000000/fossilQueue \
  --attribute-names ApproximateNumberOfMessages | jq '.Attributes.ApproximateNumberOfMessages'"
```

2. **Proof Generation Failures**

```bash
# Check proof provider configuration
docker logs fossil-monorepo-message-handler-1 | grep -i "proof provider"

# Verify feature flags
docker exec fossil-monorepo-message-handler-1 printenv | grep -E "(ENABLE_PROOF|USE_RISC0|USE_SIMPLE_MOCK)"

# Check Bonsai API connectivity
docker logs fossil-monorepo-message-handler-1 | grep -i bonsai

# Monitor proof generation progress
docker logs -f fossil-monorepo-message-handler-1 | grep -E "(Generating proof|completed successfully|failed)"
```

3. **Job Failure Tracking**

```bash
# Check failure count
docker logs fossil-monorepo-message-handler-1 | grep -i "failure"

# Inspect job processing state
# (requires code instrumentation or database queries)

# Check SQS message re-delivery
aws --endpoint-url=http://localhost:4567 sqs get-queue-attributes \
  --queue-url http://localhost:4567/000000000000/fossilQueue \
  --attribute-names ApproximateNumberOfMessagesNotVisible | jq
```

**Message Handler Debug Mode:**

```bash
# Run with trace logging
cd proving-service/crates/message-handler
RUST_LOG=trace,sqlx=debug cargo run --features mock-proof 2>&1 | tee message-handler.log

# Filter for specific job
tail -f message-handler.log | grep "job_abc123"
```

## Database Debugging

### Connection Issues

**Diagnosis:**

```bash
# Test connection
psql postgresql://postgres:postgres@localhost:5434/postgres -c "SELECT NOW();"

# Check database is accepting connections
docker exec fossil-monorepo-fossil_api_db-1 psql -U postgres -c "SELECT version();"

# Check connection pool
# (requires database query: SELECT * FROM pg_stat_activity;)
```

**Solutions:**

```bash
# Restart database
docker restart fossil-monorepo-fossil_api_db-1

# Check database logs
docker logs fossil-monorepo-fossil_api_db-1 | tail -100

# Reset database
docker stop fossil-monorepo-fossil_api_db-1
docker rm fossil-monorepo-fossil_api_db-1
docker volume rm fossil-monorepo_fossil_api_db_data
make dev-up
```

### Query Debugging

**Enable SQL Query Logging:**

```bash
# Set log level for sqlx
export RUST_LOG=info,sqlx=debug

# Run service
cd fossil-api
cargo run --bin server
```

**Sample Query Log:**

```
[2024-01-15T10:30:00Z DEBUG sqlx::query] SELECT * FROM job_requests WHERE job_id = $1; params=["job_abc123"]
[2024-01-15T10:30:00Z DEBUG sqlx::query] rows affected: 1
```

**Direct Database Inspection:**

```bash
# Connect to database
psql postgresql://postgres:postgres@localhost:5434/postgres

# List tables
\dt

# Inspect table schema
\d job_requests

# Query recent jobs
SELECT job_id, status, created_at, updated_at
FROM job_requests
ORDER BY created_at DESC
LIMIT 10;

# Find failed jobs
SELECT job_id, status, error_message
FROM job_requests
WHERE status = 'failed';

# Check API keys
SELECT api_key, name, created_at, is_active
FROM api_keys
ORDER BY created_at DESC;

# Exit
\q
```

### Slow Query Analysis

**Identify Slow Queries:**

```sql
-- Enable query timing
\timing on

-- Test query performance
EXPLAIN ANALYZE
SELECT * FROM job_requests
WHERE vault_address = '0x123'
AND status = 'pending';

-- Check for missing indexes
SELECT schemaname, tablename, indexname
FROM pg_indexes
WHERE tablename = 'job_requests';

-- Analyze table statistics
ANALYZE job_requests;
```

**Performance Tuning:**

```sql
-- Create index for common queries
CREATE INDEX IF NOT EXISTS idx_job_requests_status
ON job_requests(status);

CREATE INDEX IF NOT EXISTS idx_job_requests_vault_address
ON job_requests(vault_address);

-- Composite index
CREATE INDEX IF NOT EXISTS idx_job_requests_status_vault
ON job_requests(status, vault_address);
```

### Database Reset Procedures

**Full Reset:**

```bash
# Stop all services
make dev-down

# Remove volumes
docker volume ls | grep fossil-monorepo
docker volume rm fossil-monorepo_fossil_api_db_data
docker volume rm fossil-monorepo_proving_service_db_data
docker volume rm fossil-monorepo_indexer_db_data

# Restart
make dev-up
```

**Partial Reset (keep data):**

```bash
# Restart database containers only
docker restart fossil-monorepo-fossil_api_db-1
docker restart fossil-monorepo-proving_service_db-1
```

**Manual Migration:**

```bash
# Connect to database
psql postgresql://postgres:postgres@localhost:5434/postgres

# Run migrations manually
\i fossil-api/crates/db-access/migrations/001_initial_schema.sql
\i fossil-api/crates/db-access/migrations/002_add_indexes.sql
```

## Network Debugging

### RPC Endpoint Issues

**StarkNet RPC Debugging:**

```bash
# Test basic RPC call
curl -X POST http://localhost:5050 \
  -H "Content-Type: application/json" \
  -d '{"jsonrpc":"2.0","method":"starknet_chainId","params":[],"id":1}'

# Get latest block
curl -X POST http://localhost:5050 \
  -H "Content-Type: application/json" \
  -d '{"jsonrpc":"2.0","method":"starknet_blockNumber","params":[],"id":1}'

# Call contract
curl -X POST http://localhost:5050 \
  -H "Content-Type: application/json" \
  -d '{
    "jsonrpc": "2.0",
    "method": "starknet_call",
    "params": {
      "request": {
        "contract_address": "'$FOSSIL_STORE_ADDRESS'",
        "entry_point_selector": "0x...",
        "calldata": []
      },
      "block_id": "latest"
    },
    "id": 1
  }'
```

**Test with starkli:**

```bash
# Get chain ID
starkli chain-id --rpc http://localhost:5050

# Get block number
starkli block-number --rpc http://localhost:5050

# Call contract function
starkli call \
  --rpc http://localhost:5050 \
  $FOSSIL_STORE_ADDRESS \
  get_avg_fees_in_range \
  1672531200 1672617600
```

### HTTP Request Debugging

**Trace HTTP Requests:**

```bash
# Use curl with verbose output
curl -v -X POST "http://localhost:3000/pricing_data" \
  -H "Content-Type: application/json" \
  -H "X-API-Key: $API_KEY" \
  -d '{...}' 2>&1 | tee request.log

# Capture request/response with httpie
http --verbose POST http://localhost:3000/pricing_data \
  X-API-Key:$API_KEY \
  program_id=TEST \
  vault_address=$PITCHLAKE_VAULT_12MIN \
  params:='{"twap":[1672531200,1672617600],...}'

# Use netcat to inspect raw traffic
nc -l 8080 &
curl http://localhost:8080/test
```

### Container Network Debugging

**Test Inter-Container Communication:**

```bash
# Test from Fossil API to Proving Service
docker exec fossil-monorepo-fossil-api-1 curl -v http://proving-service-api:3001/health

# Test from Message Handler to Katana
docker exec fossil-monorepo-message-handler-1 \
  curl -X POST http://katana:5050 \
  -H "Content-Type: application/json" \
  -d '{"jsonrpc":"2.0","method":"starknet_chainId","params":[],"id":1}'

# Test DNS resolution
docker exec fossil-monorepo-fossil-api-1 nslookup proving-service-api
docker exec fossil-monorepo-fossil-api-1 ping -c 3 katana

# Inspect network
docker network inspect fossil-monorepo_default

# Check container IPs
docker inspect -f '{{range .NetworkSettings.Networks}}{{.IPAddress}}{{end}}' \
  fossil-monorepo-fossil-api-1
```

### Proxy and CORS Issues

**CORS Debugging:**

```bash
# Test CORS headers
curl -v -H "Origin: http://localhost:3000" \
  -H "Access-Control-Request-Method: POST" \
  -H "Access-Control-Request-Headers: X-API-Key" \
  -X OPTIONS \
  http://localhost:3000/pricing_data

# Check ALLOWED_ORIGINS configuration
echo $ALLOWED_ORIGINS

# Update CORS configuration
# Edit .env.local:
# ALLOWED_ORIGINS=http://localhost:3000,http://127.0.0.1:3000,http://example.com
```

## Proof Generation Issues

### RISC Zero / Bonsai Problems

**Common Errors:**

1. **API Key Issues**

```bash
# Verify API key format
echo $BONSAI_API_KEY

# Test Bonsai API connectivity (requires valid key)
# Check message handler logs for Bonsai API responses
docker logs fossil-monorepo-message-handler-1 | grep -i bonsai
```

2. **Proof Timeout**

```bash
# Increase timeout
# Edit proving-service/crates/message-handler/src/main.rs
# Change Duration::from_secs(3600) to Duration::from_secs(7200)

# Check timeout configuration
docker logs fossil-monorepo-message-handler-1 | grep "timeout"

# Use mock proofs for development
export USE_SIMPLE_MOCK=true
```

3. **Insufficient Data**

```bash
# Check fee data availability
docker logs fossil-monorepo-message-handler-1 | grep "Insufficient onchain fee data"

# Verify timestamp range
# Need at least 1440 hours (POC) or 5760 hours (Production)

# Check Fossil Store contract
starkli call \
  --rpc http://localhost:5050 \
  $FOSSIL_STORE_ADDRESS \
  get_avg_fees_in_range \
  $START_TS $END_TS
```

### Mock Proof Configuration

**Enable Mock Proofs:**

```bash
# Method 1: Use simple mock (no dependencies)
export USE_SIMPLE_MOCK=true
export ENABLE_PROOF=false

# Method 2: Use RISC0 mock proofs
export USE_RISC0_INTEGRATION=true
export USE_SIMPLE_MOCK=false

# Rebuild with mock feature
cd proving-service/crates/message-handler
cargo build --features mock-proof

# Run with mock proofs
cargo run --features mock-proof
```

### Proof Composition Debugging

**Component-Level Testing:**

```bash
# Test individual sub-proofs
cd proving-service/crates/message-handler
RUST_LOG=debug cargo test test_hashing --features mock-proof -- --nocapture
RUST_LOG=debug cargo test test_max_return --features mock-proof -- --nocapture
RUST_LOG=debug cargo test test_twap --features mock-proof -- --nocapture

# Test full composition
RUST_LOG=debug cargo test test_proof_composition --features mock-proof -- --nocapture
```

**Debugging Proof Assumptions:**

```rust
// Check assumption verification in logs
docker logs fossil-monorepo-message-handler-1 | grep -i "assumption"

// Sample log output:
// [DEBUG] Added assumption for hashing receipt
// [DEBUG] Added assumption for max_return receipt
// [DEBUG] Added assumption for twap receipt
// [DEBUG] Verifying 7 assumptions in final proof
```

## StarkNet Issues

### Transaction Failures

**Common Transaction Errors:**

1. **Insufficient Balance**

```bash
# Check account balance
starkli balance --rpc http://localhost:5050 $STARKNET_ACCOUNT_ADDRESS

# Fund account (Katana predeployed accounts have balance)
# For custom accounts, use faucet or transfer
```

2. **Invalid Nonce**

```bash
# Get current nonce
starkli nonce --rpc http://localhost:5050 $STARKNET_ACCOUNT_ADDRESS

# Check transaction status
starkli transaction-status --rpc http://localhost:5050 $TX_HASH
```

3. **Contract Not Found**

```bash
# Verify contract is deployed
starkli class-hash-at --rpc http://localhost:5050 $CONTRACT_ADDRESS

# Check contract addresses are set
echo $PITCHLAKE_VERIFIER_CONTRACT
echo $FOSSIL_STORE_ADDRESS
echo $HASH_STORAGE_ADDRESS

# Redeploy contracts if needed
make dev-down && make dev-up
```

### Contract Call Debugging

**Test Contract Calls:**

```bash
# Call Fossil Store contract
starkli call \
  --rpc http://localhost:5050 \
  $FOSSIL_STORE_ADDRESS \
  get_avg_fees_in_range \
  1672531200 1672617600

# Call verifier contract
starkli call \
  --rpc http://localhost:5050 \
  $PITCHLAKE_VERIFIER_CONTRACT \
  verify_proof_for_pitch_lake \
  [proof_data...] \
  [job_request...]

# Invoke (send transaction)
starkli invoke \
  --rpc http://localhost:5050 \
  --account $STARKNET_ACCOUNT \
  $HASH_STORAGE_ADDRESS \
  hash_avg_fees_and_store \
  1672531200
```

### StarkNet Handler Debugging

**Enable StarkNet Handler Logs:**

```bash
# Run with debug logging
cd proving-service/crates/starknet-handler
RUST_LOG=debug,starknet=trace cargo test -- --nocapture

# Check retry logic
docker logs fossil-monorepo-message-handler-1 | grep -i "retry"

# Monitor transaction submissions
docker logs fossil-monorepo-message-handler-1 | grep "Proof verified onchain"
```

**Common StarkNet Handler Issues:**

```bash
# 1. RPC connection issues
# Check STARKNET_RPC_URL
echo $STARKNET_RPC_URL

# Test RPC
curl -X POST $STARKNET_RPC_URL \
  -H "Content-Type: application/json" \
  -d '{"jsonrpc":"2.0","method":"starknet_chainId","params":[],"id":1}'

# 2. Account configuration
# Verify private key and address
echo $STARKNET_ACCOUNT_ADDRESS
echo $STARKNET_PRIVATE_KEY

# 3. Contract addresses
# Check all contract addresses are set
env | grep -E "(FOSSIL_STORE|HASH_STORAGE|PITCHLAKE|VERIFIER)"
```

## Performance Profiling

### Identifying Bottlenecks

**Monitor Resource Usage:**

```bash
# Overall Docker stats
docker stats

# Specific container stats
docker stats fossil-monorepo-message-handler-1 --no-stream

# CPU and memory usage
docker exec fossil-monorepo-message-handler-1 ps aux

# Disk I/O
docker exec fossil-monorepo-fossil_api_db-1 iostat -x 5
```

**Application-Level Profiling:**

```bash
# Add timing instrumentation
# Use tokio-console for async profiling

# Install tokio-console
cargo install tokio-console

# Run with console support
# (requires feature flag in Cargo.toml)
tokio-console
```

### Database Performance

**Query Performance Analysis:**

```sql
-- Connect to database
psql postgresql://postgres:postgres@localhost:5434/postgres

-- Enable timing
\timing on

-- Analyze query plan
EXPLAIN ANALYZE
SELECT * FROM job_requests
WHERE status = 'pending'
AND created_at > NOW() - INTERVAL '1 hour';

-- Check slow queries (if pg_stat_statements is enabled)
SELECT
  query,
  calls,
  mean_exec_time,
  max_exec_time
FROM pg_stat_statements
ORDER BY mean_exec_time DESC
LIMIT 10;

-- Check table bloat
SELECT
  schemaname,
  tablename,
  pg_size_pretty(pg_total_relation_size(schemaname||'.'||tablename)) AS size
FROM pg_tables
WHERE schemaname = 'public'
ORDER BY pg_total_relation_size(schemaname||'.'||tablename) DESC;
```

**Optimize Queries:**

```sql
-- Add indexes for common queries
CREATE INDEX CONCURRENTLY idx_job_requests_status_created
ON job_requests(status, created_at DESC);

-- Vacuum and analyze
VACUUM ANALYZE job_requests;

-- Check index usage
SELECT
  schemaname,
  tablename,
  indexname,
  idx_scan,
  idx_tup_read,
  idx_tup_fetch
FROM pg_stat_user_indexes
WHERE schemaname = 'public';
```

### Proof Generation Performance

**Monitor Proof Times:**

```bash
# Extract proof generation times from logs
docker logs fossil-monorepo-message-handler-1 | \
  grep -E "(Generating proof|completed successfully)" | \
  awk '{print $1, $2}'

# Calculate average time
# (requires parsing timestamps)

# Check concurrent proof limit
echo $MAX_CONCURRENT_PROOFS

# Adjust for performance
export MAX_CONCURRENT_PROOFS=3  # More concurrent, but may hit rate limits
```

**Bonsai API Performance:**

```bash
# Monitor Bonsai API response times
docker logs fossil-monorepo-message-handler-1 | grep -i "bonsai" | grep "time="

# Check for rate limiting
docker logs fossil-monorepo-message-handler-1 | grep -i "rate limit"

# Optimize retry configuration
export RISC0_MAX_RETRIES=7
export RISC0_INITIAL_RETRY_DELAY_MS=3000
```

### Network Latency

**Measure Request Latency:**

```bash
# API endpoint latency
time curl -s http://localhost:3000/health

# End-to-end request timing
time ./test-local-request.sh

# Database query latency
time psql postgresql://postgres:postgres@localhost:5434/postgres -c "SELECT COUNT(*) FROM job_requests;"

# StarkNet RPC latency
time starkli block-number --rpc http://localhost:5050
```

## Useful Commands

### Docker Management

```bash
# View all running containers
docker ps

# View all containers (including stopped)
docker ps -a

# View logs
docker logs <container_name>
docker logs -f <container_name>  # Follow logs
docker logs --tail 100 <container_name>  # Last 100 lines
docker logs --since 30m <container_name>  # Last 30 minutes

# Execute command in container
docker exec <container_name> <command>
docker exec -it <container_name> bash  # Interactive shell

# Restart containers
docker restart <container_name>
docker restart $(docker ps -q)  # Restart all

# Stop containers
docker stop <container_name>
make dev-down  # Stop all Fossil containers

# Remove containers
docker rm <container_name>
docker rm $(docker ps -aq)  # Remove all stopped containers

# View container resource usage
docker stats
docker stats --no-stream  # One-time snapshot

# Inspect container
docker inspect <container_name>
docker inspect <container_name> | jq '.[0].NetworkSettings.IPAddress'

# View volumes
docker volume ls
docker volume inspect <volume_name>

# Remove volumes (caution: data loss!)
docker volume rm <volume_name>
```

### Service-Specific Logs

```bash
# Fossil API
docker logs fossil-monorepo-fossil-api-1 -f

# Proving Service API
docker logs fossil-monorepo-proving-service-api-1 -f

# Message Handler
docker logs fossil-monorepo-message-handler-1 -f

# Katana (StarkNet)
docker logs fossil-monorepo-katana-1 -f

# Databases
docker logs fossil-monorepo-fossil_api_db-1 -f
docker logs fossil-monorepo-proving_service_db-1 -f
docker logs fossil-monorepo-indexer_db-1 -f

# LocalStack
docker logs fossil-monorepo-localstack-1 -f

# Filter logs
docker logs fossil-monorepo-message-handler-1 | grep "error"
docker logs fossil-monorepo-fossil-api-1 | grep -i "job_abc123"
```

### Cargo Commands

```bash
# Build
cargo build
cargo build --release
cargo build --features mock-proof

# Run
cargo run
cargo run --bin server
cargo run --features mock-proof

# Test
cargo test
cargo test --package server
cargo test test_name
cargo test -- --nocapture  # Show println! output
RUST_LOG=debug cargo test -- --nocapture  # With logging

# Clippy (linting)
cargo clippy
cargo clippy --fix

# Format
cargo fmt
cargo fmt --check

# Check (faster than build)
cargo check

# Clean
cargo clean

# Update dependencies
cargo update
```

### Database Commands

```bash
# Connect to Fossil API database
psql postgresql://postgres:postgres@localhost:5434/postgres

# Connect to Proving Service database
psql postgresql://postgres:postgres@localhost:5435/postgres

# Connect to Indexer database (read-only)
psql postgresql://postgres:postgres@localhost:5433/postgres

# Execute single command
psql <connection_string> -c "SELECT COUNT(*) FROM job_requests;"

# Execute SQL file
psql <connection_string> -f migrations/001_initial_schema.sql

# Dump database
pg_dump postgresql://postgres:postgres@localhost:5434/postgres > backup.sql

# Restore database
psql postgresql://postgres:postgres@localhost:5434/postgres < backup.sql

# Common psql commands (inside psql session)
\l          # List databases
\c <db>     # Connect to database
\dt         # List tables
\d <table>  # Describe table
\di         # List indexes
\du         # List users
\q          # Quit
\?          # Help
```

### AWS CLI (LocalStack)

```bash
# List queues
aws --endpoint-url=http://localhost:4567 sqs list-queues

# Get queue attributes
aws --endpoint-url=http://localhost:4567 sqs get-queue-attributes \
  --queue-url http://localhost:4567/000000000000/fossilQueue \
  --attribute-names All

# Send message
aws --endpoint-url=http://localhost:4567 sqs send-message \
  --queue-url http://localhost:4567/000000000000/fossilQueue \
  --message-body '{"test":"message"}'

# Receive messages
aws --endpoint-url=http://localhost:4567 sqs receive-message \
  --queue-url http://localhost:4567/000000000000/fossilQueue \
  --max-number-of-messages 10 \
  --wait-time-seconds 20

# Delete message
aws --endpoint-url=http://localhost:4567 sqs delete-message \
  --queue-url http://localhost:4567/000000000000/fossilQueue \
  --receipt-handle <receipt_handle>

# Purge queue (delete all messages)
aws --endpoint-url=http://localhost:4567 sqs purge-queue \
  --queue-url http://localhost:4567/000000000000/fossilQueue

# Create queue
aws --endpoint-url=http://localhost:4567 sqs create-queue \
  --queue-name fossilQueue \
  --attributes VisibilityTimeout=3600

# Delete queue
aws --endpoint-url=http://localhost:4567 sqs delete-queue \
  --queue-url http://localhost:4567/000000000000/fossilQueue
```

### StarkNet CLI (starkli)

```bash
# Get chain ID
starkli chain-id --rpc http://localhost:5050

# Get block number
starkli block-number --rpc http://localhost:5050

# Get block
starkli block --rpc http://localhost:5050 latest

# Get account balance
starkli balance --rpc http://localhost:5050 $STARKNET_ACCOUNT_ADDRESS

# Get account nonce
starkli nonce --rpc http://localhost:5050 $STARKNET_ACCOUNT_ADDRESS

# Call contract (read-only)
starkli call \
  --rpc http://localhost:5050 \
  $CONTRACT_ADDRESS \
  function_name \
  arg1 arg2

# Invoke contract (send transaction)
starkli invoke \
  --rpc http://localhost:5050 \
  --account $STARKNET_ACCOUNT \
  $CONTRACT_ADDRESS \
  function_name \
  arg1 arg2

# Get transaction status
starkli transaction-status --rpc http://localhost:5050 $TX_HASH

# Get transaction receipt
starkli receipt --rpc http://localhost:5050 $TX_HASH

# Get contract class hash
starkli class-hash-at --rpc http://localhost:5050 $CONTRACT_ADDRESS

# Declare contract
starkli declare --rpc http://localhost:5050 --account $STARKNET_ACCOUNT contract.json

# Deploy contract
starkli deploy --rpc http://localhost:5050 --account $STARKNET_ACCOUNT $CLASS_HASH constructor_args
```

### cURL API Testing

```bash
# Health check
curl http://localhost:3000/health
curl http://localhost:3001/health

# Generate API key
curl -X POST "http://localhost:3000/api_key" \
  -H "Content-Type: application/json" \
  -d '{"name":"test_key"}'

# Submit pricing data request
curl -X POST "http://localhost:3000/pricing_data" \
  -H "Content-Type: application/json" \
  -H "X-API-Key: $API_KEY" \
  -d '{
    "program_id": "RISC0_MOCK_PROOF_TEST",
    "vault_address": "'$PITCHLAKE_VAULT_12MIN'",
    "params": {
      "twap": [1672531200, 1672617600],
      "max_return": [1672531200, 1672617600],
      "reserve_price": [1672531200, 1672617600]
    }
  }'

# Check job status
curl "http://localhost:3000/job_status/$JOB_ID"

# Get job result
curl "http://localhost:3000/job_result/$JOB_ID" \
  -H "X-API-Key: $API_KEY"

# Submit job to Proving Service (internal)
curl -X POST "http://localhost:3001/api/job" \
  -H "Content-Type: application/json" \
  -d '{
    "job_group_id": "test_123",
    "twap": {"start_timestamp": 1672531200, "end_timestamp": 1672617600},
    "reserve_price": {"start_timestamp": 1672531200, "end_timestamp": 1672617600},
    "max_return": {"start_timestamp": 1672531200, "end_timestamp": 1672617600},
    "vault_address": "'$PITCHLAKE_VAULT_12MIN'",
    "vault_timestamp": 1672531200
  }'

# Test with verbose output
curl -v <url>

# Test with timing
curl -w "@curl-format.txt" -o /dev/null -s <url>

# curl-format.txt:
time_namelookup:  %{time_namelookup}s
time_connect:     %{time_connect}s
time_appconnect:  %{time_appconnect}s
time_pretransfer: %{time_pretransfer}s
time_redirect:    %{time_redirect}s
time_starttransfer: %{time_starttransfer}s
time_total:       %{time_total}s
```

### Makefile Targets

```bash
# Setup and building
make setup           # Complete development environment setup
make build           # Build all services in release mode
make build-all       # Same as build
make pr              # Run linters and tests (run before PRs)

# Development environment
make dev-up          # Start all services with Docker
make dev-down        # Stop all services
make dev-services    # Start only infrastructure services
make dev-services-stop  # Stop infrastructure services

# Testing
make test-all        # Run tests for all services
make lint-all        # Run linters for all services
make fmt-all         # Format all code

# Individual service operations
cd fossil-api && make test
cd fossil-api && make lint
cd fossil-api && make build
cd proving-service && make test
cd proving-service && make lint
cd proving-service && make build

# Help
make help            # Show all available targets
```

## Quick Reference: Debugging Checklist

When encountering issues, follow this checklist:

**1. Basic Health Checks (5 minutes)**

```bash
# Service health
curl http://localhost:3000/health
curl http://localhost:3001/health

# Database connectivity
psql postgresql://postgres:postgres@localhost:5434/postgres -c "SELECT 1;"

# StarkNet RPC
curl -X POST http://localhost:5050 \
  -H "Content-Type: application/json" \
  -d '{"jsonrpc":"2.0","method":"starknet_chainId","params":[],"id":1}'

# Docker containers
docker ps
```

**2. Review Logs (10 minutes)**

```bash
# Check each service for errors
docker logs fossil-monorepo-fossil-api-1 --tail 100 | grep -i error
docker logs fossil-monorepo-proving-service-api-1 --tail 100 | grep -i error
docker logs fossil-monorepo-message-handler-1 --tail 100 | grep -i error
docker logs fossil-monorepo-katana-1 --tail 100 | grep -i error
```

**3. Verify Configuration (5 minutes)**

```bash
# Check environment variables
cat .env.local | grep -E "(DATABASE|STARKNET|PROVING|AWS)"

# Verify contract addresses
env | grep -E "(PITCHLAKE|FOSSIL|VERIFIER)"
```

**4. Test End-to-End (5 minutes)**

```bash
# Run automated test
./test-local-request.sh
```

**5. Isolate Component (10 minutes)**

```bash
# Test each component independently
# Follow "Isolating Component Failures" section above
```

**6. Deep Dive (30+ minutes)**

```bash
# Enable debug logging
export RUST_LOG=debug

# Follow "End-to-End Request Tracing" workflow
# Investigate specific component issues
```

## Next Steps

- [Local Development Guide](../getting-started/local-development.md) - Setting up development environment
- [Testing Guide](../getting-started/testing.md) - Writing and running tests
- [Architecture Overview](../architecture/overview.md) - Understanding system design
- [Environment Setup Guide](environment-setup.md) - Configuring environment variables
- [API Reference](../api-reference/fossil-api-endpoints.md) - Complete API documentation

## Getting Help

If you're still stuck after following this guide:

1. **Check existing documentation** - Review architecture and API docs
2. **Search logs systematically** - Use `grep` and `jq` to filter relevant information
3. **Reproduce minimally** - Create the smallest possible test case
4. **Check recent changes** - Use `git log` to see what changed recently
5. **Ask for help** - Share logs, configuration, and steps to reproduce

**When asking for help, include:**

- Full error message with stack trace
- Relevant log excerpts (not entire logs)
- Configuration (environment variables, with secrets redacted)
- Steps to reproduce
- What you've already tried
