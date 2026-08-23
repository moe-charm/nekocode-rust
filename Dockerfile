# LEGACY NekoCode all-in-one image (CLI + MCP + external tools)
# Rust-first users should build Dockerfile.rust-first instead.
FROM debian:bookworm-slim

ENV DEBIAN_FRONTEND=noninteractive \
    TERM=xterm-256color \
    RUSTUP_HOME=/root/.rustup \
    CARGO_HOME=/root/.cargo \
    PATH=/root/.cargo/bin:/root/.local/bin:/usr/local/go/bin:/root/go/bin:$PATH

# Base packages
RUN apt-get update && apt-get install -y --no-install-recommends \
    ca-certificates curl git python3 python3-pip python3-venv \
    nodejs npm golang clang-tidy \
    && rm -rf /var/lib/apt/lists/*

# Rust toolchain + clippy
RUN curl -sSf https://sh.rustup.rs | sh -s -- -y && \
    /root/.cargo/bin/rustup component add clippy

# External tools
RUN pip3 install --no-cache-dir vulture && \
    npm install -g eslint && \
    GO111MODULE=on GOPATH=/root/go go install honnef.co/go/tools/cmd/staticcheck@latest || true && \
    ln -sf /root/go/bin/staticcheck /usr/local/bin/staticcheck || true

WORKDIR /app

# Copy only what we need to run CLI + MCP
COPY releases/ /opt/nekocode/releases/
COPY mcp-nekocode-server/ /app/mcp-nekocode-server/

# Wrappers inside container
RUN printf '%s\n' '#!/usr/bin/env bash' \
      'set -euo pipefail' \
      'exec /opt/nekocode/releases/nekocode "$@"' \
    > /usr/local/bin/nekocode && chmod +x /usr/local/bin/nekocode && \
    printf '%s\n' '#!/usr/bin/env bash' \
      'set -euo pipefail' \
      'exec python3 /app/mcp-nekocode-server/mcp_server_real.py "$@"' \
    > /usr/local/bin/mcp-nekocode && chmod +x /usr/local/bin/mcp-nekocode

# Default to help
CMD ["nekocode", "--help"]
