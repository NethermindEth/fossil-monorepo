# Offchain Processor

A service that processes pricing data requests and generates pricing data responses.

## Getting Started with Make

This project includes a comprehensive Makefile to simplify common development tasks. Here are the main commands:

```bash
# Setup your development environment
make setup              # Install all dependencies
make setup-rust         # Install Rust and toolchains
make setup-postgres     # Set up PostgreSQL for development
make setup-coverage     # Install code coverage tools
make setup-platform     # Set up platform dependencies
make init-repo          # Initialize repository (git hooks, etc.)

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
make lint-codespell     # Check for spelling mistakes
make pr                 # Prepare code for a pull request

# Help
make help               # Display all available commands
```

For more details on each command, run `make help`.

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

## Usage

### Running the Application

Run the offchain processor service with:

```bash
cargo run --bin server
```

### Example Request

```bash
curl -X POST http://localhost:3000/pricing_data \
  -H "Content-Type: application/json" \
  -H "X-API-Key: 724a0c8d-9fea-4c7c-97a0-1aea894a283e" \
  -d '{
    "identifiers": ["0x50495443485f4c414b455f5631"],
    "params": {
      "twap": [1672531200, 1672574400],
      "volatility": [1672531200, 1672574400],
      "reserve_price": [1672531200, 1672574400]
    },
    "client_info": {
      "client_address": "0x018df581fe0ee497a4a3595cf62aea0bafa7ba1a54a7dcbafca37bfada67c718",
      "vault_address": "0x07b0110e7230a20881e57804d68e640777f4b55b487321556682e550f93fec7c",
      "timestamp": 1741243059
    }
  }'
```

## HTTP API

The service exposes an HTTP endpoint for pricing data requests:

### Endpoint

```bash
POST http://localhost:3000/pricing_data
```

### Request Format

Send a POST request with a JSON body in the following format:

```json
{
  "identifiers": ["0x50495443485f4c414b455f5631"],
  "params": {
    "twap": [1672531200, 1672574400],
    "volatility": [1672531200, 1672574400],
    "reserve_price": [1672531200, 1672574400]
  },
  "client_info": {
    "client_address": "0x018df581fe0ee497a4a3595cf62aea0bafa7ba1a54a7dcbafca37bfada67c718",
    "vault_address": "0x07b0110e7230a20881e57804d68e640777f4b55b487321556682e550f93fec7c",
    "timestamp": 1741243059
  }
}
```

### Headers

- `Content-Type: application/json` - Required
- `X-API-Key: <your-api-key>` - Required for authentication

## Generating an API Key

You need an API key to authenticate requests to the service. There are two ways to generate an API key:

### Using the HTTP API

```bash
curl -X POST http://localhost:3000/api_key \
  -H "Content-Type: application/json" \
  -d '{"name": "my_api_key"}'
```

The response will contain your new API key:

```json
{
  "api_key": "uuid-formatted-api-key"
}
```

### Using the Command Line Tool

Alternatively, you can use the provided command line tool:

```bash
cargo run --bin create_api_key "my_api_key"
```

This will output the generated API key to the console.

## Development Setup

### PostgreSQL Setup

The service uses PostgreSQL for data storage. For local development:

1. Start the PostgreSQL container:

   ```bash
   make setup-postgres
   ```

### LocalStack Setup

For local AWS service emulation:

1. Start the LocalStack container:

   ```bash
   make setup-localstack
   ```

## Code Coverage

The project includes comprehensive code coverage tracking using cargo-tarpaulin.

### Viewing Coverage Reports Locally

Run the tests with coverage enabled and generate an HTML report:

```bash
make coverage
```

### Opening the Report

To automatically open the report in your default browser:

```bash
make coverage-view
```

### Generating a Coverage Badge

To generate a coverage badge for your README:

```bash
make coverage-badge
```

### Cleaning Up Coverage Data

To clean up coverage artifacts:

```bash
make coverage-clean
```
