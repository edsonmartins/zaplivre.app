# ZapLivre Identity Server Dockerfile

FROM rust:1.88-slim-bookworm AS builder

WORKDIR /app

# Install dependencies
RUN apt-get update && apt-get install -y \
    pkg-config \
    libssl-dev \
    && rm -rf /var/lib/apt/lists/*

# Copy workspace files
COPY Cargo.toml Cargo.lock ./
COPY core ./core
COPY server ./server

# Build identity server
RUN cargo build --release --package zaplivre-identity-server

# Runtime stage
FROM debian:bookworm-slim

RUN apt-get update && apt-get install -y \
    ca-certificates \
    curl \
    && rm -rf /var/lib/apt/lists/*

WORKDIR /app

# Copy binary
COPY --from=builder /app/target/release/zaplivre-identity-server /usr/local/bin/zaplivre-identity-server

EXPOSE 8083

ENV RUST_LOG=info
ENV BIND_ADDR=0.0.0.0:8083

CMD ["zaplivre-identity-server"]
