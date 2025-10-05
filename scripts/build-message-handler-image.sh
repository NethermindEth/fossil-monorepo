#!/bin/bash
set -e

# Navigate to the project root directory
cd "$(git rev-parse --show-toplevel)"

# Build the binary locally with mock-proof features
echo "Building message-handler binary locally with mock-proof features..."
cd proving-service
cargo build --release --bin message-handler --package message-handler --features proof-composition
cd ..

echo "Found message-handler binary at: proving-service/target/release/message-handler"

# Find the methods directory that contains the 'out' subdirectory
# Try proof-composition methods first, then fall back to mock-proof methods
METHODS_DIR=""
for dir in proving-service/target/release/build/proof-composition-twap-maxreturn-reserveprice-floating-hashing-methods-*; do
    if [ -d "$dir/out" ]; then
        METHODS_DIR="$dir"
        echo "Found proof-composition methods directory"
        break
    fi
done

# Fall back to mock-proof methods if proof-composition not found
if [ -z "$METHODS_DIR" ]; then
    for dir in proving-service/target/release/build/mock-proof-composition-methods-*; do
        if [ -d "$dir/out" ]; then
            METHODS_DIR="$dir"
            echo "Found mock-proof-composition methods directory"
            break
        fi
    done
fi

if [ -z "$METHODS_DIR" ]; then
    echo "Error: Could not find methods directory with 'out' subdirectory"
    echo "Available proof directories:"
    ls -la proving-service/target/release/build/ | grep -i "proof\|method" || echo "No proof/method directories found"
    exit 1
fi

echo "Found methods directory with out/ subdirectory: $METHODS_DIR"

# Build the Docker image (without copying files yet)
echo "Building base Docker image..."
docker build -t fossil-message-handler:base -f docker/Dockerfile.message-handler .

# Create and start a temporary container
echo "Creating and starting temporary container..."
CONTAINER_ID=$(docker run -d --entrypoint sleep fossil-message-handler:base infinity)

# Make the binary executable locally first
echo "Making binary executable locally..."
chmod +x proving-service/target/release/message-handler

# Copy the binary to the container
echo "Copying binary to container..."
docker cp proving-service/target/release/message-handler $CONTAINER_ID:/usr/local/bin/message-handler

# Copy RISC0 method ELFs to the container
echo "Copying method ELFs to container..."
if [ -d "$METHODS_DIR/out" ]; then
    # Get the full directory name (including hash) to preserve the exact path structure
    METHODS_FULL_NAME=$(basename "$METHODS_DIR")
    CONTAINER_METHODS_DIR="/app/target/release/build/${METHODS_FULL_NAME}"

    echo "Copying from: $METHODS_DIR/out/"
    echo "Copying to: $CONTAINER_METHODS_DIR/out/"

    # Create the directory structure in the running container first
    docker exec $CONTAINER_ID mkdir -p "$CONTAINER_METHODS_DIR"

    # Copy the method ELFs
    docker cp $METHODS_DIR/out/. $CONTAINER_ID:$CONTAINER_METHODS_DIR/out/

    # Verify the files were copied
    echo "Verifying files were copied..."
    docker exec $CONTAINER_ID ls -la "$CONTAINER_METHODS_DIR/out/" || echo "Warning: Could not verify files"
else
    echo "Warning: Method ELFs directory not found at $METHODS_DIR/out"
    docker stop $CONTAINER_ID
    docker rm $CONTAINER_ID
    exit 1
fi

# Stop the container before committing
echo "Stopping container..."
docker stop $CONTAINER_ID

# Commit the container as the final image with the correct entrypoint
echo "Committing container as final image..."
docker commit --change='ENTRYPOINT ["/usr/local/bin/message-handler-wrapper.sh"]' $CONTAINER_ID fossil-message-handler:with-files

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
