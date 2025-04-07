#!/bin/bash

# Define colors
GREEN='\033[0;32m'
BLUE='\033[0;34m'
YELLOW='\033[0;33m'
RED='\033[0;31m'
NC='\033[0m' # No Color

# Set default env file if no argument is provided
ENV_FILE=".env.local"
if [ $# -gt 0 ]; then
    ENV_FILE="$1"
fi

echo -e "${BLUE}==================================================${NC}"
echo -e "${BLUE}Deploying to Starknet Devnet using environment file: ${YELLOW}$ENV_FILE${NC}"
echo -e "${BLUE}==================================================${NC}\n"

# Build contracts
if [ ! -f "./target/dev/mock_contracts_MockHashStorage.contract_class.json" ]; then
    echo -e "${YELLOW}Building contracts...${NC}"
    scarb build
else
    echo -e "${GREEN}Contract already built, skipping build step${NC}"
fi

echo -e "${BLUE}Using environment file: ${YELLOW}$ENV_FILE${NC}"

# Load environment variables
source $ENV_FILE

export STARKNET_RPC=$STARKNET_RPC
export STARKNET_ACCOUNT=$STARKNET_ACCOUNT

echo -e "${BLUE}Using Starknet RPC: ${YELLOW}$STARKNET_RPC${NC}"

# Declare Hash Storage contract
echo -e "\n${BLUE}==================================================${NC}"
echo -e "${BLUE} Declaring ECIP contract${NC}"
echo -e "${BLUE}==================================================${NC}"
ECIP_CLASS_HASH=$(starkli declare ./target/dev/mock_contracts_UniversalECIP.contract_class.json --private-key $STARKNET_PRIVATE_KEY | grep -o '0x[a-fA-F0-9]\{64\}' | head -1)
echo -e "Class hash: ${GREEN}$ECIP_CLASS_HASH${NC}"

# Declare Risc0 Groth16 Verifier contract
echo -e "\n${BLUE}==================================================${NC}"
echo -e "${BLUE}Declaring and deploying Risc0 Groth16 Verifier contract${NC}"
echo -e "${BLUE}==================================================${NC}"
GROTH16_VERIFIER_CLASS_HASH=$(starkli declare ./target/dev/mock_contracts_Risc0Groth16VerifierBN254.contract_class.json --private-key $STARKNET_PRIVATE_KEY | grep -o '0x[a-fA-F0-9]\{64\}' | head -1)
echo -e "Class hash: ${GREEN}$GROTH16_VERIFIER_CLASS_HASH${NC}"

# Deploy Groth16 Verifier contract
echo -e "${YELLOW}Deploying Groth16 Verifier contract${NC}"
GROTH16_VERIFIER_CONTRACT_ADDRESS=$(starkli deploy $GROTH16_VERIFIER_CLASS_HASH $ECIP_CLASS_HASH --private-key $STARKNET_PRIVATE_KEY --salt 1 -w | grep -o '0x[a-fA-F0-9]\{64\}' | head -1)
echo -e "Contract address: ${GREEN}$GROTH16_VERIFIER_CONTRACT_ADDRESS${NC}"

# Declare PitchLake Client contract
echo -e "\n${BLUE}==================================================${NC}"
echo -e "${BLUE}Declaring and deploying PitchLake Client contract${NC}"
echo -e "${BLUE}==================================================${NC}"
PITCH_LAKE_CLIENT_CLASS_HASH=$(starkli declare ./target/dev/mock_contracts_MockPitchLakeClient.contract_class.json --private-key $STARKNET_PRIVATE_KEY | grep -o '0x[a-fA-F0-9]\{64\}' | head -1)
echo -e "Class hash: ${GREEN}$PITCH_LAKE_CLIENT_CLASS_HASH${NC}"

# Deploy PitchLake Client contract
echo -e "${YELLOW}Deploying PitchLake Client contract${NC}"
PITCH_LAKE_CLIENT_CONTRACT_ADDRESS=$(starkli deploy $PITCH_LAKE_CLIENT_CLASS_HASH --private-key $STARKNET_PRIVATE_KEY --salt 1 -w | grep -o '0x[a-fA-F0-9]\{64\}' | head -1)
echo -e "Contract address: ${GREEN}$PITCH_LAKE_CLIENT_CONTRACT_ADDRESS${NC}"

# Declare Pitch Lake Verifier contract
echo -e "\n${BLUE}==================================================${NC}"
echo -e "${BLUE}Declaring and deploying Pitch Lake Verifier contract${NC}"
echo -e "${BLUE}==================================================${NC}"
PITCH_LAKE_VERIFIER_CLASS_HASH=$(starkli declare ./target/dev/mock_contracts_PitchLakeVerifier.contract_class.json --private-key $STARKNET_PRIVATE_KEY | grep -o '0x[a-fA-F0-9]\{64\}' | head -1)
echo -e "Class hash: ${GREEN}$PITCH_LAKE_VERIFIER_CLASS_HASH${NC}"

echo -e "${BLUE}STARKNET_ACCOUNT_ADDRESS: ${YELLOW}$STARKNET_ACCOUNT_ADDRESS${NC}"

# Deploy Pitch Lake Verifier contract
echo -e "${YELLOW}Deploying Pitch Lake Verifier contract${NC}"
PITCH_LAKE_VERIFIER_CONTRACT_ADDRESS=$(starkli deploy $PITCH_LAKE_VERIFIER_CLASS_HASH $GROTH16_VERIFIER_CONTRACT_ADDRESS $PITCH_LAKE_CLIENT_CONTRACT_ADDRESS $STARKNET_ACCOUNT_ADDRESS --private-key $STARKNET_PRIVATE_KEY --salt 1 -w | grep -o '0x[a-fA-F0-9]\{64\}' | head -1)
echo -e "Contract address: ${GREEN}$PITCH_LAKE_VERIFIER_CONTRACT_ADDRESS${NC}"

# Declare Fossil Store contract
echo -e "\n${BLUE}==================================================${NC}"
echo -e "${BLUE}Declaring and deploying Fossil Store contract${NC}"
echo -e "${BLUE}==================================================${NC}"
FOSSIL_STORE_CLASS_HASH=$(starkli declare ./target/dev/mock_contracts_MockFossilStore.contract_class.json --private-key $STARKNET_PRIVATE_KEY | grep -o '0x[a-fA-F0-9]\{64\}' | head -1)
echo -e "Class hash: ${GREEN}$FOSSIL_STORE_CLASS_HASH${NC}"

# Deploy Fossil Store contract
echo -e "${YELLOW}Deploying Fossil Store contract${NC}"
FOSSIL_STORE_CONTRACT_ADDRESS=$(starkli deploy $FOSSIL_STORE_CLASS_HASH --private-key $STARKNET_PRIVATE_KEY --salt 1 -w | grep -o '0x[a-fA-F0-9]\{64\}' | head -1)
echo -e "Contract address: ${GREEN}$FOSSIL_STORE_CONTRACT_ADDRESS${NC}"

# Declare Hash Storage contract
echo -e "\n${BLUE}==================================================${NC}"
echo -e "${BLUE}Declaring and deploying Hash Store contract${NC}"
echo -e "${BLUE}==================================================${NC}"
HASH_STORAGE_CLASS_HASH=$(starkli declare ./target/dev/mock_contracts_MockHashStorage.contract_class.json --private-key $STARKNET_PRIVATE_KEY | grep -o '0x[a-fA-F0-9]\{64\}' | head -1)
echo -e "Class hash: ${GREEN}$HASH_STORAGE_CLASS_HASH${NC}"

# Deploy Hash Storage contract
echo -e "${YELLOW}Deploying Hash Storage contract${NC}"
HASH_STORAGE_CONTRACT_ADDRESS=$(starkli deploy $HASH_STORAGE_CLASS_HASH $FOSSIL_STORE_CONTRACT_ADDRESS --private-key $STARKNET_PRIVATE_KEY --salt 1 -w | grep -o '0x[a-fA-F0-9]\{64\}' | head -1)
echo -e "Contract address: ${GREEN}$HASH_STORAGE_CONTRACT_ADDRESS${NC}"

# Update the environment file with new contract addresses
echo -e "\n${BLUE}==================================================${NC}"
echo -e "${BLUE}Updating ${YELLOW}$ENV_FILE${BLUE} with new contract addresses${NC}"
echo -e "${BLUE}==================================================${NC}"
# Create a temporary file
TMP_FILE=$(mktemp)
# Copy permissions from original file to preserve them
cp -p "$ENV_FILE" "$TMP_FILE"
# Update environment variables in the temporary file
cat "$ENV_FILE" | sed "s|^GROTH16_VERIFIER_CONTRACT_ADDRESS=.*$|GROTH16_VERIFIER_CONTRACT_ADDRESS=$GROTH16_VERIFIER_CONTRACT_ADDRESS|g" \
                | sed "s|^PITCH_LAKE_VERIFIER_CONTRACT_ADDRESS=.*$|PITCH_LAKE_VERIFIER_CONTRACT_ADDRESS=$PITCH_LAKE_VERIFIER_CONTRACT_ADDRESS|g" \
                | sed "s|^FOSSIL_STORE_CONTRACT_ADDRESS=.*$|FOSSIL_STORE_CONTRACT_ADDRESS=$FOSSIL_STORE_CONTRACT_ADDRESS|g" \
                | sed "s|^HASH_STORAGE_CONTRACT_ADDRESS=.*$|HASH_STORAGE_CONTRACT_ADDRESS=$HASH_STORAGE_CONTRACT_ADDRESS|g" > "$TMP_FILE"
# Replace the original file with the temporary file
cat "$TMP_FILE" > "$ENV_FILE"
rm "$TMP_FILE"

echo -e "\n${GREEN}✅ Deployment complete! Environment file updated with new contract addresses${NC}"
