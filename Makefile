.DEFAULT_GOAL := help

##@ Setup

.PHONY: setup
setup: ## Install all dependencies and set up the complete development environment
	@echo "🚀 Setting up Fossil Monorepo development environment..."
	@echo ""
	@echo "1️⃣  Checking Docker..."
	@if ! command -v docker >/dev/null 2>&1; then \
		echo "   ❌ Docker is not installed!"; \
		echo "   Please install Docker Desktop from https://www.docker.com/products/docker-desktop/"; \
		echo "   After installation, make sure Docker is running and try again."; \
		exit 1; \
	else \
		echo "   ✅ Docker is installed"; \
	fi
	@if ! docker info >/dev/null 2>&1; then \
		echo "   ❌ Docker daemon is not running!"; \
		echo "   Please start Docker Desktop and try again."; \
		exit 1; \
	else \
		echo "   ✅ Docker daemon is running"; \
	fi
	@echo ""
	@echo "2️⃣  Installing Rust..."
	@if ! command -v rustup &> /dev/null; then \
		curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y; \
		. $$HOME/.cargo/env; \
	fi
	@rustup toolchain install stable
	@rustup default stable
	@rustup component add rustfmt clippy
	@echo "   ✅ Rust stable installed"
	@echo ""
	@echo "3️⃣  Installing RISC Zero..."
	@if ! command -v rzup &> /dev/null; then \
		curl -L https://risczero.com/install | bash; \
		. $$HOME/.cargo/env; \
		$$HOME/.risc0/bin/rzup install; \
	else \
		$$HOME/.risc0/bin/rzup install; \
	fi
	@echo "   ✅ RISC Zero installed"
	@echo ""
	@echo "4️⃣  Checking asdf version manager..."
	@if ! command -v asdf &> /dev/null; then \
		echo "   ❌ asdf is not installed!"; \
		echo ""; \
		echo "   Please install asdf by following the instructions at:"; \
		echo "   👉 https://asdf-vm.com/guide/getting-started.html"; \
		echo ""; \
		echo "   Quick install for macOS/Linux:"; \
		echo "   1. git clone https://github.com/asdf-vm/asdf.git ~/.asdf --branch v0.14.0"; \
		echo "   2. Add to your shell profile (~/.bashrc or ~/.zshrc):"; \
		echo "      . \"$$HOME/.asdf/asdf.sh\""; \
		echo "   3. Restart your terminal"; \
		echo "   4. Run 'make setup' again"; \
		exit 1; \
	else \
		echo "   ✅ asdf is installed"; \
	fi
	@asdf plugin add scarb 2>/dev/null || true
	@asdf plugin add starknet-foundry 2>/dev/null || true
	@asdf plugin add starkli 2>/dev/null || true
	@echo ""
	@echo "5️⃣  Installing StarkNet tools from .tool-versions..."
	@asdf install
	@echo "   ✅ Installed versions from .tool-versions:"
	@echo "      - Scarb 2.12.1"
	@echo "      - StarkNet Foundry 0.49.0"
	@echo "      - Starkli 0.4.2"
	@echo ""
	@echo "6️⃣  Setting up environment files..."
	@if [ ! -f .env.local ]; then \
		cp .env.example .env.local; \
		echo "   ✅ Created .env.local from .env.example"; \
	else \
		echo "   ✅ .env.local already exists"; \
	fi
	@if [ ! -f .env.docker ]; then \
		cp .env.example .env.docker; \
		sed -i.bak 's|STARKNET_RPC_URL=http://localhost:5050|STARKNET_RPC_URL=http://katana:5050|g' .env.docker; \
		sed -i.bak 's|OFFCHAIN_PROCESSOR_DATABASE_URL=postgresql://postgres:postgres@localhost:5434/postgres|OFFCHAIN_PROCESSOR_DATABASE_URL=postgresql://postgres:postgres@offchain_processor_db:5432/postgres|g' .env.docker; \
		sed -i.bak 's|PROVING_SERVICE_DATABASE_URL=postgresql://postgres:postgres@localhost:5435/postgres|PROVING_SERVICE_DATABASE_URL=postgresql://postgres:postgres@proving_service_db:5432/postgres|g' .env.docker; \
		sed -i.bak 's|AWS_ENDPOINT_URL=http://localhost:4567|AWS_ENDPOINT_URL=http://localstack:4566|g' .env.docker; \
		sed -i.bak 's|SQS_QUEUE_URL=http://localhost:4567/000000000000/fossilQueue|SQS_QUEUE_URL=http://localstack:4566/000000000000/fossilQueue|g' .env.docker; \
		sed -i.bak 's|PROVING_SERVICE_URL=http://127.0.0.1:3001|PROVING_SERVICE_URL=http://proving-service-api:3001|g' .env.docker; \
		rm -f .env.docker.bak; \
		echo "   ✅ Created .env.docker with Docker service URLs"; \
	else \
		echo "   ✅ .env.docker already exists"; \
	fi
	@echo ""
	@echo "✅ Setup complete! You can now run:"
	@echo "   make dev-up    - Start the development environment"
	@echo "   make dev-down  - Stop the development environment"
	@echo ""
	@echo "💡 Note: You may need to restart your shell or run:"
	@echo "   source ~/.asdf/asdf.sh"
	@echo "   to use the StarkNet tools immediately."

##@ Development

.PHONY: dev-up
dev-up: ## Start all local development services
	@echo "🚀 Starting local development services..."
	@echo "📋 Step 1: Starting infrastructure services..."
	docker-compose -f docker-compose.local.yml up -d katana proving_service_db offchain_processor_db localstack
	@echo "⏳ Waiting for Katana to be healthy..."
	@timeout=60; while [ $$timeout -gt 0 ]; do \
		if docker-compose -f docker-compose.local.yml exec -T katana sh -c 'curl -s -X POST -H "Content-Type: application/json" -d "{\"jsonrpc\":\"2.0\",\"method\":\"starknet_chainId\",\"params\":[],\"id\":1}" http://localhost:5050' > /dev/null 2>&1; then \
			echo "✅ Katana is healthy"; \
			break; \
		fi; \
		echo "   Waiting for Katana... ($$timeout seconds left)"; \
		sleep 2; \
		timeout=$$((timeout-2)); \
	done
	@if [ $$timeout -le 0 ]; then \
		echo "❌ Timeout waiting for Katana to be healthy"; \
		exit 1; \
	fi
	@echo "🔧 Step 2: Deploying contracts..."
	docker-compose -f docker-compose.deploy.yml run --rm contract-deployer
	@echo "🚀 Step 3: Starting application services..."
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
