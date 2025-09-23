#!/bin/bash
set -e

# Navigate to the project root directory
cd "$(git rev-parse --show-toplevel)"

echo "🔧 Building message-handler Docker image with mock-proof features..."

# Build the Docker image with the binary built inside the container
echo "Building Docker image with binary built inside container..."
docker build --platform linux/arm64 -t fossil-message-handler:with-files -f docker/Dockerfile.message-handler-build .

echo "✅ Message handler image ready: fossil-message-handler:with-files"
