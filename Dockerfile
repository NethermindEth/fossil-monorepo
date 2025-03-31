FROM rust:1.82 as builder

# Create a new empty shell project
WORKDIR /usr/app

# Copy over manifests and source code
COPY . .

# Install system dependencies
RUN apt-get update && apt-get install -y \
    build-essential

COPY . .
# Build dependencies - this is the caching Docker layer!
RUN cargo build --release

# Our final base
FROM debian:bookworm-slim

WORKDIR /usr/app

RUN apt-get update && apt-get install -y libssl-dev ca-certificates && rm -rf /var/lib/apt/lists/*

COPY --from=builder /usr/app/target/release/message_handlers .

# Set the startup command
CMD ["/usr/app/message_handlers"]
