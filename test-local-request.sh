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

REQUEST_DATA=$(starkli call $PITCHLAKE_VAULT_12MIN get_request_to_start_first_round --rpc $STARKNET_RPC_URL)
log_info "Request data: $REQUEST_DATA"

# Extract calldata from response - parse each hex value from the array
VAULT_ADDRESS=$(echo "$REQUEST_DATA" | sed -n '3p' | sed 's/^[[:space:]]*//' | sed 's/[",]//g')
TIMESTAMP=$(echo "$REQUEST_DATA" | sed -n '4p' | sed 's/^[[:space:]]*//' | sed 's/[",]//g')
PROGRAM_ID=$(echo "$REQUEST_DATA" | sed -n '5p' | sed 's/^[[:space:]]*//' | sed 's/[",]//g')

# Convert hex values to decimal for JSON
TIMESTAMP_DECIMAL=$((TIMESTAMP))

# Get proving delay for validation purposes only
PROVING_DELAY_HEX=$(starkli call $PITCHLAKE_VAULT_12MIN get_proving_delay --rpc $STARKNET_RPC_URL | grep -o '0x[0-9a-f]*')
PROVING_DELAY=$((PROVING_DELAY_HEX))

# Calculate the maximum provable timestamp for reference
CURRENT_TIMESTAMP=$(date +%s)
MAX_PROVABLE_TIMESTAMP=$((CURRENT_TIMESTAMP - PROVING_DELAY))

log_info "Timestamp validation:"
log_info "  Current timestamp: $CURRENT_TIMESTAMP"
log_info "  Proving delay: $PROVING_DELAY seconds"
log_info "  Max provable timestamp: $MAX_PROVABLE_TIMESTAMP"
log_info "  Original request timestamp: $TIMESTAMP_DECIMAL"

# Use the original timestamp from the vault contract - this is what the contract expects
log_info "Using original timestamp from vault contract as required"

# Use the correct program ID that the contract expects
PROGRAM_ID_DECIMAL="24847450290753728453128705585"

log_info "Parsed data:"
log_info "  Vault Address: $VAULT_ADDRESS"
log_info "  Timestamp: $TIMESTAMP (decimal: $TIMESTAMP_DECIMAL)"
log_info "  Program ID: $PROGRAM_ID (using correct decimal: $PROGRAM_ID_DECIMAL)"

# Step 3: Calculate correct timestamp ranges from vault contract
log_info "🕐 Calculating correct timestamp ranges from vault..."

# Get round duration from vault
ROUND_DURATION_HEX=$(starkli call $PITCHLAKE_VAULT_12MIN get_round_duration --rpc $STARKNET_RPC_URL | grep -o '0x[0-9a-f]*')
ROUND_DURATION=$((ROUND_DURATION_HEX))

# Get deployment date from round 1 (since we're starting first round)
DEPLOYMENT_DATE_HEX=$(starkli call $OPTION_ROUND_12MIN get_deployment_date --rpc $STARKNET_RPC_URL | grep -o '0x[0-9a-f]*')
UPPER_BOUND=$((DEPLOYMENT_DATE_HEX))

# Calculate the expected timestamp ranges based on vault contract logic
TWAP_START=$((UPPER_BOUND - ROUND_DURATION))
TWAP_END=$UPPER_BOUND
RESERVE_PRICE_START=$((UPPER_BOUND - 3 * ROUND_DURATION))
RESERVE_PRICE_END=$UPPER_BOUND
VOLATILITY_START=$RESERVE_PRICE_START
VOLATILITY_END=$UPPER_BOUND

log_info "Calculated timestamp ranges:"
log_info "  Round duration: $ROUND_DURATION seconds"
log_info "  Upper bound: $UPPER_BOUND"
log_info "  TWAP: [$TWAP_START, $TWAP_END]"
log_info "  Reserve price: [$RESERVE_PRICE_START, $RESERVE_PRICE_END]"
log_info "  Volatility: [$VOLATILITY_START, $VOLATILITY_END]"

# Step 4: Send test request to fossil-api
log_info "📊 Sending pricing data request..."

log_info "🔍 Debug: Request data being sent:"
log_info "  PROGRAM_ID_DECIMAL: $PROGRAM_ID_DECIMAL"
log_info "  TWAP_START: $TWAP_START"
log_info "  TWAP_END: $TWAP_END"
log_info "  VOLATILITY_START: $VOLATILITY_START"
log_info "  VOLATILITY_END: $VOLATILITY_END"
log_info "  RESERVE_PRICE_START: $RESERVE_PRICE_START"
log_info "  RESERVE_PRICE_END: $RESERVE_PRICE_END"
log_info "  VAULT_ADDRESS: $VAULT_ADDRESS"
log_info "  TIMESTAMP_DECIMAL: $TIMESTAMP_DECIMAL"

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

# Step 5: Extract job ID from response
JOB_ID=$(echo $RESPONSE | grep -o '"job_id":"[^"]*"' | cut -d'"' -f4)

if [ -z "$JOB_ID" ]; then
    log_error "Failed to extract job ID from response"
    log_error "Full response: $RESPONSE"
    exit 1
fi

log_info "✅ Job created with ID: $JOB_ID"

# Step 6: Monitor job status
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

# Step 7: Get detailed job result
log_info "📋 Getting detailed job result..."
RESULT_RESPONSE=$(curl -s "$FOSSIL_API_URL/job_result/$JOB_ID" \
  -H "X-API-Key: $API_KEY")

log_info "Detailed result: $RESULT_RESPONSE"

# Step 8: Test batch job status
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
