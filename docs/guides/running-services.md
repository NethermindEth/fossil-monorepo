# Running Services Guide

This guide covers different ways to run services in the Fossil monorepo, from Docker-based development to native execution for maximum iteration speed.

## Table of Contents
- [Overview](#overview)
- [Running with Docker](#running-with-docker)
- [Running Services Individually](#running-services-individually)
- [Docker vs Native Comparison](#docker-vs-native-comparison)
- [Service Dependencies](#service-dependencies)
- [Port Configuration](#port-configuration)
- [Environment Selection](#environment-selection)
- [Hot Reloading for Development](#hot-reloading-for-development)
- [Production Mode](#production-mode)
- [Troubleshooting Startup Issues](#troubleshooting-startup-issues)

## Overview

The Fossil monorepo provides multiple execution strategies:

1. **Full Docker Stack** - Complete environment in containers (recommended for new developers)
2. **Hybrid Approach** - Infrastructure in Docker, services run natively (recommended for active development)
3. **Individual Services** - Run specific services for targeted development

### Service Architecture

```
┌─────────────────┐     ┌──────────────────┐     ┌─────────────────┐
│   Fossil API    │────▶│ Proving Service  │────▶│ Message Handler │
│   (Port 3000)   │     │   API (3001)     │     │  (Background)   │
└─────────────────┘     └──────────────────┘     └─────────────────┘
         │                       │                         │
         ▼                       ▼                         ▼
┌─────────────────┐     ┌──────────────────┐     ┌─────────────────┐
│ Fossil API DB   │     │ Proving Svc DB   │     │   LocalStack    │
│  (Port 5434)    │     │   (Port 5435)    │     │   (Port 4567)   │
└─────────────────┘     └──────────────────┘     └─────────────────┘
```

## Running with Docker

### Full Local Development Stack

The `dev-up` command starts everything including a local Katana testnet:

```bash
# Complete setup (first time)
make setup

# Start all services with local testnet
make dev-up
```

**What this starts:**
- Katana (local StarkNet testnet on port 5050)
- PostgreSQL databases (Fossil API on 5434, Proving Service on 5435)
- LocalStack (AWS services emulation on 4567)
- Contract deployer (one-time deployment to Katana)
- Fossil API service (port 3000)
- Proving Service API (port 3001)
- Message Handler (background service)

**Environment used:** `.env.docker`

**View logs:**
```bash
make logs

# Or specific service
docker compose -f docker-compose.local.yml logs -f fossil-api
docker compose -f docker-compose.local.yml logs -f proving-service-api
docker compose -f docker-compose.local.yml logs -f message-handler
```

**Stop services:**
```bash
# Stop and remove volumes (clean slate)
make dev-down

# Stop without removing volumes
docker compose -f docker-compose.local.yml stop
```

### Services with External Testnet

For testing against Sepolia or other external networks:

```bash
# Start services without Katana (uses .env.sepolia)
make services-up
```

**What this starts:**
- PostgreSQL databases (Fossil API on 5434, Proving Service on 5435)
- LocalStack (AWS services emulation on 4567)
- Fossil API service (port 3000)
- Proving Service API (port 3001)
- Message Handler (background service)

**Environment used:** `.env.sepolia`

**Stop services:**
```bash
make services-down
```

### Docker Compose Commands

For more control, use docker compose directly:

```bash
# Start specific services
docker compose -f docker-compose.local.yml up -d fossil_api_db proving_service_db

# Rebuild and start
docker compose -f docker-compose.local.yml up -d --build fossil-api

# Scale message handler (if needed)
docker compose -f docker-compose.local.yml up -d --scale message-handler=2

# Check service status
docker compose -f docker-compose.local.yml ps

# View resource usage
docker stats

# Execute commands in running container
docker compose -f docker-compose.local.yml exec fossil-api sh
```

## Running Services Individually

For faster iteration during development, run services natively while infrastructure runs in Docker.

### Prerequisites

```bash
# Install Rust and tools
make setup

# Start only infrastructure services
docker compose -f docker-compose.local.yml up -d katana fossil_api_db proving_service_db localstack

# Wait for databases to be ready
docker compose -f docker-compose.local.yml ps
```

### Fossil API

The HTTP server for offchain data processing.

**Location:** `/home/ametel/source/fossil-monorepo/fossil-api`

**Run in development mode:**
```bash
cd /home/ametel/source/fossil-monorepo/fossil-api

# Load environment and run
export $(grep -v '^#' ../.env.local | xargs)
cargo run --bin server
```

**Build release version:**
```bash
cd /home/ametel/source/fossil-monorepo/fossil-api
make build

# Run the binary directly
./target/release/server
```

**Available binaries:**
- `server` - Main HTTP API server (default)
- `create_api_key` - Utility to create API keys

**Example with specific binary:**
```bash
# Create an API key
cargo run --bin create_api_key

# Run with custom port
SERVER_PORT=3005 cargo run --bin server
```

**Environment variables:**
- `OFFCHAIN_PROCESSOR_DATABASE_URL` - Database connection string
- `INDEXER_DATABASE_URL` - Read-only indexer database
- `PROVING_SERVICE_URL` - Proving service endpoint (default: http://127.0.0.1:3001)
- `SERVER_PORT` - HTTP port (default: 3000)
- `RUST_LOG` - Log level (debug, info, warn, error)

### Proving Service API

The HTTP API for proof job management.

**Location:** `/home/ametel/source/fossil-monorepo/proving-service`

**Run in development mode:**
```bash
cd /home/ametel/source/fossil-monorepo/proving-service

# Load environment and run
export $(grep -v '^#' ../.env.local | xargs)
cargo run --bin proving-service
```

**Build release version:**
```bash
cd /home/ametel/source/fossil-monorepo/proving-service
make build

# Run with mock-proof feature (faster for testing)
cargo run --bin proving-service --release --features mock-proof
```

**Environment variables:**
- `PROVING_SERVICE_DATABASE_URL` - Database connection string
- `AWS_ENDPOINT_URL` - LocalStack or AWS endpoint
- `SQS_QUEUE_URL` - SQS queue URL for job dispatching
- `SERVER_PORT` - HTTP port (default: 3001)
- `RUST_LOG` - Log level

### Message Handler

Background service that processes SQS messages and generates proofs.

**Location:** `/home/ametel/source/fossil-monorepo/proving-service`

**Run in development mode:**
```bash
cd /home/ametel/source/fossil-monorepo/proving-service

# Load environment and run
export $(grep -v '^#' ../.env.local | xargs)
cargo run --bin message-handler --features mock-proof
```

**Build release version:**
```bash
cd /home/ametel/source/fossil-monorepo/proving-service

# Build with mock proofs for faster testing
cargo build --release --features mock-proof --bin message-handler

# Run the binary
./target/release/message-handler
```

**Available binaries:**
- `message-handler` - Main SQS message processor
- `example-message-handler` - Example implementation
- `bonsai-test` - Test Bonsai API integration

**Feature flags:**
- `mock-proof` - Use mock proof generation (fast, for testing)
- `proof-composition` - Full proof composition with RISC Zero (slow, production)

**Environment variables:**
- `PROVING_SERVICE_DATABASE_URL` - Database connection string
- `AWS_ENDPOINT_URL` - LocalStack or AWS endpoint
- `SQS_QUEUE_URL` - SQS queue to poll for jobs
- `STARKNET_RPC_URL` - StarkNet RPC endpoint
- `ENABLE_PROOF` - Enable/disable proof generation
- `USE_RISC0_INTEGRATION` - Use RISC Zero for proofs
- `BONSAI_API_KEY` - Bonsai API key (if using Bonsai)
- `MAX_CONCURRENT_PROOFS` - Limit concurrent proof jobs

### Running Multiple Services

Use terminal multiplexer or separate terminals:

```bash
# Terminal 1: Infrastructure
docker compose -f docker-compose.local.yml up katana fossil_api_db proving_service_db localstack

# Terminal 2: Fossil API
cd fossil-api && cargo run --bin server

# Terminal 3: Proving Service API
cd proving-service && cargo run --bin proving-service --features mock-proof

# Terminal 4: Message Handler
cd proving-service && cargo run --bin message-handler --features mock-proof
```

**Using tmux:**
```bash
# Create session
tmux new-session -s fossil

# Split panes (Ctrl+b then %)
# Navigate panes (Ctrl+b then arrow keys)
# Run each service in different panes
```

**Using screen:**
```bash
# Create session
screen -S fossil

# Create new window (Ctrl+a then c)
# Switch windows (Ctrl+a then n/p)
# Run each service in different windows
```

## Docker vs Native Comparison

| Aspect | Docker | Native |
|--------|--------|--------|
| **Setup Time** | Slower (image builds) | Faster (direct cargo run) |
| **Iteration Speed** | Slower (rebuild container) | Faster (instant recompile) |
| **Environment Parity** | High (same as production) | Variable (depends on host) |
| **Resource Usage** | Higher (container overhead) | Lower (direct execution) |
| **Debugging** | More complex (container access) | Easier (direct access) |
| **Hot Reload** | Requires volume mounts | Easy with cargo-watch |
| **Isolation** | Complete | Shared with host |
| **Database Access** | Auto-configured | Manual env setup |
| **Logs** | docker logs / docker compose | stdout/stderr |
| **Port Conflicts** | Configurable mapping | Direct conflict risk |

**Recommendations:**

- **New developers:** Start with Docker (`make dev-up`)
- **Active development:** Hybrid (infrastructure in Docker, services native)
- **Testing integrations:** Full Docker stack
- **Production:** Docker with proper orchestration (Kubernetes, ECS)

## Service Dependencies

### Fossil API Dependencies

**Required:**
- PostgreSQL (Fossil API DB) on port 5434
- PostgreSQL (Indexer DB) on port 5433 (if using real data)

**Optional:**
- Proving Service API on port 3001 (for proof job submission)
- LocalStack on port 4567 (for AWS integration)

**Startup order:**
```bash
# 1. Start databases
docker compose -f docker-compose.local.yml up -d fossil_api_db

# 2. Wait for database ready
until pg_isready -h localhost -p 5434; do sleep 1; done

# 3. Run migrations (automatic on startup)
# 4. Start Fossil API
cd fossil-api && cargo run --bin server
```

### Proving Service API Dependencies

**Required:**
- PostgreSQL (Proving Service DB) on port 5435
- LocalStack (SQS) on port 4567

**Optional:**
- Message Handler (to process queued jobs)

**Startup order:**
```bash
# 1. Start database and LocalStack
docker compose -f docker-compose.local.yml up -d proving_service_db localstack

# 2. Wait for services ready
until pg_isready -h localhost -p 5435; do sleep 1; done
aws --endpoint-url=http://localhost:4567 sqs list-queues

# 3. Run migrations (automatic on startup)
# 4. Start Proving Service API
cd proving-service && cargo run --bin proving-service --features mock-proof
```

### Message Handler Dependencies

**Required:**
- PostgreSQL (Proving Service DB) on port 5435
- LocalStack (SQS) on port 4567
- StarkNet RPC (Katana on port 5050 or external testnet)

**Optional:**
- Bonsai API (for production proof generation)

**Startup order:**
```bash
# 1. Start all infrastructure
docker compose -f docker-compose.local.yml up -d proving_service_db localstack katana

# 2. Wait for services ready
until pg_isready -h localhost -p 5435; do sleep 1; done
aws --endpoint-url=http://localhost:4567 sqs list-queues

# 3. Deploy contracts (if using Katana)
docker compose -f docker-compose.deploy.yml run --rm contract-deployer

# 4. Start Message Handler
cd proving-service && cargo run --bin message-handler --features mock-proof
```

### Dependency Graph

```
Fossil API
├── fossil_api_db (required)
├── indexer_db (optional)
└── proving-service-api (optional)

Proving Service API
├── proving_service_db (required)
└── localstack (required)

Message Handler
├── proving_service_db (required)
├── localstack (required)
├── katana/starknet-rpc (required)
└── bonsai-api (optional)
```

## Port Configuration

### Default Port Mapping

| Service | Native Port | Docker Internal | Docker Host |
|---------|-------------|-----------------|-------------|
| Fossil API | 3000 | 3000 | 3000 |
| Proving Service API | 3001 | 3001 | 3001 |
| Message Handler | N/A | N/A | N/A |
| Fossil API DB | 5432 | 5432 | 5434 |
| Proving Service DB | 5432 | 5432 | 5435 |
| Indexer DB | 5432 | 5432 | 5433 |
| Katana | 5050 | 5050 | 5050 |
| LocalStack | 4566 | 4566 | 4567 |

### Changing Ports

**For native execution:**

```bash
# Change Fossil API port
export SERVER_PORT=3005
cargo run --bin server

# Change Proving Service port
export SERVER_PORT=3010
cargo run --bin proving-service
```

**For Docker:**

Edit `docker-compose.local.yml`:

```yaml
services:
  fossil-api:
    ports:
      - "3005:3000"  # Map host 3005 to container 3000
    environment:
      SERVER_PORT: 3000  # Keep internal port
```

**For database ports:**

Edit `docker-compose.local.yml`:

```yaml
services:
  fossil_api_db:
    ports:
      - "5440:5432"  # Change host port
```

Update `.env.local`:
```bash
OFFCHAIN_PROCESSOR_DATABASE_URL=postgresql://postgres:postgres@localhost:5440/postgres
```

### Resolving Port Conflicts

**Check what's using a port:**
```bash
# Linux
lsof -i :3000
netstat -tulpn | grep 3000

# macOS
lsof -i :3000

# Kill process
kill -9 <PID>
```

**Change conflicting port:**
```bash
# Option 1: Change application port
export SERVER_PORT=3005

# Option 2: Stop conflicting service
docker stop <container_name>

# Option 3: Modify docker-compose port mapping
```

## Environment Selection

The monorepo uses different environment files for different deployment scenarios.

### Environment File Priority

1. **Docker Compose** explicitly specifies env file:
   - `docker-compose.local.yml` → `.env.docker`
   - `docker-compose.services.yml` → `.env.sepolia`

2. **Native execution** sources environment:
   - Load `.env.local` manually: `export $(grep -v '^#' .env.local | xargs)`
   - Use `dotenv` crate (automatic if `.env` exists)

3. **Shell environment** variables override file values

### Environment Files Overview

| File | Use Case | Network | Database Hosts |
|------|----------|---------|----------------|
| `.env.example` | Template | Any | localhost |
| `.env.local` | Native dev | Katana (localhost:5050) | localhost:543X |
| `.env.docker` | Docker dev | Katana (katana:5050) | <service>_db:5432 |
| `.env.sepolia` | Testnet | Sepolia RPC | localhost:543X or remote |

### Using Specific Environment

**Docker with custom env file:**
```bash
# Use .env.sepolia instead of default
docker compose --env-file .env.sepolia -f docker-compose.services.yml up -d

# Override specific variable
ENABLE_PROOF=false docker compose -f docker-compose.local.yml up -d
```

**Native with custom env file:**
```bash
# Load .env.sepolia
export $(grep -v '^#' .env.sepolia | xargs)
cargo run --bin server

# Or use env_file
env $(cat .env.sepolia | xargs) cargo run --bin server

# Or create symlink
ln -sf .env.sepolia .env
cargo run  # dotenv crate loads .env automatically
```

**Check loaded environment:**
```bash
# Print specific variable
echo $STARKNET_RPC_URL
echo $OFFCHAIN_PROCESSOR_DATABASE_URL

# View all variables
printenv | grep -E '(STARKNET|DATABASE|AWS|PROVING)'

# Inside running Docker container
docker compose -f docker-compose.local.yml exec fossil-api env | grep DATABASE
```

### Environment Variables by Service

**Fossil API:**
- `OFFCHAIN_PROCESSOR_DATABASE_URL`
- `INDEXER_DATABASE_URL`
- `PROVING_SERVICE_URL`
- `SERVER_PORT`
- `ALLOWED_ORIGINS` (CORS)

**Proving Service API:**
- `PROVING_SERVICE_DATABASE_URL`
- `AWS_ENDPOINT_URL`
- `SQS_QUEUE_URL`
- `SERVER_PORT`

**Message Handler:**
- `PROVING_SERVICE_DATABASE_URL`
- `AWS_ENDPOINT_URL`
- `SQS_QUEUE_URL`
- `STARKNET_RPC_URL`
- `STARKNET_ACCOUNT_ADDRESS`
- `STARKNET_PRIVATE_KEY`
- `BONSAI_API_KEY` (optional)
- `ENABLE_PROOF`
- `USE_RISC0_INTEGRATION`
- `MAX_CONCURRENT_PROOFS`

## Hot Reloading for Development

For rapid iteration, use `cargo-watch` to automatically rebuild and restart on file changes.

### Install cargo-watch

```bash
cargo install cargo-watch
```

### Basic Usage

**Fossil API with auto-reload:**
```bash
cd /home/ametel/source/fossil-monorepo/fossil-api

# Watch and run
cargo watch -x 'run --bin server'

# Watch with clear screen
cargo watch -c -x 'run --bin server'

# Watch and run tests
cargo watch -x test
```

**Proving Service API with auto-reload:**
```bash
cd /home/ametel/source/fossil-monorepo/proving-service

# Watch and run with features
cargo watch -x 'run --bin proving-service --features mock-proof'

# Watch specific files
cargo watch -w src/main.rs -w src/handlers/ -x 'run --bin proving-service'
```

**Message Handler with auto-reload:**
```bash
cd /home/ametel/source/fossil-monorepo/proving-service

# Watch and run
cargo watch -x 'run --bin message-handler --features mock-proof'
```

### Advanced cargo-watch Options

```bash
# Run tests and then run binary
cargo watch -x test -x 'run --bin server'

# Watch specific paths
cargo watch -w src -w Cargo.toml -x run

# Ignore target directory (default, but can specify)
cargo watch -i target/ -x run

# Execute shell command on change
cargo watch -s 'cargo build && ./target/debug/server'

# Delay restart (useful for file cascades)
cargo watch --delay 2 -x run

# Watch and check only (no run)
cargo watch -x check
```

### Hot Reload with Environment

```bash
# Load env and watch
export $(grep -v '^#' ../.env.local | xargs) && \
cargo watch -c -x 'run --bin server'

# Or create a script
cat > watch.sh << 'EOF'
#!/bin/bash
export $(grep -v '^#' ../.env.local | xargs)
cargo watch -c -x 'run --bin server'
EOF
chmod +x watch.sh
./watch.sh
```

### Multiple Services with Hot Reload

Use tmux or screen to run multiple watch sessions:

```bash
# tmux example
tmux new-session -s fossil \; \
  send-keys 'cd fossil-api && cargo watch -x "run --bin server"' C-m \; \
  split-window -h \; \
  send-keys 'cd proving-service && cargo watch -x "run --bin proving-service --features mock-proof"' C-m \; \
  split-window -v \; \
  send-keys 'cd proving-service && cargo watch -x "run --bin message-handler --features mock-proof"' C-m
```

### Caveats

- First compilation is slow; subsequent rebuilds are incremental
- Database schema changes require manual migration
- Environment variable changes require watch restart
- Mock-proof feature is much faster for development than full proof composition

## Production Mode

Production deployments use release builds with optimizations.

### Building Release Binaries

**Build all projects:**
```bash
# From repository root
make build

# This runs:
# - cd proving-service && cargo build --release
# - cd fossil-api && cargo build --release
```

**Build individual services:**
```bash
# Fossil API
cd /home/ametel/source/fossil-monorepo/fossil-api
cargo build --release

# Proving Service (with proof composition)
cd /home/ametel/source/fossil-monorepo/proving-service
cargo build --release --features proof-composition

# Message Handler (with proof composition)
cd /home/ametel/source/fossil-monorepo/proving-service
cargo build --release --features proof-composition --bin message-handler
```

**Binary locations:**
- Fossil API: `/home/ametel/source/fossil-monorepo/fossil-api/target/release/server`
- Proving Service: `/home/ametel/source/fossil-monorepo/proving-service/target/release/proving-service`
- Message Handler: `/home/ametel/source/fossil-monorepo/proving-service/target/release/message-handler`

### Running Release Binaries

**With environment file:**
```bash
# Load production environment
export $(grep -v '^#' .env.production | xargs)

# Run binary
./target/release/server

# Or in one line
env $(cat .env.production | xargs) ./target/release/server
```

**With systemd:**

Create `/etc/systemd/system/fossil-api.service`:
```ini
[Unit]
Description=Fossil API Service
After=network.target postgresql.service

[Service]
Type=simple
User=fossil
WorkingDirectory=/opt/fossil/fossil-api
EnvironmentFile=/opt/fossil/.env.production
ExecStart=/opt/fossil/fossil-api/target/release/server
Restart=on-failure
RestartSec=10

[Install]
WantedBy=multi-user.target
```

Manage service:
```bash
sudo systemctl daemon-reload
sudo systemctl enable fossil-api
sudo systemctl start fossil-api
sudo systemctl status fossil-api

# View logs
sudo journalctl -u fossil-api -f
```

### Docker Production Images

**Build production images:**
```bash
# Message Handler with pre-compiled proofs
make build-message-handler-image

# Fossil API
docker build -f docker/Dockerfile.fossil-api -t fossil-api:production .

# Proving Service API
docker build -f docker/Dockerfile.proving-service-api -t fossil-proving-service-api:production .
```

**Run production container:**
```bash
docker run -d \
  --name fossil-api \
  --env-file .env.production \
  -p 3000:3000 \
  fossil-api:production
```

### Production Optimization

**Compile-time optimizations:**

Edit `Cargo.toml`:
```toml
[profile.release]
opt-level = 3           # Maximum optimization
lto = true              # Link-time optimization
codegen-units = 1       # Better optimization (slower compile)
panic = 'abort'         # Smaller binary
strip = true            # Strip symbols
```

**Runtime optimizations:**

```bash
# Set production environment variables
export RUST_LOG=info                    # Reduce log verbosity
export MAX_CONCURRENT_PROOFS=10         # Tune concurrency
export DATABASE_POOL_SIZE=20            # Connection pooling
export ENABLE_METRICS=true              # Enable monitoring

# Use production feature flags
export USE_MOCK_PRICING_DATA=false
export VERIFY_PROOFS_ONCHAIN=true
export USE_SIMPLE_MOCK=false
```

**Resource limits:**

```bash
# Using systemd
LimitNOFILE=65535
LimitNPROC=4096

# Using Docker
docker run -d \
  --memory="2g" \
  --cpus="2" \
  --ulimit nofile=65535:65535 \
  fossil-api:production
```

## Troubleshooting Startup Issues

### Service Fails to Start

**Problem:** Service exits immediately or fails to bind to port

**Solutions:**

```bash
# Check if port is in use
lsof -i :3000
lsof -i :3001

# Check environment variables
echo $OFFCHAIN_PROCESSOR_DATABASE_URL
echo $PROVING_SERVICE_DATABASE_URL

# Verify database is running
pg_isready -h localhost -p 5434

# Check for compilation errors
cargo check --bin server
cargo check --bin proving-service

# Run with verbose logging
RUST_LOG=debug cargo run --bin server

# Check if .env file exists
ls -la .env .env.local .env.docker
```

### Database Connection Failures

**Problem:** `connection refused` or `FATAL: role does not exist`

**Solutions:**

```bash
# Verify database is running
docker compose -f docker-compose.local.yml ps | grep db

# Check database logs
docker compose -f docker-compose.local.yml logs fossil_api_db

# Test connection manually
psql postgresql://postgres:postgres@localhost:5434/postgres

# Common issues:
# 1. Wrong port in connection string
# 2. Database container not ready yet
# 3. Firewall blocking connection

# Wait for database ready
until pg_isready -h localhost -p 5434; do
  echo "Waiting for database..."
  sleep 1
done

# Restart database
docker compose -f docker-compose.local.yml restart fossil_api_db
```

### SQS/LocalStack Connection Issues

**Problem:** Cannot connect to SQS queue

**Solutions:**

```bash
# Check LocalStack is running
docker compose -f docker-compose.local.yml ps | grep localstack

# Check LocalStack logs
docker compose -f docker-compose.local.yml logs localstack

# List queues
aws --endpoint-url=http://localhost:4567 sqs list-queues

# Create queue if missing
aws --endpoint-url=http://localhost:4567 sqs create-queue --queue-name fossilQueue

# Verify queue URL
aws --endpoint-url=http://localhost:4567 sqs get-queue-url --queue-name fossilQueue

# Recreate LocalStack
docker compose -f docker-compose.local.yml restart localstack

# Or fully reset
docker compose -f docker-compose.local.yml down -v
make dev-up
```

### StarkNet RPC Connection Failures

**Problem:** `Error: node not reachable` or RPC timeout

**Solutions:**

```bash
# Check if Katana is running
docker compose -f docker-compose.local.yml ps | grep katana

# View Katana logs
docker compose -f docker-compose.local.yml logs katana

# Test RPC connection
curl -X POST http://localhost:5050 \
  -H "Content-Type: application/json" \
  -d '{"jsonrpc":"2.0","method":"starknet_chainId","params":[],"id":1}'

# Expected response: {"id":1,"jsonrpc":"2.0","result":"0x534e5f474f45524c49"}

# Restart Katana
docker compose -f docker-compose.local.yml restart katana

# Check if contracts are deployed
grep PITCHLAKE_VERIFIER_CONTRACT .env.local

# Redeploy contracts
docker compose -f docker-compose.deploy.yml run --rm contract-deployer
```

### Cargo Build Failures

**Problem:** Compilation errors or dependency issues

**Solutions:**

```bash
# Clean and rebuild
cargo clean
cargo build

# Update dependencies
cargo update

# Check for feature conflicts
cargo tree

# Ensure correct Rust version
rustup update stable
rustup default stable

# For RISC Zero dependencies (proving-service)
# May need to install rzup
curl -L https://risczero.com/install | bash
rzup install

# Platform-specific dependencies (Ubuntu/Debian)
sudo apt-get install -y protobuf-compiler libpq-dev pkg-config

# Platform-specific dependencies (macOS)
brew install protobuf postgresql
```

### Docker Build Failures

**Problem:** Docker image build fails

**Solutions:**

```bash
# Check Docker daemon is running
docker info

# Clean up build cache
docker builder prune -a

# Rebuild without cache
docker compose -f docker-compose.local.yml build --no-cache fossil-api

# Check Dockerfile syntax
docker build -f docker/Dockerfile.fossil-api -t test .

# Check available disk space
df -h

# Increase Docker resources (Docker Desktop)
# Settings → Resources → increase Memory/CPU

# View build logs
docker compose -f docker-compose.local.yml build fossil-api 2>&1 | tee build.log
```

### Environment Variable Issues

**Problem:** Service uses wrong configuration or variables not loaded

**Solutions:**

```bash
# Verify environment file exists
ls -la .env.local .env.docker .env.sepolia

# Check file permissions
chmod 600 .env.local

# Validate environment file format
# No spaces around = sign
# Correct: KEY=value
# Wrong: KEY = value

# Print loaded variables
printenv | grep -E '(DATABASE|STARKNET|AWS)'

# Inside Docker container
docker compose -f docker-compose.local.yml exec fossil-api printenv | grep DATABASE

# Reload environment
export $(grep -v '^#' .env.local | xargs)

# Check for docker-compose env_file path
# Must be relative to docker-compose.yml location
```

### Permission Errors

**Problem:** Permission denied errors

**Solutions:**

```bash
# For Docker volumes
# Stop services
docker compose -f docker-compose.local.yml down

# Remove volumes
docker volume rm fossil-monorepo_fossil_api_local_data
docker volume rm fossil-monorepo_proving_service_local_data

# Restart
make dev-up

# For native execution
# Check file permissions
ls -la target/release/

# Make binary executable
chmod +x target/release/server

# Check database directory permissions (if using local postgres)
ls -ld /var/lib/postgresql/data
```

### Health Check Failures

**Problem:** Docker container marked as unhealthy

**Solutions:**

```bash
# Check health status
docker compose -f docker-compose.local.yml ps

# View health check logs
docker inspect <container_id> | jq '.[0].State.Health'

# Common issues:
# 1. Service not binding to 0.0.0.0 (binds to 127.0.0.1 only)
# 2. Health endpoint not implemented
# 3. Service takes too long to start

# Increase health check timeout
# Edit docker-compose.local.yml:
# healthcheck:
#   interval: 30s
#   timeout: 10s
#   retries: 5
#   start_period: 60s

# Manually test health endpoint
curl http://localhost:3000/health
curl http://localhost:3001/health
```

### Memory/Resource Issues

**Problem:** Out of memory or high CPU usage

**Solutions:**

```bash
# Check resource usage
docker stats

# For Rust compilation
# Reduce parallelism
cargo build -j 2

# Increase Docker resources (Docker Desktop)
# Settings → Resources

# For native execution
# Monitor process
top -p $(pgrep -f server)
htop

# Limit concurrent proofs
export MAX_CONCURRENT_PROOFS=1

# Use mock-proof feature during development
cargo run --features mock-proof

# Enable release mode for better performance
cargo run --release
```

## Next Steps

- [Environment Setup Guide](environment-setup.md) - Detailed environment configuration
- [Database Management](database-management.md) - Managing databases and migrations
- [Deployment Guide](deployment.md) - Production and staging deployment
- [Local Development Guide](../getting-started/local-development.md) - Complete workflow
- [Testing Guide](testing.md) - Running and writing tests
