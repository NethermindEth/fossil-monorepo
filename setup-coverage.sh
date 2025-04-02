#!/bin/bash
set -e

echo "Setting up code coverage tools..."

# Ensure Rust is installed
if ! command -v rustc &> /dev/null; then
    echo "Rust not found. Installing Rust..."
    curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y
    source $HOME/.cargo/env
fi

# Add LLVM tools component
echo "Installing LLVM tools component..."
rustup component add llvm-tools-preview

# Install grcov
if ! command -v grcov &> /dev/null; then
    echo "Installing grcov..."
    cargo install grcov
else
    echo "grcov already installed."
fi

# Create coverage directory
mkdir -p .coverage
echo "Created .coverage directory for storing coverage data"

# Add environment variables to .bashrc or .zshrc
if [[ "$SHELL" == *"zsh"* ]]; then
    SHELL_RC="$HOME/.zshrc"
else
    SHELL_RC="$HOME/.bashrc"
fi

echo "Adding environment variables to $SHELL_RC"
if ! grep -q "RUSTFLAGS=\"-C instrument-coverage\"" "$SHELL_RC"; then
    echo '
# Rust code coverage settings
export RUSTFLAGS="-C instrument-coverage"
export LLVM_PROFILE_FILE=".coverage/fossil-%p-%m.profraw"
' >> "$SHELL_RC"
    echo "Environment variables added to $SHELL_RC"
else
    echo "Environment variables already exist in $SHELL_RC"
fi

echo "Code coverage tools setup complete!"
echo "To start using code coverage:"
echo "1. Run: source $SHELL_RC"
echo "2. Run: make coverage"
echo "3. View the report at .coverage/html/index.html" 