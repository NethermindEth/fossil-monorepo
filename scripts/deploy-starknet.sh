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
SHA2INPUT_HASH=$(starkli declare ../target/dev/sha2_input_Sha2Input.contract_class.json --account $STARKNET_ACCOUNT --rpc $STARKNET_RPC_URL  -w | grep -o '0x[a-fA-F0-9]\{64\}' | head -1)
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
ECIP_HASH=$(starkli declare ../target/dev/pitchlake_verifier_UniversalECIP.contract_class.json --account $STARKNET_ACCOUNT --rpc $STARKNET_RPC_URL -w | grep -o '0x[a-fA-F0-9]\{64\}' | head -1)
echo -e "${GREEN}Class hash declared: ${BOLD}$ECIP_HASH${NC}"
echo

# Declare and deploy Groth16 Verifier contract
echo -e "${YELLOW}Declaring Groth16 Verifier contract...${NC}"
VERIFIER_HASH=$(starkli declare ../target/dev/pitchlake_verifier_Risc0Groth16VerifierBN254.contract_class.json --account $STARKNET_ACCOUNT --rpc $STARKNET_RPC_URL -w | grep -o '0x[a-fA-F0-9]\{64\}' | head -1)
echo -e "${GREEN}Class hash declared: ${BOLD}$VERIFIER_HASH${NC}"
echo

echo -e "${YELLOW}Deploying Groth16 Verifier contract...${NC}"
VERIFIER_ADDRESS=$(starkli deploy $VERIFIER_HASH $ECIP_HASH --account $STARKNET_ACCOUNT --rpc $STARKNET_RPC_URL -w | grep -o '0x[a-fA-F0-9]\{64\}' | head -1)
echo -e "${GREEN}Contract deployed at: ${BOLD}$VERIFIER_ADDRESS${NC}"
echo

# Declare and deploy PitchLake Verifier contract
echo -e "${YELLOW}Declaring PitchLake Verifier contract...${NC}"
PITCHLAKE_VERIFIER_HASH=$(starkli declare ../target/dev/pitchlake_verifier_PitchLakeVerifier.contract_class.json --account $STARKNET_ACCOUNT --rpc $STARKNET_RPC_URL -w | grep -o '0x[a-fA-F0-9]\{64\}' | head -1)
echo -e "${GREEN}Class hash declared: ${BOLD}$PITCHLAKE_VERIFIER_HASH${NC}"
echo

echo -e "${YELLOW}Deploying PitchLake Verifier contract...${NC}"
PITCHLAKE_VERIFIER_ADDRESS=$(starkli deploy $PITCHLAKE_VERIFIER_HASH $VERIFIER_ADDRESS $STARKNET_ACCOUNT_ADDRESS --account $STARKNET_ACCOUNT --rpc $STARKNET_RPC_URL -w | grep -o '0x[a-fA-F0-9]\{64\}' | head -1)
echo -e "${GREEN}Contract deployed at: ${BOLD}$PITCHLAKE_VERIFIER_ADDRESS${NC}"
echo

# Declare OptionRound contract
echo -e "${YELLOW}Declaring OptionRound contract...${NC}"
OPTION_ROUND_CLASS_HASH=$(starkli declare ../target/dev/pitchlake_verifier_OptionRound.contract_class.json --account $STARKNET_ACCOUNT --rpc $STARKNET_RPC_URL -w | grep -o '0x[a-fA-F0-9]\{64\}' | head -1)
echo -e "${GREEN}OptionRound class hash declared: ${BOLD}$OPTION_ROUND_CLASS_HASH${NC}"
echo

# Declare and deploy PitchLake Vault contract
echo -e "${YELLOW}Declaring PitchLake Vault contract...${NC}"
PITCHLAKE_VAULT_HASH=$(starkli declare ../target/dev/pitchlake_verifier_Vault.contract_class.json --account $STARKNET_ACCOUNT --rpc $STARKNET_RPC_URL -w | grep -o '0x[a-fA-F0-9]\{64\}' | head -1)
echo -e "${GREEN}Class hash declared: ${BOLD}$PITCHLAKE_VAULT_HASH${NC}"
echo

# Constructor arguments for PitchLake Vault
ETH_ADDRESS=0x49d36570d4e46f48e99674bd3fcc84644ddd6b96f7c741b1562b82f9e004dc7  # Mainnet ETH address
ALPHA=5000  # 50% risk factor in basis points
STRIKE_LEVEL=0  # Strike level (0 = at the money)
ROUND_TRANSITION_DURATION=180  # 3 minutes
AUCTION_DURATION=180  # 3 minutes
ROUND_DURATION=720  # 12 minutes
PROGRAM_ID=0x504954434c4c414b455f5631  # 'PITCHLAKE_V1' as felt252
PROVING_DELAY=120  # 2 minutes

echo -e "${YELLOW}Deploying PitchLake Vault contract...${NC}"
echo -e "${BLUE}Vault constructor arguments:${NC}"
echo "  PITCHLAKE_VAULT_HASH: $PITCHLAKE_VAULT_HASH"
echo "  PITCHLAKE_VERIFIER_ADDRESS: $PITCHLAKE_VERIFIER_ADDRESS"
echo "  ETH_ADDRESS: $ETH_ADDRESS"
echo "  OPTION_ROUND_CLASS_HASH: $OPTION_ROUND_CLASS_HASH"
echo "  ALPHA: $ALPHA"
echo "  STRIKE_LEVEL: $STRIKE_LEVEL"
echo "  ROUND_TRANSITION_DURATION: $ROUND_TRANSITION_DURATION"
echo "  AUCTION_DURATION: $AUCTION_DURATION"
echo "  ROUND_DURATION: $ROUND_DURATION"
echo "  PROGRAM_ID: $PROGRAM_ID"
echo "  PROVING_DELAY: $PROVING_DELAY"
echo
PITCHLAKE_VAULT_ADDRESS=$(starkli deploy $PITCHLAKE_VAULT_HASH $PITCHLAKE_VERIFIER_ADDRESS $ETH_ADDRESS $OPTION_ROUND_CLASS_HASH $ALPHA $STRIKE_LEVEL $ROUND_TRANSITION_DURATION $AUCTION_DURATION $ROUND_DURATION $PROGRAM_ID $PROVING_DELAY --account $STARKNET_ACCOUNT --rpc $STARKNET_RPC_URL -w | grep -o '0x[a-fA-F0-9]\{64\}' | head -1)
echo -e "${GREEN}Contract deployed at: ${BOLD}$PITCHLAKE_VAULT_ADDRESS${NC}"
echo

# Deploy OptionRound contract
echo -e "${YELLOW}Deploying OptionRound contract...${NC}"
# Constructor arguments for OptionRound (using ConstructorArgs struct)
ROUND_ID=1  # First round
STRIKE_PRICE=1000000000000000000  # 1 ETH in wei
CAP_LEVEL=200  # 200% cap level (2x)
RESERVE_PRICE=100000000000000000  # 0.1 ETH reserve price

echo -e "${BLUE}OptionRound constructor arguments:${NC}"
echo "  OPTION_ROUND_CLASS_HASH: $OPTION_ROUND_CLASS_HASH"
echo "  PITCHLAKE_VAULT_ADDRESS: $PITCHLAKE_VAULT_ADDRESS"
echo "  ROUND_ID: $ROUND_ID"
echo "  STRIKE_PRICE: $STRIKE_PRICE"
echo "  CAP_LEVEL: $CAP_LEVEL"
echo "  RESERVE_PRICE: $RESERVE_PRICE"
echo "  ROUND_TRANSITION_DURATION: $ROUND_TRANSITION_DURATION"
echo "  AUCTION_DURATION: $AUCTION_DURATION"
echo "  ROUND_DURATION: $ROUND_DURATION"
echo

# Deploy with constructor args: vault_address, round_id, pricing_data.strike_price, pricing_data.cap_level, pricing_data.reserve_price, round_transition_duration, auction_duration, round_duration
OPTION_ROUND_ADDRESS=$(starkli deploy $OPTION_ROUND_CLASS_HASH $PITCHLAKE_VAULT_ADDRESS $ROUND_ID u256:$STRIKE_PRICE $CAP_LEVEL u256:$RESERVE_PRICE $ROUND_TRANSITION_DURATION $AUCTION_DURATION $ROUND_DURATION --account $STARKNET_ACCOUNT --rpc $STARKNET_RPC_URL -w | grep -o '0x[a-fA-F0-9]\{64\}' | head -1)
echo -e "${GREEN}OptionRound deployed at: ${BOLD}$OPTION_ROUND_ADDRESS${NC}"
echo

# Store first vault for reference (12 minute vault)
PITCHLAKE_VAULT_ADDRESS_12MIN=$PITCHLAKE_VAULT_ADDRESS
OPTION_ROUND_ADDRESS_12MIN=$OPTION_ROUND_ADDRESS

# Deploy 3 hour vault
echo -e "${YELLOW}Deploying 3 hour PitchLake Vault contract...${NC}"
ROUND_TRANSITION_DURATION_3H=1800  # 30 minutes
AUCTION_DURATION_3H=1800           # 30 minutes
ROUND_DURATION_3H=10800            # 3 hours
ALPHA_3H=2500                      # 25% risk factor

echo -e "${BLUE}3 Hour Vault constructor arguments:${NC}"
echo "  PITCHLAKE_VERIFIER_ADDRESS: $PITCHLAKE_VERIFIER_ADDRESS"
echo "  ETH_ADDRESS: $ETH_ADDRESS"
echo "  OPTION_ROUND_CLASS_HASH: $OPTION_ROUND_CLASS_HASH"
echo "  ALPHA: $ALPHA_3H"
echo "  STRIKE_LEVEL: $STRIKE_LEVEL"
echo "  ROUND_TRANSITION_DURATION: $ROUND_TRANSITION_DURATION_3H"
echo "  AUCTION_DURATION: $AUCTION_DURATION_3H"
echo "  ROUND_DURATION: $ROUND_DURATION_3H"
echo "  PROGRAM_ID: $PROGRAM_ID"
echo "  PROVING_DELAY: $PROVING_DELAY"
echo

sleep 5
PITCHLAKE_VAULT_ADDRESS_3H=$(starkli deploy $PITCHLAKE_VAULT_HASH $PITCHLAKE_VERIFIER_ADDRESS $ETH_ADDRESS $OPTION_ROUND_CLASS_HASH $ALPHA_3H $STRIKE_LEVEL $ROUND_TRANSITION_DURATION_3H $AUCTION_DURATION_3H $ROUND_DURATION_3H $PROGRAM_ID $PROVING_DELAY --account $STARKNET_ACCOUNT --rpc $STARKNET_RPC_URL -w | grep -o '0x[a-fA-F0-9]\{64\}' | head -1)
echo -e "${GREEN}3 Hour Vault deployed at: ${BOLD}$PITCHLAKE_VAULT_ADDRESS_3H${NC}"
echo

# Deploy OptionRound for 3 hour vault
echo -e "${YELLOW}Deploying OptionRound for 3 hour vault...${NC}"
sleep 5
OPTION_ROUND_ADDRESS_3H=$(starkli deploy $OPTION_ROUND_CLASS_HASH $PITCHLAKE_VAULT_ADDRESS_3H $ROUND_ID u256:$STRIKE_PRICE $CAP_LEVEL u256:$RESERVE_PRICE $ROUND_TRANSITION_DURATION_3H $AUCTION_DURATION_3H $ROUND_DURATION_3H --account $STARKNET_ACCOUNT --rpc $STARKNET_RPC_URL -w | grep -o '0x[a-fA-F0-9]\{64\}' | head -1)
echo -e "${GREEN}3 Hour OptionRound deployed at: ${BOLD}$OPTION_ROUND_ADDRESS_3H${NC}"
echo

# Deploy 1 month vault
echo -e "${YELLOW}Deploying 1 month PitchLake Vault contract...${NC}"
ROUND_TRANSITION_DURATION_1M=10800  # 3 hours
AUCTION_DURATION_1M=10800           # 3 hours
ROUND_DURATION_1M=2592000           # 1 month (30 days)
ALPHA_1M=1250                       # 12.5% risk factor

echo -e "${BLUE}1 Month Vault constructor arguments:${NC}"
echo "  PITCHLAKE_VERIFIER_ADDRESS: $PITCHLAKE_VERIFIER_ADDRESS"
echo "  ETH_ADDRESS: $ETH_ADDRESS"
echo "  OPTION_ROUND_CLASS_HASH: $OPTION_ROUND_CLASS_HASH"
echo "  ALPHA: $ALPHA_1M"
echo "  STRIKE_LEVEL: $STRIKE_LEVEL"
echo "  ROUND_TRANSITION_DURATION: $ROUND_TRANSITION_DURATION_1M"
echo "  AUCTION_DURATION: $AUCTION_DURATION_1M"
echo "  ROUND_DURATION: $ROUND_DURATION_1M"
echo "  PROGRAM_ID: $PROGRAM_ID"
echo "  PROVING_DELAY: $PROVING_DELAY"
echo

sleep 5
PITCHLAKE_VAULT_ADDRESS_1M=$(starkli deploy $PITCHLAKE_VAULT_HASH $PITCHLAKE_VERIFIER_ADDRESS $ETH_ADDRESS $OPTION_ROUND_CLASS_HASH $ALPHA_1M $STRIKE_LEVEL $ROUND_TRANSITION_DURATION_1M $AUCTION_DURATION_1M $ROUND_DURATION_1M $PROGRAM_ID $PROVING_DELAY --account $STARKNET_ACCOUNT --rpc $STARKNET_RPC_URL -w | grep -o '0x[a-fA-F0-9]\{64\}' | head -1)
echo -e "${GREEN}1 Month Vault deployed at: ${BOLD}$PITCHLAKE_VAULT_ADDRESS_1M${NC}"
echo

# Deploy OptionRound for 1 month vault
echo -e "${YELLOW}Deploying OptionRound for 1 month vault...${NC}"
sleep 5
OPTION_ROUND_ADDRESS_1M=$(starkli deploy $OPTION_ROUND_CLASS_HASH $PITCHLAKE_VAULT_ADDRESS_1M $ROUND_ID u256:$STRIKE_PRICE $CAP_LEVEL u256:$RESERVE_PRICE $ROUND_TRANSITION_DURATION_1M $AUCTION_DURATION_1M $ROUND_DURATION_1M --account $STARKNET_ACCOUNT --rpc $STARKNET_RPC_URL -w | grep -o '0x[a-fA-F0-9]\{64\}' | head -1)
echo -e "${GREEN}1 Month OptionRound deployed at: ${BOLD}$OPTION_ROUND_ADDRESS_1M${NC}"
echo

echo -e "${GREEN}${BOLD}All vaults deployed:${NC}"
echo "  12 Min Vault: $PITCHLAKE_VAULT_ADDRESS_12MIN"
echo "  3 Hour Vault: $PITCHLAKE_VAULT_ADDRESS_3H"
echo "  1 Month Vault: $PITCHLAKE_VAULT_ADDRESS_1M"
echo

# Initialize vaults with pricing data requests
echo -e "${YELLOW}${BOLD}Initializing vaults with pricing data...${NC}"

# Initialize 12 minute vault
echo -e "${BLUE}Fulfilling 12 minute vault pricing request...${NC}"
sleep 10
echo "Getting request data from 12 minute vault..."
REQUEST_DATA_12MIN=$(starkli call $PITCHLAKE_VAULT_ADDRESS_12MIN get_request_to_start_first_round  --rpc $STARKNET_RPC_URL)
echo "Request data: $REQUEST_DATA_12MIN"

# Extract calldata from response (this mimics the sncast sed/awk processing)
CALLDATA_12MIN=$(echo "$REQUEST_DATA_12MIN" | grep -o '\[.*\]' | sed 's/\[//;s/\]//' | tr ',' ' ')
echo "Extracted calldata: $CALLDATA_12MIN"

# sleep 10
# echo "Invoking fossil callback for 12 minute vault..."
# starkli invoke $PITCHLAKE_VERIFIER_ADDRESS fossil_callback $CALLDATA_12MIN 0x6 u256:10000000000 0x00 0x0d05 u256:2000000000 0x00 0x00 --account $STARKNET_ACCOUNT --rpc $STARKNET_RPC_URL -w
# echo -e "${GREEN}12 minute vault initialized${NC}"

# # Initialize 3 hour vault
# echo -e "${BLUE}Fulfilling 3 hour vault pricing request...${NC}"
# sleep 10
# echo "Getting request data from 3 hour vault..."
# REQUEST_DATA_3H=$(starkli call $PITCHLAKE_VAULT_ADDRESS_3H get_request_to_start_first_round --rpc $STARKNET_RPC_URL)
# echo "Request data: $REQUEST_DATA_3H"

# CALLDATA_3H=$(echo "$REQUEST_DATA_3H" | grep -o '\[.*\]' | sed 's/\[//;s/\]//' | tr ',' ' ')
# echo "Extracted calldata: $CALLDATA_3H"

# sleep 10
# echo "Invoking fossil callback for 3 hour vault..."
# starkli invoke $PITCHLAKE_VERIFIER_ADDRESS fossil_callback $CALLDATA_3H 0x6 u256:10000000000 0x00 0x0d05 u256:2000000000 0x00 0x00 --account $STARKNET_ACCOUNT --rpc $STARKNET_RPC_URL -w
# echo -e "${GREEN}3 hour vault initialized${NC}"

# # Initialize 1 month vault
# echo -e "${BLUE}Fulfilling 1 month vault pricing request...${NC}"
# sleep 10
# echo "Getting request data from 1 month vault..."
# REQUEST_DATA_1M=$(starkli call $PITCHLAKE_VAULT_ADDRESS_1M get_request_to_start_first_round --rpc $STARKNET_RPC_URL)
# echo "Request data: $REQUEST_DATA_1M"

# CALLDATA_1M=$(echo "$REQUEST_DATA_1M" | grep -o '\[.*\]' | sed 's/\[//;s/\]//' | tr ',' ' ')
# echo "Extracted calldata: $CALLDATA_1M"

# sleep 10
# echo "Invoking fossil callback for 1 month vault..."
# starkli invoke $PITCHLAKE_VERIFIER_ADDRESS fossil_callback $CALLDATA_1M 0x6 u256:10000000000 0x00 0x0d05 u256:2000000000 0x00 0x00 --account $STARKNET_ACCOUNT --rpc $STARKNET_RPC_URL -w
# echo -e "${GREEN}1 month vault initialized${NC}"

# echo -e "${GREEN}${BOLD}All vaults initialized!${NC}"

echo -e "\n${GREEN}${BOLD}All contracts deployed and initialized!${NC}"

# Update the environment files with the new addresses
for env_file in "${ENV_FILES[@]}"; do
    if [ ! -f "$env_file" ]; then
        echo -e "${RED}Warning: $env_file not found, skipping...${NC}"
        continue
    fi
    update_env_var "$env_file" "HASH_STORAGE_ADDRESS" "$SHA2INPUT_ADDRESS"
    update_env_var "$env_file" "UNIVERSAL_ECIP_CONTRACT" "$ECIP_HASH"
    update_env_var "$env_file" "GROTH16_VERIFIER_CONTRACT" "$VERIFIER_ADDRESS"
    update_env_var "$env_file" "OPTION_ROUND_CLASS_HASH" "$OPTION_ROUND_CLASS_HASH"
    update_env_var "$env_file" "PITCHLAKE_VERIFIER_CONTRACT" "$PITCHLAKE_VERIFIER_ADDRESS"
    # Vault addresses
    update_env_var "$env_file" "PITCHLAKE_VAULT_12MIN" "$PITCHLAKE_VAULT_ADDRESS_12MIN"
    update_env_var "$env_file" "PITCHLAKE_VAULT_3H" "$PITCHLAKE_VAULT_ADDRESS_3H"
    update_env_var "$env_file" "PITCHLAKE_VAULT_1M" "$PITCHLAKE_VAULT_ADDRESS_1M"
    # Option round addresses
    update_env_var "$env_file" "OPTION_ROUND_12MIN" "$OPTION_ROUND_ADDRESS_12MIN"
    update_env_var "$env_file" "OPTION_ROUND_3H" "$OPTION_ROUND_ADDRESS_3H"
    update_env_var "$env_file" "OPTION_ROUND_1M" "$OPTION_ROUND_ADDRESS_1M"
    # Legacy compatibility (points to 12 min vault)
    update_env_var "$env_file" "PITCHLAKE_VAULT" "$PITCHLAKE_VAULT_ADDRESS_12MIN"
    update_env_var "$env_file" "OPTION_ROUND_ADDRESS" "$OPTION_ROUND_ADDRESS_12MIN"
done

# If in docker mode, sync the addresses to .env.local
if [ "$ENV_TYPE" = "docker" ] && [ -f "$SECONDARY_ENV" ]; then
    echo -e "${BLUE}Syncing addresses to $SECONDARY_ENV...${NC}"
    update_env_var "$SECONDARY_ENV" "HASH_STORAGE_ADDRESS" "$SHA2INPUT_ADDRESS"
    update_env_var "$SECONDARY_ENV" "UNIVERSAL_ECIP_CONTRACT" "$ECIP_HASH"
    update_env_var "$SECONDARY_ENV" "GROTH16_VERIFIER_CONTRACT" "$VERIFIER_ADDRESS"
    update_env_var "$SECONDARY_ENV" "OPTION_ROUND_CLASS_HASH" "$OPTION_ROUND_CLASS_HASH"
    update_env_var "$SECONDARY_ENV" "PITCHLAKE_VERIFIER_CONTRACT" "$PITCHLAKE_VERIFIER_ADDRESS"
    # Vault addresses
    update_env_var "$SECONDARY_ENV" "PITCHLAKE_VAULT_12MIN" "$PITCHLAKE_VAULT_ADDRESS_12MIN"
    update_env_var "$SECONDARY_ENV" "PITCHLAKE_VAULT_3H" "$PITCHLAKE_VAULT_ADDRESS_3H"
    update_env_var "$SECONDARY_ENV" "PITCHLAKE_VAULT_1M" "$PITCHLAKE_VAULT_ADDRESS_1M"
    # Option round addresses
    update_env_var "$SECONDARY_ENV" "OPTION_ROUND_12MIN" "$OPTION_ROUND_ADDRESS_12MIN"
    update_env_var "$SECONDARY_ENV" "OPTION_ROUND_3H" "$OPTION_ROUND_ADDRESS_3H"
    update_env_var "$SECONDARY_ENV" "OPTION_ROUND_1M" "$OPTION_ROUND_ADDRESS_1M"
    # Legacy compatibility (points to 12 min vault)
    update_env_var "$SECONDARY_ENV" "PITCHLAKE_VAULT" "$PITCHLAKE_VAULT_ADDRESS_12MIN"
    update_env_var "$SECONDARY_ENV" "OPTION_ROUND_ADDRESS" "$OPTION_ROUND_ADDRESS_12MIN"
fi

# Return to original directory
cd "$ORIGINAL_DIR"

# Source the updated primary environment file
source "${ENV_FILES[0]}"

echo -e "${GREEN}${BOLD}Environment variables successfully updated in ${ENV_FILES[0]}${NC}"

# Create completion flag for healthcheck
touch /tmp/deployment_complete.flag
