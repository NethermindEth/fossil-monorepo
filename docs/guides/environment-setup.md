# Environment Setup Guide

This guide provides detailed information about configuring environment variables for the Fossil monorepo across different deployment scenarios.

## Table of Contents
- [Environment File Overview](#environment-file-overview)
- [Environment Variables Reference](#environment-variables-reference)
- [Configuration by Deployment Type](#configuration-by-deployment-type)
- [Secrets Management](#secrets-management)
- [Troubleshooting](#troubleshooting)

## Environment File Overview

The Fossil monorepo uses a centralized environment configuration system with multiple environment files for different scenarios:

### Environment Files

| File | Purpose | When to Use |
|------|---------|-------------|
| `.env.example` | Template with all variables and documentation | Reference and copying |
| `.env.local` | Local development (services on host) | Running services natively |
| `.env.docker` | Docker development (services in containers) | Using `make dev-up` |
| `.env.sepolia` | Sepolia testnet configuration | Testing on Sepolia |
| `.env.production` | Production configuration | Production deployments |

### Setup Process

```bash
# Initial setup creates .env.local and .env.docker
make setup

# Or manually:
cp .env.example .env.local
cp .env.example .env.docker

# Edit files as needed for your environment
```

### Configuration Hierarchy

```
Root Level Environment Files (.env.local, .env.docker, .env.sepolia)
├── Shared by all services
├── Service-specific overrides possible in service directories
└── Loaded by Docker Compose and service binaries
```

## Environment Variables Reference

### StarkNet Configuration

#### RPC and Network

```bash
# StarkNet RPC endpoint
# Local (Katana): http://localhost:5050
# Sepolia: https://starknet-sepolia.public.blastapi.io/rpc/v0_7
# Mainnet: https://starknet-mainnet.public.blastapi.io/rpc/v0_7
STARKNET_RPC_URL=http://localhost:5050

# Network identifier
# Values: DEVNET_KATANA | SEPOLIA | MAINNET
NETWORK=DEVNET_KATANA
```

#### Account Configuration

```bash
# Account name (Katana predeployed account)
STARKNET_ACCOUNT=katana-0

# Account address (felt252)
STARKNET_ACCOUNT_ADDRESS=0x127fd5f1fe78a71f8bcd1fec63e3fe2f0486b6ecd5c86a0466c3a21fa5cfcec

# Private key for signing transactions
# WARNING: Never commit real private keys to version control
STARKNET_PRIVATE_KEY=0xc5b2fcab997346f3ea1c00b002ecf6f382c5f9c9659a3894eb783c5320f912
```

#### Contract Addresses

These are populated automatically by the deployment script when running `make dev-up`:

```bash
# Core verification contracts
UNIVERSAL_ECIP_CONTRACT=        # Universal ECIP verifier
GROTH16_VERIFIER_CONTRACT=      # Groth16 proof verifier
PITCHLAKE_VERIFIER_CONTRACT=    # PitchLake-specific verifier
HASH_STORAGE_ADDRESS=           # Hash storage contract

# Vault contracts (different time configurations)
PITCHLAKE_VAULT_12MIN=          # 12 minute rounds, 50% risk factor
PITCHLAKE_VAULT_3H=             # 3 hour rounds, 25% risk factor
PITCHLAKE_VAULT_1M=             # 1 month rounds, 12.5% risk factor

# Legacy compatibility (points to 12min vault)
PITCHLAKE_VAULT=
OPTION_ROUND_ADDRESS=

# Option round contracts
OPTION_ROUND_12MIN=
OPTION_ROUND_3H=
OPTION_ROUND_1M=

# Class hash for deploying new rounds
OPTION_ROUND_CLASS_HASH=
```

#### Fossil Store Contract (Upstream Infrastructure)

```bash
# =============================================================================
# FOSSIL STORE CONFIGURATION (Upstream Fossil Infrastructure)
# =============================================================================
# Fossil Store contract address on Starknet
# This contract is part of the broader Fossil infrastructure (MMR Builder, Light Client)
# and is NOT managed in this repository.
#
# Purpose: Provides validated hourly average base fee data from Ethereum L1
# Queried by: proving-service/crates/starknet-handler (get_avg_fees_in_range)
# Data Source: Populated by upstream Fossil MMR Builder with cryptographically verified data
#
# ⚠️ Important: This address should NOT be changed unless coordinating with the
# Fossil infrastructure team, as it points to the canonical data source.
FOSSIL_STORE_ADDRESS=0x00e581139553c8666f60b6646f277a336f99f108f8e5fa7cb300b6a6ce7c3b8c
```

### Database Configuration

#### Fossil API Database

```bash
# Main application database (read/write)
# Local development:
OFFCHAIN_PROCESSOR_DATABASE_URL=postgresql://postgres:postgres@localhost:5434/postgres

# Docker development:
OFFCHAIN_PROCESSOR_DATABASE_URL=postgresql://postgres:postgres@fossil_api_db:5432/postgres

# Production (use managed database):
OFFCHAIN_PROCESSOR_DATABASE_URL=postgresql://user:password@db.example.com:5432/fossil_api
```

**Schema:** API keys, job requests, job status, pricing data results

#### Indexer Database (Legacy - Mostly Unused)

```bash
# Read-only database populated by fossil indexer
# ⚠️ Note: In current implementation, the Pitchlake Coprocessor primarily fetches
# data from the Fossil Store Contract on Starknet, not from this database.
# This configuration is maintained for legacy compatibility.

# Local development:
INDEXER_DATABASE_URL=postgresql://postgres:postgres@localhost:5433/postgres

# Docker development:
INDEXER_DATABASE_URL=postgresql://postgres:postgres@indexer_db:5432/postgres
```

**Schema:** Historical blockchain data (blocks, transactions, events)
**Current Usage:** Minimal - Most data queries now use Fossil Store Contract on Starknet

#### Proving Service Database

```bash
# Job tracking and proof storage (read/write)
# Local development:
PROVING_SERVICE_DATABASE_URL=postgresql://postgres:postgres@localhost:5435/postgres

# Docker development:
PROVING_SERVICE_DATABASE_URL=postgresql://postgres:postgres@proving_service_db:5432/postgres

# Production:
PROVING_SERVICE_DATABASE_URL=postgresql://user:password@db.example.com:5432/proving_service
```

**Schema:** Job queue, proof metadata, verification results

### Service URLs

```bash
# Proving Service API endpoint
# Local development (host):
PROVING_SERVICE_URL=http://127.0.0.1:3001

# Docker development (container):
PROVING_SERVICE_URL=http://proving-service-api:3001

# Production:
PROVING_SERVICE_URL=https://proving-service.example.com
```

### AWS Configuration

For local development, uses LocalStack to emulate AWS services:

```bash
# AWS Region
AWS_REGION=us-east-1

# LocalStack credentials (use 'test' for local)
AWS_ACCESS_KEY_ID=test
AWS_SECRET_ACCESS_KEY=test

# AWS endpoint
# Local development:
AWS_ENDPOINT_URL=http://localhost:4567

# Docker development:
AWS_ENDPOINT_URL=http://localstack:4566

# Production (omit for real AWS):
# AWS_ENDPOINT_URL=

# SQS Queue URL
# Local:
SQS_QUEUE_URL=http://localhost:4567/000000000000/fossilQueue

# Docker:
SQS_QUEUE_URL=http://localstack:4566/000000000000/fossilQueue

# Production:
SQS_QUEUE_URL=https://sqs.us-east-1.amazonaws.com/123456789012/fossilQueue
```

### Bonsai Configuration

RISC Zero proof generation via Bonsai API:

```bash
# Bonsai API credentials
BONSAI_API_KEY="YOUR_API_KEY_HERE"
BONSAI_API_URL="https://api.bonsai.xyz/"

# Retry configuration for Bonsai API
RISC0_MAX_RETRIES=7                    # Max retry attempts
RISC0_INITIAL_RETRY_DELAY_MS=3000     # Initial delay in ms

# Concurrent proof limiting
MAX_CONCURRENT_PROOFS=1                # Limit parallel proofs
```

**Getting a Bonsai API Key:**
1. Visit https://bonsai.xyz
2. Sign up for an account
3. Generate an API key from the dashboard
4. Add to your `.env.local` file

### Feature Flags

Control system behavior with feature flags:

```bash
# Use mock pricing data (for testing)
USE_MOCK_PRICING_DATA=true

# Enable onchain proof verification
VERIFY_PROOFS_ONCHAIN=true

# Enable proof generation
ENABLE_PROOF=true

# Use RISC Zero integration
USE_RISC0_INTEGRATION=true

# Use simple mock proofs (testing only)
USE_SIMPLE_MOCK=false
```

### CORS Configuration

```bash
# Allowed origins for CORS (comma-separated)
ALLOWED_ORIGINS=http://localhost:3000,http://127.0.0.1:3000
```

### StarkNet Retry Configuration

```bash
# Maximum retry attempts for StarkNet transactions
STARKNET_MAX_RETRIES=3

# Initial backoff delay (ms)
STARKNET_INITIAL_BACKOFF_MS=100

# Maximum backoff delay (ms)
STARKNET_MAX_BACKOFF_MS=1000
```

## Configuration by Deployment Type

### Local Development (Docker)

This is the recommended setup for most development work.

**Setup:**
```bash
make dev-up
```

**`.env.docker` Configuration:**
```bash
# StarkNet
STARKNET_RPC_URL=http://katana:5050
NETWORK=DEVNET_KATANA

# Databases (Docker service names)
OFFCHAIN_PROCESSOR_DATABASE_URL=postgresql://postgres:postgres@fossil_api_db:5432/postgres
INDEXER_DATABASE_URL=postgresql://postgres:postgres@indexer_db:5432/postgres
PROVING_SERVICE_DATABASE_URL=postgresql://postgres:postgres@proving_service_db:5432/postgres

# Services (Docker service names)
PROVING_SERVICE_URL=http://proving-service-api:3001

# AWS (LocalStack)
AWS_ENDPOINT_URL=http://localstack:4566
SQS_QUEUE_URL=http://localstack:4566/000000000000/fossilQueue

# Feature flags
ENABLE_PROOF=true
USE_RISC0_INTEGRATION=true
VERIFY_PROOFS_ONCHAIN=true
```

### Native Development (No Docker for Services)

For faster iteration during development.

**Setup:**
```bash
# Start only infrastructure services
docker compose -f docker-compose.local.yml up -d katana fossil_api_db proving_service_db localstack

# Run services natively
cd fossil-api && cargo run --bin server
cd proving-service && cargo run --bin proving-service
cd proving-service && cargo run --bin message-handler
```

**`.env.local` Configuration:**
```bash
# StarkNet (external connection)
STARKNET_RPC_URL=http://localhost:5050
NETWORK=DEVNET_KATANA

# Databases (localhost ports)
OFFCHAIN_PROCESSOR_DATABASE_URL=postgresql://postgres:postgres@localhost:5434/postgres
INDEXER_DATABASE_URL=postgresql://postgres:postgres@localhost:5433/postgres
PROVING_SERVICE_DATABASE_URL=postgresql://postgres:postgres@localhost:5435/postgres

# Services (localhost)
PROVING_SERVICE_URL=http://127.0.0.1:3001

# AWS (external connection to LocalStack)
AWS_ENDPOINT_URL=http://localhost:4567
SQS_QUEUE_URL=http://localhost:4567/000000000000/fossilQueue

# Feature flags
ENABLE_PROOF=true
USE_RISC0_INTEGRATION=true
VERIFY_PROOFS_ONCHAIN=true
```

### Sepolia Testnet

Testing with real StarkNet testnet.

**`.env.sepolia` Configuration:**
```bash
# StarkNet Sepolia
STARKNET_RPC_URL=https://starknet-sepolia.public.blastapi.io/rpc/v0_7
NETWORK=SEPOLIA

# Real account (from wallet)
STARKNET_ACCOUNT_ADDRESS=0x<your_account_address>
STARKNET_PRIVATE_KEY=0x<your_private_key>

# Deployed contract addresses on Sepolia
PITCHLAKE_VERIFIER_CONTRACT=0x<deployed_address>
PITCHLAKE_VAULT_12MIN=0x<deployed_address>
# ... etc

# Production-like databases (could be RDS)
OFFCHAIN_PROCESSOR_DATABASE_URL=postgresql://user:pass@sepolia-db.example.com:5432/fossil_api
PROVING_SERVICE_DATABASE_URL=postgresql://user:pass@sepolia-db.example.com:5432/proving_service

# Real AWS SQS
AWS_REGION=us-east-1
AWS_ENDPOINT_URL=  # Empty for real AWS
SQS_QUEUE_URL=https://sqs.us-east-1.amazonaws.com/123456789012/fossilQueue-sepolia

# Bonsai (real API key required)
BONSAI_API_KEY="your_real_api_key"

# Feature flags
ENABLE_PROOF=true
USE_RISC0_INTEGRATION=true
VERIFY_PROOFS_ONCHAIN=true
USE_MOCK_PRICING_DATA=false
```

### Production

**Important:** Use secrets management (AWS Secrets Manager, HashiCorp Vault, etc.) for production credentials.

**`.env.production` Configuration:**
```bash
# StarkNet Mainnet
STARKNET_RPC_URL=https://starknet-mainnet.public.blastapi.io/rpc/v0_7
NETWORK=MAINNET

# Account from secure key management
STARKNET_ACCOUNT_ADDRESS=${SECRET_STARKNET_ADDRESS}
STARKNET_PRIVATE_KEY=${SECRET_STARKNET_KEY}

# Production contract addresses
PITCHLAKE_VERIFIER_CONTRACT=0x<mainnet_address>
PITCHLAKE_VAULT_12MIN=0x<mainnet_address>

# RDS Databases with connection pooling
OFFCHAIN_PROCESSOR_DATABASE_URL=${SECRET_FOSSIL_API_DB_URL}
PROVING_SERVICE_DATABASE_URL=${SECRET_PROVING_SERVICE_DB_URL}

# Production services
PROVING_SERVICE_URL=https://proving-service.fossil.example.com

# Real AWS
AWS_REGION=us-east-1
# AWS_ENDPOINT_URL not set (uses real AWS)
SQS_QUEUE_URL=https://sqs.us-east-1.amazonaws.com/123456789012/fossilQueue-prod

# Bonsai production
BONSAI_API_KEY=${SECRET_BONSAI_API_KEY}
BONSAI_API_URL="https://api.bonsai.xyz/"

# Production tuning
RISC0_MAX_RETRIES=10
MAX_CONCURRENT_PROOFS=5
STARKNET_MAX_RETRIES=5

# Production features
ENABLE_PROOF=true
USE_RISC0_INTEGRATION=true
VERIFY_PROOFS_ONCHAIN=true
USE_MOCK_PRICING_DATA=false
```

## Secrets Management

### Local Development

For local development, secrets can be stored in `.env.local` and `.env.docker` files (which are gitignored).

**Never commit:**
- Real private keys
- Real API keys
- Production credentials
- Database passwords

### Production

Use proper secrets management:

#### AWS Secrets Manager

```bash
# Store secret
aws secretsmanager create-secret \
  --name fossil/bonsai-api-key \
  --secret-string "your-api-key"

# Retrieve in application
export BONSAI_API_KEY=$(aws secretsmanager get-secret-value \
  --secret-id fossil/bonsai-api-key \
  --query SecretString \
  --output text)
```

#### Docker Secrets

```yaml
# docker-compose.yml
services:
  proving-service:
    secrets:
      - bonsai_api_key
      - starknet_private_key

secrets:
  bonsai_api_key:
    external: true
  starknet_private_key:
    external: true
```

#### Environment Variable Substitution

```bash
# .env.production
BONSAI_API_KEY=${BONSAI_API_KEY}
STARKNET_PRIVATE_KEY=${STARKNET_PRIVATE_KEY}
```

Then provide via environment:
```bash
export BONSAI_API_KEY="real-key"
export STARKNET_PRIVATE_KEY="real-key"
docker compose up
```

### Security Best Practices

1. **Rotation**: Rotate secrets regularly
2. **Least Privilege**: Use read-only database users where possible
3. **Encryption**: Encrypt sensitive environment files
4. **Audit**: Log access to secrets
5. **Separate Environments**: Use different secrets for dev/staging/prod

## Troubleshooting

### Contract Addresses Not Set

**Problem:** After deployment, contract addresses are empty

**Solution:**
```bash
# Redeploy contracts
make dev-down
make dev-up

# Check deployment logs
docker logs fossil-monorepo-contract-deployer

# Verify .env.local was updated
grep PITCHLAKE_VERIFIER_CONTRACT .env.local
```

### Database Connection Errors

**Problem:** `connection refused` or `FATAL: password authentication failed`

**Solutions:**

```bash
# Check if databases are running
docker ps | grep postgres

# Verify connection string format
# Format: postgresql://user:password@host:port/database
echo $OFFCHAIN_PROCESSOR_DATABASE_URL

# Test connection directly
psql postgresql://postgres:postgres@localhost:5434/postgres

# Check Docker network
docker network inspect fossil-monorepo_default
```

### SQS Connection Errors

**Problem:** Cannot connect to SQS queue

**Solutions:**

```bash
# Check LocalStack is running
docker logs fossil-monorepo-localstack-1

# Verify queue exists
aws --endpoint-url=http://localhost:4567 sqs list-queues

# Recreate queue if needed
aws --endpoint-url=http://localhost:4567 sqs create-queue --queue-name fossilQueue
```

### StarkNet RPC Errors

**Problem:** `Error: node not reachable` or timeout errors

**Solutions:**

```bash
# Check Katana is running
docker logs fossil-monorepo-katana-1

# Test RPC connection
curl -X POST http://localhost:5050 \
  -H "Content-Type: application/json" \
  -d '{"jsonrpc":"2.0","method":"starknet_chainId","params":[],"id":1}'

# Restart Katana
docker restart fossil-monorepo-katana-1
```

### Bonsai API Errors

**Problem:** `401 Unauthorized` or `API key invalid`

**Solutions:**

```bash
# Verify API key is set
echo $BONSAI_API_KEY

# Check API key format (should start with specific prefix)
# Test with Bonsai CLI if available

# Regenerate API key from Bonsai dashboard
```

### Wrong Environment Loaded

**Problem:** Service uses wrong environment configuration

**Solutions:**

```bash
# Check which env file is loaded
# Services load .env.local by default for native runs
# Docker Compose uses .env.docker

# Explicitly specify environment
docker compose --env-file .env.docker up

# For native runs
export $(cat .env.local | xargs)
cargo run
```

### Port Conflicts

**Problem:** `port already in use`

**Solutions:**

```bash
# Find process using port
lsof -i :3000
lsof -i :5050

# Kill process or change port in configuration
# Update STARKNET_RPC_URL, service ports, etc.
```

## Next Steps

- [Database Management](database-management.md) - Managing databases and migrations
- [Running Services](running-services.md) - Different ways to run services
- [Local Development](../getting-started/local-development.md) - Complete local setup guide
- [Deployment Guide](deployment.md) - Production deployment
