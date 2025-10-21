FROM rust:1.75-slim as builder

WORKDIR /app
COPY . .
RUN cargo build --release

FROM debian:bookworm-slim

RUN apt-get update && apt-get install -y ca-certificates && rm -rf /var/lib/apt/lists/*

COPY --from=builder /app/target/release/consensus-ring /usr/local/bin/consensus-ring

ENV CONFIG_PATH=/etc/consensus-ring/config.toml

CMD ["consensus-ring"]

