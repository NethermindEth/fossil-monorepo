.DEFAULT_GOAL := help

##@ Setup
.PHONY: setup
setup: setup-shared setup-ps setup-op ## Set up the complete development environment for all projects
	@echo "✅ Complete development environment set up successfully!"

.PHONY: setup-shared
setup-shared: ## Install shared dependencies
	@echo "🔧 Setting up shared dependencies..."
	make setup-rust
	make setup-coverage
	@echo "✅ Shared dependencies installed"

.PHONY: setup-ps
setup-ps: ## Set up Proving Service
	@echo "🔧 Setting up Proving Service environment..."
	make setup-postgres
	make setup-localstack
	cd proving-service && make setup-dev-env
	@echo "✅ Proving Service environment set up"

.PHONY: setup-op
setup-op: ## Set up Offchain Processor
	@echo "🔧 Setting up Offchain Processor environment..."
	docker compose -f offchain-processor/docker-compose.test.yml up -d offchain_processor_db
	cd offchain-processor && make setup-platform
	@echo "✅ Offchain Processor environment set up"

.PHONY: setup-rust
setup-rust: ## Install Rust and toolchains
	@echo "🔧 Checking Rust installation..."
	@if ! command -v rustup &> /dev/null; then \
		echo "Installing Rust..."; \
		curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y; \
	else \
		echo "✅ Rust already installed"; \
	fi
	@if ! rustup toolchain list | grep -q "nightly"; then \
		echo "Installing Rust nightly..."; \
		rustup toolchain install nightly; \
		rustup default nightly; \
	else \
		echo "✅ Rust nightly already installed"; \
	fi
	rustup component add rustfmt clippy
	rustup component add rustfmt clippy --toolchain nightly

.PHONY: setup-postgres
setup-postgres: ## Set up PostgreSQL for development
	docker compose -f proving-service/docker/docker-compose.test.yml up -d postgres

.PHONY: setup-localstack
setup-localstack: ## Set up LocalStack for AWS services
	docker compose -f proving-service/docker/docker-compose.sqs.yml up -d
	./proving-service/scripts/setup-localstack.sh

.PHONY: setup-coverage
setup-coverage: ## Install code coverage tools
	@echo "🔧 Setting up code coverage tools..."
	cargo install cargo-tarpaulin
	@rustup component add llvm-tools-preview
	@if ! command -v grcov &> /dev/null; then \
		echo "Installing grcov..."; \
		cargo install grcov; \
	else \
		echo "✅ grcov already installed"; \
	fi

##@ Monorepo Management

.PHONY: build-all
build-all: ## Build all projects in release mode.
	make ps-build
	make op-build

.PHONY: build-all-debug
build-all-debug: ## Build all projects in debug mode.
	cd proving-service && cargo build
	cd offchain-processor && cargo build

.PHONY: test-all
test-all: ## Run tests for all projects.
	make ps-test
	make op-test

.PHONY: lint-all
lint-all: ## Run linters for all projects.
	cd proving-service && make lint
	cd offchain-processor && make lint

.PHONY: pr
pr: ## Prepare all projects for a pull request.
	cd proving-service && make pr
	cd offchain-processor && make pr
	@echo "✅ All projects prepared for PR"

.PHONY: clean-all
clean-all: ## Clean all projects.
	make ps-clean
	make op-clean

##@ Proving Service

.PHONY: ps-build
ps-build: ## Build Proving Service in release mode.
	cd proving-service && make build

.PHONY: ps-test
ps-test: ## Run tests for Proving Service.
	cd proving-service && make test

.PHONY: ps-run
ps-run: ## Run Proving Service.
	cd proving-service && cargo run

.PHONY: ps-clean
ps-clean: ## Clean Proving Service build artifacts.
	cd proving-service && cargo clean
	rm -rf proving-service/target

##@ Offchain Processor

.PHONY: op-build
op-build: ## Build Offchain Processor in release mode.
	cd offchain-processor && cargo build --release

.PHONY: op-test
op-test: ## Run tests for Offchain Processor.
	cd offchain-processor && make test

.PHONY: op-run
op-run: ## Run Offchain Processor.
	cd offchain-processor && cargo run

.PHONY: op-clean
op-clean: ## Clean Offchain Processor build artifacts.
	cd offchain-processor && cargo clean
	rm -rf offchain-processor/target

##@ Development Environment

.PHONY: dev-services
dev-up: ## Start all development services.
	docker compose -f proving-service/docker/docker-compose.test.yml up -d
	docker compose -f proving-service/docker/docker-compose.sqs.yml up -d
	docker compose -f offchain-processor/docker-compose.test.yml up -d

.PHONY: dev-services-stop
dev-down: ## Stop all development services.
	docker compose -f proving-service/docker/docker-compose.test.yml down
	docker compose -f proving-service/docker/docker-compose.sqs.yml down
	docker compose -f offchain-processor/docker-compose.test.yml down

##@ Integration Testing

.PHONY: integration-up
integration-up: ## Start integration testing services (requires external fossil-network).
	@echo "🚀 Starting integration testing services..."
	@echo "⚠️  Note: This requires external fossil-network services to be running:"
	@echo "   - fossil-light-client (Katana on port 5050)"
	@echo "   - fossil-headers-db (Indexer + DB on ports 3000/5432)"
	docker compose -f docker-compose.integration.yml up -d proving_service_db offchain_processor_db localstack
	@echo "✅ Integration services started"
	@echo "💡 To deploy mock contracts: make integration-deploy-contracts"

.PHONY: integration-down
integration-down: ## Stop integration testing services.
	docker compose -f docker-compose.integration.yml down

.PHONY: integration-clean
integration-clean: ## Stop integration testing services and remove volumes.
	docker compose -f docker-compose.integration.yml down -v

.PHONY: integration-deploy-contracts
integration-deploy-contracts: ## Deploy contracts to external Katana instance.
	@echo "🔧 Deploying contracts to external Katana..."
	docker compose -f docker-compose.integration.yml --profile deploy up contract-deployer

.PHONY: integration-test
integration-test: ## Run full integration test suite with external services.
	@echo "🧪 Running integration tests..."
	@echo "⚠️  Ensure external services are running and mock contracts are deployed"
	make ps-test
	make op-test
	@echo "✅ Integration tests completed"

.PHONY: integration-logs
integration-logs: ## View logs from integration services.
	docker compose -f docker-compose.integration.yml logs -f

##@ Local Integration Testing

.PHONY: local-integration-up
local-integration-up: ## Start local integration testing services with Katana.
	@echo "🚀 Starting local integration testing services..."
	@echo "📡 This includes a local Katana testnet on port 5050"
	docker compose -f docker-compose.local.yml up -d katana proving_service_db offchain_processor_db localstack
	@echo "✅ Local integration services started"
	@echo "💡 To deploy mock contracts: make local-integration-deploy-contracts"

.PHONY: local-integration-down
local-integration-down: ## Stop local integration testing services.
	docker compose -f docker-compose.local.yml down

.PHONY: local-integration-clean
local-integration-clean: ## Stop local integration testing services and remove volumes.
	docker compose -f docker-compose.local.yml down -v

.PHONY: local-integration-deploy-contracts
local-integration-deploy-contracts: ## Deploy contracts to local Katana instance.
	@echo "🔧 Deploying contracts to local Katana..."
	docker compose -f docker-compose.local.yml --profile deploy up contract-deployer

.PHONY: local-integration-test
local-integration-test: ## Run full integration test suite with local services.
	@echo "🧪 Running local integration tests..."
	@echo "💡 Using local Katana testnet and databases"
	make ps-test
	make op-test
	@echo "✅ Local integration tests completed"

.PHONY: local-integration-logs
local-integration-logs: ## View logs from local integration services.
	docker compose -f docker-compose.local.yml logs -f

##@ Code Coverage

.PHONY: coverage-all
coverage-all: ## Run code coverage for all projects
	@echo "🔍 Running coverage for Proving Service..."
	cd proving-service && make coverage-clean && \
	{ docker compose -f docker/docker-compose.test.yml up -d && \
		CARGO_INCREMENTAL=0 \
		RUSTFLAGS="-C instrument-coverage -C codegen-units=1" \
		LLVM_PROFILE_FILE=".coverage/fossil-%p-%m.profraw" \
		cargo test --workspace && \
		grcov . --binary-path ./target/debug/ -s . -t html  --ignore-not-existing --ignore "/*" --ignore "tests/*" -o .coverage/html && \
		grcov . --binary-path ./target/debug/ -s . -t lcov  --ignore-not-existing --ignore "/*" --ignore "tests/*" -o .coverage/lcov.info && \
		echo "Coverage report generated at .coverage/html/index.html"; \
		docker compose -f docker/docker-compose.test.yml down -v; \
	}
	@echo "🔍 Running coverage for Offchain Processor..."
	cd offchain-processor && make coverage-clean && \
	{ docker compose -f docker-compose.test.yml up -d offchain_processor_db && \
		CARGO_INCREMENTAL=0 \
		RUSTFLAGS="-C instrument-coverage -C codegen-units=1" \
		LLVM_PROFILE_FILE=".coverage/fossil-%p-%m.profraw" \
		cargo test --workspace --all-features && \
		grcov . --binary-path ./target/debug/ -s . -t html --ignore-not-existing --ignore "/*" --ignore "tests/*" -o .coverage/html && \
		grcov . --binary-path ./target/debug/ -s . -t lcov --ignore-not-existing --ignore "/*" --ignore "tests/*" -o .coverage/lcov.info && \
		echo "Coverage report generated at .coverage/html/index.html"; \
		docker compose -f docker-compose.test.yml down -v; \
	}
	@echo "✅ Coverage reports generated for all projects"
	@echo "📊 Proving Service coverage: proving-service/.coverage/html/index.html"
	@echo "📊 Offchain Processor coverage: offchain-processor/.coverage/html/index.html"

##@ Testing
.PHONY: test-clean
test-clean: ## Clean up test environment
	docker compose -f proving-service/docker/docker-compose.test.yml down -v
	docker compose -f proving-service/docker/docker-compose.sqs.yml down -v
	docker compose -f offchain-processor/docker-compose.test.yml down -v
	docker compose -f docker-compose.integration.yml down -v

##@ Help

.PHONY: help
help: ## Display this help.
	@awk 'BEGIN {FS = ":.*##"; printf "\nUsage:\n  make \033[36m<target>\033[0m\n"} /^[a-zA-Z_-]+:.*?##/ { printf "  \033[36m%-15s\033[0m %s\n", $$1, $$2 } /^##@/ { printf "\n\033[1m%s\033[0m\n", substr($$0, 5) } ' $(MAKEFILE_LIST)
