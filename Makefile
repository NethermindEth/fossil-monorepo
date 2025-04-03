.DEFAULT_GOAL := help

##@ Setup
.PHONY: setup
setup: ## Install all dependencies
	make setup-rust
	make setup-postgres
	make setup-localstack
	make setup-coverage

.PHONY: setup-rust
setup-rust: ## Install Rust and toolchains
	curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y
	rustup component add rustfmt clippy
	rustup toolchain install nightly
	rustup component add rustfmt clippy --toolchain nightly

.PHONY: setup-postgres
setup-postgres: ## Set up PostgreSQL for development
	docker compose -f proving-service/docker/docker-compose.test.yml up -d postgres

.PHONY: setup-localstack
setup-localstack: ## Set up LocalStack for AWS services
	docker compose -f proving-service/docker/docker-compose.sqs.yml up -d

.PHONY: setup-coverage
setup-coverage: ## Install code coverage tools
	cargo install cargo-tarpaulin

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

.PHONY: clean-all
clean-all: ## Clean all projects.
	make ps-clean
	make op-clean

##@ Proving Service

.PHONY: ps-build
ps-build: ## Build Proving Service in release mode.
	cd proving-service && cargo build --release

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
dev-services: ## Start all development services.
	docker compose -f proving-service/docker/docker-compose.test.yml up -d
	docker compose -f proving-service/docker/docker-compose.sqs.yml up -d
	docker compose -f offchain-processor/docker-compose.test.yml up -d

.PHONY: dev-services-stop
dev-services-stop: ## Stop all development services.
	docker compose -f proving-service/docker/docker-compose.test.yml down
	docker compose -f proving-service/docker/docker-compose.sqs.yml down
	docker compose -f offchain-processor/docker-compose.test.yml down

##@ Code Coverage

.PHONY: coverage
coverage: ## Run tests with coverage and generate HTML report
	cargo tarpaulin --workspace --out html

.PHONY: coverage-view
coverage-view: ## Open the coverage report in a browser
	xdg-open tarpaulin-report.html

.PHONY: coverage-xml
coverage-xml: ## Generate code coverage report in XML format for CI
	cargo tarpaulin --workspace --out xml

.PHONY: coverage-clean
coverage-clean: ## Clean up coverage artifacts
	rm -f tarpaulin-report.html cobertura.xml

.PHONY: coverage-summary
coverage-summary: ## Display a text summary of the coverage report
	cargo tarpaulin --workspace --out stdout

.PHONY: coverage-badge
coverage-badge: ## Generate a badge for the README
	cargo tarpaulin --workspace --out json
	python3 -c "import json; data = json.load(open('tarpaulin-report.json')); print(f'Coverage: {data[\"files\"][\"total\"][\"coverage\"]}%')"

##@ Testing
.PHONY: test-clean
test-clean: ## Clean up test environment
	docker compose -f proving-service/docker/docker-compose.test.yml down -v
	docker compose -f proving-service/docker/docker-compose.sqs.yml down -v
	docker compose -f offchain-processor/docker-compose.test.yml down -v

##@ Help

.PHONY: help
help: ## Display this help.
	@awk 'BEGIN {FS = ":.*##"; printf "\nUsage:\n  make \033[36m<target>\033[0m\n"} /^[a-zA-Z_-]+:.*?##/ { printf "  \033[36m%-15s\033[0m %s\n", $$1, $$2 } /^##@/ { printf "\n\033[1m%s\033[0m\n", substr($$0, 5) } ' $(MAKEFILE_LIST)