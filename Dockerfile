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

# Build all binaries (PKCS#11 enabled by default)
RUN --mount=type=cache,target=/app/target \
    --mount=type=cache,target=/usr/local/cargo/registry \
    cargo build --release --workspace \
    && cp target/release/multisig-signer /usr/local/bin/ \
    && cp target/release/frost-service /usr/local/bin/

FROM debian:bullseye-slim

# Install runtime dependencies including SoftHSM2 (PKCS#11 provider)
# SoftHSM is mandatory for FROST nodes - provides hardware-backed key storage
# Compatible with YubiKey, enterprise HSM, and cloud HSM for production
RUN apt-get update && apt-get install -y \
    ca-certificates \
    softhsm2 \
    opensc \
    && rm -rf /var/lib/apt/lists/*

# Create SoftHSM directories
RUN mkdir -p /var/lib/softhsm/tokens && \
    chmod 755 /var/lib/softhsm/tokens

# Copy all binaries
COPY --from=builder /usr/local/bin/multisig-signer /usr/local/bin/
COPY --from=builder /usr/local/bin/frost-service /usr/local/bin/

# Copy entrypoint script for SoftHSM auto-initialization
COPY docker-entrypoint.sh /usr/local/bin/
RUN chmod +x /usr/local/bin/docker-entrypoint.sh

ENV CONFIG_PATH=/etc/config.toml

# Use entrypoint script for auto-initialization
ENTRYPOINT ["/usr/local/bin/docker-entrypoint.sh"]
CMD ["multisig-signer"]
