# Fossil Prover Service

[![Rust CI](https://github.com/NethermindEth/fossil-prover-service/workflows/Rust%20CI/badge.svg)](https://github.com/NethermindEth/fossil-prover-service/actions?query=workflow%3A%22Rust+CI%22)
[![Coverage](https://img.shields.io/badge/coverage-53.3%25-yellow)](https://github.com/NethermindEth/fossil-prover-service)

A service that processes jobs through AWS SQS and exposes an HTTP API for job submission.

## Table of Contents

- [Architecture & Project Structure](#architecture--project-structure)
- [Development Setup](#development-setup)
- [Running the Services](#running-the-services)
- [HTTP API Documentation](#http-api)
- [Contributing](#contributing-and-pull-requests)
- [Testing & Code Coverage](#code-coverage)
- [Advanced Topics](#feature-flags)

## Architecture & Project Structure

The project is organized into multiple crates and supporting directories:

- `message-handler` - Handles message processing from queues
- `proving-service` - Provides the HTTP API for job submission
- `db` - Database interface for persisting data
- `scripts` - Shell scripts for various development tasks
- `docker` - Docker-related configuration files

The system is composed of two main services:

1. **HTTP API Service**: Accepts job requests and sends them to the message queue
2. **Message Handler Service**: Processes jobs from the queue, generates proofs, and tracks results

## Development Setup

### Prerequisites

Before running any of the applications, you need to set up the required services:

1. **Start all development services:**

   ```bash
   # Start both PostgreSQL and LocalStack services
   make dev-services
   ```

2. **Stop all services when done:**

   ```bash
   # Stop all development services
   make dev-services-stop
   ```

3. **Clean up when needed:**

   ```bash
   # Clean all artifacts
   make clean
   ```

### Make Commands

This project includes a comprehensive Makefile to simplify common development tasks:

```bash
# Setup your development environment
make setup              # Install all dependencies
make setup-rust         # Install Rust and toolchains
make setup-postgres     # Set up PostgreSQL for development
make setup-localstack   # Set up LocalStack for AWS services
make setup-coverage     # Install code coverage tools

# Development
make build              # Build the project in release mode
make build-debug        # Build the project in debug mode
make dev-services       # Start all development services
make dev-services-stop  # Stop all development services

# Testing
make test               # Run all tests with database dependencies
make test-clean         # Clean up test environment

# Code Quality
make lint               # Run all linters
make fmt                # Format code with rustfmt
make clippy             # Run clippy linter
make pr                 # Prepare code for a pull request

# Help
make help               # Display all available commands
```

For more details on each command, run `make help`.

### LocalStack SQS Setup

The service uses AWS SQS for message queuing. The `make dev-services` command sets this up automatically, but if you need to manage it separately:

```bash
./scripts/setup-localstack.sh
```

## Running the Services

### HTTP API Service

Run the HTTP API service with:

```bash
cargo run -p proving-service
```

This will start the HTTP server on <http://127.0.0.1:3001> and connect to the SQS queue.

### Message Handler Service

The message handler processes jobs from the SQS queue and can be run with different proof generation options.

#### Using the Run Script (Recommended)

```bash
# Run without proof generation (fastest, for development)
./scripts/run-message-handler.sh

# Run with full proof composition (requires all dependencies)
./scripts/run-message-handler.sh --proof-composition

# Run with mock proofs (faster than full composition, good for testing)
./scripts/run-message-handler.sh --mock-proof

# Enable proofs with the default feature set
./scripts/run-message-handler.sh --enable-proof
```

#### Testing the Queue Processing

To test the message handler by sending a sample job to the queue and processing it:

```bash
# This will send a test job to the queue and run the handler
./scripts/test-message-handler.sh
```

#### Manual Configuration

If you need more control, you can run the service directly with Cargo:

```bash
# Run without proof generation
ENABLE_PROOF=false cargo run -p message-handler --bin message-handler

# Run with proof composition enabled
ENABLE_PROOF=true cargo run -p message-handler --bin message-handler --features "proof-composition"

# Run with mock proof generation
ENABLE_PROOF=true cargo run -p message-handler --bin message-handler --features "mock-proof"
```

## HTTP API

The service exposes a single HTTP endpoint for submitting jobs:

### Endpoint

```bash
POST http://127.0.0.1:3001/api/job
```

### Request Format

```json
{
    "job_group_id": "job_123",
    "twap": {
        "start_timestamp": 1234567890,
        "end_timestamp": 1234567891
    },
    "reserve_price": {
        "start_timestamp": 1234567890,
        "end_timestamp": 1234567891
    },
    "max_return": {
        "start_timestamp": 1234567890,
        "end_timestamp": 1234567891
    }
}
```

The `job_group_id` field is required and is used in the ID of the combined job. The service creates a single combined job containing time ranges for all three components (twap, reserve_price, max_return).

### Response Format

#### Success Response

```json
{
    "status": "success",
    "message": "Job dispatched successfully",
    "job_group_id": "job_123"
}
```

#### Error Response

```json
{
    "status": "error",
    "message": "Failed to dispatch job: error message",
    "job_group_id": "job_123"
}
```

### Example Usage with curl

```bash
curl -X POST http://127.0.0.1:3001/api/job \
  -H "Content-Type: application/json" \
  -d '{
    "job_group_id": "job_123",
    "twap": {
        "start_timestamp": 1234567890,
        "end_timestamp": 1234567891
    },
    "reserve_price": {
        "start_timestamp": 1234567890,
        "end_timestamp": 1234567891
    },
    "max_return": {
        "start_timestamp": 1234567890,
        "end_timestamp": 1234567891
    }
  }'
```

### Example Usage with Rust

```rust
use reqwest;
use serde_json::json;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let client = reqwest::Client::new();
    
    let response = client
        .post("http://127.0.0.1:3001/api/job")
        .json(&json!({
            "job_group_id": "job_123",
            "twap": {
                "start_timestamp": 1234567890,
                "end_timestamp": 1234567891
            },
            "reserve_price": {
                "start_timestamp": 1234567890,
                "end_timestamp": 1234567891
            },
            "max_return": {
                "start_timestamp": 1234567890,
                "end_timestamp": 1234567891
            }
        }))
        .send()
        .await?;
    
    let result = response.json::<serde_json::Value>().await?;
    println!("Response: {:?}", result);
    
    Ok(())
}
```

## Contributing and Pull Requests

**IMPORTANT:** Before submitting a pull request, always run:

```bash
make pr
```

This command:

1. Formats all code consistently
2. Runs clippy to catch common issues
3. Runs tests to verify your changes
4. Ensures your PR will pass CI checks

Running `make pr` locally saves time by catching issues early rather than waiting for CI failures after submission.

### Continuous Integration

The project uses GitHub Actions for continuous integration:

- **Unit Tests**: Run on every PR and push to main
- **Integration Tests**: Run on every PR and push to main
- **Code Coverage**: Generated during test runs with coverage reporting shown in the README badge

## Code Coverage

The project includes comprehensive code coverage tracking using LLVM's source-based code coverage tools and grcov.

### Setting Up Coverage Tools

The easiest way to set up code coverage tools is to run:

```bash
./scripts/setup-coverage.sh
```

This script will:

1. Install the LLVM tools component via rustup
2. Install grcov if not already installed
3. Create a dedicated `.coverage` directory for all coverage files
4. Set up necessary environment variables

Alternatively, set up the tools manually:

```bash
# Install LLVM tools
rustup component add llvm-tools-preview

# Install grcov
cargo install grcov

# Create coverage directory
mkdir -p .coverage
```

### Coverage Commands

```bash
# Run tests with coverage
make coverage           # Generate HTML report
make coverage-view      # Open the report in browser
make coverage-xml       # Generate XML report for CI
make coverage-clean     # Clean up artifacts
make coverage-summary   # Display text summary
make coverage-badge     # Generate badge for README
```

## Feature Flags

The project uses Cargo feature flags to enable optional functionality:

### Proof Composition Features

- **proof-composition**: Enables the full proof composition system
- **mock-proof**: Enables a mock proof system (faster for testing)

```bash
# Build without proof composition (faster compilation)
cargo build

# With full proof composition 
cargo build --features "message-handler/proof-composition"

# With mock proof system
cargo build --features "message-handler/mock-proof"
```

### Runtime Proof Generation Control

In addition to the compile-time feature flags, the message handler service supports the `ENABLE_PROOF` environment variable:

- `ENABLE_PROOF=true`: Attempts to generate proofs using the compiled-in method
- `ENABLE_PROOF=false`: Skips proof generation and acknowledges jobs without processing

This allows you to run the service with proof generation compiled in but disabled, useful for development and testing scenarios.

The run scripts automatically set this environment variable based on the command-line arguments.
