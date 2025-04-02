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
CLASS_HASH_HASH_STORAGE=$(starkli declare ./target/dev/mock_contracts_MockHashStorage.contract_class.json --strk --rpc $STARKNET_RPC --private-key $STARKNET_PRIVATE_KEY --account $STARKNET_ACCOUNT | grep -o '0x[a-fA-F0-9]\{64\}' | head -1)

echo -e "\nDeclaring Fossil Light Client contract"
CLASS_HASH_FOSSIL_LIGHT_CLIENT=$(starkli declare ./target/dev/mock_contracts_MockFossilLightClient.contract_class.json --strk --rpc $STARKNET_RPC --private-key $STARKNET_PRIVATE_KEY --account $STARKNET_ACCOUNT | grep -o '0x[a-fA-F0-9]\{64\}' | head -1)

echo "Class hash Hash Storage: $CLASS_HASH_HASH_STORAGE"
echo "Class hash Fossil Light Client: $CLASS_HASH_FOSSIL_LIGHT_CLIENT"

# Deploy contract
echo "Deploying contracts"
CONTRACT_ADDRESS_HASH_STORAGE=$(starkli deploy $CLASS_HASH_HASH_STORAGE --strk --private-key $STARKNET_PRIVATE_KEY  --account $STARKNET_ACCOUNT --rpc $STARKNET_RPC --salt 1 -w | grep -o '0x[a-fA-F0-9]\{64\}' | head -1)
CONTRACT_ADDRESS_FOSSIL_LIGHT_CLIENT=$(starkli deploy $CLASS_HASH_FOSSIL_LIGHT_CLIENT --strk --private-key $STARKNET_PRIVATE_KEY  --account $STARKNET_ACCOUNT --rpc $STARKNET_RPC --salt 1 -w | grep -o '0x[a-fA-F0-9]\{64\}' | head -1)

echo "Contract address Hash Storage: $CONTRACT_ADDRESS_HASH_STORAGE"
echo "Contract address Fossil Light Client: $CONTRACT_ADDRESS_FOSSIL_LIGHT_CLIENT"