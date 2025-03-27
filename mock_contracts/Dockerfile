FROM debian:bookworm-slim

WORKDIR /app


# Install dependencies
RUN apt-get update && apt-get install -y curl git

# Install Starkup with auto-confirm
RUN curl --proto '=https' --tlsv1.2 -sSf https://sh.starkup.dev | sh -s -- --yes

RUN curl https://get.starkli.sh | sh
ENV PATH="${PATH}:/root/.local/bin:/root/.asdf/shims:/root/.starkli/bin"
RUN starkliup
