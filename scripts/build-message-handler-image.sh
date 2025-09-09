#!/bin/bash
set -e

# Navigate to the project root directory
cd "$(git rev-parse --show-toplevel)"

# Build the binary locally with mock-proof features
echo "Building message-handler binary locally with mock-proof features..."
cd proving-service
cargo build --release --bin message-handler --features mock-proof,starknet-handler
cd ..

echo "Found message-handler binary at: proving-service/target/release/message-handler"

# Build the Docker image (without copying files yet)
echo "Building base Docker image..."
docker build -t fossil-message-handler:base -f docker/Dockerfile.message-handler .

# Create a temporary container
echo "Creating temporary container..."
CONTAINER_ID=$(docker create fossil-message-handler:base)

# Make the binary executable locally first
echo "Making binary executable locally..."
chmod +x proving-service/target/release/message-handler

# Copy the binary to the container
echo "Copying binary to container..."
docker cp proving-service/target/release/message-handler $CONTAINER_ID:/usr/local/bin/message-handler

# Commit the container as the final image directly
echo "Committing container as final image..."
docker commit $CONTAINER_ID fossil-message-handler:with-files

# Remove the temporary container
echo "Cleaning up temporary container..."
docker rm $CONTAINER_ID

# Keep the CMD from Dockerfile for container compatibility
echo "Final image ready with CMD wrapper..."

# Clean up intermediate images
echo "Cleaning up intermediate images..."
# Don't remove the base image as it might be in use
# docker rmi fossil-message-handler:base || true

echo "Done! The fossil-message-handler:with-files image is now ready."