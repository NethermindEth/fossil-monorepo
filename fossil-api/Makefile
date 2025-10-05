.DEFAULT_GOAL := help

##@ Setup

.PHONY: setup setup-rust setup-postgres setup-localstack setup-coverage setup-foundry setup-risc0 setup-starknet setup-platform init-repo

setup: setup-rust setup-postgres setup-coverage setup-platform init-repo ## Install all dependencies
	@echo "✅ All dependencies installed successfully!"

.PHONY: setup-rust
setup-rust: ## Install Rust and toolchains
	@echo "🔧 Checking Rust installation..."
	@if ! command -v rustup &> /dev/null; then \
		echo "Installing Rust..."; \
		curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y; \
	else \
		echo "✅ Rust already installed"; \
	fi
	@if ! rustup toolchain list | grep -q "stable"; then \
		echo "Installing Rust stable..."; \
		rustup toolchain install stable; \
		rustup default stable; \
	else \
		echo "✅ Rust stable already installed"; \
	fi

.PHONY: setup-postgres
setup-postgres: ## Set up PostgreSQL for development
	docker compose -f docker-compose.test.yml up -d fossil_api_db

.PHONY: setup-localstack
setup-localstack: ## Set up LocalStack for AWS services
	@echo "LocalStack not needed for fossil-api"

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

.PHONY: setup-foundry
setup-foundry: ## Set up Foundry
	@echo "Foundry not needed for fossil-api"

.PHONY: setup-risc0
setup-risc0: ## Set up RISC0
	@echo "RISC0 not needed for fossil-api"

.PHONY: setup-starknet
setup-starknet: ## Set up Starknet
	@echo "Starknet/Scarb not needed for fossil-api"

.PHONY: setup-platform
setup-platform: ## Set up platform dependencies
	@echo "🔧 Setting up platform dependencies..."
	@echo "⚠️ Note: Some dependencies may require sudo permissions"
	@if [ "$(shell uname)" = "Linux" ]; then \
		echo "On Linux, you may need to run the following command manually with sudo:"; \
		echo "sudo apt-get update && sudo apt-get install -y protobuf-compiler libpq-dev pkg-config python3-codespell"; \
	elif [ "$(shell uname)" = "Darwin" ]; then \
		echo "On macOS, you may need to run the following command manually:"; \
		echo "brew install protobuf postgresql codespell"; \
	fi

.PHONY: init-repo
init-repo: ## Initialize repository (git hooks, etc.)
	@echo "🔧 Initializing repository..."
	# Add any repo initialization steps here

##@ Build

.PHONY: build
build: ## Build the project in release mode.
	cargo build --release

.PHONY: build-debug
build-debug: ## Build the project in debug mode.
	cargo build

##@ Development

.PHONY: dev-up
dev-up: ## Start all development services.
	docker compose -f docker-compose.test.yml up -d

.PHONY: dev-down
dev-down: ## Stop all development services.
	docker compose -f docker-compose.test.yml down

##@ Test

.PHONY: test
test: ## Run all tests with database dependencies.
	cargo test --workspace --all-features

.PHONY: test-clean
test-clean: ## Clean up test environment.
	docker compose -f docker-compose.test.yml down -v

##@ Coverage

.PHONY: coverage-dir
coverage-dir: ## Create coverage directory if it doesn't exist
	@mkdir -p .coverage

.PHONY: coverage
coverage: coverage-dir ## Run tests with code coverage and generate HTML report.
	@rustup component add llvm-tools-preview
	docker compose -f docker-compose.test.yml up -d fossil_api_db &&\
	{ CARGO_INCREMENTAL=0 \
	RUSTFLAGS="-C instrument-coverage -C codegen-units=1" \
	LLVM_PROFILE_FILE=".coverage/fossil-%p-%m.profraw" \
	cargo test --workspace --all-features; \
	status=$$?; \
	grcov . --binary-path ./target/debug/ -s . -t html --branch --ignore-not-existing --ignore "/*" --ignore "tests/*" -o .coverage/html &&\
	grcov . --binary-path ./target/debug/ -s . -t lcov --branch --ignore-not-existing --ignore "/*" --ignore "tests/*" -o .coverage/lcov.info &&\
	echo "Coverage report generated at .coverage/html/index.html"; \
	docker compose -f docker-compose.test.yml down -v; \
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
	./scripts/clippy.sh

.PHONY: ensure-codespell
ensure-codespell:
	@if ! command -v codespell &> /dev/null && ! python3 -m codespell --help &> /dev/null; then \
		echo "⚠️ codespell not found. On Ubuntu/Debian, install with: sudo apt install python3-codespell"; \
		echo "⚠️ On macOS, install with: brew install codespell"; \
		echo "⚠️ Skipping codespell check..."; \
		exit 0; \
	fi

.PHONY: lint-codespell
lint-codespell: ## Check for spelling mistakes.
	@if command -v codespell &> /dev/null; then \
		codespell --skip="*.html,./target/*,./tarpaulin-report.html,*.profraw" --ignore-words-list="nd,te,ue,Nd,Te,Ue"; \
	elif python3 -m codespell --help &> /dev/null; then \
		python3 -m codespell --skip="*.html,./target/*,./tarpaulin-report.html,*.profraw" --ignore-words-list="nd,te,ue,Nd,Te,Ue"; \
	else \
		echo "⚠️ Skipping codespell check. Please install codespell for spell checking."; \
	fi

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
