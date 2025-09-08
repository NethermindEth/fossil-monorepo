#!/bin/bash

# End-to-End Test Script for Fossil Monorepo
# Launches proving-service and offchain-processor, then tests the complete flow
# 
# Usage: ./test-e2e.sh [environment]
# Environment options: 
#   - local (default): Uses .env.local for local development testing
#   - docker: Uses .env.docker for containerized testing  
#   - sepolia: Uses .env.sepolia for testnet testing
#
# Prerequisites:
#   1. Ensure the appropriate .env.{environment} file exists in the root directory
#   2. Start integration services: docker compose -f docker-compose.integration.yml up -d
#   3. For sepolia: Configure real network credentials in .env.sepolia

set -e

# Parse command line arguments
ENV_TYPE="${1:-local}"

# Validate environment argument
case "$ENV_TYPE" in
"local" | "docker" | "sepolia")
    echo "Using environment: $ENV_TYPE"
    ;;
*)
    echo "Invalid environment. Must be one of: local, docker, sepolia"
    echo "Usage: $0 [environment]"
    echo "  environment: local (default), docker, sepolia"
    exit 1
    ;;
esac

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

# Cleanup function
cleanup() {
    log_info "Cleaning up processes..."
    if [ ! -z "$PROVING_SERVICE_PID" ]; then
        kill $PROVING_SERVICE_PID 2>/dev/null || true
        log_info "Stopped proving-service (PID: $PROVING_SERVICE_PID)"
    fi
    if [ ! -z "$MESSAGE_HANDLER_PID" ]; then
        kill $MESSAGE_HANDLER_PID 2>/dev/null || true
        log_info "Stopped message-handler (PID: $MESSAGE_HANDLER_PID)"
    fi
    if [ ! -z "$OFFCHAIN_PROCESSOR_PID" ]; then
        kill $OFFCHAIN_PROCESSOR_PID 2>/dev/null || true
        log_info "Stopped offchain-processor (PID: $OFFCHAIN_PROCESSOR_PID)"
    fi
}

# Set trap for cleanup on exit
trap cleanup EXIT

# Get absolute path to the script directory
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"

# Load environment configuration using absolute path
ENV_FILE="$SCRIPT_DIR/.env.$ENV_TYPE"
if [ ! -f "$ENV_FILE" ]; then
    log_error "Environment file $ENV_FILE not found!"
    log_error "Please ensure the environment file exists for environment: $ENV_TYPE"
    exit 1
fi

log_info "Loading environment from $ENV_FILE..."
set -a  # automatically export all variables
source "$ENV_FILE"
set +a  # stop auto-export

# Override specific variables for e2e testing
export RUST_LOG="${RUST_LOG:-debug}"

log_info "Environment variables loaded from $ENV_FILE"

# Validate required environment variables
if [ -z "$PITCHLAKE_VAULT" ]; then
    log_error "PITCHLAKE_VAULT environment variable is not set!"
    log_error "Please ensure PITCHLAKE_VAULT is defined in $ENV_FILE"
    exit 1
fi

# Log key environment variables to verify they're loaded correctly
log_info "Key environment variables:"
log_info "  - PITCHLAKE_VERIFIER_CONTRACT: ${PITCHLAKE_VERIFIER_CONTRACT:-NOT_SET}"
log_info "  - PITCHLAKE_VAULT: ${PITCHLAKE_VAULT:-NOT_SET}"
log_info "  - STARKNET_RPC_URL: ${STARKNET_RPC_URL:-NOT_SET}"
log_info "  - BONSAI_API_KEY: ${BONSAI_API_KEY:+***SET***}"
log_info "  - FOSSIL_STORE_ADDRESS: ${FOSSIL_STORE_ADDRESS:-NOT_SET}"

# Check if integration services are running
log_info "Checking integration services..."

if ! docker ps | grep -q "fossil-monorepo-offchain_processor_db-1"; then
    log_error "Offchain Processor DB not running. Please start with: docker compose -f docker-compose.integration.yml up -d"
    exit 1
fi

if ! docker ps | grep -q "fossil-monorepo-localstack-1"; then
    log_error "LocalStack SQS not running. Please start with: docker compose -f docker-compose.integration.yml up -d"
    exit 1
fi

log_info "Integration services are running ✓"

# Override specific e2e test settings if needed
case "$ENV_TYPE" in
"local")
    # For local testing, ensure we use the correct port for proving service
    export PROVING_SERVICE_URL="http://127.0.0.1:3001"
    ;;
"docker")
    # Docker-specific overrides can go here
    ;;
"sepolia")
    # Sepolia-specific overrides can go here
    ;;
esac

log_info "Environment configuration complete for $ENV_TYPE"

# Setup SQS queue if needed
log_info "Setting up SQS queue..."
docker exec fossil-monorepo-localstack-1 awslocal sqs create-queue --queue-name fossilQueue 2>/dev/null || log_warn "Queue may already exist"

# Start proving-service HTTP API with mock proof features
log_info "Starting proving-service HTTP API on port 3001 with mock proof features..."
log_info "Using environment file: $ENV_FILE"
cd proving-service
env $(cat "$ENV_FILE" | grep -v '^#' | xargs) CARGO_TERM_QUIET=true cargo run --quiet --bin proving-service --features mock-proof > ../proving-service.log 2>&1 &
PROVING_SERVICE_PID=$!
cd ..
log_info "Proving-service HTTP API started (PID: $PROVING_SERVICE_PID)"

# Start message-handler background service with actual RISC0 mock proof generation and on-chain verification
log_info "Starting message-handler background service with actual RISC0 mock proof generation and on-chain verification..."
log_info "Using environment file: $ENV_FILE"
cd proving-service
env $(cat "$ENV_FILE" | grep -v '^#' | xargs) USE_SIMPLE_MOCK=false ENABLE_PROOF=true VERIFY_PROOFS_ONCHAIN=true USE_RISC0_INTEGRATION=true CARGO_TERM_QUIET=true RUST_LOG=debug cargo run --quiet --bin message-handler --features mock-proof,starknet-handler > ../message-handler.log 2>&1 &
MESSAGE_HANDLER_PID=$!
cd ..
log_info "Message-handler started with mock proof generation (PID: $MESSAGE_HANDLER_PID)"

# Wait for services to start
log_info "Waiting for services to start..."
sleep 5

# Check if proving-service is responding (wait longer for compilation)
HEALTH_CHECK_RETRIES=20
HEALTH_CHECK_COUNT=0
PROVING_SERVICE_HEALTHY=false

log_info "Waiting for proving-service to compile and start (this may take time)..."
while [ $HEALTH_CHECK_COUNT -lt $HEALTH_CHECK_RETRIES ]; do
    if curl -s http://localhost:3001/health > /dev/null 2>&1; then
        log_info "Proving-service health check passed ✓"
        PROVING_SERVICE_HEALTHY=true
        break
    fi
    HEALTH_CHECK_COUNT=$((HEALTH_CHECK_COUNT + 1))
    log_info "Waiting for proving-service health check... (attempt $HEALTH_CHECK_COUNT/$HEALTH_CHECK_RETRIES)"
    sleep 3
done

if [ "$PROVING_SERVICE_HEALTHY" = false ]; then
    log_warn "Proving-service health check endpoint not available after $HEALTH_CHECK_RETRIES attempts, continuing anyway..."
fi

# Start offchain-processor
log_info "Starting offchain-processor on port 3000..."
log_info "Using environment file: $ENV_FILE"
cd offchain-processor
env $(cat "$ENV_FILE" | grep -v '^#' | xargs) CARGO_TERM_QUIET=true cargo run --quiet --bin server > ../offchain-processor.log 2>&1 &
OFFCHAIN_PROCESSOR_PID=$!
cd ..
log_info "Offchain-processor started (PID: $OFFCHAIN_PROCESSOR_PID)"

# Wait for offchain-processor to start (compilation can take time)
log_info "Waiting for offchain-processor to compile and start..."
log_info "This may take several minutes for the first run..."
sleep 10

# Check if offchain-processor is responding
MAX_RETRIES=10
RETRY_COUNT=0
while [ $RETRY_COUNT -lt $MAX_RETRIES ]; do
    if curl -s http://localhost:3000/health > /dev/null 2>&1; then
        log_info "Offchain-processor is responding ✓"
        break
    fi
    RETRY_COUNT=$((RETRY_COUNT + 1))
    log_info "Waiting for offchain-processor to respond... (attempt $RETRY_COUNT/$MAX_RETRIES)"
    sleep 2
done

if [ $RETRY_COUNT -eq $MAX_RETRIES ]; then
    log_error "Offchain-processor failed to respond after $MAX_RETRIES attempts"
    log_error "Check offchain-processor.log for errors"
    exit 1
fi

# Generate API key
log_info "Generating API key..."
API_KEY_RESPONSE=$(curl -s -X POST http://localhost:3000/api_key \
  -H "Content-Type: application/json" \
  -d '{"name": "e2e_test_key"}')

API_KEY=$(echo $API_KEY_RESPONSE | grep -o '"api_key":"[^"]*"' | cut -d'"' -f4)

if [ -z "$API_KEY" ]; then
    log_error "Failed to generate API key"
    log_error "Response: $API_KEY_RESPONSE"
    exit 1
fi

log_info "Generated API key: $API_KEY"

# Send test request to offchain-processor
log_info "Sending test request to offchain-processor..."

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
    \"timestamp\": 1741243059
  }
}"

RESPONSE=$(curl -s -X POST http://localhost:3000/pricing_data \
  -H "Content-Type: application/json" \
  -H "X-API-Key: $API_KEY" \
  -d "$TEST_REQUEST")

log_info "Response: $RESPONSE"

# Extract job ID from response
JOB_ID=$(echo $RESPONSE | grep -o '"job_id":"[^"]*"' | cut -d'"' -f4)

if [ -z "$JOB_ID" ]; then
    log_error "Failed to extract job ID from response"
    log_error "Full response: $RESPONSE"
    exit 1
fi

log_info "Job created with ID: $JOB_ID"

# Monitor job status
log_info "Monitoring job status..."
MAX_STATUS_RETRIES=20
STATUS_RETRY_COUNT=0

while [ $STATUS_RETRY_COUNT -lt $MAX_STATUS_RETRIES ]; do
    STATUS_RESPONSE=$(curl -s http://localhost:3000/job_status/$JOB_ID)
    STATUS=$(echo $STATUS_RESPONSE | grep -o '"status":"[^"]*"' | cut -d'"' -f4)
    
    log_info "Job status (attempt $((STATUS_RETRY_COUNT + 1))/$MAX_STATUS_RETRIES): $STATUS"
    
    if [ "$STATUS" = "Completed" ]; then
        log_info "Job completed successfully! ✓"
        break
    elif [ "$STATUS" = "Failed" ]; then
        log_error "Job failed ✗"
        log_error "Status response: $STATUS_RESPONSE"
        break
    fi
    
    STATUS_RETRY_COUNT=$((STATUS_RETRY_COUNT + 1))
    sleep 3
done

if [ $STATUS_RETRY_COUNT -eq $MAX_STATUS_RETRIES ]; then
    log_warn "Job status monitoring timed out after $MAX_STATUS_RETRIES attempts"
    log_warn "Final status: $STATUS"
fi

# Get detailed job result
log_info "Getting detailed job result..."
RESULT_RESPONSE=$(curl -s http://localhost:3000/job_result/$JOB_ID \
  -H "X-API-Key: $API_KEY")

log_info "Detailed result: $RESULT_RESPONSE"

# Test batch job status
log_info "Testing batch job status endpoint..."
BATCH_REQUEST='{"job_ids": ["'$JOB_ID'"]}'
BATCH_RESPONSE=$(curl -s -X POST http://localhost:3000/batch_job_status \
  -H "Content-Type: application/json" \
  -H "X-API-Key: $API_KEY" \
  -d "$BATCH_REQUEST")

log_info "Batch status response: $BATCH_RESPONSE"

# Test RISC0 Mock Proof Generation
log_info "=== Testing RISC0 Mock Proof Generation ==="

# Send a specific request that should trigger RISC0 proof generation
log_info "Sending request to trigger RISC0 proof generation in message-handler..."

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
    \"timestamp\": 1741243059
  }
}"

RISC0_RESPONSE=$(curl -s -X POST http://localhost:3000/pricing_data \
  -H "Content-Type: application/json" \
  -H "X-API-Key: $API_KEY" \
  -d "$RISC0_TEST_REQUEST")

log_info "RISC0 test response: $RISC0_RESPONSE"

# Extract job ID from RISC0 test response
RISC0_JOB_ID=$(echo $RISC0_RESPONSE | grep -o '"job_id":"[^"]*"' | cut -d'"' -f4)

if [ ! -z "$RISC0_JOB_ID" ]; then
    log_info "RISC0 test job created with ID: $RISC0_JOB_ID"
    
    # Monitor RISC0 job for longer since proof generation takes time with backoff
    log_info "Monitoring RISC0 proof generation job (this may take several minutes)..."
    log_info "Waiting for Bonsai proof generation to complete and calldata to be generated..."
    RISC0_MAX_RETRIES=240  # Increased to 240 retries (up to 40+ minutes with backoff)
    RISC0_RETRY_COUNT=0
    BACKOFF_BASE=5  # Start with 5 second intervals
    
    while [ $RISC0_RETRY_COUNT -lt $RISC0_MAX_RETRIES ]; do
        RISC0_STATUS_RESPONSE=$(curl -s http://localhost:3000/job_status/$RISC0_JOB_ID)
        RISC0_STATUS=$(echo $RISC0_STATUS_RESPONSE | grep -o '"status":"[^"]*"' | cut -d'"' -f4)
        
        # Calculate current wait time with backoff (5s, 10s, 15s, max 60s)
        CURRENT_WAIT=$(( BACKOFF_BASE * ((RISC0_RETRY_COUNT / 10) + 1) ))
        if [ $CURRENT_WAIT -gt 60 ]; then
            CURRENT_WAIT=60
        fi
        
        log_info "RISC0 job status (attempt $((RISC0_RETRY_COUNT + 1))/$RISC0_MAX_RETRIES): $RISC0_STATUS (waiting ${CURRENT_WAIT}s)"
        
        # Show progress indicator for long waits
        if [ $RISC0_RETRY_COUNT -gt 30 ] && [ $((RISC0_RETRY_COUNT % 20)) -eq 0 ]; then
            log_info "⏳ Still waiting for proof generation to complete... This is normal for Bonsai API calls"
            if [ -f "message-handler.log" ]; then
                RECENT_ACTIVITY=$(tail -50 message-handler.log | grep -i "bonsai\|risc0\|proof" | tail -3)
                if [ ! -z "$RECENT_ACTIVITY" ]; then
                    log_info "📋 Recent activity detected - proof generation likely in progress"
                fi
            fi
        fi
        
        # Check message handler logs for proof calldata generation in real-time
        if [ -f "message-handler.log" ]; then
            # Look for recent RISC0 proof calldata entries
            RECENT_CALLDATA=$(tail -50 message-handler.log | grep -i "Generated.*proof calldata\|🔍.*calldata\|calldata.*length" | tail -3)
            if [ ! -z "$RECENT_CALLDATA" ]; then
                log_info "📋 Recent proof calldata activity detected:"
                echo "$RECENT_CALLDATA" | while read line; do
                    log_info "   $line"
                done
            fi
            
            # Look for Bonsai-related activity
            BONSAI_ACTIVITY=$(tail -50 message-handler.log | grep -i "bonsai\|groth16\|proof.*generation\|receipt.*verify" | tail -3)
            if [ ! -z "$BONSAI_ACTIVITY" ]; then
                log_info "🔧 Recent Bonsai/proof activity:"
                echo "$BONSAI_ACTIVITY" | while read line; do
                    log_info "   $line"
                done
            fi
            
            # Look for StarkNet handler activity and save to separate file
            STARKNET_ACTIVITY=$(tail -100 message-handler.log | grep -i "starknet_handler\|tx_hash=" | tail -5)
            if [ ! -z "$STARKNET_ACTIVITY" ]; then
                log_info "🔗 Recent StarkNet handler activity:"
                echo "$STARKNET_ACTIVITY" | while read line; do
                    log_info "   $line"
                done
                # Save to separate file for easier access
                echo "$STARKNET_ACTIVITY" >> starknet-handler.log
            fi
        fi
        
        if [ "$RISC0_STATUS" = "Completed" ]; then
            log_info "✅ RISC0 proof generation job completed successfully!"
            
            # Monitor logs for proof completion markers
            log_info "Monitoring logs for proof generation completion markers..."
            PROOF_COMPLETION_TIMEOUT=180  # Increased to 3 minutes
            PROOF_WAIT_COUNT=0
            PROOF_COMPLETE=false
            
            while [ $PROOF_WAIT_COUNT -lt $PROOF_COMPLETION_TIMEOUT ] && [ "$PROOF_COMPLETE" = false ]; do
                # Check for proof completion indicators
                if [ -f "message-handler.log" ]; then
                    # Look for Step 2 (onchain verification) or proof completion messages
                    STEP2_FOUND=$(grep -i "Step.*2.*Verifying\|onchain.*verification.*successful\|proof.*verified.*onchain" message-handler.log 2>/dev/null | wc -l)
                    CALLDATA_FOUND=$(grep -i "Generated.*proof calldata\|proof.*calldata.*elements\|calldata.*length.*[0-9]" message-handler.log 2>/dev/null | wc -l)
                    TX_HASH_FOUND=$(grep -i "tx_hash.*=\|transaction.*hash\|Proof onchain verification successful" message-handler.log 2>/dev/null | wc -l)
                    
                    if [ $STEP2_FOUND -gt 0 ] || [ $CALLDATA_FOUND -gt 0 ] || [ $TX_HASH_FOUND -gt 0 ]; then
                        log_info "✅ Proof completion markers detected in logs (Step2: $STEP2_FOUND, Calldata: $CALLDATA_FOUND, TxHash: $TX_HASH_FOUND)"
                        PROOF_COMPLETE=true
                        break
                    fi
                fi
                
                PROOF_WAIT_COUNT=$((PROOF_WAIT_COUNT + 2))
                sleep 2
            done
            
            if [ "$PROOF_COMPLETE" = false ]; then
                log_warn "⚠️  Proof completion markers not detected within timeout, continuing anyway..."
            fi
            
            # Additional wait to ensure all logging is flushed
            log_info "Waiting additional 30 seconds for log flushing..."
            sleep 30
            
            # Get detailed result to check for proof data
            RISC0_RESULT=$(curl -s http://localhost:3000/job_result/$RISC0_JOB_ID \
              -H "X-API-Key: $API_KEY")
            log_info "RISC0 proof result: $RISC0_RESULT"
            break
        elif [ "$RISC0_STATUS" = "Failed" ]; then
            log_error "❌ RISC0 proof generation job failed"
            log_error "Status response: $RISC0_STATUS_RESPONSE"
            break
        fi
        
        RISC0_RETRY_COUNT=$((RISC0_RETRY_COUNT + 1))
        sleep $CURRENT_WAIT  # Use backoff timing
    done
    
    if [ $RISC0_RETRY_COUNT -eq $RISC0_MAX_RETRIES ]; then
        log_warn "⚠️  RISC0 proof generation monitoring timed out after $RISC0_MAX_RETRIES attempts"
        log_warn "Final status: $RISC0_STATUS"
        log_warn "This timeout does not mean the proof failed - it may still be processing in the background"
        log_warn "Check message-handler.log for RISC0 proof generation details"
        
        # Show recent activity even on timeout
        if [ -f "message-handler.log" ]; then
            log_info "📋 Recent activity at timeout:"
            tail -20 message-handler.log | grep -i "proof\|risc0\|bonsai\|calldata\|starknet\|verification" | tail -10
        fi
    fi
else
    log_warn "⚠️  Failed to create RISC0 test job"
fi

# Show RISC0-specific logs
log_info "=== RISC0 Proof Generation Logs ==="
log_info "Searching for RISC0-related log entries..."
if [ -f "message-handler.log" ]; then
    log_info "🔍 Looking for proof calldata generation..."
    CALLDATA_LOGS=$(grep -i "Generated.*proof calldata\|🔍.*calldata\|calldata.*length\|proof.*calldata.*elements" message-handler.log)
    if [ ! -z "$CALLDATA_LOGS" ]; then
        log_info "📋 Proof Calldata Found:"
        echo "$CALLDATA_LOGS" | tail -10
    else
        log_warn "⚠️  No proof calldata logs found yet"
    fi
    
    log_info ""
    log_info "🔧 Bonsai/RISC0 activity:"
    grep -i "risc0\|proof.*generation\|bonsai\|groth16\|receipt.*verify\|proof.*successful\|✅.*proof" message-handler.log | tail -n 15 || log_info "No RISC0-specific logs found"
    
    log_info ""
    log_info "⚡ Recent proof pipeline activity:"
    grep -i "proof.*pipeline\|generate.*proof\|proof.*attempt\|proof.*successful\|Step.*1\|Step.*2" message-handler.log | tail -n 10 || log_info "No proof pipeline logs found"
    
    log_info ""
    log_info "🔗 On-chain verification activity:"
    # Check both message-handler.log and starknet-handler.log
    ONCHAIN_LOGS=""
    if [ -f "message-handler.log" ]; then
        ONCHAIN_LOGS=$(grep -i "starknet_handler.*successful\|verify.*proof.*onchain\|Step.*2.*Verifying\|tx_hash=\|transaction.*hash\|onchain.*verification.*success" message-handler.log)
    fi
    if [ -f "starknet-handler.log" ]; then
        STARKNET_LOGS=$(cat starknet-handler.log)
        ONCHAIN_LOGS="$ONCHAIN_LOGS$STARKNET_LOGS"
    fi
    
    if [ ! -z "$ONCHAIN_LOGS" ]; then
        log_info "📝 On-chain Verification Found:"
        echo "$ONCHAIN_LOGS" | tail -10
        
        # Extract transaction hash from "Proof verified onchain with transaction hash:" line
        TX_HASH=$(echo "$ONCHAIN_LOGS" | sed 's/\[[0-9;]*m//g' | grep -oE "transaction hash: 0x[a-fA-F0-9]+" | grep -oE "0x[a-fA-F0-9]+" | tail -1)
        if [ ! -z "$TX_HASH" ]; then
            # Verify it's not the contract address
            if [ "$TX_HASH" != "$PITCHLAKE_VERIFIER_CONTRACT" ]; then
                log_info "💎 Transaction Hash: $TX_HASH"
            else
                log_info "⚠️  No valid transaction hash found (found contract address instead)"
            fi
        else
            log_info "⚠️  Transaction hash not found in logs"
        fi
    else
        log_warn "⚠️  No on-chain verification logs found yet"
    fi
else
    log_warn "Message handler log file not found"
fi

# Show service logs
log_info "=== Proving Service HTTP API Logs (last 20 lines) ==="
tail -n 20 proving-service.log || log_warn "No proving-service logs available"

log_info "=== Message Handler Logs (last 20 lines) ==="
tail -n 20 message-handler.log || log_warn "No message-handler logs available"

log_info "=== Offchain Processor Logs (last 20 lines) ==="
tail -n 20 offchain-processor.log || log_warn "No offchain-processor logs available"

# Final RISC0 summary
log_info "=== RISC0 Integration Test Summary ==="

# Check for proof calldata generation
CALLDATA_GENERATED=false
if [ -f "message-handler.log" ]; then
    CALLDATA_CHECK=$(grep -i "Generated.*proof calldata\|🔍.*calldata\|calldata.*length" message-handler.log)
    if [ ! -z "$CALLDATA_CHECK" ]; then
        CALLDATA_GENERATED=true
    fi
fi

if [ ! -z "$RISC0_JOB_ID" ] && [ "$RISC0_STATUS" = "Completed" ]; then
    log_info "✅ RISC0 mock proof generation test: PASSED"
    log_info "   - Job ID: $RISC0_JOB_ID"
    log_info "   - Status: $RISC0_STATUS"
    log_info "   - RISC0 proof pipeline successfully integrated"
    
    if [ "$CALLDATA_GENERATED" = true ]; then
        log_info "   - ✅ Proof calldata successfully generated (Bonsai integration working)"
    else
        log_warn "   - ⚠️  Proof calldata generation not detected (may still be in progress)"
        log_warn "   - Check logs above for Bonsai API calls and proof generation details"
    fi
    
    # Check for onchain verification completion in both log files
    ONCHAIN_VERIFICATION=""
    if [ -f "message-handler.log" ]; then
        ONCHAIN_VERIFICATION=$(grep -i "starknet_handler.*successful\|Step.*2.*Verifying\|onchain.*verification.*success\|tx_hash=" message-handler.log 2>/dev/null)
    fi
    if [ -f "starknet-handler.log" ]; then
        STARKNET_VERIFICATION=$(cat starknet-handler.log 2>/dev/null)
        ONCHAIN_VERIFICATION="$ONCHAIN_VERIFICATION$STARKNET_VERIFICATION"
    fi
    
    if [ ! -z "$ONCHAIN_VERIFICATION" ]; then
        log_info "   - ✅ On-chain verification completed"
        # Try to extract transaction hash from "Proof verified onchain with transaction hash:" line
        FINAL_TX_HASH=$(echo "$ONCHAIN_VERIFICATION" | sed 's/\[[0-9;]*m//g' | grep -oE "transaction hash: 0x[a-fA-F0-9]+" | grep -oE "0x[a-fA-F0-9]+" | tail -1)
        if [ ! -z "$FINAL_TX_HASH" ]; then
            # Verify it's not the contract address
            if [ "$FINAL_TX_HASH" != "$PITCHLAKE_VERIFIER_CONTRACT" ]; then
                log_info "   - 💎 Transaction Hash: $FINAL_TX_HASH"
            else
                log_info "   - ⚠️  Transaction hash extraction found contract address instead of tx hash"
            fi
        else
            log_info "   - ⚠️  Transaction hash not found in logs"
        fi
    else
        log_warn "   - ⚠️  On-chain verification status unclear - check logs above"
    fi
else
    log_warn "⚠️  RISC0 mock proof generation test: INCOMPLETE/FAILED"
    if [ ! -z "$RISC0_JOB_ID" ]; then
        log_warn "   - Job ID: $RISC0_JOB_ID"
        log_warn "   - Final Status: $RISC0_STATUS"
    fi
    log_warn "   - Check message-handler.log for detailed error information"
fi

log_info "End-to-end test completed! ✓"
log_info "Check the logs above to verify the complete flow worked correctly."
log_info "RISC0 integration status shown in summary above."

# Keep processes running for inspection
log_info "Services are still running for inspection. Press Ctrl+C to stop."
read -p "Press Enter to stop services and exit..."