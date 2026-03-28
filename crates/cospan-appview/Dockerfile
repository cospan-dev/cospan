# Stage 1: Build
FROM rust:1.85-bookworm AS builder

RUN apt-get update && apt-get install -y cmake g++ pkg-config libssl-dev && rm -rf /var/lib/apt/lists/*

WORKDIR /app

# Copy workspace files
COPY Cargo.toml Cargo.lock ./
COPY crates/ crates/
COPY packages/ packages/

# Build the cospan-appview binary
RUN cargo build --release -p cospan-appview

# Stage 2: Runtime
FROM debian:bookworm-slim

RUN apt-get update && apt-get install -y ca-certificates libssl3 curl && rm -rf /var/lib/apt/lists/*

COPY --from=builder /app/target/release/cospan-appview /usr/local/bin/
COPY --from=builder /app/crates/cospan-appview/migrations /app/migrations

WORKDIR /app

USER 1001

EXPOSE 3000

CMD ["cospan-appview"]
