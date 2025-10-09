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

# Load sepolia environment variables
source .env.sepolia

# Use local URLs (services running via make dev-up)
FOSSIL_API_URL="http://localhost:3000"
PROVING_SERVICE_URL="http://localhost:3001"

log_info "Testing Fossil Monorepo locally using Docker services"
log_info "Fossil API: $FOSSIL_API_URL"
log_info "Proving Service: $PROVING_SERVICE_URL"
log_info "PitchLake Vault (12min): $PITCHLAKE_VAULT_12MIN"

# Check if services are running
log_info "Checking if services are responding..."

if ! curl -s "$FOSSIL_API_URL/health" > /dev/null 2>&1; then
    log_error "Fossil API not responding at $FOSSIL_API_URL"
    log_error "Make sure you've run: make dev-up"
    exit 1
fi
log_info "✅ Fossil API is healthy"

if ! curl -s "$PROVING_SERVICE_URL/health" > /dev/null 2>&1; then
    log_error "Proving Service not responding at $PROVING_SERVICE_URL"
    log_error "Make sure you've run: make dev-up"
    exit 1
fi
log_info "✅ Proving Service is healthy"

# Step 1: Generate API key
log_info "🔑 Generating API key..."
API_KEY_RESPONSE=$(curl -s -X POST "$FOSSIL_API_URL/api_key" \
  -H "Content-Type: application/json" \
  -d '{"name": "local_test_key"}')

API_KEY=$(echo $API_KEY_RESPONSE | grep -o '"api_key":"[^"]*"' | cut -d'"' -f4)

if [ -z "$API_KEY" ]; then
    log_error "Failed to generate API key"
    log_error "Response: $API_KEY_RESPONSE"
    exit 1
fi

log_info "✅ Generated API key: $API_KEY"

# Step 2: Get request data from StarkNet vault
log_info "📡 Getting request data from StarkNet vault..."

REQUEST_DATA=$(starkli call $PITCHLAKE_VAULT_12MIN get_request_to_settle_round --rpc $STARKNET_RPC_URL)
log_info "Request data: $REQUEST_DATA"

# Parse the response array - it contains all values in order:
# [0] program_id
# [1] vault_address
# [2] twap_start
# [3] twap_end
# [4] max_return_start
# [5] max_return_end
# [6] reserve_price_start
# [7] reserve_price_end

PROGRAM_ID_HEX=$(echo "$REQUEST_DATA" | sed -n '2p' | sed 's/^[[:space:]]*//' | sed 's/[",]//g')
VAULT_ADDRESS_HEX=$(echo "$REQUEST_DATA" | sed -n '3p' | sed 's/^[[:space:]]*//' | sed 's/[",]//g')
TWAP_START_HEX=$(echo "$REQUEST_DATA" | sed -n '4p' | sed 's/^[[:space:]]*//' | sed 's/[",]//g')
TWAP_END_HEX=$(echo "$REQUEST_DATA" | sed -n '5p' | sed 's/^[[:space:]]*//' | sed 's/[",]//g')
VOLATILITY_START_HEX=$(echo "$REQUEST_DATA" | sed -n '6p' | sed 's/^[[:space:]]*//' | sed 's/[",]//g')
VOLATILITY_END_HEX=$(echo "$REQUEST_DATA" | sed -n '7p' | sed 's/^[[:space:]]*//' | sed 's/[",]//g')
RESERVE_PRICE_START_HEX=$(echo "$REQUEST_DATA" | sed -n '8p' | sed 's/^[[:space:]]*//' | sed 's/[",]//g')
RESERVE_PRICE_END_HEX=$(echo "$REQUEST_DATA" | sed -n '9p' | sed 's/^[[:space:]]*//' | sed 's/[",]//g')

# Convert hex to decimal
PROGRAM_ID_DECIMAL=$((PROGRAM_ID_HEX))
# Keep vault address as hex string (it's too large for bash arithmetic)
VAULT_ADDRESS="$VAULT_ADDRESS_HEX"
TWAP_START=$((TWAP_START_HEX))
TWAP_END=$((TWAP_END_HEX))
VOLATILITY_START=$((VOLATILITY_START_HEX))
VOLATILITY_END=$((VOLATILITY_END_HEX))
RESERVE_PRICE_START=$((RESERVE_PRICE_START_HEX))
RESERVE_PRICE_END=$((RESERVE_PRICE_END_HEX))

log_info "Parsed data from vault:"
log_info "  Program ID: $PROGRAM_ID_DECIMAL"
log_info "  Vault Address: $VAULT_ADDRESS"
log_info "  TWAP: [$TWAP_START, $TWAP_END]"
log_info "  Max Return (Volatility): [$VOLATILITY_START, $VOLATILITY_END]"
log_info "  Reserve Price: [$RESERVE_PRICE_START, $RESERVE_PRICE_END]"

# Step 3: Send test request to fossil-api
log_info "📊 Sending pricing data request..."

log_info "🔍 Debug: Request data being sent:"
log_info "  PROGRAM_ID_DECIMAL: $PROGRAM_ID_DECIMAL"
log_info "  VAULT_ADDRESS: $VAULT_ADDRESS"
log_info "  TWAP_START: $TWAP_START"
log_info "  TWAP_END: $TWAP_END"
log_info "  VOLATILITY_START: $VOLATILITY_START"
log_info "  VOLATILITY_END: $VOLATILITY_END"
log_info "  RESERVE_PRICE_START: $RESERVE_PRICE_START"
log_info "  RESERVE_PRICE_END: $RESERVE_PRICE_END"

TEST_REQUEST="{
  \"program_id\": \"$PROGRAM_ID_DECIMAL\",
  \"vault_address\": \"$VAULT_ADDRESS\",
  \"params\": {
    \"twap\": [$TWAP_START, $TWAP_END],
    \"max_return\": [$VOLATILITY_START, $VOLATILITY_END],
    \"reserve_price\": [$RESERVE_PRICE_START, $RESERVE_PRICE_END]
  }
}"

log_info "🔍 Full JSON request:"
echo "$TEST_REQUEST" | jq .

RESPONSE=$(curl -s -X POST "$FOSSIL_API_URL/pricing_data" \
  -H "Content-Type: application/json" \
  -H "X-API-Key: $API_KEY" \
  -d "$TEST_REQUEST")

log_info "Response: $RESPONSE"

# Step 4: Extract job ID from response
JOB_ID=$(echo $RESPONSE | grep -o '"job_id":"[^"]*"' | cut -d'"' -f4)

if [ -z "$JOB_ID" ]; then
    log_error "Failed to extract job ID from response"
    log_error "Full response: $RESPONSE"
    exit 1
fi

log_info "✅ Job created with ID: $JOB_ID"

# Step 5: Monitor job status
log_info "👁️  Monitoring job status..."
MAX_STATUS_RETRIES=30
STATUS_RETRY_COUNT=0

while [ $STATUS_RETRY_COUNT -lt $MAX_STATUS_RETRIES ]; do
    STATUS_RESPONSE=$(curl -s "$FOSSIL_API_URL/job_status/$JOB_ID")
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

# Step 6: Get detailed job result
log_info "📋 Getting detailed job result..."
RESULT_RESPONSE=$(curl -s "$FOSSIL_API_URL/job_result/$JOB_ID" \
  -H "X-API-Key: $API_KEY")

log_info "Detailed result: $RESULT_RESPONSE"

# Step 7: Test batch job status
log_info "📦 Testing batch job status endpoint..."
BATCH_REQUEST='{"job_ids": ["'$JOB_ID'"]}'
BATCH_RESPONSE=$(curl -s -X POST "$FOSSIL_API_URL/batch_job_status" \
  -H "Content-Type: application/json" \
  -H "X-API-Key: $API_KEY" \
  -d "$BATCH_REQUEST")

log_info "Batch status response: $BATCH_RESPONSE"


log_info "🎉 Local test completed!"
log_info "📊 Summary:"
log_info "   - Job: $JOB_ID ($STATUS)"
log_info ""
log_info "💡 To check service logs:"
log_info "   docker logs fossil-monorepo-message-handler-1 -f"
log_info "   docker logs fossil-monorepo-fossil-api-1 -f"
log_info "   docker logs fossil-monorepo-proving-service-api-1 -f"
