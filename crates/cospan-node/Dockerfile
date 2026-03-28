# Stage 1: Build
FROM rust:1.88-bookworm AS builder

RUN apt-get update && apt-get install -y cmake g++ pkg-config libssl-dev && rm -rf /var/lib/apt/lists/*

WORKDIR /app

COPY Cargo.toml Cargo.lock ./
COPY crates/ crates/
COPY packages/ packages/

RUN cargo build --release -p cospan-node

# Stage 2: Runtime
FROM debian:bookworm-slim

RUN apt-get update && apt-get install -y ca-certificates libssl3 && rm -rf /var/lib/apt/lists/*

COPY --from=builder /app/target/release/cospan-node /usr/local/bin/

RUN mkdir -p /data && chown 1001:1001 /data

USER 1001

EXPOSE 3001

CMD ["cospan-node"]
