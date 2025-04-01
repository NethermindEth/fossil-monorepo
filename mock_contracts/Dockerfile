FROM rust:slim-bookworm AS builder
ENV DEBIAN_FRONTEND=noninteractive
RUN apt-get update && apt-get install -y curl git ca-certificates pkg-config libssl-dev jq && rm -rf /var/lib/apt/lists/*

# Install Rust
ENV CARGO_HOME=/usr/local/cargo
ENV RUSTUP_HOME=/usr/local/rustup
RUN rustup default stable

# Install Scarb
RUN mkdir -p /root/.local/bin
ENV PATH="/root/.local/bin:$CARGO_HOME/bin:$PATH"
RUN curl --proto '=https' --tlsv1.2 -sSf https://docs.swmansion.com/scarb/install.sh | sh -s -- -v 2.9.4

#Install Starkli
RUN curl https://get.starkli.sh | sh && /root/.starkli/bin/starkliup 
ENV PATH="/root/.local/bin:/root/.starkli/bin:$CARGO_HOME/bin:$PATH"

FROM debian:bookworm-slim
RUN apt-get update && apt-get install -y ca-certificates jq git build-essential && rm -rf /var/lib/apt/lists/*
COPY --from=builder /usr/local/cargo /usr/local/cargo
COPY --from=builder /usr/local/rustup /usr/local/rustup
COPY --from=builder /root/.local /root/.local
COPY --from=builder /root/.starkli /root/.starkli
ENV CARGO_HOME=/usr/local/cargo
ENV RUSTUP_HOME=/usr/local/rustup
ENV PATH="$CARGO_HOME/bin:/root/.local/bin:/root/.starkli/bin:$PATH"
WORKDIR /app
CMD ["bash"]