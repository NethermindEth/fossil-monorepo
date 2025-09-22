#!/bin/bash
set -e

# Navigate to the project root directory
cd "$(git rev-parse --show-toplevel)"

echo "🔧 Building message-handler Docker image with mock-proof features..."

# Build the Docker image (Docker will handle the architecture automatically)
echo "Building Docker image with binary built inside container..."
docker build -t fossil-message-handler:with-files -f docker/Dockerfile.message-handler .

echo "✅ Message handler image ready: fossil-message-handler:with-files"
