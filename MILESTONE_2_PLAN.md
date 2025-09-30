# Milestone 2 Development Plan - Fossil Monorepo

## Project Overview
This document outlines the detailed plan for completing Milestone 2 tasks for the Fossil monorepo. The codebase consists of two main services:
- **Proving Service**: Handles proof generation and verification with SQS message queuing
- **Fossil API**: Processes offchain data and interfaces with PitchLake (PL)

## Task Analysis and Implementation Plan

### 1. External API endpoints (for PL) - 1 day

**Current State:**
- Fossil API has existing API endpoints at `fossil-api/crates/server/src/lib.rs:47-71`
- Current endpoints: `/pricing_data` (POST, secured), `/health` (GET), `/api_key` (POST), `/job_status/{job_id}` (GET)
- PitchLake integration exists in `get_pricing_data.rs:306-360` via HTTP calls

**Implementation Plan:**
- **File Locations**: `fossil-api/crates/server/src/handlers/get_pricing_data.rs`, `fossil-api/crates/server/src/lib.rs`
- Review and enhance existing `/pricing_data` endpoint for PL requirements
- Add any missing external endpoints needed for PL integration
- Ensure proper error handling and response formats for external consumers
- Update API documentation and request/response schemas

**Effort**: 1 day - mostly configuration and documentation updates

---

### 2. Read fees and hash from chain - 1 day

**Current State:**
- No direct blockchain/chain reading functionality found in current codebase
- Database access exists for fee data at `proving-service/crates/db/src/models.rs:7` with `get_block_base_fee_by_time_range`
- Mock contracts exist in `proving-service/mock_contracts/` for StarkNet integration

**Implementation Plan:**
- **File Locations**: Create new module in `fossil-api/crates/` or `proving-service/crates/`
- Implement blockchain client integration (likely Ethereum/StarkNet based on mock contracts)
- Add chain reading functionality to fetch fee data and hashes
- Integrate with existing database models for storing chain data
- Add configuration for RPC endpoints and chain connection parameters

**Effort**: 1 day - new module creation and blockchain integration

---

### 3. Prover Service - Accept Proof Request - 2 days

**Current State:**
- Proving Service already accepts proof requests at `/api/job` endpoint (`proving-service/crates/proving-service/src/routes.rs:16`)
- Handler processes TWAP, Reserve Price, and Max Return jobs (`proving-service/crates/proving-service/src/handlers/jobs.rs:38-118`)
- Jobs are dispatched to SQS queue for processing

**Implementation Plan:**
- **File Locations**: `proving-service/crates/proving-service/src/handlers/jobs.rs`, `proving-service/crates/message-handler/src/services/jobs.rs`
- Enhance existing proof request handling to support additional request types
- Improve request validation and error handling
- Add support for batch processing and priority queuing
- Enhance job status tracking and progress reporting
- Add authentication and authorization for proof requests

**Effort**: 2 days - enhancing existing functionality and adding new features

---

### 4. Send Proof to Bonsai - 3 days

**Current State:**
- BonsaiProofProvider exists in `proving-service/crates/message-handler/src/proof_composition/mod.rs:56-298`
- Complex proof generation pipeline with RISC0 integration
- Feature-gated behind `proof-composition` flag

**Implementation Plan:**
- **File Locations**: `proving-service/crates/message-handler/src/proof_composition/mod.rs`, new Bonsai client module
- Implement Bonsai API client for proof submission
- Enhance existing BonsaiProofProvider to submit generated proofs
- Add retry logic and error handling for Bonsai communication
- Implement proof status tracking and completion callbacks
- Add configuration for Bonsai endpoints and credentials

**Effort**: 3 days - significant integration work with external service

---

### 5. Test Queueing - 1 day

**Current State:**
- SQS message queue implementation exists (`proving-service/crates/message-handler/src/queue/sqs_message_queue.rs`)
- Local message queue for testing (`proving-service/crates/message-handler/src/queue/local_message_queue.rs`)
- Comprehensive tests in ProofJobHandler (`proving-service/crates/message-handler/src/services/proof_job_handler.rs:302-680`)

**Implementation Plan:**
- **File Locations**: `proving-service/crates/message-handler/src/queue/`, test files
- Add comprehensive queue integration tests
- Test SQS failover and retry mechanisms
- Load testing for queue performance
- Test message ordering and deduplication
- Add monitoring and alerting for queue health

**Effort**: 1 day - focused testing work

---

### 6. Send Results to Pitchlake - 2 days

**Current State:**
- HTTP client integration exists in `get_pricing_data.rs:306-360` for calling proving service
- No direct result callback to PitchLake found

**Implementation Plan:**
- **File Locations**: New callback handler in `proving-service/crates/message-handler/src/`, `fossil-api/crates/server/src/handlers/`
- Implement result callback system to notify PitchLake of completion
- Add webhook/callback endpoints for proof completion
- Integrate with job status tracking system
- Add retry logic for failed result deliveries
- Implement secure authentication for result callbacks

**Effort**: 2 days - new callback system implementation

---

### 7. E2E Testing of Flow within Fossil - 5 days

**Current State:**
- Unit tests exist across components (236 test occurrences found)
- Test infrastructure includes Docker Compose for services
- Mock implementations for testing individual components

**Implementation Plan:**
- **File Locations**: `tests/` directory, integration test modules
- Create comprehensive E2E test suites covering full workflow
- Test PitchLake → Fossil API → Proving Service → Bonsai flow
- Add performance and load testing
- Implement test data generation and cleanup
- Add CI/CD pipeline integration tests
- Create automated regression test suite

**Effort**: 5 days - comprehensive testing infrastructure

---

### 8. E2E Joint Testing Support with PL - 3 days

**Current State:**
- Existing API endpoints ready for external integration
- Mock/test frameworks in place

**Implementation Plan:**
- **File Locations**: `tests/e2e/`, documentation, API specs
- Create test environments for PitchLake integration
- Develop shared test data and scenarios
- Implement API contract testing
- Add monitoring and observability for joint testing
- Create debugging and troubleshooting tools
- Document integration protocols and test procedures

**Effort**: 3 days - external integration testing setup

---

### 9. Dockerise the solution - 5 days

**Current State:**
- Dockerfiles exist for both services:
  - Proving Service: `proving-service/docker/Dockerfile`
  - Fossil API: `fossil-api/crates/server/Dockerfile`
- Docker Compose files for development and testing

**Implementation Plan:**
- **File Locations**: Docker files, `docker-compose*.yml`, deployment configs
- Optimize existing Dockerfiles for production use
- Create multi-stage builds for smaller image sizes
- Add Docker Compose orchestration for full system deployment
- Implement environment-specific configurations
- Add health checks and monitoring
- Create deployment documentation and scripts
- Implement secret management and security hardening

**Effort**: 5 days - production-ready containerization

---

### 10. Update Developer Documentation - 3 days

**Current State:**
- Basic README files exist for each service
- CLAUDE.md provides development guidance

**Implementation Plan:**
- **File Locations**: `README.md`, `docs/`, API documentation
- Comprehensive API documentation with examples
- Architecture diagrams and system overview
- Development setup and contribution guidelines
- Deployment and operations documentation
- Troubleshooting and FAQ sections
- Integration guides for external services

**Effort**: 3 days - comprehensive documentation

---

### 11. KT Document - 2 days

**Implementation Plan:**
- **File Locations**: `docs/knowledge-transfer/`
- Create knowledge transfer documentation covering:
  - System architecture and design decisions
  - Key components and their interactions  
  - External integrations and dependencies
  - Operational procedures and monitoring
  - Common issues and troubleshooting
  - Future roadmap and technical debt

**Effort**: 2 days - knowledge transfer documentation

---

### 12. Cleanup Code Comments - 1 day

**Current State:**
- Code contains development comments and TODOs

**Implementation Plan:**
- **File Locations**: All source files
- Remove debug comments and unused code
- Clean up TODO comments and implement or document
- Add proper documentation comments for public APIs
- Standardize code formatting and style
- Remove test-only code from production builds

**Effort**: 1 day - code cleanup and polish

---

## Priority Implementation Order

1. **Setup Phase**: External API endpoints (1 day) + Read fees from chain (1 day)
2. **Core Features**: Prover Service enhancements (2 days) + Send Proof to Bonsai (3 days)  
3. **Integration**: Send Results to Pitchlake (2 days) + Test Queueing (1 day)
4. **Testing Phase**: E2E Testing within Fossil (5 days) + Joint Testing with PL (3 days)
5. **Deployment**: Dockerise solution (5 days)
6. **Documentation**: Update Developer Docs (3 days) + KT Document (2 days) + Code Cleanup (1 day)

**Total Estimated Effort**: 29 days

## Key Technical Considerations

- All blockchain integration should support both testnet and mainnet configurations
- Implement comprehensive error handling and retry mechanisms
- Add proper logging and monitoring throughout the system
- Ensure secure handling of API keys and sensitive data
- Design for horizontal scalability and high availability
- Maintain backward compatibility during enhancements