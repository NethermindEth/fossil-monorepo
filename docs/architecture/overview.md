# Architecture Overview

This document provides a high-level overview of the Fossil monorepo architecture, component relationships, and key design decisions.

## Table of Contents
- [System Overview](#system-overview)
- [Component Architecture](#component-architecture)
- [Technology Stack](#technology-stack)
- [Data Flow](#data-flow)
- [Design Principles](#design-principles)
- [Deployment Architecture](#deployment-architecture)

## System Overview

This repository implements the **Pitchlake Coprocessor**, a RISC Zero proof generation and StarkNet verification system that provides cryptographic proofs for financial data calculations used in the Pitchlake options market. The Pitchlake Coprocessor consists of three main services:

1. **Fossil API** - HTTP API for job management and pricing data requests
2. **Proving Service** - RISC Zero proof generation via Bonsai API
3. **StarkNet Contracts** - Onchain proof verification and data storage

### Relationship to Broader Fossil Infrastructure

The Pitchlake Coprocessor operates as part of the Fossil ecosystem, which is a trustless data infrastructure that records Ethereum Layer 1 (L1) base gas fee data on Starknet.

**Upstream Components (Not in this repository):**
- **Fossil Postures Database** - Stores finalized Ethereum block headers indexed by an external indexer
- **MMR Builder** - Processes blocks in batches of 1024, validates headers, computes hourly average base fees, and constructs Merkle Mountain Ranges (MMR) with proofs stored on Starknet
- **Light Client** - Continuously updates Fossil's state with new finalized Ethereum blocks via L1 → L2 messaging
- **L1MessageProxy** - Receives L1 → L2 messages containing finalized block data from Ethereum
- **Fossil Store Contract** - Maintains proof metadata, MMR root hashes, IPFS references, and hourly average base fee data on Starknet

**This Repository (Pitchlake Coprocessor):**
- Queries validated hourly average base fee data from Fossil Store contract on Starknet
- Executes verifiable pricing computations in RISC0 zkVM (TWAP, max return, reserve price)
- Generates zero-knowledge proofs for Pitchlake market pricing calculations
- Submits proofs to Starknet for verification

**Data Flow:**
```
Ethereum L1 → Indexer → MMR Builder → Fossil Store (Starknet) → Pitchlake Coprocessor → Proof Verification (Starknet)
```

For more details on the broader Fossil architecture (MMR Builder, Light Client), see the Fossil Technical Specification.

### High-Level Architecture

```
┌─────────────────────────────────────────────────────────────────┐
│                          Client/User                            │
└────────────────────────┬────────────────────────────────────────┘
                         │
                         │ HTTP/JSON API
                         ▼
┌─────────────────────────────────────────────────────────────────┐
│                        Fossil API                               │
│  ┌──────────────┐  ┌──────────────┐  ┌──────────────────────┐ │
│  │   Server     │  │  DB Access   │  │  Authentication      │ │
│  │   (Axum)     │  │  (SQLx)      │  │  (API Keys)          │ │
│  └──────┬───────┘  └──────┬───────┘  └──────────────────────┘ │
│         │                 │                                     │
└─────────┼─────────────────┼─────────────────────────────────────┘
          │                 │
          │ HTTP            │ PostgreSQL
          ▼                 ▼
┌─────────────────────────────────────────────────────────────────┐
│                     Proving Service                             │
│  ┌───────────────────────────────────────────────────────────┐ │
│  │                   Proving Service API                      │ │
│  │              (Job Management & Dispatch)                   │ │
│  └──────────────────────────┬────────────────────────────────┘ │
│                             │                                   │
│                             │ AWS SQS                           │
│                             ▼                                   │
│  ┌───────────────────────────────────────────────────────────┐ │
│  │                   Message Handler                          │ │
│  │  ┌──────────────┐  ┌──────────────┐  ┢─────────────────┐ │ │
│  │  │ SQS Polling  │  │ Proof Gen    │  │ StarkNet        │ │ │
│  │  │              │  │ (RISC0/      │  │ Submission      │ │ │
│  │  │              │  │  Bonsai)     │  │                 │ │ │
│  │  └──────────────┘  └──────────────┘  └─────────────────┘ │ │
│  └───────────────────────────────────────────────────────────┘ │
│                             │                                   │
│                             │ PostgreSQL                        │
│  ┌──────────────────────────▼──────────────────────────────┐  │
│  │                    Database Layer                        │  │
│  │              (Job Status, Metadata, Results)             │  │
│  └──────────────────────────────────────────────────────────┘  │
└─────────────────────────────┬───────────────────────────────────┘
                              │
                              │ RPC/JSON
                              ▼
┌─────────────────────────────────────────────────────────────────┐
│                    StarkNet Network                             │
│  ┌──────────────────┐  ┢──────────────────┐  ┢──────────────┐  │
│  │ Fossil Hash      │  │ PitchLake        │  │ Mock         │  │
│  │ Store Contract   │  │ Verifier         │  │ Contracts    │  │
│  └──────────────────┘  └──────────────────┘  └──────────────┘  │
└─────────────────────────────────────────────────────────────────┘
```

## Component Architecture

### Fossil API

**Purpose:** User-facing HTTP API for submitting pricing data requests and retrieving results.

**Key Responsibilities:**
- API key authentication and management
- Request validation and job creation
- Job status tracking and result retrieval
- Forwarding requests to Proving Service

**Crates:**
- `server` - HTTP server implementation (Axum)
- `db-access` - Database models, migrations, and queries (SQLx)

**Technology:**
- **Language:** Rust (Edition 2021)
- **Framework:** Axum (async HTTP)
- **Database:** PostgreSQL (via SQLx)
- **Auth:** API key-based authentication

See: [Fossil API Architecture](fossil-api.md)

### Proving Service

**Purpose:** Backend service for RISC Zero proof generation and StarkNet verification.

**Key Responsibilities:**
- Job dispatch and queue management
- RISC Zero proof generation via Bonsai API
- StarkNet transaction submission
- Proof verification tracking

**Crates:**
- `proving-service` - HTTP API for job submission
- `message-handler` - SQS message processing and proof composition
- `db` - Database models and queries
- `starknet-handler` - StarkNet integration and transaction handling

**Technology:**
- **Language:** Rust (Edition 2024)
- **Framework:** Axum (API), Tokio (async runtime)
- **Database:** PostgreSQL (via SQLx)
- **Queue:** AWS SQS (LocalStack for local dev)
- **Proofs:** RISC Zero zkVM, Bonsai API

See: [Proving Service Architecture](proving-service.md)

### StarkNet Contracts

**Purpose:** Onchain proof verification and data storage on StarkNet.

**Key Responsibilities:**
- Proof verification
- Hash storage and retrieval
- Access control and ownership
- Integration with PitchLake vaults

**Contracts:**
- `fossil-hash-store` - Hash storage and verification
- `pitchlake-verifier` - RISC Zero proof verification
- `mocks` - Test contracts for local development

**Technology:**
- **Language:** Cairo 2.12.1
- **Framework:** Starknet Foundry, Scarb
- **Network:** StarkNet (Sepolia testnet, Mainnet)

See: [StarkNet Contracts Architecture](starknet-contracts.md)

## Technology Stack

### Backend Services

| Component | Technology | Version | Purpose |
|-----------|-----------|---------|---------|
| Language | Rust | 1.70+ | Core implementation language |
| Runtime | Tokio | 1.39+ | Async runtime |
| HTTP Framework | Axum | 0.8+ | Web server framework |
| Database | PostgreSQL | 14+ | Persistent data storage |
| ORM | SQLx | 0.8+ | Database queries and migrations |
| Message Queue | AWS SQS | - | Async job processing |
| Proof Generation | RISC Zero | 2.x | Zero-knowledge proofs |
| StarkNet RPC | starknet-rs | 0.16+ | StarkNet integration |

### Smart Contracts

| Component | Technology | Version | Purpose |
|-----------|-----------|---------|---------|
| Language | Cairo | 2.12.1 | StarkNet smart contracts |
| Build Tool | Scarb | 2.12.1 | Cairo package manager |
| Testing | Starknet Foundry | 0.49.0 | Contract testing framework |
| CLI | Starkli | 0.4.2 | StarkNet CLI tool |

### Development Tools

| Tool | Version | Purpose |
|------|---------|---------|
| Docker | 24.0+ | Containerization and local services |
| Docker Compose | 2.20+ | Multi-container orchestration |
| asdf | 0.14+ | Version manager for StarkNet tools |
| grcov | - | Code coverage reporting |
| LocalStack | - | Local AWS services emulation |

## Data Flow

### Request Processing Flow

```
1. Client Request
   ↓
2. Fossil API (Authentication & Validation)
   ↓
3. Job Creation & Storage (PostgreSQL)
   ↓
4. Forward to Proving Service API
   ↓
5. Queue Job in SQS
   ↓
6. Message Handler Polls Queue
   ↓
7. Fetch Data from Database
   ↓
8. Generate RISC Zero Proof (Bonsai)
   ↓
9. Submit Proof to StarkNet
   ↓
10. Update Job Status
   ↓
11. Client Retrieves Result
```

### Data Storage

**Fossil API Database:**
- API keys and authentication
- Job metadata and status
- Client request data
- Pricing data results

**Proving Service Database:**
- Job queue status
- Proof metadata
- StarkNet transaction hashes
- Verification results

**Fossil Store Contract (StarkNet):**
- Hourly average base fee data from Ethereum L1
- MMR root hashes for data integrity
- IPFS references to complete MMR snapshots
- Proof journals from zkVM executions

**StarkNet Verification Contracts:**
- Verified proofs for Pitchlake calculations
- Computation results (TWAP, max return, reserve price)
- Access control data

See: [Data Flow Documentation](data-flow.md)

## Design Principles

### 1. Separation of Concerns

Each service has a clearly defined responsibility:
- **Fossil API** handles client interaction and business logic
- **Proving Service** handles proof generation and verification
- **Contracts** handle onchain verification and storage

### 2. Asynchronous Processing

Long-running tasks (proof generation) are handled asynchronously:
- Jobs are queued in SQS
- Message handler processes jobs in background
- Client polls for status and results
- Prevents timeout issues and improves scalability

### 3. Database per Service

Each service has its own database:
- **Fossil API DB** - Client-facing data
- **Proving Service DB** - Internal proof processing
- **Fossil Store Contract** (read-only) - Historical Ethereum L1 base fee data maintained by upstream Fossil infrastructure

This enables independent scaling and reduces coupling.

### 4. Type Safety

Strong typing throughout the stack:
- Rust's type system prevents many bugs at compile time
- Cairo's type system ensures contract safety
- Database schemas are checked at compile time (SQLx)

### 5. Error Handling

Comprehensive error handling:
- Result types for fallible operations
- Structured error types with context
- Error logging and tracing
- Graceful degradation

### 6. Testability

All components are designed for testing:
- Unit tests in source modules
- Integration tests with real databases
- Mock implementations for external services
- End-to-end testing scripts

### 7. Configuration via Environment

All configuration is externalized:
- Environment variables for secrets
- Multiple environment files (local, docker, production)
- No hardcoded configuration

### 8. Observability

Built-in logging and monitoring:
- Structured logging with `tracing`
- Health check endpoints
- Detailed error messages
- Job status tracking

## Deployment Architecture

### Local Development

```
┌─────────────────────────────────────────────────────────────┐
│                    Docker Compose                            │
│                                                              │
│  ┌──────────┐  ┌──────────┐  ┌───────────┐  ┌───────────┐ │
│  │  Katana  │  │PostgreSQL│  │LocalStack │  │  Fossil   │ │
│  │(StarkNet)│  │   (x2)   │  │   (SQS)   │  │  Services │ │
│  └──────────┘  └──────────┘  └───────────┘  └───────────┘ │
└─────────────────────────────────────────────────────────────┘
```

**Command:** `make dev-up`

**Services:**
- Katana (port 5050) - Local StarkNet devnet
- PostgreSQL (ports 5434, 5435) - Databases
- LocalStack (port 4567) - Mock AWS SQS
- Fossil API (port 3000)
- Proving Service API (port 3001)
- Message Handler

### Production (AWS ECS)

```
┌──────────────────────────────────────────────────────────────┐
│                         AWS Cloud                             │
│                                                               │
│  ┌─────────────┐  ┌─────────────┐  ┌──────────────────────┐ │
│  │   ECS       │  │    RDS      │  │       SQS            │ │
│  │ (Services)  │  │ (PostgreSQL)│  │   (Message Queue)    │ │
│  └─────────────┘  └─────────────┘  └──────────────────────┘ │
│                                                               │
│  ┌─────────────┐  ┌─────────────┐                           │
│  │     ALB     │  │  Secrets    │                           │
│  │(Load Balancer)│ │  Manager   │                           │
│  └─────────────┘  └─────────────┘                           │
└──────────────────────────────────────────────────────────────┘
         │
         │ HTTPS
         ▼
┌──────────────────────────────────┐
│      StarkNet Network            │
│    (Sepolia / Mainnet)           │
└──────────────────────────────────┘
```

**Infrastructure:**
- ECS Fargate for containerized services
- RDS PostgreSQL for managed databases
- SQS for job queue
- Application Load Balancer for HTTP traffic
- Secrets Manager for sensitive configuration

See: [Deployment Guide](../guides/deployment.md)

### Network Architecture

**Local Development:**
- All services run on localhost
- Docker network for inter-service communication
- Port mapping for external access

**Production:**
- Private VPC for services
- Public subnets for load balancers
- Private subnets for ECS tasks and RDS
- Security groups for access control
- StarkNet RPC via public endpoints

## Security Considerations

### API Authentication

- API key-based authentication
- Keys stored hashed in database
- Rate limiting per API key
- Key rotation supported

### Database Security

- Connection pooling with limits
- Prepared statements (SQL injection prevention)
- Database encryption at rest
- TLS for connections in production

### Proof Integrity

- RISC Zero cryptographic proofs
- StarkNet onchain verification
- Immutable proof storage
- Verifiable computation

### Network Security

- HTTPS only in production
- VPC isolation
- Security groups and firewall rules
- Secrets management

## Scalability

### Horizontal Scaling

- **Fossil API:** Stateless, can run multiple instances behind load balancer
- **Proving Service API:** Stateless, can run multiple instances
- **Message Handler:** Can run multiple workers processing SQS queue
- **Database:** Can use read replicas for read-heavy operations

### Vertical Scaling

- Adjust container CPU/memory based on load
- Database instance size can be increased
- RISC Zero proof generation benefits from more CPU/memory

### Queue-Based Architecture

- SQS provides buffering for bursty traffic
- Job processing decoupled from API requests
- Can handle traffic spikes without overload

## Next Steps

Dive deeper into specific components:
- [Data Flow Documentation](data-flow.md) - Detailed request/response flows
- [Fossil API Architecture](fossil-api.md) - API service deep dive
- [Proving Service Architecture](proving-service.md) - Proof generation details
- [StarkNet Contracts](starknet-contracts.md) - Smart contract architecture

Or explore operational guides:
- [Local Development](../getting-started/local-development.md) - Running locally
- [Deployment Guide](../guides/deployment.md) - Production deployment
- [Debugging Guide](../guides/debugging.md) - Troubleshooting
