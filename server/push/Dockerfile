# MePassa Push Notification Server Dockerfile

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

# Build push server
RUN cargo build --release --package mepassa-push

# Runtime stage
FROM debian:bookworm-slim

RUN apt-get update && apt-get install -y \
    ca-certificates \
    libssl3 \
    libpq5 \
    && rm -rf /var/lib/apt/lists/*

WORKDIR /app

# Copy binary
COPY --from=builder /app/target/release/mepassa-push /usr/local/bin/mepassa-push

# Create certs directory
RUN mkdir -p /etc/push

EXPOSE 8081

ENV RUST_LOG=info
ENV SERVER_PORT=8081

CMD ["mepassa-push"]
