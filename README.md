# Fossil Monorepo - Local Development

This repository contains a complete RISC0 proof generation and StarkNet verification system with two main services:

- **Proving Service**: Handles RISC0 proof generation via Bonsai API and StarkNet onchain verification
- **Offchain Processor**: HTTP API for job management and pricing data requests

## Quick Start

### 1. Initial Setup

```bash
make setup
```

This installs all dependencies:
- Rust toolchain with required components
- RISC0 toolchain via rzup
- StarkNet tools (Starkli, Scarb, Starknet Foundry) via asdf
- Environment file configuration

### 2. Build Message Handler Image

```bash
make build-message-handler-image
```

This builds the Docker image with RISC0 guest binaries for proof generation.

### 3. Start Development Stack

```bash
make dev-up
```

This starts the complete local development environment:
1. **Infrastructure**: Katana (StarkNet devnet), PostgreSQL databases, LocalStack (AWS SQS)
2. **Contract Deployment**: Automatically deploys fresh contracts to Katana
3. **Services**: Offchain Processor, Proving Service API, Message Handler

### 4. Stop Development Stack

```bash
make dev-down
```

## Environment Configuration

The system uses three environment files:

### `.env.local` (Host services)
For running services directly on your machine:
- `STARKNET_RPC_URL=http://localhost:5050` - Points to local Katana
- Contract addresses updated by deployment script
- Database URLs pointing to `localhost` ports

### `.env.docker` (Container services)  
For services running inside Docker containers:
- `STARKNET_RPC_URL=http://katana:5050` - Points to Katana container
- Same contract addresses as `.env.local`
- Database URLs using Docker service names

### Key Environment Variables
- `PITCHLAKE_VERIFIER_CONTRACT` - StarkNet contract for proof verification
- `PITCHLAKE_VAULT` - Mock vault contract address
- `BONSAI_API_KEY` - RISC0 Bonsai API key for proof generation
- `ENABLE_PROOF=true` - Enable proof generation in message handler
- `USE_RISC0_INTEGRATION=true` - Use RISC0 for proof generation
- `VERIFY_PROOFS_ONCHAIN=true` - Submit proofs to StarkNet for verification

## Testing the System

### Running the Test Script

```bash
./test-local-request.sh
```

This comprehensive test:

1. **Health Checks**: Verifies all services are responding
2. **API Key Generation**: Creates authentication token
3. **Basic Job**: Submits a pricing data request
4. **RISC0 Proof Job**: Tests full proof generation pipeline
5. **Result Verification**: Checks job completion and results

### Manual API Testing

#### 1. Generate API Key
```bash
curl -X POST "http://localhost:3000/api_key" \
  -H "Content-Type: application/json" \
  -d '{"name": "test_key"}'
```

#### 2. Submit Pricing Data Request
```bash
curl -X POST "http://localhost:3000/pricing_data" \
  -H "Content-Type: application/json" \
  -H "X-API-Key: YOUR_API_KEY" \
  -d '{
    "identifiers": ["RISC0_MOCK_PROOF_TEST"],
    "params": {
      "twap": [1672531200, 1672617600],
      "volatility": [1672531200, 1672617600], 
      "reserve_price": [1672531200, 1672617600]
    },
    "client_info": {
      "client_address": "0x018df581fe0ee497a4a3595cf62aea0bafa7ba1a54a7dcbafca37bfada67c718",
      "vault_address": "YOUR_PITCHLAKE_VAULT_ADDRESS",
      "timestamp": 1672574400
    }
  }'
```

#### 3. Check Job Status
```bash
curl "http://localhost:3000/job_status/JOB_ID"
```

#### 4. Get Job Result
```bash
curl "http://localhost:3000/job_result/JOB_ID" \
  -H "X-API-Key: YOUR_API_KEY"
```

## Service URLs

When `make dev-up` is running:

- **Offchain Processor**: http://localhost:3000
- **Proving Service API**: http://localhost:3001  
- **Katana StarkNet Devnet**: http://localhost:5050
- **LocalStack (AWS SQS)**: http://localhost:4567
- **PostgreSQL Databases**: 
  - Proving Service: localhost:5435
  - Offchain Processor: localhost:5434

## Monitoring

Check service logs:
```bash
# Message Handler (RISC0 proof generation)
docker logs fossil-monorepo-message-handler-1 -f

# Offchain Processor (HTTP API)
docker logs fossil-monorepo-offchain-processor-1 -f

# Proving Service API
docker logs fossil-monorepo-proving-service-api-1 -f
```

## Development Workflow

1. **Make Changes**: Edit code in `proving-service/` or `offchain-processor/`
2. **Rebuild Images**: Run `make build-message-handler-image` if needed
3. **Restart Services**: `make dev-down && make dev-up`
4. **Test Changes**: Run `./test-local-request.sh`

## Troubleshooting

- **Contract address mismatches**: Restart with `make dev-down && make dev-up`
- **RISC0 binary issues**: Rebuild with `make build-message-handler-image`
- **Service health issues**: Check logs with `docker logs` commands above
