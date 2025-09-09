.DEFAULT_GOAL := help

##@ Setup

.PHONY: setup
setup: ## Install all dependencies and set up the complete development environment
	@echo "🔧 Setting up complete development environment..."
	@echo "Installing Rust..."
	@if ! command -v rustup &> /dev/null; then \
		curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y; \
	fi
	@if ! rustup toolchain list | grep -q "nightly"; then \
		rustup toolchain install nightly; \
		rustup default nightly; \
	fi
	rustup component add rustfmt clippy
	rustup component add rustfmt clippy --toolchain nightly
	@echo "Installing coverage tools..."
	cargo install cargo-tarpaulin
	rustup component add llvm-tools-preview
	@if ! command -v grcov &> /dev/null; then \
		cargo install grcov; \
	fi
	@echo "Setting up Proving Service..."
	cd proving-service && make setup-dev-env
	@echo "Setting up Offchain Processor..."
	cd offchain-processor && make setup-platform
	@echo "✅ Complete development environment ready!"

##@ Development

.PHONY: dev-up
dev-up: ## Start all local development services
	@echo "🚀 Starting local development services..."
	docker-compose -f docker-compose.local.yml up -d
	@echo "✅ Services started:"
	@echo "  📡 Katana: http://localhost:5050"
	@echo "  📊 Offchain Processor: http://localhost:3000"
	@echo "  🗄️  Databases: Proving Service (5435), Offchain Processor (5434)"
	@echo "  ☁️  LocalStack: http://localhost:4567"

.PHONY: dev-down
dev-down: ## Stop services and clean up (removes volumes)
	@echo "🛑 Stopping and cleaning up..."
	docker-compose -f docker-compose.local.yml down -v
	@echo "✅ Environment cleaned"

.PHONY: logs
logs: ## View logs from all services
	docker-compose -f docker-compose.local.yml logs -f

##@ Build & Test

.PHONY: build
build: ## Build all projects in release mode
	cd proving-service && cargo build --release
	cd offchain-processor && cargo build --release
	@echo "✅ Build complete"

.PHONY: build-message-handler-image
build-message-handler-image: ## Build message-handler Docker image with pre-compiled mock-proof binary
	@echo "🔧 Building message-handler Docker image with mock-proof features..."
	./scripts/build-message-handler-image.sh
	@echo "✅ Message handler image ready: fossil-message-handler:with-files"

.PHONY: test
test: ## Run all tests
	cd proving-service && make test
	cd offchain-processor && make test
	@echo "✅ Tests complete"

##@ Help

.PHONY: help
help: ## Display this help
	@awk 'BEGIN {FS = ":.*##"; printf "\nUsage:\n  make \033[36m<target>\033[0m\n"} /^[a-zA-Z_-]+:.*?##/ { printf "  \033[36m%-10s\033[0m %s\n", $$1, $$2 } /^##@/ { printf "\n\033[1m%s\033[0m\n", substr($$0, 5) } ' $(MAKEFILE_LIST)