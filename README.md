# Fossil Monorepo

<div align="center">

[![CI Status](https://github.com/NethermindEth/fossil-prover-service/actions/workflows/monorepo-ci.yml/badge.svg)](https://github.com/NethermindEth/fossil-prover-service/actions/workflows/monorepo-ci.yml)
![PS Coverage][ps-coverage-badge]
![OP Coverage][op-coverage-badge]

</div>

This monorepo contains two separate projects:

1. **Proving Service**: A service that handles proof generation and verification
2. **Offchain Processor**: A service that processes offchain data and prepares it for proof generation

## Getting Started

### One-Command Setup

To set up the complete development environment:

```bash
make setup
```

This will:

- Install Rust and required toolchains
- Set up PostgreSQL databases for both projects
- Configure LocalStack for AWS services
- Install code coverage tools
- Set up project-specific dependencies

You can also set up individual components:

```bash
make setup-ps    # Set up Proving Service only
make setup-op    # Set up Offchain Processor only
make setup-rust  # Set up Rust only
```

## Development Workflow

> **IMPORTANT:** Always run `make pr` before submitting your changes!
>
> This ensures your code:
>
> - Passes all linters (formatting and static analysis)
> - Passes all tests in both projects
> - Is ready for review without CI pipeline failures

## Monorepo Structure

```text
fossil-monorepo/
├── proving-service/     # Contains the Proving Service implementation
└── offchain-processor/  # Contains the Offchain Processor implementation
```

## Monorepo Commands

The root Makefile provides convenience commands for working across both projects:

```bash
# Setup
make setup            # Set up complete development environment
make setup-ps         # Set up Proving Service only
make setup-op         # Set up Offchain Processor only

# Building
make build-all         # Build all projects in release mode
make build-all-debug   # Build all projects in debug mode
make ps-build          # Build Proving Service only
make op-build          # Build Offchain Processor only

# Testing
make test-all          # Test all projects
make ps-test           # Test Proving Service only
make op-test           # Test Offchain Processor only

# Linting and PR Preparation
make lint-all          # Run linters on all projects
make pr                # Run linters and tests to prepare for a pull request

# Development Services
make dev-services      # Start all development services
make dev-services-stop # Stop all development services

# Cleaning
make clean-all         # Clean all projects
make ps-clean          # Clean Proving Service only
make op-clean          # Clean Offchain Processor only

# Help
make help              # Show all available commands
```

## Project Documentation

For detailed information about each project, see their respective READMEs:

- [Proving Service](proving-service/README.md)
- [Offchain Processor](offchain-processor/README.md)

[ps-coverage-badge]: https://img.shields.io/badge/coverage-78.5%25-green
[op-coverage-badge]: https://img.shields.io/badge/coverage-82.3%25-green
