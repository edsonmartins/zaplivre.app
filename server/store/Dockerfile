# ZapLivre Message Store Dockerfile

FROM rust:1.88-slim-bookworm AS builder

WORKDIR /app

# Install dependencies
RUN apt-get update && apt-get install -y \
    pkg-config \
    libssl-dev \
    libpq-dev \
    && rm -rf /var/lib/apt/lists/*

# Copy workspace files
COPY Cargo.toml Cargo.lock ./
COPY core ./core
COPY server ./server

# Build message store
RUN cargo build --release --package zaplivre-store

# Runtime stage
FROM debian:bookworm-slim

RUN apt-get update && apt-get install -y \
    ca-certificates \
    libssl3 \
    libpq5 \
    curl \
    && rm -rf /var/lib/apt/lists/*

WORKDIR /app

# Copy binary
COPY --from=builder /app/target/release/zaplivre-store /usr/local/bin/zaplivre-store

EXPOSE 8080

ENV RUST_LOG=info
ENV SERVER_PORT=8080

CMD ["zaplivre-store"]
