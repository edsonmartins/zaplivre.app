# ZapLivre Signaling Server

WebSocket signaling server used as a fallback when P2P signaling fails.

## Run (local)

```bash
cd server/signaling
cargo run
```

Server listens on `0.0.0.0:8086`.

## Docker

```bash
docker build -t zaplivre-signaling:latest -f server/signaling/Dockerfile .
docker run --rm -p 8086:8086 zaplivre-signaling:latest
```

## Client config

Set the env var `SIGNALING_SERVER_URL` for the apps/core:

```
SIGNALING_SERVER_URL=ws://your-host:8086/ws
```
