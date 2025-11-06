# Build stage
FROM rust:1.91-slim AS builder

WORKDIR /build

# Copy source
COPY Cargo.toml ./
COPY src ./src

# Build release binary
RUN --mount=type=cache,target=/build/target \
    --mount=type=cache,target=/usr/local/cargo/registry \
    cargo build --release && cp /build/target/release/frost-relay-server /usr/local/bin/


# Runtime stage (minimal)
FROM debian:bookworm-slim

# Install CA certificates for HTTPS
RUN apt-get update && apt-get install -y ca-certificates && rm -rf /var/lib/apt/lists/*

# Copy binary from builder
COPY --from=builder /usr/local/bin/frost-relay-server /usr/local/bin/

# Expose ports
EXPOSE 9090 9091

# Set environment
ENV RUST_LOG=info

# Run relay server
CMD ["/usr/local/bin/frost-relay-server"]


