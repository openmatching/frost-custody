FROM rust:bullseye AS builder

# Install dependencies for compilation (including RocksDB requirements)
RUN --mount=type=cache,target=/var/lib/apt/lists \
    apt-get update && apt-get install -y \
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

RUN --mount=type=cache,target=/app/target \
    --mount=type=cache,target=/usr/local/cargo/registry \
    cargo build --release --workspace \
    && cp target/release/multisig-signer /usr/local/bin/ \
    && cp target/release/frost-service /usr/local/bin/

FROM debian:bullseye-slim

RUN apt-get update && apt-get install -y ca-certificates && rm -rf /var/lib/apt/lists/*

# Copy all binaries
COPY --from=builder /usr/local/bin/multisig-signer /usr/local/bin/
COPY --from=builder /usr/local/bin/frost-service /usr/local/bin/

ENV CONFIG_PATH=/etc/config.toml

# Default entrypoint (can be overridden)
ENTRYPOINT ["multisig-signer"]
