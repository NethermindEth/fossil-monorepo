#!/bin/bash

# Test script to reproduce the same request as test-e2e.sh but using local Docker services
# This script sends requests to the running services from make dev-up

set -e

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
NC='\033[0m' # No Color

# Logging functions
log_info() {
    echo -e "${GREEN}[INFO]${NC} $1"
}

log_warn() {
    echo -e "${YELLOW}[WARN]${NC} $1"
}

log_error() {
    echo -e "${RED}[ERROR]${NC} $1"
}

# Load local environment variables
source .env.local

# Use local URLs (services running via make dev-up)
OFFCHAIN_PROCESSOR_URL="http://localhost:3000"
PROVING_SERVICE_URL="http://localhost:3001"

log_info "Testing Fossil Monorepo locally using Docker services"
log_info "Offchain Processor: $OFFCHAIN_PROCESSOR_URL"
log_info "Proving Service: $PROVING_SERVICE_URL"
log_info "PitchLake Vault: $PITCHLAKE_VAULT"

# Check if services are running
log_info "Checking if services are responding..."

if ! curl -s "$OFFCHAIN_PROCESSOR_URL/health" > /dev/null 2>&1; then
    log_error "Offchain Processor not responding at $OFFCHAIN_PROCESSOR_URL"
    log_error "Make sure you've run: make dev-up"
    exit 1
fi
log_info "✅ Offchain Processor is healthy"

if ! curl -s "$PROVING_SERVICE_URL/health" > /dev/null 2>&1; then
    log_error "Proving Service not responding at $PROVING_SERVICE_URL"
    log_error "Make sure you've run: make dev-up"
    exit 1
fi
log_info "✅ Proving Service is healthy"

# Step 1: Generate API key
log_info "🔑 Generating API key..."
API_KEY_RESPONSE=$(curl -s -X POST "$OFFCHAIN_PROCESSOR_URL/api_key" \
  -H "Content-Type: application/json" \
  -d '{"name": "local_test_key"}')

API_KEY=$(echo $API_KEY_RESPONSE | grep -o '"api_key":"[^"]*"' | cut -d'"' -f4)

if [ -z "$API_KEY" ]; then
    log_error "Failed to generate API key"
    log_error "Response: $API_KEY_RESPONSE"
    exit 1
fi

log_info "✅ Generated API key: $API_KEY"

# Step 2: Send test request to offchain-processor
log_info "📊 Sending pricing data request..."

TEST_REQUEST="{
  \"identifiers\": [\"0x50495443485f4c414b455f5631\"],
  \"params\": {
    \"twap\": [1672531200, 1672574400],
    \"volatility\": [1672531200, 1672574400],
    \"reserve_price\": [1672531200, 1672574400]
  },
  \"client_info\": {
    \"client_address\": \"0x018df581fe0ee497a4a3595cf62aea0bafa7ba1a54a7dcbafca37bfada67c718\",
    \"vault_address\": \"$PITCHLAKE_VAULT\",
    \"timestamp\": $(date +%s)
  }
}"

RESPONSE=$(curl -s -X POST "$OFFCHAIN_PROCESSOR_URL/pricing_data" \
  -H "Content-Type: application/json" \
  -H "X-API-Key: $API_KEY" \
  -d "$TEST_REQUEST")

log_info "Response: $RESPONSE"

# Step 3: Extract job ID from response
JOB_ID=$(echo $RESPONSE | grep -o '"job_id":"[^"]*"' | cut -d'"' -f4)

if [ -z "$JOB_ID" ]; then
    log_error "Failed to extract job ID from response"
    log_error "Full response: $RESPONSE"
    exit 1
fi

log_info "✅ Job created with ID: $JOB_ID"

# Step 4: Monitor job status
log_info "👁️  Monitoring job status..."
MAX_STATUS_RETRIES=30
STATUS_RETRY_COUNT=0

while [ $STATUS_RETRY_COUNT -lt $MAX_STATUS_RETRIES ]; do
    STATUS_RESPONSE=$(curl -s "$OFFCHAIN_PROCESSOR_URL/job_status/$JOB_ID")
    STATUS=$(echo $STATUS_RESPONSE | grep -o '"status":"[^"]*"' | cut -d'"' -f4)

    log_info "Job status (attempt $((STATUS_RETRY_COUNT + 1))/$MAX_STATUS_RETRIES): $STATUS"

    if [ "$STATUS" = "Completed" ]; then
        log_info "✅ Job completed successfully!"
        break
    elif [ "$STATUS" = "Failed" ]; then
        log_error "❌ Job failed"
        log_error "Status response: $STATUS_RESPONSE"
        break
    fi

    STATUS_RETRY_COUNT=$((STATUS_RETRY_COUNT + 1))
    sleep 3
done

if [ $STATUS_RETRY_COUNT -eq $MAX_STATUS_RETRIES ]; then
    log_warn "⚠️  Job status monitoring timed out after $MAX_STATUS_RETRIES attempts"
    log_warn "Final status: $STATUS"
fi

# Step 5: Get detailed job result
log_info "📋 Getting detailed job result..."
RESULT_RESPONSE=$(curl -s "$OFFCHAIN_PROCESSOR_URL/job_result/$JOB_ID" \
  -H "X-API-Key: $API_KEY")

log_info "Detailed result: $RESULT_RESPONSE"

# Step 6: Test batch job status
log_info "📦 Testing batch job status endpoint..."
BATCH_REQUEST='{"job_ids": ["'$JOB_ID'"]}'
BATCH_RESPONSE=$(curl -s -X POST "$OFFCHAIN_PROCESSOR_URL/batch_job_status" \
  -H "Content-Type: application/json" \
  -H "X-API-Key: $API_KEY" \
  -d "$BATCH_REQUEST")

log_info "Batch status response: $BATCH_RESPONSE"

# Step 7: Test RISC0 Mock Proof Generation (like in test-e2e.sh)
log_info "🧪 Testing RISC0 Mock Proof Generation..."

RISC0_TEST_REQUEST="{
  \"identifiers\": [\"RISC0_MOCK_PROOF_TEST\"],
  \"params\": {
    \"twap\": [1672531200, 1672617600],
    \"volatility\": [1672531200, 1672617600],
    \"reserve_price\": [1672531200, 1672617600]
  },
  \"client_info\": {
    \"client_address\": \"0x018df581fe0ee497a4a3595cf62aea0bafa7ba1a54a7dcbafca37bfada67c718\",
    \"vault_address\": \"$PITCHLAKE_VAULT\",
    \"timestamp\": $(date +%s)
  }
}"

RISC0_RESPONSE=$(curl -s -X POST "$OFFCHAIN_PROCESSOR_URL/pricing_data" \
  -H "Content-Type: application/json" \
  -H "X-API-Key: $API_KEY" \
  -d "$RISC0_TEST_REQUEST")

log_info "RISC0 test response: $RISC0_RESPONSE"

# Extract RISC0 job ID
RISC0_JOB_ID=$(echo $RISC0_RESPONSE | grep -o '"job_id":"[^"]*"' | cut -d'"' -f4)

if [ ! -z "$RISC0_JOB_ID" ]; then
    log_info "✅ RISC0 test job created with ID: $RISC0_JOB_ID"

    # Monitor RISC0 job for longer since proof generation takes time
    log_info "⏳ Monitoring RISC0 proof generation job (this may take several minutes)..."
    RISC0_MAX_RETRIES=60  # 5 minutes max
    RISC0_RETRY_COUNT=0

    while [ $RISC0_RETRY_COUNT -lt $RISC0_MAX_RETRIES ]; do
        RISC0_STATUS_RESPONSE=$(curl -s "$OFFCHAIN_PROCESSOR_URL/job_status/$RISC0_JOB_ID")
        RISC0_STATUS=$(echo $RISC0_STATUS_RESPONSE | grep -o '"status":"[^"]*"' | cut -d'"' -f4)

        log_info "RISC0 job status (attempt $((RISC0_RETRY_COUNT + 1))/$RISC0_MAX_RETRIES): $RISC0_STATUS"

        if [ "$RISC0_STATUS" = "Completed" ]; then
            log_info "✅ RISC0 proof generation completed successfully!"

            # Get detailed RISC0 result
            RISC0_RESULT=$(curl -s "$OFFCHAIN_PROCESSOR_URL/job_result/$RISC0_JOB_ID" \
              -H "X-API-Key: $API_KEY")
            log_info "RISC0 proof result: $RISC0_RESULT"
            break
        elif [ "$RISC0_STATUS" = "Failed" ]; then
            log_error "❌ RISC0 proof generation job failed"
            log_error "Status response: $RISC0_STATUS_RESPONSE"
            break
        fi

        RISC0_RETRY_COUNT=$((RISC0_RETRY_COUNT + 1))
        sleep 5
    done

    if [ $RISC0_RETRY_COUNT -eq $RISC0_MAX_RETRIES ]; then
        log_warn "⚠️  RISC0 proof generation monitoring timed out"
        log_warn "Final status: $RISC0_STATUS"
        log_warn "Check Docker logs: docker logs fossil-monorepo-message-handler-1"
    fi
else
    log_warn "⚠️  Failed to create RISC0 test job"
fi

log_info "🎉 Local test completed!"
log_info "📊 Summary:"
log_info "   - Basic job: $JOB_ID ($STATUS)"
if [ ! -z "$RISC0_JOB_ID" ]; then
    log_info "   - RISC0 job: $RISC0_JOB_ID ($RISC0_STATUS)"
fi
log_info ""
log_info "💡 To check service logs:"
log_info "   docker logs fossil-monorepo-message-handler-1 -f"
log_info "   docker logs fossil-monorepo-offchain-processor-1 -f"
log_info "   docker logs fossil-monorepo-proving-service-api-1 -f"
