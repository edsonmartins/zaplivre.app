# MePassa Signaling Server Dockerfile

FROM rust:1.88-slim-bookworm as builder

WORKDIR /app
RUN apt-get update && apt-get install -y pkg-config libssl-dev && rm -rf /var/lib/apt/lists/*

COPY Cargo.toml Cargo.lock ./
COPY core ./core
COPY server/signaling ./server/signaling

RUN cargo build --release --package mepassa-signaling

FROM debian:bookworm-slim

RUN apt-get update && apt-get install -y ca-certificates && rm -rf /var/lib/apt/lists/*

COPY --from=builder /app/target/release/mepassa-signaling /usr/local/bin/mepassa-signaling

EXPOSE 8086

CMD ["mepassa-signaling"]
