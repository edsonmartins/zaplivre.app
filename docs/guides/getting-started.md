# Getting Started with ZapLivre

## Welcome!

ZapLivre is a hybrid P2P messaging platform that prioritizes privacy, reliability, and cost-effectiveness. This guide will help you get started with development.

## Prerequisites

### Required

- **Rust**: 1.75 or higher
  ```bash
  curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
  rustup default stable
  ```

- **Docker** & **Docker Compose**: For running server components
  ```bash
  # macOS
  brew install docker docker-compose

  # Linux
  sudo apt-get install docker.io docker-compose
  ```

- **Git**: For version control
  ```bash
  git --version  # Should be 2.x
  ```

### Platform-Specific

#### Android Development

- **JDK 17**: Java Development Kit
- **Android Studio**: Latest stable version
- **Android SDK**: API 24+ (Android 7.0)
- **Android NDK**: For Rust cross-compilation

#### iOS Development

- **Xcode**: Latest stable version (macOS only)
- **CocoaPods**: Dependency management
  ```bash
  sudo gem install cocoapods
  ```

#### Desktop Development

- **Node.js**: 20.x LTS
  ```bash
  # macOS
  brew install node@20

  # Linux
  curl -fsSL https://deb.nodesource.com/setup_20.x | sudo -E bash -
  sudo apt-get install -y nodejs
  ```

## Quick Start

### 1. Clone the Repository

```bash
git clone https://github.com/integralltech/zaplivre.git
cd zaplivre
```

### 2. Setup Development Environment

```bash
# Copy environment variables
cp .env.example .env

# Install Rust dependencies
cd core && cargo fetch && cd ..

# Or use Make
make setup
```

### 3. Start Server Components

```bash
# Start PostgreSQL, Redis, TURN server, etc.
docker-compose up -d

# Check service health
make health
```

Expected output:
```
PostgreSQL: ✓ OK
Redis: ✓ OK
Message Store: ✓ OK
Push Server: ✓ OK
```

### 3.1 Swarm (VPS / Production)

Para deploy via Docker Swarm, use o `stack.yml` na raiz e build das imagens antes:

```bash
docker build -f server/bootstrap/Dockerfile -t zaplivre-bootstrap:latest .
docker build -f server/store/Dockerfile -t zaplivre-store:latest .
docker build -f server/push/Dockerfile -t zaplivre-push:latest .
docker build -f server/turn-credentials/Dockerfile -t zaplivre-turn-credentials:latest .
docker stack deploy -c stack.yml zaplivre
```

Veja também o guia completo em `docs/guides/self-hosting.md`.

### 4. Build the Core Library

```bash
cd core
cargo build

# Or run tests
cargo test

# Or use Make from root
make build
```

### 5. Run an Example

```bash
cd core
cargo run --example simple_chat
```

You should see:
```
[INFO] ZapLivre Core initialized
[INFO] Peer ID: zaplivre_QmXXXXXX...
[INFO] Listening on: /ip4/127.0.0.1/tcp/4001
```

## Project Structure

```
zaplivre/
├── core/                    # Rust core library
│   ├── src/                 # Source code
│   ├── tests/               # Integration tests
│   ├── benches/             # Benchmarks
│   └── examples/            # Example applications
├── android/                 # Android app (Kotlin)
├── ios/                     # iOS app (Swift)
├── desktop/                 # Desktop app (Tauri)
├── server/                  # Server components
│   ├── bootstrap/           # Bootstrap nodes
│   ├── store/               # Message store
│   └── push/                # Push notifications
├── docs/                    # Documentation
├── docker-compose.yml       # Dev environment
└── Makefile                 # Common tasks
```

## Development Workflow

### Core Library Development

```bash
# Watch mode (auto-rebuild on changes)
cargo watch -x check -x test

# Format code
cargo fmt

# Lint code
cargo clippy -- -D warnings

# Or use Make
make check  # Runs fmt, lint, and test
```

### Android Development

```bash
# Build core for Android
cd core
cargo ndk -t arm64-v8a -o ../android/app/src/main/jniLibs build --release

# Build Android app
cd ../android
./gradlew build

# Install on device
./gradlew installDebug

# Or use Make
make dev-android
```

### iOS Development

```bash
# Build core for iOS
cd core
cargo lipo --release --targets aarch64-apple-ios

# Open Xcode
cd ../ios
open ZapLivre.xcworkspace
```

### Desktop Development

```bash
cd desktop

# Install dependencies
npm install

# Run in development mode
npm run tauri dev

# Or use Make
make dev-desktop
```

## Common Tasks

### View Logs

```bash
# All services
make logs

# Specific service
make logs-postgres
make logs-redis
make logs-bootstrap
```

### Database Operations

```bash
# Open PostgreSQL shell
make db-shell

# Reset database (WARNING: deletes all data)
make db-reset
```

### Health Checks

```bash
# Check all services
make health

# Check individual service
curl http://localhost:8080/health  # Message store
curl http://localhost:8081/health  # Push server
```

## Testing

### Unit Tests

```bash
cd core
cargo test
```

### Integration Tests

```bash
cd core
cargo test --test integration
```

### Manual Testing

1. **Start two clients:**
   ```bash
   # Terminal 1
   cargo run --example simple_chat -- --port 4001

   # Terminal 2
   cargo run --example simple_chat -- --port 4002
   ```

2. **Send message from Client 1:**
   ```
   > /connect <client2_peer_id>
   > Hello from client 1!
   ```

3. **Receive message on Client 2:**
   ```
   [INFO] Message received: Hello from client 1!
   ```

## Troubleshooting

### Build Errors

**Error**: `libssl-dev not found`

**Solution** (Linux):
```bash
sudo apt-get install pkg-config libssl-dev
```

**Solution** (macOS):
```bash
brew install openssl
export PKG_CONFIG_PATH="/usr/local/opt/openssl/lib/pkgconfig"
```

---

### Docker Issues

**Error**: `Cannot connect to Docker daemon`

**Solution**:
```bash
# Start Docker daemon
sudo systemctl start docker  # Linux
open -a Docker  # macOS
```

---

### Port Already in Use

**Error**: `Address already in use (os error 98)`

**Solution**:
```bash
# Find process using port
lsof -i :4001

# Kill process
kill -9 <PID>
```

---

### Database Connection Failed

**Error**: `Connection refused (os error 111)`

**Solution**:
```bash
# Check if PostgreSQL is running
docker-compose ps postgres

# Restart PostgreSQL
docker-compose restart postgres
```

## Next Steps

1. **Read the Architecture**: [Hybrid Architecture](../architecture/01-hybrid-architecture.md)
2. **Explore the API**: [Client API Reference](../api/client-api.md)
3. **Contribute**: [Contributing Guide](../../CONTRIBUTING.md)
4. **Join the Community**:
   - Discord: (coming soon)
   - Matrix: (coming soon)
   - GitHub Discussions: https://github.com/integralltech/zaplivre/discussions

## Resources

- [Rust Book](https://doc.rust-lang.org/book/)
- [libp2p Documentation](https://docs.libp2p.io)
- [Signal Protocol Specs](https://signal.org/docs/)
- [WebRTC Documentation](https://webrtc.org/getting-started/overview)
- [Tauri Guide](https://tauri.app/v2/guides/)

## Getting Help

- **GitHub Issues**: Bug reports and feature requests
- **GitHub Discussions**: Questions and general discussion
- **Email**: contato@integralltech.com.br

---

Happy coding! 🚀
