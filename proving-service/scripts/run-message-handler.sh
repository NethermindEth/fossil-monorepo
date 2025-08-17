#!/bin/bash

# Determine if we're running with proof composition
PROOF_FEATURE=""
ENABLE_PROOF="false"
USE_SIMPLE_MOCK="false"

# Process arguments
while [[ $# -gt 0 ]]; do
  case $1 in
    --proof-composition)
      PROOF_FEATURE="--features proof-composition"
      ENABLE_PROOF="true"
      shift
      ;;
    --mock-proof)
      PROOF_FEATURE="--features mock-proof"
      ENABLE_PROOF="true"
      shift
      ;;
    --simple-mock)
      ENABLE_PROOF="true"
      USE_SIMPLE_MOCK="true"
      shift
      ;;
    --enable-proof)
      ENABLE_PROOF="true"
      shift
      ;;
    *)
      echo "Unknown option: $1"
      echo "Usage: $0 [--proof-composition|--mock-proof|--simple-mock] [--enable-proof]"
      exit 1
      ;;
  esac
done

# Set the environment variables
export ENABLE_PROOF=$ENABLE_PROOF
export USE_SIMPLE_MOCK=$USE_SIMPLE_MOCK
export RUST_LOG=debug

echo "Starting message handler service with configuration:"
echo "- ENABLE_PROOF=$ENABLE_PROOF (controls runtime behavior)"
echo "- USE_SIMPLE_MOCK=$USE_SIMPLE_MOCK (uses simplified mock for testing)"
echo "- Feature flags: $PROOF_FEATURE (controls compile-time behavior)"
echo ""

# Run the service with the appropriate environment variables
echo "Building and running message handler..."
cargo run -p message-handler --bin message-handler $PROOF_FEATURE 