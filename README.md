# Fossil Monorepo

This repository implements the **Pitchlake Coprocessor**, a specialized component of the Fossil infrastructure that performs verifiable computations for the Pitchlake options market using zero-knowledge proofs.

## Overview

**Fossil** is a trustless data infrastructure that records Ethereum Layer 1 (L1) base gas fee data on Starknet. The broader Fossil system includes components for historical data reconstruction (MMR Builder) and real-time synchronization (Light Client), which maintain a cryptographically verifiable record of Ethereum's base fee history.

This repository contains the **Pitchlake Coprocessor**, which consumes validated Ethereum L1 base fee data from the broader Fossil infrastructure and performs verifiable pricing computations for the Pitchlake options market. The Pitchlake Coprocessor consists of three main components:

- **Fossil API** - HTTP API for job management and pricing data requests
- **Proving Service** - RISC Zero proof generation via Bonsai API for financial calculations (TWAP, max return, reserve price)
- **StarkNet Contracts** - Onchain proof verification and computation result storage

**Note:** The MMR Builder and Light Client components (which handle Ethereum block ingestion and MMR construction) are part of the upstream Fossil infrastructure and are not included in this repository.

## Documentation

### 📚 Getting Started
- [Installation Guide](docs/getting-started/installation.md) - Set up your development environment
- [Local Development](docs/getting-started/local-development.md) - Run the complete development stack
- [Testing Guide](docs/getting-started/testing.md) - Run tests and write new ones

### 🏗️ Architecture
- [Architecture Overview](docs/architecture/overview.md) - High-level system design
- [Data Flow](docs/architecture/data-flow.md) - End-to-end request processing
- [Proving Service](docs/architecture/proving-service.md) - Proof generation architecture
- [Fossil API](docs/architecture/fossil-api.md) - API service architecture
- [StarkNet Contracts](docs/architecture/starknet-contracts.md) - Smart contract architecture

### 📖 Guides
- [Environment Setup](docs/guides/environment-setup.md) - Configure environment variables
- [Database Management](docs/guides/database-management.md) - Migrations and queries
- [Running Services](docs/guides/running-services.md) - Docker vs native execution
- [Debugging](docs/guides/debugging.md) - Troubleshooting and logging
- [Deployment](docs/guides/deployment.md) - Production deployment (AWS ECS)

### 🔌 API Reference
- [Fossil API Endpoints](docs/api-reference/fossil-api-endpoints.md) - Complete API documentation
- [Proving Service Endpoints](docs/api-reference/proving-service-endpoints.md) - Internal API
- [Authentication](docs/api-reference/authentication.md) - API key management

### 📦 Crates
**Proving Service:**
- [db](docs/crates/proving-service/db.md) - Database layer
- [message-handler](docs/crates/proving-service/message-handler.md) - SQS and proof generation
- [proving-service](docs/crates/proving-service/proving-service.md) - HTTP API
- [starknet-handler](docs/crates/proving-service/starknet-handler.md) - StarkNet integration

**Fossil API:**
- [db-access](docs/crates/fossil-api/db-access.md) - Database and authentication
- [server](docs/crates/fossil-api/server.md) - HTTP server

### 📜 Contracts
- [Fossil Hash Store](docs/contracts/fossil-hash-store.md) - Hierarchical hash storage
- [PitchLake Verifier](docs/contracts/pitchlake-verifier.md) - RISC Zero proof verification
- [Mocks](docs/contracts/mocks.md) - Testing contracts

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

### 2. Start Development Stack

```bash
make dev-up
```

This starts the complete local development environment:
1. **Infrastructure**: Katana (StarkNet devnet), PostgreSQL databases, LocalStack (AWS SQS)
2. **Contract Deployment**: Automatically deploys fresh contracts to Katana including multiple vault configurations (12min, 3hour, 1month)
3. **Services**: Fossil API, Proving Service API, Message Handler

### 3. Stop Development Stack

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
- `PITCHLAKE_VAULT` - Default vault contract address (12-minute rounds)
- `PITCHLAKE_VAULT_12MIN` - 12-minute vault contract address
- `PITCHLAKE_VAULT_3H` - 3-hour vault contract address  
- `PITCHLAKE_VAULT_1M` - 1-month vault contract address
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
3. **Contract Integration**: Automatically retrieves request data from StarkNet vault contracts
4. **Timestamp Calculation**: Dynamically calculates valid timestamp ranges based on vault contract parameters
5. **Pricing Data Request**: Submits properly formatted requests with correct vault addresses and timestamp ranges
6. **Job Monitoring**: Tracks job status with detailed progress reporting
7. **Result Verification**: Retrieves and validates job completion and results
8. **Batch Testing**: Tests batch job status endpoints

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
    "program_id": "RISC0_MOCK_PROOF_TEST",
    "vault_address": "YOUR_PITCHLAKE_VAULT_ADDRESS",
    "params": {
      "twap": [1672531200, 1672617600],
      "max_return": [1672531200, 1672617600],
      "reserve_price": [1672531200, 1672617600]
    }
  }'
```

**Note**: Replace `YOUR_PITCHLAKE_VAULT_ADDRESS` with one of the deployed vault addresses:
- `PITCHLAKE_VAULT_12MIN` for 12-minute rounds
- `PITCHLAKE_VAULT_3H` for 3-hour rounds  
- `PITCHLAKE_VAULT_1M` for 1-month rounds

**Tip**: The `test-local-request.sh` script automatically calculates valid timestamp ranges and vault addresses by querying the deployed contracts, eliminating the need for manual timestamp calculation.

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

- **Fossil API**: http://localhost:3000
- **Proving Service API**: http://localhost:3001
- **Katana StarkNet Devnet**: http://localhost:5050
- **LocalStack (AWS SQS)**: http://localhost:4567
- **PostgreSQL Databases**:
  - Proving Service: localhost:5435
  - Fossil API: localhost:5434

## Monitoring

Check service logs:
```bash
# Message Handler (RISC0 proof generation)
docker logs fossil-monorepo-message-handler-1 -f

# Fossil API (HTTP API)
docker logs fossil-monorepo-fossil-api-1 -f

# Proving Service API
docker logs fossil-monorepo-proving-service-api-1 -f
```

## Development Workflow

1. **Make Changes**: Edit code in `proving-service/` or `fossil-api/`
2. **Restart Services**: `make dev-down && make dev-up` (restarts all services with fresh contracts)
3. **Test Changes**: Run `./test-local-request.sh` (automatically adapts to new contract deployments)
4. **Advanced Testing**: Use individual API endpoints for specific component testing

## Key Features

- **Asynchronous Proof Generation** - Jobs queued in SQS for background processing
- **RISC Zero Integration** - zkVM proofs via Bonsai API
- **StarkNet Verification** - Onchain proof verification with Groth16
- **Hierarchical Hashing** - Efficient data integrity verification
- **API Key Authentication** - Secure access control
- **Multi-Vault Support** - 12-minute, 3-hour, and 1-month vaults
- **Event Monitoring** - Background StarkNet event tracking
- **Comprehensive Testing** - Unit, integration, and E2E tests

## Troubleshooting

For detailed troubleshooting, see the [Debugging Guide](docs/guides/debugging.md).

**Common Issues:**
- **Contract address mismatches**: Restart with `make dev-down && make dev-up` (deploys fresh contracts)
- **Service health issues**: Check logs with `docker logs` commands above
- **Test script failures**: Ensure all services are healthy before running `./test-local-request.sh`
- **Timestamp validation errors**: The test script automatically calculates valid ranges, but manual requests must use appropriate timestamps relative to contract deployment
- **API key issues**: Each test run generates a fresh API key; reuse keys from previous runs if needed

## Technology Stack

**Backend:**
- Rust (stable) - Core implementation
- Tokio - Async runtime
- Axum - HTTP framework
- SQLx - Database ORM
- PostgreSQL - Data storage
- AWS SQS - Job queue (LocalStack for local dev)

**Blockchain:**
- StarkNet - Smart contract platform
- Cairo 2.x - Contract language
- RISC Zero - zkVM proof generation
- Groth16 - Proof verification scheme

**Infrastructure:**
- Docker & Docker Compose - Containerization
- AWS ECS - Production deployment
- Katana - Local StarkNet devnet

## Project Structure

```
fossil-monorepo/
├── docs/                      # Complete documentation
│   ├── getting-started/       # Setup and testing guides
│   ├── architecture/          # System design docs
│   ├── guides/               # Operational guides
│   ├── api-reference/        # API documentation
│   ├── crates/               # Crate-specific docs
│   └── contracts/            # Contract documentation
├── fossil-api/               # HTTP API service
│   ├── crates/
│   │   ├── server/          # HTTP server
│   │   └── db-access/       # Database layer
├── proving-service/          # Proof generation service
│   ├── crates/
│   │   ├── proving-service/ # HTTP API
│   │   ├── message-handler/ # SQS & proof generation
│   │   ├── db/             # Database layer
│   │   └── starknet-handler/# StarkNet integration
├── starknet-contracts/       # Smart contracts
│   ├── fossil-hash-store/   # Hash storage contract
│   ├── pitchlake-verifier/  # Proof verifier
│   └── mocks/               # Test contracts
├── scripts/                  # Deployment scripts
└── docker/                   # Docker configurations
```

## Contributing

Before submitting changes:

```bash
# Run all tests and linters
make pr

# Individual operations
make test-all    # Run all tests
make lint-all    # Run linters
make build-all   # Build all projects
```

See [CLAUDE.md](CLAUDE.md) for development workflow details.

## Current Limitations and Future Considerations

### Bonsai Prover Deprecation

Fossil currently relies on the **Bonsai remote prover**, a managed proving service operated by the RISC Zero team. However, the RISC Zero team has announced plans to **deprecate Bonsai**, and they recommend migrating to **Boundless**, a decentralized and trustless proving marketplace.

This transition is non-trivial, as it introduces architectural and operational implications:

- Integration with Boundless requires modifications to Fossil's proof submission and verification workflows
- Proof batching, verification latency, and cost structures will change compared to the managed Bonsai environment
- Security guarantees remain equivalent but require additional protocol-level coordination

### Evaluation of Alternative Proving Systems

Given the computational complexity of Pitchlake's pricing models and the size of Fossil's aggregated datasets, **RISC Zero may not be the most efficient long-term proving system**.

The development team maintaining Fossil and Pitchlake should consider:

- **Migrating to Boundless** if maintaining zkVM compatibility is a priority
- **Evaluating SP1 (Succinct)** or similar high-performance zkVMs as potential replacements for RISC Zero
  - SP1 offers lower proof generation latency and improved scalability for large, data-heavy computations
  - Migration would require adapting Fossil's proof format and verification contracts but could significantly reduce compute costs

**Future contributors should carefully assess the trade-offs between maintaining RISC0 compatibility versus migrating to a more performant proving backend.**

For detailed technical considerations, see:
- [Proving Service Architecture](docs/architecture/proving-service.md#future-considerations)
- [Deployment Guide](docs/guides/deployment.md)

## License

**MIT License**

Copyright (c) 2024 Fossil Team

Permission is hereby granted, free of charge, to any person obtaining a copy of this software and associated documentation files (the "Software"), to deal in the Software without restriction, including without limitation the rights to use, copy, modify, merge, publish, distribute, sublicense, and/or sell copies of the Software, and to permit persons to whom the Software is furnished to do so, subject to the following conditions:

The above copyright notice and this permission notice shall be included in all copies or substantial portions of the Software.

THE SOFTWARE IS PROVIDED "AS IS", WITHOUT WARRANTY OF ANY KIND, EXPRESS OR IMPLIED, INCLUDING BUT NOT LIMITED TO THE WARRANTIES OF MERCHANTABILITY, FITNESS FOR A PARTICULAR PURPOSE AND NONINFRINGEMENT. IN NO EVENT SHALL THE AUTHORS OR COPYRIGHT HOLDERS BE LIABLE FOR ANY CLAIM, DAMAGES OR OTHER LIABILITY, WHETHER IN AN ACTION OF CONTRACT, TORT OR OTHERWISE, ARISING FROM, OUT OF OR IN CONNECTION WITH THE SOFTWARE OR THE USE OR OTHER DEALINGS IN THE SOFTWARE.

## Support

- **Documentation**: Start with [Getting Started](docs/getting-started/installation.md)
- **Issues**: Report bugs and request features via GitHub Issues
- **API Reference**: [Fossil API Endpoints](docs/api-reference/fossil-api-endpoints.md)
