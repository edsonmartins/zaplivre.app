# MePassa Signaling Server Dockerfile

FROM rust:1.88-slim-bookworm as builder

WORKDIR /app
RUN apt-get update && apt-get install -y pkg-config libssl-dev && rm -rf /var/lib/apt/lists/*

# Copy the full workspace: the root Cargo.toml declares all server/* crates as
# members, so every member manifest must be present for cargo to parse it.
COPY Cargo.toml Cargo.lock ./
COPY core ./core
COPY server ./server

RUN cargo build --release --package mepassa-signaling

FROM debian:bookworm-slim

RUN apt-get update && apt-get install -y ca-certificates curl && rm -rf /var/lib/apt/lists/*

COPY --from=builder /app/target/release/mepassa-signaling /usr/local/bin/mepassa-signaling

EXPOSE 8086

CMD ["mepassa-signaling"]
