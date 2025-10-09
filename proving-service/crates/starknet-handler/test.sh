#!/bin/bash

# Test script for starknet-handler crate

echo "🧪 Running StarkNet Handler Tests"
echo "================================="

# Check if local StarkNet node is running
echo "📡 Checking if local StarkNet node is running on localhost:5050..."
if curl -s -X POST http://localhost:5050 \
  -H "Content-Type: application/json" \
  -d '{"jsonrpc":"2.0","method":"starknet_chainId","params":[],"id":1}' \
  | grep -q "result"; then
    echo "✅ Local StarkNet node is running"
    NODE_RUNNING=true
else
    echo "❌ Local StarkNet node is not running on localhost:5050"
    echo "ℹ️  Integration tests will be skipped"
    NODE_RUNNING=false
fi

echo ""

# Run unit tests (always available)
echo "🔬 Running unit tests..."
cargo test --lib --package starknet-handler

echo ""

# Run integration tests only if node is running
if [ "$NODE_RUNNING" = true ]; then
    echo "🌐 Running integration tests with local node..."
    echo "⚠️  Note: Some tests may fail if the fossil_store contract is not deployed"
    
    # Load environment from .env.test if it exists
    if [ -f ".env.test" ]; then
        echo "📄 Loading configuration from .env.test..."
        export $(cat .env.test | grep -v '^#' | xargs)
    else
        echo "⚠️  .env.test file not found, using default values"
        # Set test environment variables
        export STARKNET_RPC=http://localhost:5050
        export FOSSIL_STORE_ADDRESS=0x027a6e498bfad98a5145ac507f90fac6268cce6c0373c6eecd38de46be655b55
        export STARKNET_MAX_RETRIES=3
        export STARKNET_INITIAL_BACKOFF_MS=100
        export STARKNET_MAX_BACKOFF_MS=1000
    fi
    
    echo "   Using contract address: $FOSSIL_STORE_ADDRESS"
    
    cargo test --package starknet-handler -- --ignored
else
    echo "⏭️  Skipping integration tests (no local node)"
    echo ""
    echo "To run integration tests:"
    echo "1. Start a local StarkNet node on localhost:5050"
    echo "2. Deploy the fossil_store contract with get_avg_fees_in_range function"
    echo "3. Update FOSSIL_STORE_ADDRESS in .env.test"
    echo "4. Run: cargo test --package starknet-handler -- --ignored"
fi

echo ""
echo "📋 Test Summary:"
echo "- Unit tests: Always run"
echo "- Integration tests: Require local StarkNet node"
echo "- Use --ignored flag to run integration tests manually"

echo ""
echo "🔧 Example usage test:"
echo "cargo run --bin starknet-handler-example --features binary"