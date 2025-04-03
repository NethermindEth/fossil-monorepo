.DEFAULT_GOAL := help

##@ Build

.PHONY: build
build: ## Build the project in release mode.
	cargo build --release

.PHONY: build-debug
build-debug: ## Build the project in debug mode.
	cargo build

##@ Test

.PHONY: test
test: ## Run all tests.
	cargo test --workspace --all-features

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