#!/bin/bash

# Wait for contract deployment to complete and environment to be updated
set -e

ENV_FILE="/app/.env.docker"
MAX_WAIT=300  # 5 minutes maximum wait
WAIT_INTERVAL=5  # Check every 5 seconds

echo "🔄 Waiting for contract deployment to complete..."
echo "   Monitoring environment file for contract addresses..."

# Wait for environment file to contain contract addresses
# Since contract-deployer runs in a separate container, we can't check its completion flag
# Instead, we monitor the shared .env.docker file for updated contract addresses
elapsed=0
while [ $elapsed -lt $MAX_WAIT ]; do
    if [ -f "$ENV_FILE" ]; then
        # Check if PITCHLAKE_VERIFIER_CONTRACT is set and not empty
        VERIFIER_ADDRESS=$(grep "^PITCHLAKE_VERIFIER_CONTRACT=" "$ENV_FILE" 2>/dev/null | cut -d'=' -f2 || echo "")
        
        if [[ -n "$VERIFIER_ADDRESS" && "$VERIFIER_ADDRESS" =~ ^0x[a-fA-F0-9]{64}$ ]]; then
            echo "✅ Contract addresses found in $ENV_FILE"
            echo "   PITCHLAKE_VERIFIER_CONTRACT: $VERIFIER_ADDRESS"
            
            # Source the updated environment file to make variables available
            set -a  # automatically export all variables
            source "$ENV_FILE"
            set +a
            
            echo "✅ Environment variables loaded from updated file"
            break
        fi
    fi
    
    echo "⏳ Waiting for contract addresses... ($elapsed/${MAX_WAIT}s)"
    sleep $WAIT_INTERVAL
    elapsed=$((elapsed + WAIT_INTERVAL))
done

if [ $elapsed -ge $MAX_WAIT ]; then
    echo "❌ Timeout waiting for contract addresses in environment file"
    echo "   Current ENV_FILE contents:"
    [ -f "$ENV_FILE" ] && cat "$ENV_FILE" | head -20
    exit 1
fi

echo "🚀 Starting message-handler with updated contract addresses..."

# Execute the original command passed to this script
exec "$@"