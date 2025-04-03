#!/bin/bash
echo "Deploying to Starknet Devnet"

# Build contracts
if [ ! -f "./target/dev/mock_contracts_MockHashStorage.contract_class.json" ]; then
    echo "Building contracts..."
    scarb build
else
    echo "Contract already built, skipping build step"
fi

# Load environment variables
source .env.local

echo "$STARKNET_RPC"

# Declare contract
echo -e "\nDeclaring Hash Store contract"
CLASS_HASH=$(starkli declare ./target/dev/mock_contracts_MockHashStorage.contract_class.json --rpc $STARKNET_RPC --private-key $STARKNET_PRIVATE_KEY --account $STARKNET_ACCOUNT | grep -o '0x[a-fA-F0-9]\{64\}' | head -1)

echo "Class hash: $CLASS_HASH"

# Deploy contract
echo "Contract deployed to $CONTRACT_ADDRESS"
CONTRACT_ADDRESS=$(starkli deploy $CLASS_HASH --private-key $STARKNET_PRIVATE_KEY  --account $STARKNET_ACCOUNT --rpc $STARKNET_RPC --salt 1 -w | grep -o '0x[a-fA-F0-9]\{64\}' | head -1)

echo "Contract address: $CONTRACT_ADDRESS"
