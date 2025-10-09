# Local Development Guide

This guide covers setting up and running the Fossil monorepo development environment on your local machine.

## Table of Contents
- [Prerequisites](#prerequisites)
- [Initial Setup](#initial-setup)
- [Starting the Development Environment](#starting-the-development-environment)
- [Making Your First Request](#making-your-first-request)
- [Development Workflow](#development-workflow)
- [Service Architecture](#service-architecture)
- [Troubleshooting](#troubleshooting)

## Prerequisites

Before starting, ensure you've completed the [Installation Guide](installation.md).

**Quick check:**
```bash
# Verify all tools are installed
docker --version
cargo --version
cargo risczero --version
scarb --version
```

## Initial Setup

### 1. Clone and Setup

If you haven't already:

```bash
# Clone the repository
git clone <repository-url>
cd fossil-monorepo

# Run automated setup
make setup
```

This command will:
- Install all required dependencies
- Configure environment files (`.env.local`, `.env.docker`)
- Build both services in release mode

**Time:** ~15-30 minutes (first time only)

### 2. Environment Configuration

The setup process creates two environment files:

**`.env.local`** - For running services directly on your host machine
```bash
# Key variables (localhost-based URLs)
STARKNET_RPC_URL=http://localhost:5050
OFFCHAIN_PROCESSOR_DATABASE_URL=postgresql://postgres:postgres@localhost:5434/postgres
PROVING_SERVICE_DATABASE_URL=postgresql://postgres:postgres@localhost:5435/postgres
```

**`.env.docker`** - For services running inside Docker containers
```bash
# Key variables (Docker service names)
STARKNET_RPC_URL=http://katana:5050
OFFCHAIN_PROCESSOR_DATABASE_URL=postgresql://postgres:postgres@fossil_api_db:5432/postgres
PROVING_SERVICE_DATABASE_URL=postgresql://postgres:postgres@proving_service_db:5432/postgres
```

See [Environment Setup Guide](../guides/environment-setup.md) for detailed configuration.

## Starting the Development Environment

### Full Stack Startup

Start all services with a single command:

```bash
make dev-up
```

This command orchestrates the complete development stack:

**Step 1: Infrastructure Services**
- Starts Katana (StarkNet devnet) on port 5050
- Starts PostgreSQL databases (ports 5434, 5435)
- Starts LocalStack (AWS SQS) on port 4567
- Waits for Katana to become healthy

**Step 2: Contract Deployment**
- Deploys fresh contracts to Katana
- Deploys multiple vault configurations:
  - 12-minute round vault
  - 3-hour round vault
  - 1-month round vault
- Updates `.env.local` and `.env.docker` with contract addresses

**Step 3: Application Services**
- Starts Fossil API on port 3000
- Starts Proving Service API on port 3001
- Starts Message Handler (background proof processor)

**Total startup time:** ~2-3 minutes

### Service URLs

Once `make dev-up` completes, the following services are available:

| Service | URL | Description |
|---------|-----|-------------|
| Fossil API | http://localhost:3000 | Main HTTP API for job submission |
| Proving Service API | http://localhost:3001 | Internal API for proof generation |
| Katana (StarkNet) | http://localhost:5050 | Local StarkNet devnet |
| LocalStack (SQS) | http://localhost:4567 | Local AWS SQS emulator |
| PostgreSQL (Fossil API) | localhost:5434 | Fossil API database |
| PostgreSQL (Proving Service) | localhost:5435 | Proving Service database |

### Health Checks

Verify all services are running:

```bash
# Fossil API
curl http://localhost:3000/health

# Proving Service API
curl http://localhost:3001/health

# Katana (StarkNet)
curl -X POST http://localhost:5050 \
  -H "Content-Type: application/json" \
  -d '{"jsonrpc":"2.0","method":"starknet_chainId","params":[],"id":1}'
```

### Viewing Logs

Monitor service logs in real-time:

```bash
# All services
docker compose -f docker-compose.local.yml logs -f

# Specific services
docker logs fossil-monorepo-fossil-api-1 -f          # Fossil API
docker logs fossil-monorepo-proving-service-api-1 -f  # Proving Service API
docker logs fossil-monorepo-message-handler-1 -f      # Message Handler
docker logs fossil-monorepo-katana-1 -f               # Katana
```

## Making Your First Request

### Automated Test Script

The easiest way to test the system is using the included test script:

```bash
./test-local-request.sh
```

This comprehensive script:
1. Checks service health
2. Generates an API key
3. Retrieves request data from StarkNet vault contracts
4. Calculates valid timestamp ranges automatically
5. Submits a pricing data request
6. Monitors job progress with detailed status updates
7. Retrieves and displays results

**Expected output:**
```
[INFO] Testing Fossil Monorepo locally using Docker services
[INFO] ✅ Fossil API is healthy
[INFO] ✅ Proving Service is healthy
[INFO] 🔑 Generated API key: sk_test_xxxxx
[INFO] 📡 Getting request data from StarkNet vault...
[INFO] 📤 Submitting pricing data request...
[INFO] ✅ Job submitted successfully: job_xxxxx
[INFO] 📊 Job status: Processing
[INFO] ✅ Job completed successfully!
```

### Manual API Testing

#### 1. Generate an API Key

```bash
curl -X POST "http://localhost:3000/api_key" \
  -H "Content-Type: application/json" \
  -d '{"name": "my_dev_key"}'
```

**Response:**
```json
{
  "api_key": "sk_test_1234567890abcdef",
  "name": "my_dev_key",
  "created_at": "2024-01-15T10:30:00Z"
}
```

#### 2. Submit a Pricing Data Request

```bash
curl -X POST "http://localhost:3000/pricing_data" \
  -H "Content-Type: application/json" \
  -H "X-API-Key: sk_test_1234567890abcdef" \
  -d '{
    "program_id": "RISC0_MOCK_PROOF_TEST",
    "vault_address": "0x004018ae0157b10d08cb1f70d34e32fd8b25e4ad1d70afc89616c3b300257fd9",
    "params": {
      "twap": [1672531200, 1672617600],
      "max_return": [1672531200, 1672617600],
      "reserve_price": [1672531200, 1672617600]
    }
  }'
```

**Note:** Replace the vault address with the actual deployed address from `.env.local`:
- `PITCHLAKE_VAULT_12MIN` for 12-minute rounds
- `PITCHLAKE_VAULT_3H` for 3-hour rounds
- `PITCHLAKE_VAULT_1M` for 1-month rounds

**Response:**
```json
{
  "job_id": "job_abc123xyz",
  "status": "pending"
}
```

#### 3. Check Job Status

```bash
curl "http://localhost:3000/job_status/job_abc123xyz"
```

**Response:**
```json
{
  "job_id": "job_abc123xyz",
  "status": "processing",
  "created_at": "2024-01-15T10:30:00Z",
  "updated_at": "2024-01-15T10:30:15Z"
}
```

**Possible statuses:**
- `pending` - Job queued for processing
- `processing` - Proof generation in progress
- `completed` - Job completed successfully
- `failed` - Job failed (check error message)

#### 4. Get Job Result

```bash
curl "http://localhost:3000/job_result/job_abc123xyz" \
  -H "X-API-Key: sk_test_1234567890abcdef"
```

**Response:**
```json
{
  "job_id": "job_abc123xyz",
  "status": "completed",
  "result": {
    "twap": "...",
    "max_return": "...",
    "reserve_price": "..."
  },
  "proof": {
    "image_id": "...",
    "journal": "...",
    "seal": "..."
  }
}
```

## Development Workflow

### Making Code Changes

#### Edit-Rebuild-Test Cycle

1. **Make your changes** to the code in `proving-service/` or `fossil-api/`

2. **Restart services** to pick up changes:
   ```bash
   # Stop all services
   make dev-down

   # Rebuild and restart
   make dev-up
   ```

3. **Test your changes**:
   ```bash
   ./test-local-request.sh
   ```

#### Hot Reload for Faster Development

For faster iteration, you can run services outside Docker:

**Terminal 1: Start infrastructure only**
```bash
# Start databases, Katana, LocalStack
docker compose -f docker-compose.local.yml up -d katana fossil_api_db proving_service_db localstack
```

**Terminal 2: Run Fossil API**
```bash
cd fossil-api
source ../.env.local
cargo run --bin server
```

**Terminal 3: Run Proving Service API**
```bash
cd proving-service
source ../.env.local
cargo run --bin proving-service
```

**Terminal 4: Run Message Handler**
```bash
cd proving-service
source ../.env.local
cargo run --bin message-handler
```

Now you can make changes and restart individual services with `Ctrl+C` and re-running `cargo run`.

### Running Tests

```bash
# Run all tests (both projects)
make test-all

# Run tests for individual projects
cd proving-service && make test
cd fossil-api && make test

# Run specific test
cargo test test_name

# Run with output
cargo test -- --nocapture
```

See [Testing Guide](testing.md) for comprehensive testing documentation.

### Linting and Formatting

Before committing code:

```bash
# Run all linters and tests
make pr

# Individual operations
make lint-all   # Run linters
make fmt-all    # Format code
```

## Service Architecture

Understanding the local development architecture:

```
┌─────────────────────────────────────────────────────────────┐
│                         Your Machine                         │
│                                                              │
│  ┌────────────────────────────────────────────────────┐    │
│  │              Docker Containers                     │    │
│  │                                                    │    │
│  │  ┌──────────┐  ┌──────────┐  ┌───────────────┐  │    │
│  │  │  Katana  │  │PostgreSQL│  │  LocalStack   │  │    │
│  │  │ (StarkNet)│  │  (DBs)  │  │    (SQS)     │  │    │
│  │  └─────┬────┘  └─────┬────┘  └───────┬───────┘  │    │
│  │        │             │                │           │    │
│  └────────┼─────────────┼────────────────┼──────────┘    │
│           │             │                │                │
│  ┌────────┴─────────────┴────────────────┴──────────┐    │
│  │           Application Services (Docker)          │    │
│  │                                                   │    │
│  │  ┌──────────────┐  ┌──────────────────────────┐ │    │
│  │  │  Fossil API  │  │  Proving Service         │ │    │
│  │  │  (Port 3000) │  │  - API (Port 3001)       │ │    │
│  │  │              │  │  - Message Handler       │ │    │
│  │  └──────────────┘  └──────────────────────────┘ │    │
│  └───────────────────────────────────────────────────┘    │
│                                                            │
└─────────────────────────────────────────────────────────────┘
```

### Request Flow

1. **Client** sends request to Fossil API (port 3000)
2. **Fossil API** validates request, creates job, forwards to Proving Service
3. **Proving Service API** receives job, queues message in SQS
4. **Message Handler** polls SQS, processes job:
   - Fetches data from database
   - Generates RISC0 proof via Bonsai API
   - Submits proof to StarkNet (Katana) for verification
5. **Proving Service** updates job status
6. **Client** polls for results via Fossil API

## Troubleshooting

### Services Won't Start

**Check Docker daemon:**
```bash
docker info
```

**Check port conflicts:**
```bash
# Ports 3000, 3001, 5050, 5434, 5435, 4567 must be free
lsof -i :3000
lsof -i :3001
lsof -i :5050
```

**View service logs:**
```bash
docker compose -f docker-compose.local.yml logs
```

### Contract Deployment Fails

```bash
# Manually trigger contract deployment
docker compose -f docker-compose.deploy.yml run --rm contract-deployer
```

### Database Issues

**Reset databases:**
```bash
# Stop services
make dev-down

# Remove database volumes
docker volume rm fossil-monorepo_proving_service_db_data
docker volume rm fossil-monorepo_fossil_api_db_data

# Restart (will recreate databases)
make dev-up
```

### API Returns Errors

**Check service health:**
```bash
curl http://localhost:3000/health
curl http://localhost:3001/health
```

**View detailed logs:**
```bash
docker logs fossil-monorepo-fossil-api-1 --tail 100
```

### Proof Generation Hangs

**Check Bonsai API key:**
```bash
# Verify BONSAI_API_KEY is set in .env.docker
grep BONSAI_API_KEY .env.docker
```

**Check message handler logs:**
```bash
docker logs fossil-monorepo-message-handler-1 -f
```

### Timestamp Validation Errors

When manually submitting requests, ensure timestamps are:
- Valid Unix timestamps (seconds since epoch)
- Within the range supported by the vault contract
- End timestamp is after start timestamp

**Use the test script** to automatically calculate valid timestamps:
```bash
./test-local-request.sh
```

## Next Steps

- [Testing Guide](testing.md) - Learn how to write and run tests
- [API Reference](../api-reference/fossil-api-endpoints.md) - Complete API documentation
- [Architecture Overview](../architecture/overview.md) - Understand system design
- [Debugging Guide](../guides/debugging.md) - Advanced debugging techniques

## Quick Reference

```bash
# Start everything
make dev-up

# Stop everything
make dev-down

# Restart services
make dev-down && make dev-up

# View logs
docker compose -f docker-compose.local.yml logs -f

# Test the system
./test-local-request.sh

# Run tests
make test-all

# Lint before committing
make pr
```
