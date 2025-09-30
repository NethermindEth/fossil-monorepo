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
make dev-up             # Start all development services
make dev-down           # Stop all development services

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

## SQLx Offline Compilation

This project uses SQLx with offline compilation to avoid requiring a database connection during builds. The `.sqlx` directory contains pre-generated query metadata that allows compilation without a database.

### For Developers

- **Building**: No database required! Just run `cargo build` or `make build`
- **The `.sqlx` directory**: Contains query metadata - **commit this to version control**
- **Automatic offline mode**: Configured in `.cargo/config.toml` with `SQLX_OFFLINE=true`

### Updating Query Metadata

When you modify SQL queries, you need to regenerate the `.sqlx` metadata:

```bash
# 1. Start the database
make setup-postgres

# 2. Run migrations
OFFCHAIN_PROCESSOR_DATABASE_URL="postgres://postgres:postgres@localhost:5434/postgres" sqlx migrate run --source crates/db-access/migrations

# 3. Regenerate query metadata
OFFCHAIN_PROCESSOR_DATABASE_URL="postgres://postgres:postgres@localhost:5434/postgres" cargo sqlx prepare --workspace

# 4. Commit the updated .sqlx directory
git add .sqlx
git commit -m "Update SQLx query metadata"
```

### CI/CD Benefits

- ✅ No database required in CI pipelines
- ✅ Faster build times
- ✅ More reliable builds (no network dependencies)
- ✅ Works in restricted environments

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
    "program_id": "PITCHLAKE_V1",
    "vault_address": "0x07b0110e7230a20881e57804d68e640777f4b55b487321556682e550f93fec7c",
    "params": {
      "twap": [1672531200, 1672574400],
      "max_return": [1672531200, 1672574400],
      "reserve_price": [1672531200, 1672574400]
    }
  }'
```

## HTTP API

The service exposes multiple HTTP endpoints for pricing data requests and job management:

### Main Endpoints

#### Pricing Data Request
```bash
POST http://localhost:3000/pricing_data
```

#### Job Status Check
```bash
GET http://localhost:3000/job_status/{job_id}
```

#### Enhanced Job Result (with detailed information)
```bash
GET http://localhost:3000/job_result/{job_id}
```
*Requires API key authentication*

#### Batch Job Status Check
```bash
POST http://localhost:3000/batch_job_status
```
*Requires API key authentication*

#### Job Metrics and Analytics
```bash
GET http://localhost:3000/job_metrics?hours=24
```
*Requires API key authentication*

#### Webhook Callback
```bash
POST http://localhost:3000/webhook/{job_id}
```
*Public endpoint for external systems to send notifications*

### API Endpoint Details

#### 1. Pricing Data Request

**Endpoint:** `POST /pricing_data`
**Authentication:** Required (API Key)

Request Format:
```json
{
  "program_id": "PITCHLAKE_V1",
  "vault_address": "0x07b0110e7230a20881e57804d68e640777f4b55b487321556682e550f93fec7c",
  "params": {
    "twap": [1672531200, 1672574400],
    "max_return": [1672531200, 1672574400],
    "reserve_price": [1672531200, 1672574400]
  }
}
```

Response:
```json
{
  "job_id": "550e8400-e29b-41d4-a716-446655440000",
  "message": "New job request registered and processing initiated.",
  "status": "Pending"
}
```

#### 2. Enhanced Job Result

**Endpoint:** `GET /job_result/{job_id}`
**Authentication:** Required (API Key)

Response:
```json
{
  "job_id": "550e8400-e29b-41d4-a716-446655440000",
  "status": "Completed",
  "result": {
    "twap": 123.45,
    "max_return": 67.89,
    "reserve_price": 234.56
  },
  "created_at": "2024-01-15T10:30:00Z",
  "completed_at": "2024-01-15T10:32:00Z"
}
```

#### 3. Batch Job Status Check

**Endpoint:** `POST /batch_job_status`
**Authentication:** Required (API Key)

Request:
```json
{
  "job_ids": [
    "550e8400-e29b-41d4-a716-446655440000",
    "550e8400-e29b-41d4-a716-446655440001"
  ]
}
```

Response:
```json
{
  "jobs": [
    {
      "job_id": "550e8400-e29b-41d4-a716-446655440000",
      "status": "Completed",
      "result": {...},
      "created_at": "2024-01-15T10:30:00Z",
      "completed_at": "2024-01-15T10:32:00Z"
    }
  ],
  "not_found": [
    "550e8400-e29b-41d4-a716-446655440001"
  ]
}
```

#### 4. Job Metrics

**Endpoint:** `GET /job_metrics?hours=24`
**Authentication:** Required (API Key)

Response:
```json
{
  "total_jobs": 150,
  "pending_jobs": 5,
  "completed_jobs": 140,
  "failed_jobs": 5,
  "average_completion_time_seconds": 45.2
}
```

#### 5. Webhook Callback

**Endpoint:** `POST /webhook/{job_id}`
**Authentication:** Not required (Public)

This endpoint allows external systems (like PitchLake) to send notifications about job processing.

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
