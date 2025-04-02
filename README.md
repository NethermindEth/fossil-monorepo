# Prover service

[![Rust CI](https://github.com/NethermindEth/fossil-prover-service/workflows/Rust%20CI/badge.svg)](https://github.com/NethermindEth/fossil-prover-service/actions?query=workflow%3A%22Rust+CI%22)
[![Coverage](https://img.shields.io/badge/coverage-53.3%25-yellow)](https://github.com/NethermindEth/fossil-prover-service)

A service that processes jobs through AWS SQS and exposes an HTTP API for job submission.

## Getting Started with Make

This project includes a comprehensive Makefile to simplify common development tasks. Here are the main commands:

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

# Code Coverage
make coverage           # Run tests with coverage and generate HTML report
make coverage-view      # Open the coverage report in a browser
make coverage-xml       # Generate code coverage report in XML format for CI
make coverage-clean     # Clean up coverage artifacts
make coverage-summary   # Display a text summary of the coverage report
make coverage-badge     # Generate a badge for the README

# Code Quality
make lint               # Run all linters
make fmt                # Format code with rustfmt
make clippy             # Run clippy linter
make pr                 # Prepare code for a pull request

# Help
make help               # Display all available commands
```

For more details on each command, run `make help`.

## Project Structure

The project is organized into multiple crates and supporting directories:

- `message-handler` - Handles message processing from queues
- `proving-service` - Provides the HTTP API for job submission
- `db` - Database interface for persisting data
- `scripts` - Shell scripts for various development tasks
- `docker` - Docker-related configuration files

## Development Setup

### LocalStack SQS Setup

The service uses AWS SQS for message queuing. For local development, you can use LocalStack to create a local SQS service:

1. Start the LocalStack container:

   ```bash
   docker-compose -f docker/docker-compose.sqs.yml up -d
   ```

2. Set up the SQS queue:

   ```bash
   ./scripts/setup-localstack.sh
   ```

3. Verify the queue was created successfully by checking the output of the script.

### Running the Application

You can run both services separately:

#### HTTP API Service

Run the HTTP API service with:

```bash
cargo run -p proving-service
```

This will start the HTTP server on <http://127.0.0.1:3000> and connect to the SQS queue.

#### Message Handler Service

Run the message handler service with:

```bash
cargo run -p message-handler
```

This will start a service that consumes messages from the SQS queue.

## HTTP API

The service exposes a single HTTP endpoint for submitting jobs:

### Endpoint

```bash
POST http://127.0.0.1:3000/api/job
```

### Request Format

Send a POST request with a JSON body in the following format:

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

The `job_group_id` field is required and groups all three proofs together. Each proof type (twap, reserve_price, max_return) requires its own time range.

### Response Format

#### Success Response

```json
{
    "status": "success",
    "message": "All jobs dispatched successfully",
    "job_group_id": "job_123"
}
```

#### Error Response

```json
{
    "status": "error",
    "message": "TWAP job failed: error1, Reserve Price job failed: error2",
    "job_group_id": "job_123"
}
```

### Example Usage with curl

```bash
curl -X POST http://127.0.0.1:3000/api/job \
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
        .post("http://127.0.0.1:3000/api/job")
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

## Continuous Integration

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

Alternatively, you can set up the tools manually:

```bash
# Install LLVM tools
rustup component add llvm-tools-preview

# Install grcov
cargo install grcov

# Create coverage directory
mkdir -p .coverage
```

### Viewing Coverage Reports Locally

Run the tests with coverage enabled and generate an HTML report:

```bash
make coverage
```

This will:

1. Install LLVM tools component if necessary
2. Start any required dependencies
3. Run the test suite with coverage instrumentation
4. Generate an HTML report at `.coverage/html/index.html`

### Generating a Coverage Badge

To generate a coverage badge for your README:

```bash
make coverage-badge
```

This command will:

1. Extract the coverage percentage from the HTML report
2. Generate a badge image in `.coverage/badge/coverage.svg`
3. Print instructions for adding the badge to your README

### Opening the Report

To automatically open the report in your default browser:

```bash
make coverage-view
```

Alternatively, you can use the dedicated browser-opening script:

```bash
./scripts/open-coverage.sh
```

This script will attempt to find and use an appropriate browser on your system.

You can also manually open the HTML file at `.coverage/html/index.html` in your browser to view a detailed coverage report.

### Cleaning Up Coverage Data

To clean up coverage artifacts:

```bash
make coverage-clean
```

This removes all profiling data and generated reports by deleting the `.coverage` directory.
