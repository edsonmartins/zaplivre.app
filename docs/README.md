# MePassa Documentation

Welcome to the MePassa documentation! This folder contains comprehensive guides, architecture documentation, and API references.

## 📚 Documentation Structure

### Architecture (`architecture/`)

Deep-dive into the technical architecture of MePassa:

1. **[Hybrid Architecture](architecture/01-hybrid-architecture.md)**
   - Overview of P2P + Server hybrid model
   - Traffic distribution (80% P2P, 15% relay, 5% offline)
   - Cost comparisons and trade-offs

2. **[Core Components](architecture/02-core-components.md)**
   - Detailed breakdown of `mepassa-core` modules
   - Identity, Crypto, Network, Storage, VoIP, etc.
   - FFI bindings and API design

3. **[Networking Details](architecture/03-networking.md)** _(planned)_
   - libp2p configuration
   - NAT traversal (TURN, STUN, hole-punching)
   - DHT and peer discovery

4. **[Cryptography](architecture/04-crypto.md)** _(planned)_
   - Signal Protocol implementation
   - Key exchange (X3DH)
   - Double Ratchet algorithm

5. **[VoIP Architecture](architecture/05-voip.md)** _(planned)_
   - WebRTC setup
   - Audio/video codecs
   - Call signaling

6. **[Server Infrastructure](architecture/06-server-infrastructure.md)** _(planned)_
   - Bootstrap nodes
   - Message store
   - Push notifications

### API Reference (`api/`)

API documentation for developers:

- **[Client API](api/client-api.md)** _(planned)_
  - Rust API reference
  - Kotlin bindings
  - Swift bindings
  - JavaScript/TypeScript (Desktop)

### Guides (`guides/`)

Step-by-step guides for common tasks:

- **[Getting Started](guides/getting-started.md)**
  - Prerequisites and setup
  - First-time project build
  - Running examples
  - Troubleshooting

- **[Development Setup](guides/development-setup.md)** _(planned)_
  - IDE configuration
  - Debugging tips
  - Development workflow

- **[Building for Production](guides/building-for-production.md)** _(planned)_
  - Release builds
  - Code signing
  - Distribution

- **[Self-Hosting Guide](guides/self-hosting.md)**
  - Deploy your own server infrastructure
  - GitHub Secrets setup for CI

### Specifications (`specs/`)

Protocol and format specifications:

- **[Message Protocol](specs/message-protocol.md)** _(planned)_
  - Protocol Buffers definitions
  - Message types
  - Versioning strategy

- **[Database Schema](specs/database-schema.md)** _(planned)_
  - SQLite schema
  - PostgreSQL schema
  - Migrations

## 🚀 Quick Links

### For New Contributors

1. Start with [Getting Started](guides/getting-started.md)
2. Read the [Hybrid Architecture](architecture/01-hybrid-architecture.md) overview
3. Check [CONTRIBUTING.md](../CONTRIBUTING.md) for contribution guidelines
4. Join our community (Discord/Matrix - coming soon)

### For Users

1. [Installation Guide](guides/getting-started.md#quick-start)
2. [User Manual](guides/user-manual.md) _(planned)_
3. [FAQ](FAQ.md) _(planned)_

### For Enterprise

1. [Self-Hosting Guide](guides/self-hosting.md) _(planned)_
2. [Security Whitepaper](security-whitepaper.md) _(planned)_
3. [LGPD Compliance](lgpd-compliance.md) _(planned)_

## 📖 External Resources

- **Official Website**: https://mepassa.app _(coming soon)_
- **GitHub Repository**: https://github.com/integralltech/mepassa
- **Technical Blog**: _(coming soon)_

### Related Technologies

- [libp2p](https://docs.libp2p.io): P2P networking framework
- [Signal Protocol](https://signal.org/docs/): E2E encryption specification
- [WebRTC](https://webrtc.org): Real-time communication
- [Rust](https://doc.rust-lang.org/book/): Programming language
- [UniFFI](https://mozilla.github.io/uniffi-rs/): FFI bindings generator

## 🤝 Contributing to Docs

Documentation improvements are always welcome! To contribute:

1. Fork the repository
2. Edit files in the `docs/` folder
3. Submit a pull request

### Documentation Standards

- Use Markdown format
- Include code examples where applicable
- Add diagrams for complex concepts (use Mermaid)
- Keep language clear and concise
- Test all commands and code snippets

## 📝 License

This documentation is part of the MePassa project and is licensed under [AGPL-3.0](../LICENSE).

---

**Questions or feedback?** Open an issue or discussion on [GitHub](https://github.com/integralltech/mepassa).
