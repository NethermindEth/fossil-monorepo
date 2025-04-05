#!/bin/bash

# Set default env file if no argument is provided
ENV_FILE=".env.local"
if [ $# -gt 0 ]; then
    ENV_FILE="$1"
fi

echo "Deploying to Starknet Devnet using environment file: $ENV_FILE"

# Build contracts
if [ ! -f "./target/dev/mock_contracts_MockHashStorage.contract_class.json" ]; then
    echo "Building contracts..."
    scarb build
else
    echo "Contract already built, skipping build step"
fi

# Load environment variables
source $ENV_FILE

echo "$STARKNET_RPC"

# Declare contract
echo -e "\nDeclaring Hash Store contract"
HASH_STORAGE_CLASS_HASH=$(starkli declare ./target/dev/mock_contracts_MockHashStorage.contract_class.json --private-key $STARKNET_PRIVATE_KEY | grep -o '0x[a-fA-F0-9]\{64\}' | head -1)
echo "Class hash: $HASH_STORAGE_CLASS_HASH"

# Deploy contract
echo "Deploying Hash Storage contract"
HASH_STORAGE_CONTRACT_ADDRESS=$(starkli deploy $HASH_STORAGE_CLASS_HASH --private-key $STARKNET_PRIVATE_KEY --salt 1 -w | grep -o '0x[a-fA-F0-9]\{64\}' | head -1)
echo "Contract address: $HASH_STORAGE_CONTRACT_ADDRESS"

# Declare contract
echo -e "\nDeclaring Fossil Store contract"
FOSSIL_STORE_CLASS_HASH=$(starkli declare ./target/dev/mock_contracts_MockFossilStore.contract_class.json --private-key $STARKNET_PRIVATE_KEY | grep -o '0x[a-fA-F0-9]\{64\}' | head -1)
echo "Class hash: $FOSSIL_STORE_CLASS_HASH"

# Deploy contract
echo "Deploying Fossil Store contract"
FOSSIL_STORE_CONTRACT_ADDRESS=$(starkli deploy $FOSSIL_STORE_CLASS_HASH --private-key $STARKNET_PRIVATE_KEY --salt 1 -w | grep -o '0x[a-fA-F0-9]\{64\}' | head -1)
echo "Contract address: $FOSSIL_STORE_CONTRACT_ADDRESS"

# Update the environment file with new contract addresses
echo "Updating $ENV_FILE with new contract addresses"
# Create a temporary file
TMP_FILE=$(mktemp)
# Copy permissions from original file to preserve them
cp -p "$ENV_FILE" "$TMP_FILE"
# Update environment variables in the temporary file
cat "$ENV_FILE" | sed "s|^export HASH_STORAGE_CONTRACT_ADDRESS=.*$|export HASH_STORAGE_CONTRACT_ADDRESS=$HASH_STORAGE_CONTRACT_ADDRESS|" | \
                 sed "s|^export FOSSIL_STORE_CONTRACT_ADDRESS=.*$|export FOSSIL_STORE_CONTRACT_ADDRESS=$FOSSIL_STORE_CONTRACT_ADDRESS|" > "$TMP_FILE"
# Replace the original file with the temporary file
cat "$TMP_FILE" > "$ENV_FILE"
rm "$TMP_FILE"

echo "Environment file updated with new contract addresses"
