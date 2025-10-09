# Fossil Monorepo Documentation Index

Complete index of all documentation in the Fossil monorepo.

## Quick Links

- [📚 Getting Started](#getting-started)
- [🏗️ Architecture](#architecture)
- [📖 Guides](#guides)
- [🔌 API Reference](#api-reference)
- [📦 Crates](#crates)
- [📜 Contracts](#contracts)

## Documentation Statistics

- **Total Files:** 26 markdown documents
- **Total Lines:** ~27,000 lines
- **Total Size:** ~844KB
- **Categories:** 6 main categories

## Getting Started

Essential guides for setting up and running the Fossil system.

| Document | Description | Topics |
|----------|-------------|--------|
| [Installation](getting-started/installation.md) | System setup and dependencies | Rust, RISC0, StarkNet tools, Docker |
| [Local Development](getting-started/local-development.md) | Running the dev environment | `make dev-up`, service URLs, testing |
| [Testing](getting-started/testing.md) | Running and writing tests | Unit tests, integration tests, coverage |

## Architecture

High-level system design and component architecture.

| Document | Description | Key Topics |
|----------|-------------|------------|
| [Overview](architecture/overview.md) | System architecture | Components, technology stack, design principles |
| [Data Flow](architecture/data-flow.md) | End-to-end request flow | Sequence diagrams, error flows, state machines |
| [Proving Service](architecture/proving-service.md) | Proof generation service | Crates, SQS, RISC0, job processing |
| [Fossil API](architecture/fossil-api.md) | HTTP API service | Server, handlers, authentication, events |
| [StarkNet Contracts](architecture/starknet-contracts.md) | Smart contracts | Hash store, verifier, deployment |

## Guides

Operational guides for development and deployment.

| Document | Description | Key Topics |
|----------|-------------|------------|
| [Environment Setup](guides/environment-setup.md) | Environment configuration | `.env` files, variables, secrets management |
| [Database Management](guides/database-management.md) | Database operations | Migrations, schema, queries, backups |
| [Running Services](guides/running-services.md) | Service execution | Docker vs native, ports, hot reload |
| [Debugging](guides/debugging.md) | Troubleshooting | Logging, common issues, performance |
| [Deployment](guides/deployment.md) | Production deployment | AWS ECS, CI/CD, monitoring, rollback |

## API Reference

Complete API documentation for all endpoints.

| Document | Description | Endpoints |
|----------|-------------|-----------|
| [Fossil API](api-reference/fossil-api-endpoints.md) | Public API endpoints | 7 endpoints with examples |
| [Proving Service](api-reference/proving-service-endpoints.md) | Internal API | Job submission, health check |
| [Authentication](api-reference/authentication.md) | API key auth | Key generation, rotation, security |

## Crates

Detailed documentation for each Rust crate.

### Proving Service Crates

| Crate | Description | Key Components |
|-------|-------------|----------------|
| [db](crates/proving-service/db.md) | Database layer | Connection pooling, BlockHeader model |
| [message-handler](crates/proving-service/message-handler.md) | SQS & proof generation | Queue system, proof composition, RISC0 |
| [proving-service](crates/proving-service/proving-service.md) | HTTP API | Axum server, job dispatcher |
| [starknet-handler](crates/proving-service/starknet-handler.md) | StarkNet integration | Proof verification, hash operations |

### Fossil API Crates

| Crate | Description | Key Components |
|-------|-------------|----------------|
| [db-access](crates/fossil-api/db-access.md) | Database & auth | Models, migrations, API keys |
| [server](crates/fossil-api/server.md) | HTTP server | Handlers, middleware, event monitor |

## Contracts

StarkNet smart contract documentation.

| Contract | Description | Key Features |
|----------|-------------|--------------|
| [Fossil Hash Store](contracts/fossil-hash-store.md) | Hash storage | Hierarchical hashing, 2-tier system |
| [PitchLake Verifier](contracts/pitchlake-verifier.md) | Proof verification | Groth16, RISC Zero, journal decoding |
| [Mocks](contracts/mocks.md) | Test contracts | MockFossilStore, TestUpgrade |

## Cross-Reference Map

### By Topic

**Authentication & Security:**
- [Authentication Guide](api-reference/authentication.md)
- [API Key Management](api-reference/fossil-api-endpoints.md#authentication)
- [Security Best Practices](guides/environment-setup.md#secrets-management)
- [Server Authentication](crates/fossil-api/server.md#middleware)

**Database:**
- [Database Management](guides/database-management.md)
- [Proving Service DB](crates/proving-service/db.md)
- [Fossil API DB](crates/fossil-api/db-access.md)
- [Migrations](guides/database-management.md#migrations)

**Proof Generation:**
- [Proving Service Architecture](architecture/proving-service.md)
- [Message Handler](crates/proving-service/message-handler.md)
- [RISC0 Integration](crates/proving-service/message-handler.md#risc0-integration)
- [Proof Composition](crates/proving-service/message-handler.md#proof-composition)

**StarkNet:**
- [StarkNet Contracts](architecture/starknet-contracts.md)
- [StarkNet Handler](crates/proving-service/starknet-handler.md)
- [Hash Store Contract](contracts/fossil-hash-store.md)
- [Verifier Contract](contracts/pitchlake-verifier.md)

**Testing:**
- [Testing Guide](getting-started/testing.md)
- [Mock Contracts](contracts/mocks.md)
- [Debugging](guides/debugging.md)
- [Local Development](getting-started/local-development.md#testing)

**Deployment:**
- [Deployment Guide](guides/deployment.md)
- [Environment Setup](guides/environment-setup.md)
- [Running Services](guides/running-services.md)
- [Production Configuration](guides/deployment.md#production-deployment)

### By User Journey

**New Developer Setup:**
1. [Installation](getting-started/installation.md)
2. [Local Development](getting-started/local-development.md)
3. [Testing](getting-started/testing.md)
4. [Architecture Overview](architecture/overview.md)

**API Integration:**
1. [Fossil API Endpoints](api-reference/fossil-api-endpoints.md)
2. [Authentication](api-reference/authentication.md)
3. [Data Flow](architecture/data-flow.md)
4. [Debugging](guides/debugging.md)

**Service Development:**
1. [Architecture Overview](architecture/overview.md)
2. [Proving Service](architecture/proving-service.md) or [Fossil API](architecture/fossil-api.md)
3. [Database Management](guides/database-management.md)
4. [Running Services](guides/running-services.md)
5. Specific crate docs

**Contract Development:**
1. [StarkNet Contracts](architecture/starknet-contracts.md)
2. [Fossil Hash Store](contracts/fossil-hash-store.md)
3. [PitchLake Verifier](contracts/pitchlake-verifier.md)
4. [Mocks](contracts/mocks.md)

**Production Deployment:**
1. [Deployment Guide](guides/deployment.md)
2. [Environment Setup](guides/environment-setup.md)
3. [Database Management](guides/database-management.md#backups)
4. [Debugging](guides/debugging.md)

## Document Maintenance

### Update Frequency

| Category | Update Trigger |
|----------|---------------|
| Getting Started | New dependencies, setup changes |
| Architecture | Major system changes, new components |
| Guides | Process changes, new environments |
| API Reference | Endpoint changes, new features |
| Crates | Code refactoring, new functions |
| Contracts | Contract upgrades, new deployments |

### Review Checklist

When updating documentation:
- [ ] Update related cross-references
- [ ] Verify code examples compile/run
- [ ] Update version numbers if applicable
- [ ] Check links are not broken
- [ ] Ensure consistent formatting
- [ ] Update DOCUMENTATION_INDEX.md if structure changes

## Contributing to Documentation

See [CLAUDE.md](../CLAUDE.md) for development workflow and documentation standards.

### Documentation Standards

- **Format:** GitHub-flavored Markdown
- **Code blocks:** Always specify language
- **Links:** Use relative paths
- **Examples:** Include working code snippets
- **Structure:** Follow existing document templates

### Adding New Documentation

1. Create file in appropriate directory
2. Follow naming convention: `kebab-case.md`
3. Add to this index
4. Add cross-references from related docs
5. Link from README.md if top-level

## Search Tips

### Finding Information

**By Error Message:**
- Check [Debugging Guide](guides/debugging.md#common-issues)
- See service-specific troubleshooting in crate docs

**By Task:**
- Setup: [Getting Started](#getting-started)
- API Usage: [API Reference](#api-reference)
- Development: [Guides](#guides)
- Understanding: [Architecture](#architecture)

**By Component:**
- Services: [Architecture](#architecture) + [Crates](#crates)
- Contracts: [Contracts](#contracts)
- Database: Search "database" or "migration"
- Auth: [Authentication](api-reference/authentication.md)

## Quick Command Reference

```bash
# View all documentation files
tree docs/

# Search documentation
grep -r "search term" docs/

# Count documentation
find docs/ -name "*.md" | wc -l

# Get documentation size
du -sh docs/
```

## External Resources

- [RISC Zero Documentation](https://dev.risczero.com/)
- [StarkNet Documentation](https://docs.starknet.io/)
- [Rust Documentation](https://doc.rust-lang.org/)
- [Axum Documentation](https://docs.rs/axum/)
- [SQLx Documentation](https://docs.rs/sqlx/)

---

**Last Updated:** 2025-01-06
**Documentation Version:** 1.0
**Total Documentation:** 26 files, ~27,000 lines, 844KB
