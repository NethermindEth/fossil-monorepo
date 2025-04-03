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
	docker compose -f docker/docker-compose.test.yml up -d postgres

.PHONY: setup-localstack
setup-localstack: ## Set up LocalStack for AWS services
	docker compose -f docker/docker-compose.sqs.yml up -d

.PHONY: setup-coverage
setup-coverage: ## Install coverage tools.
	@echo "🔧 Setting up code coverage tools..."
	@rustup component add llvm-tools-preview
	@if ! command -v grcov &> /dev/null; then \
		echo "Installing grcov..."; \
		cargo install grcov; \
		echo "✅ grcov installed"; \
	else \
		echo "✅ grcov already installed"; \
	fi
	@mkdir -p .coverage

##@ Build

.PHONY: build
build: ## Build the project in release mode.
	cargo build --release

.PHONY: build-debug
build-debug: ## Build the project in debug mode.
	cargo build

##@ Development

.PHONY: dev-services
dev-services: ## Start all development services.
	docker compose -f docker/docker-compose.test.yml up -d
	docker compose -f docker/docker-compose.sqs.yml up -d

.PHONY: dev-services-stop
dev-services-stop: ## Stop all development services.
	docker compose -f docker/docker-compose.test.yml down
	docker compose -f docker/docker-compose.sqs.yml down

##@ Test

.PHONY: test
test: ## Run all tests with database dependencies.
	cargo test --workspace --all-features

.PHONY: test-clean
test-clean: ## Clean up test environment.
	docker compose -f docker/docker-compose.test.yml down -v
	docker compose -f docker/docker-compose.sqs.yml down -v

##@ Coverage

.PHONY: coverage-dir
coverage-dir: ## Create coverage directory if it doesn't exist
	@mkdir -p .coverage

.PHONY: coverage
coverage: coverage-dir ## Run tests with code coverage and generate HTML report.
	@rustup component add llvm-tools-preview
	docker compose -f docker/docker-compose.test.yml up -d &&\
	docker compose -f docker/docker-compose.sqs.yml up -d &&\
	{ CARGO_INCREMENTAL=0 \
	RUSTFLAGS="-C instrument-coverage -C codegen-units=1" \
	LLVM_PROFILE_FILE=".coverage/fossil-%p-%m.profraw" \
	cargo test --workspace --all-features; \
	status=$$?; \
	grcov . --binary-path ./target/debug/ -s . -t html --branch --ignore-not-existing --ignore "/*" --ignore "tests/*" -o .coverage/html &&\
	grcov . --binary-path ./target/debug/ -s . -t lcov --branch --ignore-not-existing --ignore "/*" --ignore "tests/*" -o .coverage/lcov.info &&\
	echo "Coverage report generated at .coverage/html/index.html"; \
	docker compose -f docker/docker-compose.test.yml down -v; \
	docker compose -f docker/docker-compose.sqs.yml down -v; \
	exit $$status; }

.PHONY: coverage-xml
coverage-xml: coverage-dir ## Generate code coverage report in XML format for CI.
	@rustup component add llvm-tools-preview
	CARGO_INCREMENTAL=0 \
	RUSTFLAGS="-C instrument-coverage -C codegen-units=1" \
	LLVM_PROFILE_FILE=".coverage/fossil-%p-%m.profraw" \
	cargo test --workspace --all-features &&\
	grcov . --binary-path ./target/debug/ -s . -t lcov --branch --ignore-not-existing --ignore "/*" --ignore "tests/*" -o .coverage/lcov.info

.PHONY: coverage-clean
coverage-clean: ## Clean up coverage artifacts.
	rm -rf .coverage

.PHONY: coverage-view
coverage-view: ## Open coverage report in the default browser (after running make coverage).
	@if [ -f .coverage/html/index.html ]; then \
		./scripts/open-coverage.sh; \
	else \
		echo "Coverage report not found. Run 'make coverage' first."; \
	fi

.PHONY: coverage-summary
coverage-summary: ## Display a text summary of the code coverage report.
	@if [ -f .coverage/html/index.html ]; then \
		./scripts/coverage-summary.sh; \
	else \
		echo "Coverage report not found. Run 'make coverage' first."; \
	fi

.PHONY: coverage-badge
coverage-badge: ## Generate a coverage badge.
	@if [ -f .coverage/html/index.html ] && [ -f .coverage/lcov.info ]; then \
		./scripts/generate-badge.sh; \
	else \
		echo "Coverage reports not found. Run 'make coverage' first."; \
	fi

##@ Linting

.PHONY: fmt
fmt: ## Format code with rustfmt.
	cargo +nightly fmt

.PHONY: clippy
clippy: ## Run clippy linter with project-specific settings.
	cargo +nightly clippy

.PHONY: lint-codespell
lint-codespell: ## Check for spelling mistakes.
	@if ! python3 -m pip list | grep -q codespell; then \
		echo "codespell not found. Installing with pip..." >&2; \
		python3 -m pip install codespell; \
	fi
	python3 -m codespell

.PHONY: lint
lint: fmt clippy lint-codespell ## Run all linters.

##@ Pull Request

.PHONY: pr
pr: ## Prepare code for a pull request.
	make lint && \
	make test

##@ Clean

.PHONY: clean
clean: ## Clean project build artifacts.
	cargo clean

##@ Help

.PHONY: help
help: ## Display this help.
	@awk 'BEGIN {FS = ":.*##"; printf "Usage:\n  make \033[36m<target>\033[0m\n"} /^[a-zA-Z_0-9-]+:.*?##/ { printf "  \033[36m%-15s\033[0m %s\n", $$1, $$2 } /^##@/ { printf "\n\033[1m%s\033[0m\n", substr($$0, 5) } ' $(MAKEFILE_LIST) 