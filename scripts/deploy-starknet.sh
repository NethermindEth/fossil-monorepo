#!/bin/bash

# Ensure the script stops on the first error
set -e

# Store the original directory (works both in container and local environment)
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
        # Use awk to replace the line without temporary files (Docker volume safe)
        awk -v var="$var_name" -v val="$var_value" '
            BEGIN { replaced = 0 }
            $0 ~ "^" var "=" { print var "=" val; replaced = 1; next }
            { print }
            END { if (!replaced) print var "=" val }
        ' "$env_file" > "${env_file}.new" && cat "${env_file}.new" > "$env_file" && rm "${env_file}.new"
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
    local | sepolia | mainnet | docker)
        ENV_TYPE="$1"
        shift
        ;;
    *)
        echo "Unknown option: $1"
        echo "Usage: $0 [--no-build] <environment>"
        echo "Available environments: local, sepolia, mainnet, docker"
        exit 1
        ;;
    esac
done

# Check if environment argument is provided
if [ -z "$ENV_TYPE" ]; then
    echo "Usage: $0 [--no-build] <environment>"
    echo "Available environments: local, sepolia, mainnet, docker"
    exit 1
fi

# Validate environment argument
case "$ENV_TYPE" in
"local" | "sepolia" | "mainnet")
    ENV_FILES=("$ORIGINAL_DIR/.env.$ENV_TYPE")
    echo "Using environment: $ENV_TYPE (${ENV_FILES[0]})"
    ;;
"docker")
    # Update docker env first, then copy values to local env
    ENV_FILES=("$ORIGINAL_DIR/.env.docker")
    SECONDARY_ENV="$ORIGINAL_DIR/.env.local"
    echo "Using environment: $ENV_TYPE (updating ${ENV_FILES[0]} and will sync to $SECONDARY_ENV)"
    ;;
*)
    echo "Invalid environment. Must be one of: local, sepolia, mainnet, docker"
    exit 1
    ;;
esac

# Check if environment files exist
for env_file in "${ENV_FILES[@]}"; do
    if [ ! -f "$env_file" ]; then
        echo "Error: Environment file $env_file not found"
        exit 1
    fi
done

# Source the primary environment file
source "${ENV_FILES[0]}"

STARKNET_CONTRACTS_DIR="$ORIGINAL_DIR/starknet-contracts"

# Define colors
GREEN='\033[0;32m'
BLUE='\033[0;34m'
YELLOW='\033[1;33m'
NC='\033[0m' # No Color
BOLD='\033[1m'
RED='\033[0;31m'

echo -e "\n${BLUE}${BOLD}Deploying StarkNet contracts...${NC}"

# Build and deploy fossil-hash-store contracts
echo -e "\n${YELLOW}Building fossil-hash-store contract...${NC}"
cd "$STARKNET_CONTRACTS_DIR/fossil-hash-store"

if [ "$BUILD" = true ]; then
    echo -e "${BLUE}Building fossil-hash-store...${NC}"
    scarb build
else
    echo -e "${BLUE}Skipping build step for fossil-hash-store as --no-build flag was provided...${NC}"
fi

# Deploy Sha2Input contract
echo -e "${YELLOW}Declaring Sha2Input contract...${NC}"
SHA2INPUT_HASH=$(starkli declare ../target/dev/sha2_input_Sha2Input.contract_class.json --account $STARKNET_ACCOUNT --rpc $STARKNET_RPC_URL --compiler-version 2.9.1 -w | grep -o '0x[a-fA-F0-9]\{64\}' | head -1)
echo -e "${GREEN}Class hash declared: ${BOLD}$SHA2INPUT_HASH${NC}"

echo -e "${YELLOW}Deploying Sha2Input contract...${NC}"
SHA2INPUT_ADDRESS=$(starkli deploy $SHA2INPUT_HASH $STARKNET_ACCOUNT_ADDRESS 0x0 --account $STARKNET_ACCOUNT --rpc $STARKNET_RPC_URL -w | grep -o '0x[a-fA-F0-9]\{64\}' | head -1)
echo -e "${GREEN}Contract address: ${BOLD}$SHA2INPUT_ADDRESS${NC}"
echo

# Build and deploy pitchlake-verifier contracts
echo -e "\n${YELLOW}Building pitchlake-verifier contract...${NC}"
cd "$STARKNET_CONTRACTS_DIR/pitchlake-verifier"

if [ "$BUILD" = true ]; then
    echo -e "${BLUE}Building pitchlake-verifier...${NC}"
    scarb build
else
    echo -e "${BLUE}Skipping build step for pitchlake-verifier as --no-build flag was provided...${NC}"
fi

# Declare and deploy Universal ECIP contract
echo -e "${YELLOW}Declaring Universal ECIP contract...${NC}"
ECIP_HASH=$(starkli declare ../target/dev/pitchlake_verifier_UniversalECIP.contract_class.json --account $STARKNET_ACCOUNT --rpc $STARKNET_RPC_URL --compiler-version 2.9.1 -w | grep -o '0x[a-fA-F0-9]\{64\}' | head -1)
echo -e "${GREEN}Class hash declared: ${BOLD}$ECIP_HASH${NC}"
echo

# Declare and deploy Groth16 Verifier contract
echo -e "${YELLOW}Declaring Groth16 Verifier contract...${NC}"
VERIFIER_HASH=$(starkli declare ../target/dev/pitchlake_verifier_Risc0Groth16VerifierBN254.contract_class.json --account $STARKNET_ACCOUNT --rpc $STARKNET_RPC_URL --compiler-version 2.9.1 -w | grep -o '0x[a-fA-F0-9]\{64\}' | head -1)
echo -e "${GREEN}Class hash declared: ${BOLD}$VERIFIER_HASH${NC}"
echo

echo -e "${YELLOW}Deploying Groth16 Verifier contract...${NC}"
VERIFIER_ADDRESS=$(starkli deploy $VERIFIER_HASH $ECIP_HASH --account $STARKNET_ACCOUNT --rpc $STARKNET_RPC_URL -w | grep -o '0x[a-fA-F0-9]\{64\}' | head -1)
echo -e "${GREEN}Contract deployed at: ${BOLD}$VERIFIER_ADDRESS${NC}"
echo

# Declare and deploy MockPitchLakeVault contract
echo -e "${YELLOW}Declaring MockPitchLakeVault contract...${NC}"
PITCHLAKE_VAULT_HASH=$(starkli declare ../target/dev/pitchlake_verifier_MockPitchLakeVault.contract_class.json --account $STARKNET_ACCOUNT --rpc $STARKNET_RPC_URL --compiler-version 2.9.1 -w | grep -o '0x[a-fA-F0-9]\{64\}' | head -1)
echo -e "${GREEN}Class hash declared: ${BOLD}$PITCHLAKE_VAULT_HASH${NC}"
echo

echo -e "${YELLOW}Deploying MockPitchLakeVault contract...${NC}"
PITCHLAKE_VAULT_ADDRESS=$(starkli deploy $PITCHLAKE_VAULT_HASH --account $STARKNET_ACCOUNT --rpc $STARKNET_RPC_URL -w | grep -o '0x[a-fA-F0-9]\{64\}' | head -1)
echo -e "${GREEN}Contract deployed at: ${BOLD}$PITCHLAKE_VAULT_ADDRESS${NC}"
echo

# Declare and deploy PitchLake Verifier contract
echo -e "${YELLOW}Declaring PitchLake Verifier contract...${NC}"
PITCHLAKE_VERIFIER_HASH=$(starkli declare ../target/dev/pitchlake_verifier_PitchLakeVerifier.contract_class.json --account $STARKNET_ACCOUNT --rpc $STARKNET_RPC_URL --compiler-version 2.9.1 -w | grep -o '0x[a-fA-F0-9]\{64\}' | head -1)
echo -e "${GREEN}Class hash declared: ${BOLD}$PITCHLAKE_VERIFIER_HASH${NC}"
echo

echo -e "${YELLOW}Deploying PitchLake Verifier contract...${NC}"
PITCHLAKE_VERIFIER_ADDRESS=$(starkli deploy $PITCHLAKE_VERIFIER_HASH $VERIFIER_ADDRESS $STARKNET_ACCOUNT_ADDRESS --account $STARKNET_ACCOUNT --rpc $STARKNET_RPC_URL -w | grep -o '0x[a-fA-F0-9]\{64\}' | head -1)
echo -e "${GREEN}Contract deployed at: ${BOLD}$PITCHLAKE_VERIFIER_ADDRESS${NC}"
echo

echo -e "\n${GREEN}${BOLD}All contracts deployed!${NC}"

# Update the environment files with the new addresses
for env_file in "${ENV_FILES[@]}"; do
    if [ ! -f "$env_file" ]; then
        echo -e "${RED}Warning: $env_file not found, skipping...${NC}"
        continue
    fi
    update_env_var "$env_file" "HASH_STORAGE_ADDRESS" "$SHA2INPUT_ADDRESS"
    update_env_var "$env_file" "UNIVERSAL_ECIP_CONTRACT" "$ECIP_HASH"
    update_env_var "$env_file" "GROTH16_VERIFIER_CONTRACT" "$VERIFIER_ADDRESS"
    update_env_var "$env_file" "PITCHLAKE_VAULT" "$PITCHLAKE_VAULT_ADDRESS"
    update_env_var "$env_file" "PITCHLAKE_VERIFIER_CONTRACT" "$PITCHLAKE_VERIFIER_ADDRESS"
done

# If in docker mode, sync the addresses to .env.local
if [ "$ENV_TYPE" = "docker" ] && [ -f "$SECONDARY_ENV" ]; then
    echo -e "${BLUE}Syncing addresses to $SECONDARY_ENV...${NC}"
    update_env_var "$SECONDARY_ENV" "HASH_STORAGE_ADDRESS" "$SHA2INPUT_ADDRESS"
    update_env_var "$SECONDARY_ENV" "UNIVERSAL_ECIP_CONTRACT" "$ECIP_HASH"
    update_env_var "$SECONDARY_ENV" "GROTH16_VERIFIER_CONTRACT" "$VERIFIER_ADDRESS"
    update_env_var "$SECONDARY_ENV" "PITCHLAKE_VAULT" "$PITCHLAKE_VAULT_ADDRESS"
    update_env_var "$SECONDARY_ENV" "PITCHLAKE_VERIFIER_CONTRACT" "$PITCHLAKE_VERIFIER_ADDRESS"
fi

# Return to original directory
cd "$ORIGINAL_DIR"

# Source the updated primary environment file
source "${ENV_FILES[0]}"

echo -e "${GREEN}${BOLD}Environment variables successfully updated in ${ENV_FILES[0]}${NC}"

# Create completion flag for healthcheck
touch /tmp/deployment_complete.flag
