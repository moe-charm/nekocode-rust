FROM rust:1.85-bookworm AS builder

WORKDIR /src
COPY nekocode-workspace/ /src/nekocode-workspace/
RUN cargo build \
      --manifest-path /src/nekocode-workspace/Cargo.toml \
      --package nekocode \
      --release

FROM debian:bookworm-slim

RUN apt-get update \
    && apt-get install -y --no-install-recommends ca-certificates git python3 \
    && rm -rf /var/lib/apt/lists/*

COPY --from=builder /src/nekocode-workspace/target/release/nekocode /usr/local/bin/nekocode
COPY mcp-nekocode-server/mcp_server_rust_first.py /app/mcp_server.py

ENV NEKOCODE_BINARY_PATH=/usr/local/bin/nekocode
WORKDIR /work

ENTRYPOINT ["python3", "-u", "/app/mcp_server.py"]
