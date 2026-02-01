FROM rust:1.88-slim-bookworm AS builder

WORKDIR /app

# Copy workspace files
COPY Cargo.toml Cargo.lock ./
COPY core ./core
COPY server ./server

# Build release binary
RUN cargo build --release -p turn-credentials

# Runtime image
FROM debian:bookworm-slim

# Install ca-certificates for HTTPS
RUN apt-get update && \
    apt-get install -y ca-certificates && \
    rm -rf /var/lib/apt/lists/*

# Copy binary from builder
COPY --from=builder /app/target/release/turn-credentials /usr/local/bin/

# Expose port
EXPOSE 8082

# Run the service
CMD ["turn-credentials"]
