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
setup-coverage: ## Install code coverage tools
	cargo install cargo-tarpaulin

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

##@ Code Coverage

.PHONY: coverage
coverage: ## Run tests with coverage and generate HTML report.
	cargo tarpaulin --workspace --out html

.PHONY: coverage-view
coverage-view: ## Open the coverage report in a browser.
	xdg-open tarpaulin-report.html

.PHONY: coverage-xml
coverage-xml: ## Generate code coverage report in XML format for CI.
	cargo tarpaulin --workspace --out xml

.PHONY: coverage-clean
coverage-clean: ## Clean up coverage artifacts.
	rm -f tarpaulin-report.html cobertura.xml

.PHONY: coverage-summary
coverage-summary: ## Display a text summary of the coverage report.
	cargo tarpaulin --workspace --out stdout

.PHONY: coverage-badge
coverage-badge: ## Generate a badge for the README.
	cargo tarpaulin --workspace --out json
	python3 -c "import json; data = json.load(open('tarpaulin-report.json')); print(f'Coverage: {data[\"files\"][\"total\"][\"coverage\"]}%')"

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