# Fossil Monorepo

This monorepo contains two separate projects:

1. **Proving Service**: A service that handles proof generation and verification
2. **Offchain Processor**: A service that processes offchain data and prepares it for proof generation

## Monorepo Structure

```text
fossil-monorepo/
├── proving-service/     # Contains the Proving Service implementation
└── offchain-processor/  # Contains the Offchain Processor implementation
```

## Monorepo Commands

The root Makefile provides convenience commands for working across both projects:

```bash
# Building
make build-all         # Build all projects in release mode
make build-all-debug   # Build all projects in debug mode
make ps-build          # Build Proving Service only
make op-build          # Build Offchain Processor only

# Testing
make test-all          # Test all projects
make ps-test           # Test Proving Service only
make op-test           # Test Offchain Processor only

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
