FROM rust:bullseye AS builder

# Install dependencies for compilation (including RocksDB requirements)
RUN apt-get update && apt-get install -y \
    build-essential \
    clang \
    libclang-dev \
    llvm \
    libssl-dev \
    pkg-config \
    && rm -rf /var/lib/apt/lists/*

WORKDIR /app
COPY . .

# Build all binaries in one go
RUN cargo build --release --workspace

FROM debian:bullseye-slim

RUN apt-get update && apt-get install -y ca-certificates && rm -rf /var/lib/apt/lists/*

# Copy all binaries
COPY --from=builder /app/target/release/signer-node /usr/local/bin/
COPY --from=builder /app/target/release/frost-signer /usr/local/bin/
COPY --from=builder /app/target/release/frost-aggregator /usr/local/bin/

ENV CONFIG_PATH=/etc/config.toml

# Default entrypoint (can be overridden)
ENTRYPOINT ["signer-node"]
