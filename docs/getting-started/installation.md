# Installation Guide

This guide walks you through installing all dependencies required for Fossil monorepo development.

## Table of Contents
- [System Requirements](#system-requirements)
- [Quick Installation](#quick-installation)
- [Manual Installation](#manual-installation)
- [Verification](#verification)
- [Troubleshooting](#troubleshooting)

## System Requirements

### Operating System
- Linux (Ubuntu 20.04+, Debian 11+, etc.)
- macOS (11.0+)
- Windows with WSL2

### Hardware
- CPU: 4+ cores recommended
- RAM: 16GB minimum, 32GB recommended
- Disk: 50GB+ free space
- Internet connection for downloading dependencies

## Quick Installation

The fastest way to set up your development environment is using the automated setup script:

```bash
make setup
```

This single command will:
1. Check Docker installation and status
2. Install Rust toolchain (stable)
3. Install RISC Zero toolchain
4. Verify asdf version manager
5. Install StarkNet tools (Scarb, Starknet Foundry, Starkli)
6. Configure environment files
7. Build both services in release mode

**Estimated time:** 15-30 minutes (depending on your internet speed)

## Manual Installation

If you prefer to install dependencies manually or need to troubleshoot issues, follow these steps:

### 1. Docker

Docker is required for running PostgreSQL, LocalStack (AWS SQS), and the Katana StarkNet devnet.

#### Installation

**macOS:**
```bash
# Download and install Docker Desktop from:
# https://www.docker.com/products/docker-desktop/
```

**Linux (Ubuntu/Debian):**
```bash
# Install Docker Engine
curl -fsSL https://get.docker.com -o get-docker.sh
sudo sh get-docker.sh

# Add your user to the docker group
sudo usermod -aG docker $USER
newgrp docker

# Install Docker Compose
sudo apt-get install docker-compose-plugin
```

**Verification:**
```bash
docker --version
docker compose version
docker info  # Ensure daemon is running
```

### 2. Rust Toolchain

Fossil uses Rust stable with rustfmt and clippy for code formatting and linting.

#### Installation

```bash
# Install rustup (Rust version manager)
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y

# Source cargo environment
source $HOME/.cargo/env

# Install stable toolchain
rustup toolchain install stable
rustup default stable

# Install required components
rustup component add rustfmt clippy
```

**Verification:**
```bash
rustc --version    # Should show 1.70.0+
cargo --version
rustfmt --version
cargo clippy --version
```

### 3. RISC Zero Toolchain

RISC Zero is used for zero-knowledge proof generation. Install via `rzup`:

#### Installation

```bash
# Install rzup
curl -L https://risczero.com/install | bash

# Source cargo environment
source $HOME/.cargo/env

# Install RISC Zero toolchain
$HOME/.risc0/bin/rzup install
```

**Verification:**
```bash
cargo risczero --version
```

**Troubleshooting:**
- If `rzup` is not found, ensure `$HOME/.risc0/bin` is in your PATH
- You may need to restart your terminal after installation

### 4. asdf Version Manager

asdf is used to manage StarkNet tool versions consistently across the team.

#### Installation

**macOS/Linux:**
```bash
# Clone asdf repository
git clone https://github.com/asdf-vm/asdf.git ~/.asdf --branch v0.14.0

# Add to your shell profile
# For bash (~/.bashrc):
echo '. "$HOME/.asdf/asdf.sh"' >> ~/.bashrc

# For zsh (~/.zshrc):
echo '. "$HOME/.asdf/asdf.sh"' >> ~/.zshrc

# Restart your terminal or source your profile
source ~/.bashrc  # or source ~/.zshrc
```

**Verification:**
```bash
asdf --version  # Should show v0.14.0
```

**Documentation:** https://asdf-vm.com/guide/getting-started.html

### 5. StarkNet Tools

The project requires specific versions of Scarb, Starknet Foundry, and Starkli.

#### Installation

```bash
# Add asdf plugins
asdf plugin add scarb
asdf plugin add starknet-foundry
asdf plugin add starkli

# Install versions from .tool-versions file
asdf install

# Set global versions
asdf global scarb 2.12.1
asdf global starknet-foundry 0.49.0
asdf global starkli 0.4.2
```

**Verification:**
```bash
scarb --version              # Should show 2.12.1
snforge --version            # Should show 0.49.0
starkli --version            # Should show 0.4.2
```

### 6. Environment Configuration

Set up your environment files for local development:

```bash
# Copy example environment file
cp .env.example .env.local

# Create Docker environment file
cp .env.example .env.docker

# Update .env.docker with container-specific URLs
# (This is done automatically by `make setup`)
```

See [Environment Setup Guide](../guides/environment-setup.md) for detailed configuration.

### 7. Build Projects

Build both services to verify installation:

```bash
# Build both projects in release mode
make build-all

# Or build individually:
cd proving-service && cargo build --release
cd fossil-api && cargo build --release
```

## Verification

Verify your complete installation:

```bash
# Run all checks
make verify-install  # If this target exists

# Or manually verify each tool:
docker --version
docker compose version
rustc --version
cargo --version
cargo risczero --version
asdf --version
scarb --version
snforge --version
starkli --version
```

### Expected Output

```
Docker version 24.0.0+
Docker Compose version v2.20.0+
rustc 1.70.0+
cargo 1.70.0+
cargo-risczero 2.0.0+
v0.14.0
scarb 2.12.1
snforge 0.49.0
starkli 0.4.2
```

## Troubleshooting

### Docker Issues

**Problem:** `docker: command not found`
```bash
# Verify Docker installation
which docker

# Add Docker to PATH (if installed but not in PATH)
export PATH="/usr/local/bin:$PATH"
```

**Problem:** `permission denied while trying to connect to Docker daemon`
```bash
# Add user to docker group
sudo usermod -aG docker $USER
newgrp docker
```

### Rust/Cargo Issues

**Problem:** `cargo: command not found`
```bash
# Source cargo environment
source $HOME/.cargo/env

# Add to shell profile permanently
echo 'source $HOME/.cargo/env' >> ~/.bashrc
```

**Problem:** Compilation errors
```bash
# Update Rust to latest stable
rustup update stable

# Clean build cache
cargo clean
```

### RISC Zero Issues

**Problem:** `cargo risczero: command not found`
```bash
# Reinstall RISC Zero toolchain
$HOME/.risc0/bin/rzup install

# Verify installation directory
ls -la $HOME/.risc0/bin/
```

### asdf Issues

**Problem:** `asdf: command not found`
```bash
# Verify asdf installation
ls -la ~/.asdf

# Source asdf in current shell
source ~/.asdf/asdf.sh

# Add to shell profile (if not already added)
echo '. "$HOME/.asdf/asdf.sh"' >> ~/.bashrc
```

**Problem:** Plugin installation fails
```bash
# Update asdf plugins
asdf plugin update --all

# Remove and re-add plugin
asdf plugin remove scarb
asdf plugin add scarb
asdf install
```

### StarkNet Tools Issues

**Problem:** Wrong version showing
```bash
# Check which version is active
asdf current

# Reset to .tool-versions
asdf install
asdf reshim
```

### Build Issues

**Problem:** Out of memory during compilation
```bash
# Reduce parallel compilation jobs
export CARGO_BUILD_JOBS=2
cargo build --release
```

**Problem:** Disk space errors
```bash
# Clean build artifacts
cargo clean
rm -rf target/

# Clean Docker cache
docker system prune -a
```

## Next Steps

Once installation is complete, proceed to:
- [Local Development Guide](local-development.md) - Set up and run the development environment
- [Testing Guide](testing.md) - Learn how to run and write tests

## Additional Resources

- [Rust Documentation](https://doc.rust-lang.org/)
- [RISC Zero Documentation](https://dev.risczero.com/)
- [StarkNet Documentation](https://docs.starknet.io/)
- [asdf Documentation](https://asdf-vm.com/)
