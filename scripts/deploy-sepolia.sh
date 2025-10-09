#!/bin/bash

# Ensure the script stops on the first error
set -e

# Store the original directory
ORIGINAL_DIR="$(pwd)"

# Default build flag (true means we will build)
BUILD=true

# Update the environment file with new addresses
update_env_var() {
    local env_file=$1
    local var_name=$2
    local var_value=$3

    if grep -q "^$var_name=" "$env_file"; then
        echo -e "${BLUE}$var_name already exists, replacing in $env_file...${NC}"
        sed -i "s|^$var_name=.*|$var_name=$var_value|" "$env_file"
    else
        echo -e "${BLUE}Appending $var_name to $env_file...${NC}"
        echo "$var_name=$var_value" >>"$env_file"
    fi
}

# Parse command line arguments
while [[ $# -gt 0 ]]; do
    case "$1" in
    --no-build)
        BUILD=false
        shift
        ;;
    *)
        echo "Unknown option: $1"
        echo "Usage: $0 [--no-build]"
        exit 1
        ;;
    esac
done

# Set environment file
ENV_FILE="$ORIGINAL_DIR/.env.sepolia"

# Check if environment file exists
if [ ! -f "$ENV_FILE" ]; then
    echo "Error: Environment file $ENV_FILE not found"
    exit 1
fi

# Source the environment file
source "$ENV_FILE"

STARKNET_DIR="$ORIGINAL_DIR/starknet-contracts"

# Define colors
GREEN='\033[0;32m'
BLUE='\033[0;34m'
YELLOW='\033[1;33m'
NC='\033[0m' # No Color
BOLD='\033[1m'
RED='\033[0;31m'

# Change to starknet contracts directory
cd "$STARKNET_DIR"

# Conditionally build the contracts
if [ "$BUILD" = true ]; then
    echo -e "\n${BLUE}${BOLD}Building Starknet contracts...${NC}"
    scarb build
else
    echo -e "\n${BLUE}${BOLD}Skipping build step as --no-build flag was provided...${NC}"
fi

echo -e "\n${BLUE}${BOLD}Deploying Starknet contracts to Sepolia...${NC}"

# ============================================================================
# Deploy Fossil Hash Store contracts (Sha2Input)
# ============================================================================
echo -e "\n${YELLOW}Declaring Sha2Input contract...${NC}"
SHA2INPUT_HASH=$(starkli declare ./target/dev/sha2_input_Sha2Input.contract_class.json --account $STARKNET_ACCOUNT --rpc $STARKNET_RPC_URL --private-key $STARKNET_PRIVATE_KEY -w | grep -o '0x[a-fA-F0-9]\{64\}' | head -1)
echo -e "${GREEN}Class hash declared: ${BOLD}$SHA2INPUT_HASH${NC}"
echo

echo -e "${YELLOW}Deploying Sha2Input contract...${NC}"
SHA2INPUT_ADDRESS=$(starkli deploy $SHA2INPUT_HASH $STARKNET_ACCOUNT_ADDRESS $FOSSIL_STORE_ADDRESS --account $STARKNET_ACCOUNT --rpc $STARKNET_RPC_URL --private-key $STARKNET_PRIVATE_KEY -w | grep -o '0x[a-fA-F0-9]\{64\}' | head -1)
echo -e "${GREEN}Contract address: ${BOLD}$SHA2INPUT_ADDRESS${NC}"
echo

# ============================================================================
# Deploy PitchLake Verifier contracts
# ============================================================================

# Declare and deploy Universal ECIP contract
echo -e "\n${YELLOW}Declaring Universal ECIP contract...${NC}"
ECIP_HASH=$(starkli declare ./target/dev/pitchlake_verifier_UniversalECIP.contract_class.json --account $STARKNET_ACCOUNT --rpc $STARKNET_RPC_URL --private-key $STARKNET_PRIVATE_KEY -w | grep -o '0x[a-fA-F0-9]\{64\}' | head -1)
echo -e "${GREEN}Class hash declared: ${BOLD}$ECIP_HASH${NC}"
echo

# Declare and deploy Groth16 Verifier contract
echo -e "${YELLOW}Declaring Groth16 Verifier contract...${NC}"
VERIFIER_HASH=$(starkli declare ./target/dev/pitchlake_verifier_Risc0Groth16VerifierBN254.contract_class.json --account $STARKNET_ACCOUNT --rpc $STARKNET_RPC_URL --private-key $STARKNET_PRIVATE_KEY -w | grep -o '0x[a-fA-F0-9]\{64\}' | head -1)
echo -e "${GREEN}Class hash declared: ${BOLD}$VERIFIER_HASH${NC}"
echo

echo -e "${YELLOW}Deploying Groth16 Verifier contract...${NC}"
VERIFIER_ADDRESS=$(starkli deploy $VERIFIER_HASH $ECIP_HASH --account $STARKNET_ACCOUNT --rpc $STARKNET_RPC_URL --private-key $STARKNET_PRIVATE_KEY -w | grep -o '0x[a-fA-F0-9]\{64\}' | head -1)
echo -e "${GREEN}Contract deployed at: ${BOLD}$VERIFIER_ADDRESS${NC}"
echo

# Declare and deploy PitchLake Verifier contract
echo -e "${YELLOW}Declaring PitchLake Verifier contract...${NC}"
PITCHLAKE_VERIFIER_HASH=$(starkli declare ./target/dev/pitchlake_verifier_PitchLakeVerifier.contract_class.json --account $STARKNET_ACCOUNT --rpc $STARKNET_RPC_URL --private-key $STARKNET_PRIVATE_KEY -w | grep -o '0x[a-fA-F0-9]\{64\}' | head -1)
echo -e "${GREEN}Class hash declared: ${BOLD}$PITCHLAKE_VERIFIER_HASH${NC}"
echo

echo -e "${YELLOW}Deploying PitchLake Verifier contract...${NC}"
PITCHLAKE_VERIFIER_ADDRESS=$(starkli deploy $PITCHLAKE_VERIFIER_HASH $VERIFIER_ADDRESS $STARKNET_ACCOUNT_ADDRESS --account $STARKNET_ACCOUNT --rpc $STARKNET_RPC_URL --private-key $STARKNET_PRIVATE_KEY -w | grep -o '0x[a-fA-F0-9]\{64\}' | head -1)
echo -e "${GREEN}Contract deployed at: ${BOLD}$PITCHLAKE_VERIFIER_ADDRESS${NC}"
echo

echo -e "\n${GREEN}${BOLD}All contracts deployed!${NC}"

# ============================================================================
# Update environment file
# ============================================================================
echo -e "\n${BLUE}${BOLD}Updating environment variables in $ENV_FILE...${NC}"

update_env_var "$ENV_FILE" "UNIVERSAL_ECIP_CONTRACT" "$ECIP_HASH"
update_env_var "$ENV_FILE" "GROTH16_VERIFIER_CONTRACT" "$VERIFIER_ADDRESS"
update_env_var "$ENV_FILE" "PITCHLAKE_VERIFIER_CONTRACT" "$PITCHLAKE_VERIFIER_ADDRESS"
update_env_var "$ENV_FILE" "HASH_STORAGE_ADDRESS" "$SHA2INPUT_ADDRESS"

# Update legacy naming for proving service compatibility
update_env_var "$ENV_FILE" "PITCH_LAKE_VERIFIER_CONTRACT_ADDRESS" "$PITCHLAKE_VERIFIER_ADDRESS"

# Return to original directory
cd "$ORIGINAL_DIR"

# Source the updated environment file
source "$ENV_FILE"

echo -e "\n${GREEN}${BOLD}Environment variables successfully updated in $ENV_FILE${NC}"
echo -e "\n${BLUE}${BOLD}Deployment Summary:${NC}"
echo -e "${YELLOW}Universal ECIP (class hash):${NC} ${BOLD}$ECIP_HASH${NC}"
echo -e "${YELLOW}Groth16 Verifier:${NC} ${BOLD}$VERIFIER_ADDRESS${NC}"
echo -e "${YELLOW}PitchLake Verifier:${NC} ${BOLD}$PITCHLAKE_VERIFIER_ADDRESS${NC}"
echo -e "${YELLOW}Sha2Input (Hash Storage):${NC} ${BOLD}$SHA2INPUT_ADDRESS${NC}"
echo

echo -e "${GREEN}${BOLD}Deployment complete!${NC}"
