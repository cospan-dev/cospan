# Stage 1: Build
FROM rust:1.85-bookworm AS builder

WORKDIR /app

# Copy workspace files
COPY Cargo.toml Cargo.lock ./
COPY crates/ crates/

# Build the cospan-node binary
RUN cargo build --release -p cospan-node

# Stage 2: Runtime
FROM debian:bookworm-slim

RUN apt-get update && apt-get install -y ca-certificates && rm -rf /var/lib/apt/lists/*

COPY --from=builder /app/target/release/cospan-node /usr/local/bin/

USER 1001

EXPOSE 3001

CMD ["cospan-node"]
