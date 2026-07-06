# ZapLivre Push Notification Server Dockerfile

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
RUN cargo build --release --package zaplivre-push

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
COPY --from=builder /app/target/release/zaplivre-push /usr/local/bin/zaplivre-push

# Create certs directory
RUN mkdir -p /etc/push

EXPOSE 8081

ENV RUST_LOG=info
ENV PORT=8081

CMD ["zaplivre-push"]
