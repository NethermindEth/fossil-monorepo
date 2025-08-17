# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Project Overview

This is a Rust monorepo containing two separate projects:

1. **Proving Service** (`proving-service/`): Handles proof generation and verification with AWS SQS integration
2. **Offchain Processor** (`offchain-processor/`): Processes offchain data and prepares it for proof generation with PostgreSQL integration

Both projects use Rust nightly toolchain and share similar development patterns.

## Essential Commands

### Setup
```bash
make setup              # Set up complete development environment
make setup-ps           # Set up Proving Service only  
make setup-op           # Set up Offchain Processor only
```

### Development Workflow
```bash
make pr                 # ALWAYS run before submitting PRs (lints + tests all projects)
make build-all          # Build both projects in release mode
make test-all           # Run tests for both projects
make lint-all           # Run linters for both projects
```

### Individual Project Commands
```bash
# Proving Service
cd proving-service && make test    # Includes Docker PostgreSQL + LocalStack setup
cd proving-service && make lint    # fmt + clippy + codespell
cd proving-service && make build   # Release build

# Offchain Processor  
cd offchain-processor && make test    # Includes Docker PostgreSQL setup
cd offchain-processor && make lint    # fmt + clippy + codespell
cd offchain-processor && make build   # Release build
```

### Development Services
```bash
make dev-services       # Start all Docker services (PostgreSQL, LocalStack)
make dev-services-stop  # Stop all services
```

## Architecture

### Proving Service
- **Database Layer** (`crates/db/`): PostgreSQL models and queries
- **Message Handler** (`crates/message-handler/`): SQS message processing, proof composition, job dispatching
- **HTTP Service** (`crates/proving-service/`): REST API with job management endpoints
- **External Dependencies**: PostgreSQL, AWS SQS (LocalStack for development)

### Offchain Processor  
- **Database Access** (`crates/db-access/`): PostgreSQL models, migrations, auth, and queries
- **HTTP Server** (`crates/server/`): REST API with auth middleware, job status, and pricing endpoints
- **External Dependencies**: PostgreSQL only

### Shared Patterns
- Both use `cargo +nightly` for formatting and clippy with extensive lint configurations
- Both use Docker Compose for test database setup
- Both have comprehensive code coverage reporting with grcov
- Database migrations are handled differently (offchain-processor has explicit migration files)
- Tests require database services to be running

## Testing Notes
- All tests require Docker services to be running
- Test commands automatically start/stop required services
- Coverage reports generated in `.coverage/html/index.html`
- Use `make coverage` for individual project coverage reports