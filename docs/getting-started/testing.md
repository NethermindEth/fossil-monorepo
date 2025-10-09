# Testing Guide

This guide covers how to run tests, understand the test structure, and write new tests for the Fossil monorepo.

## Table of Contents
- [Quick Start](#quick-start)
- [Test Structure](#test-structure)
- [Running Tests](#running-tests)
- [Writing Tests](#writing-tests)
- [Test Coverage](#test-coverage)
- [Integration Tests](#integration-tests)
- [Best Practices](#best-practices)
- [Troubleshooting](#troubleshooting)

## Quick Start

### Run All Tests

```bash
# Test both projects
make test-all

# Test individual projects
cd proving-service && make test
cd fossil-api && make test
```

### Prerequisites

Tests require Docker to be running (for PostgreSQL databases):

```bash
# Verify Docker is running
docker info
```

## Test Structure

### Project Organization

Both `proving-service` and `fossil-api` follow similar test organization:

```
crate-name/
├── src/
│   ├── lib.rs           # Library code with inline tests
│   ├── module.rs
│   │   └── mod.rs       # Module with #[cfg(test)] blocks
│   └── ...
├── tests/
│   └── integration_tests.rs  # Integration tests
└── examples/
    └── example.rs       # Runnable examples (sometimes with tests)
```

### Test Types

**Unit Tests** (`#[cfg(test)]` modules in `src/`)
- Test individual functions and modules
- Fast execution, no external dependencies
- Located alongside the code they test

**Integration Tests** (`tests/` directory)
- Test crate public API
- May require external services (databases, etc.)
- Run in separate compilation units

**Documentation Tests** (in `///` doc comments)
- Ensure code examples in documentation work
- Run with `cargo test --doc`

## Running Tests

### All Tests

```bash
# Run all tests in both projects
make test-all

# Run all tests in workspace
cargo test --workspace

# Run all tests with all features enabled
cargo test --workspace --all-features
```

### Individual Projects

#### Proving Service

```bash
cd proving-service

# Run all tests (starts Docker PostgreSQL automatically)
make test

# Run tests without Docker setup
cargo test --workspace
```

The `make test` command:
1. Starts PostgreSQL via Docker Compose
2. Runs all workspace tests
3. Shuts down and cleans up containers

#### Fossil API

```bash
cd fossil-api

# Run all tests
make test

# Run tests with all features
cargo test --workspace --all-features
```

### Specific Tests

```bash
# Run tests for a specific crate
cargo test -p db
cargo test -p message-handler
cargo test -p server

# Run a specific test function
cargo test test_function_name

# Run tests matching a pattern
cargo test integration

# Run tests in a specific file
cargo test --test integration_tests
```

### Test Output

```bash
# Show test output (println! statements)
cargo test -- --nocapture

# Show detailed test output
cargo test -- --nocapture --test-threads=1

# Run tests with backtrace on failure
RUST_BACKTRACE=1 cargo test
```

### Filtering Tests

```bash
# Run only unit tests (exclude integration tests)
cargo test --lib

# Run only integration tests
cargo test --test '*'

# Run only doc tests
cargo test --doc

# Skip tests matching pattern
cargo test -- --skip slow_test
```

## Writing Tests

### Unit Tests

Unit tests are typically written in a `#[cfg(test)]` module at the bottom of source files:

```rust
// src/my_module.rs

pub fn add(a: i32, b: i32) -> i32 {
    a + b
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_add() {
        assert_eq!(add(2, 2), 4);
    }

    #[test]
    fn test_add_negative() {
        assert_eq!(add(-1, 1), 0);
    }

    #[test]
    #[should_panic(expected = "overflow")]
    fn test_add_overflow() {
        add(i32::MAX, 1);
    }
}
```

### Integration Tests

Integration tests go in the `tests/` directory:

```rust
// tests/integration_tests.rs

use my_crate::MyStruct;

#[test]
fn test_public_api() {
    let instance = MyStruct::new();
    assert!(instance.is_valid());
}
```

### Async Tests

For async tests, use `tokio::test`:

```rust
#[tokio::test]
async fn test_async_function() {
    let result = my_async_function().await;
    assert_eq!(result, expected_value);
}
```

### Database Tests

Tests requiring database access:

```rust
use sqlx::PgPool;

#[tokio::test]
async fn test_database_operation() {
    // Set up test database connection
    let database_url = std::env::var("TEST_DATABASE_URL")
        .unwrap_or_else(|_| "postgresql://postgres:postgres@localhost:5432/test".to_string());

    let pool = PgPool::connect(&database_url).await.unwrap();

    // Run migrations if needed
    sqlx::migrate!("./migrations")
        .run(&pool)
        .await
        .unwrap();

    // Your test logic
    let result = my_db_function(&pool).await.unwrap();
    assert_eq!(result.id, 1);

    // Cleanup if needed
    sqlx::query("DELETE FROM my_table WHERE id = $1")
        .bind(1)
        .execute(&pool)
        .await
        .unwrap();
}
```

### Test Fixtures and Helpers

Create helper functions for common test setup:

```rust
#[cfg(test)]
mod tests {
    use super::*;

    // Test fixture
    fn create_test_job() -> Job {
        Job {
            id: "test_job_123".to_string(),
            status: JobStatus::Pending,
            created_at: Utc::now(),
        }
    }

    #[test]
    fn test_with_fixture() {
        let job = create_test_job();
        assert_eq!(job.status, JobStatus::Pending);
    }
}
```

### Mocking External Services

For tests that depend on external services, use mocks:

```rust
#[cfg(test)]
mod tests {
    use mockito::{mock, server_url};

    #[tokio::test]
    async fn test_api_client() {
        // Create mock endpoint
        let _m = mock("POST", "/api/endpoint")
            .with_status(200)
            .with_header("content-type", "application/json")
            .with_body(r#"{"status": "success"}"#)
            .create();

        // Test your client against the mock
        let client = ApiClient::new(&server_url());
        let response = client.post_data().await.unwrap();

        assert_eq!(response.status, "success");
    }
}
```

## Test Coverage

### Generating Coverage Reports

Both projects support code coverage using `grcov`:

```bash
# Generate coverage for proving service
cd proving-service
make coverage

# Generate coverage for fossil API
cd fossil-api
make coverage

# View coverage report
open .coverage/html/index.html  # macOS
xdg-open .coverage/html/index.html  # Linux
```

### Coverage Process

The `make coverage` command:
1. Installs `llvm-tools-preview` if needed
2. Starts required Docker services
3. Runs tests with coverage instrumentation
4. Generates coverage data using `grcov`
5. Creates HTML report in `.coverage/html/`

### Coverage Configuration

Coverage is configured to:
- Exclude test files from coverage
- Exclude vendored dependencies
- Generate both HTML and terminal output
- Track line, branch, and function coverage

### Interpreting Coverage

**Good coverage targets:**
- **Critical paths**: 90%+ coverage
- **Business logic**: 80%+ coverage
- **Utility functions**: 70%+ coverage
- **Overall project**: 70%+ coverage

**Low coverage is acceptable for:**
- Generated code
- Simple getters/setters
- Error formatting functions
- Debug utilities

## Integration Tests

### StarkNet Integration Tests

The proving service includes integration tests for StarkNet functionality:

```bash
# Run StarkNet integration tests
cd proving-service
cargo test --package starknet-handler --test integration_tests

# Run with detailed output
RUST_LOG=debug cargo test --package starknet-handler --test integration_tests -- --nocapture
```

**Example test:**
```rust
#[tokio::test]
async fn test_sepolia_get_avg_fees_in_range() {
    let provider = /* setup StarkNet provider */;

    let result = get_avg_fees_in_range(
        &provider,
        1672531200,
        1672617600,
    ).await;

    assert!(result.is_ok());
}
```

### End-to-End Tests

The repository includes an end-to-end test script:

```bash
# Run full E2E test
./test-local-request.sh
```

This script tests the complete flow:
1. Service health checks
2. API key generation
3. Contract interaction
4. Job submission
5. Status monitoring
6. Result retrieval

## Best Practices

### Test Naming

```rust
// Good: Descriptive, indicates what is being tested
#[test]
fn test_parse_valid_timestamp_returns_ok()

#[test]
fn test_invalid_api_key_returns_unauthorized()

// Bad: Too vague
#[test]
fn test_1()

#[test]
fn it_works()
```

### Test Organization

```rust
#[cfg(test)]
mod tests {
    use super::*;

    // Group related tests together
    mod parsing {
        use super::*;

        #[test]
        fn test_parse_valid_input() { /* ... */ }

        #[test]
        fn test_parse_invalid_input() { /* ... */ }
    }

    mod validation {
        use super::*;

        #[test]
        fn test_validate_positive_timestamp() { /* ... */ }

        #[test]
        fn test_validate_negative_timestamp() { /* ... */ }
    }
}
```

### Assertions

```rust
// Use specific assertions
assert_eq!(actual, expected);
assert_ne!(actual, unexpected);
assert!(condition);

// Provide helpful error messages
assert_eq!(
    result.status,
    JobStatus::Completed,
    "Job status should be Completed after processing, got {:?}",
    result.status
);

// Test error cases
let result = dangerous_function();
assert!(result.is_err());
assert!(matches!(result.unwrap_err(), ErrorType::SpecificError));
```

### Test Independence

```rust
// Good: Each test is independent
#[test]
fn test_feature_a() {
    let state = create_fresh_state();
    // Test A
}

#[test]
fn test_feature_b() {
    let state = create_fresh_state();
    // Test B
}

// Bad: Tests depend on execution order
static mut SHARED_STATE: Option<State> = None;

#[test]
fn test_setup() {
    unsafe { SHARED_STATE = Some(State::new()); }
}

#[test]
fn test_using_state() {
    // Fails if test_setup hasn't run
    let state = unsafe { SHARED_STATE.as_ref().unwrap() };
}
```

### Test Data

```rust
// Use constants for test data
const TEST_TIMESTAMP: i64 = 1672531200;
const TEST_VAULT_ADDRESS: &str = "0x004018ae...";

#[test]
fn test_with_constants() {
    let request = Request {
        timestamp: TEST_TIMESTAMP,
        vault: TEST_VAULT_ADDRESS.to_string(),
    };
    assert!(request.is_valid());
}
```

## Troubleshooting

### Database Connection Errors

**Problem:** Tests fail with "connection refused"

```bash
# Ensure Docker is running
docker info

# Manually start test database
cd proving-service
docker compose -f docker/docker-compose.test.yml up -d

# Run tests
cargo test

# Clean up when done
docker compose -f docker/docker-compose.test.yml down -v
```

### Slow Tests

**Problem:** Tests take too long to run

```bash
# Run tests in parallel (default)
cargo test

# Run tests sequentially (useful for debugging)
cargo test -- --test-threads=1

# Run only fast tests
cargo test --lib  # Skip integration tests
```

### Flaky Tests

**Problem:** Tests pass sometimes, fail other times

Common causes:
- **Race conditions**: Use proper synchronization
- **Timing issues**: Add appropriate timeouts/waits
- **External dependencies**: Mock external services
- **Shared state**: Ensure test independence

**Debug with:**
```bash
# Run test multiple times
for i in {1..10}; do cargo test test_name || break; done

# Run with detailed logging
RUST_LOG=debug cargo test test_name -- --nocapture
```

### Out of Memory

**Problem:** Tests crash with OOM errors

```bash
# Reduce parallel test execution
cargo test -- --test-threads=2

# Clean build cache
cargo clean
```

### Permission Errors

**Problem:** Cannot access test database

```bash
# Fix Docker permissions
sudo usermod -aG docker $USER
newgrp docker

# Restart Docker daemon
sudo systemctl restart docker
```

## Next Steps

- [Contributing Guidelines](../contributing/testing-guidelines.md) - Detailed testing standards
- [Code Style Guide](../contributing/code-style.md) - Code formatting and linting
- [CI/CD Documentation](../contributing/pr-process.md) - Automated testing in CI

## Quick Reference

```bash
# Run all tests
make test-all

# Run project tests
cd proving-service && make test
cd fossil-api && make test

# Run specific tests
cargo test test_name
cargo test -p crate_name

# Generate coverage
make coverage

# Run E2E test
./test-local-request.sh

# Clean test environment
make test-clean
```
