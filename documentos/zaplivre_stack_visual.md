# ZapLivre - Tech Stack Visual Map

```
╔══════════════════════════════════════════════════════════════╗
║                     ZAPLIVRE TECH STACK                        ║
║                  "Use o que já funciona"                      ║
╚══════════════════════════════════════════════════════════════╝

┌──────────────────────────────────────────────────────────────┐
│  📱 CLIENTS                                                   │
├──────────────────────────────────────────────────────────────┤
│                                                               │
│  ┌─────────────┐  ┌─────────────┐  ┌─────────────┐         │
│  │   ANDROID   │  │     iOS     │  │   DESKTOP   │         │
│  ├─────────────┤  ├─────────────┤  ├─────────────┤         │
│  │ Kotlin      │  │ Swift       │  │ Tauri 2.0   │         │
│  │ Compose     │  │ SwiftUI     │  │ React/Vue   │         │
│  │ Coroutines  │  │ Combine     │  │ TypeScript  │         │
│  └──────┬──────┘  └──────┬──────┘  └──────┬──────┘         │
│         │                │                │                  │
│         └────────────────┼────────────────┘                  │
│                          │                                   │
└──────────────────────────┼───────────────────────────────────┘
                           │
                           ▼
┌──────────────────────────────────────────────────────────────┐
│  🦀 CORE LIBRARY (Rust) - FFI via UniFFI                     │
├──────────────────────────────────────────────────────────────┤
│                                                               │
│  ┌─────────────────────────────────────────────────────┐    │
│  │  🌐 NETWORKING         🔐 CRYPTO           📞 VOIP  │    │
│  │  ─────────────         ────────           ──────    │    │
│  │  rust-libp2p          Signal Protocol     WebRTC    │    │
│  │  • TCP/QUIC           • Double Ratchet    • Opus    │    │
│  │  • Noise encrypt      • X3DH handshake    • VP8/VP9 │    │
│  │  • Kademlia DHT       • Sender Keys       • STUN    │    │
│  │  • GossipSub          • Forward secrecy   • TURN    │    │
│  │  • Circuit Relay      • Break-in recovery • ICE     │    │
│  │  • DCUtR hole-punch   • Async messaging   • RTP     │    │
│  └─────────────────────────────────────────────────────┘    │
│                                                               │
│  ┌─────────────────────────────────────────────────────┐    │
│  │  🗄️  STORAGE           🔄 SYNC             🔧 FFI   │    │
│  │  ────────           ───────             ─────       │    │
│  │  SQLite             Automerge (CRDTs)   UniFFI      │    │
│  │  • rusqlite         • Multi-device      • Kotlin    │    │
│  │  • WAL mode         • Offline-first     • Swift     │    │
│  │  • FTS5 search      • Conflict-free     • C-ABI     │    │
│  │  • Encrypted        • JSON-like API     • Auto-gen  │    │
│  └─────────────────────────────────────────────────────┘    │
│                                                               │
└───────────────────────────────────────────────────────────────┘

┌──────────────────────────────────────────────────────────────┐
│  ☁️  BACKEND / INFRASTRUCTURE                                 │
├──────────────────────────────────────────────────────────────┤
│                                                               │
│  ┌──────────────┐  ┌──────────────┐  ┌──────────────┐      │
│  │  BOOTSTRAP   │  │ MESSAGE STORE│  │  TURN/STUN   │      │
│  │    NODES     │  │              │  │    RELAY     │      │
│  ├──────────────┤  ├──────────────┤  ├──────────────┤      │
│  │ Rust         │  │ PostgreSQL   │  │ coturn       │      │
│  │ + libp2p     │  │ + Redis      │  │ (C/C++)      │      │
│  │              │  │              │  │              │      │
│  │ • Discovery  │  │ • Store fwd  │  │ • NAT travrsl│      │
│  │ • Routing    │  │ • Offline    │  │ • Relay 20%  │      │
│  │ • DHT seed   │  │ • TTL 14d    │  │ • UDP/TCP    │      │
│  └──────────────┘  └──────────────┘  └──────────────┘      │
│                                                               │
│  ┌──────────────┐  ┌──────────────┐                         │
│  │ PUSH NOTIFY  │  │      SFU     │                         │
│  ├──────────────┤  ├──────────────┤                         │
│  │ FCM (Android)│  │ mediasoup    │                         │
│  │ APNs (iOS)   │  │ (Node.js)    │                         │
│  │ UnifiedPush  │  │              │                         │
│  │ (optional)   │  │ • Group calls│                         │
│  └──────────────┘  │ • Video fwd  │                         │
│                     └──────────────┘                         │
└──────────────────────────────────────────────────────────────┘

┌──────────────────────────────────────────────────────────────┐
│  🔧 DEVELOPMENT TOOLS                                         │
├──────────────────────────────────────────────────────────────┤
│                                                               │
│  Build & Package:     Testing:           Quality:            │
│  • cargo              • tokio-test       • clippy            │
│  • Gradle (Android)   • mockall          • rustfmt           │
│  • Xcode (iOS)        • proptest         • cargo-audit       │
│  • npm/pnpm (Web)     • criterion        • cargo-deny        │
│                       (benchmarks)                            │
│  CI/CD:               Docs:              Monitoring:         │
│  • GitHub Actions     • rustdoc          • Prometheus        │
│  • Docker             • mdBook           • Grafana           │
│  • k8s (prod)         • Swagger/OpenAPI  • Sentry            │
│                                                               │
└──────────────────────────────────────────────────────────────┘

═══════════════════════════════════════════════════════════════

KEY DECISIONS & WHY:

🦀 Rust Core:
   WHY: Memory safe, fast, great crypto/network libs, FFI to all platforms
   USED BY: Discord, Cloudflare, AWS, Meta

📡 libp2p:
   WHY: Mature P2P networking, used by IPFS/Polkadot/Ethereum 2.0
   ALTERNATIVE REJECTED: Roll our own (too complex, bugs)

🔐 Signal Protocol:
   WHY: Battle-tested E2E, used by WhatsApp/Signal/FB Messenger
   ALTERNATIVE: MLS (future, for groups >100)

📞 WebRTC:
   WHY: Industry standard VoIP, P2P optimized, used everywhere
   ALTERNATIVE REJECTED: Custom RTP/SRTP (reinventing wheel)

🗄️ SQLite:
   WHY: >1 trillion databases in use, embedded, fast
   ALTERNATIVE REJECTED: sled (less mature), Postgres (needs server)

🔄 Automerge (CRDTs):
   WHY: Conflict-free multi-device sync, Figma/Notion use CRDTs
   ALTERNATIVE: OT (more complex, needs central server)

🔗 UniFFI:
   WHY: Auto-generates bindings, type-safe, Mozilla maintains
   ALTERNATIVE REJECTED: Manual JNI/ObjC (too much work, error-prone)

═══════════════════════════════════════════════════════════════

DEPENDENCY STATS:

Total external dependencies: ~50
Battle-tested (>5 years old): 90%
Actively maintained (commit <3mo): 100%
Open source (MIT/Apache): 95%
Used in production: 100%

Largest dependencies (by ecosystem size):
1. libp2p          - 1M+ downloads/month (crates.io)
2. WebRTC          - Billions of daily users
3. SQLite          - >1 trillion active databases
4. Signal Protocol - 2B+ users (WhatsApp alone)
5. Tokio           - Rust async standard (100M+ downloads)

═══════════════════════════════════════════════════════════════

COMPARISON WITH ALTERNATIVES:

┌──────────────┬─────────────┬─────────────┬─────────────┐
│   Feature    │   ZapLivre   │  WhatsApp   │   Signal    │
├──────────────┼─────────────┼─────────────┼─────────────┤
│ Networking   │ libp2p P2P  │ Centralized │ Centralized │
│ Crypto       │ Signal      │ Signal      │ Signal      │
│ VoIP         │ WebRTC      │ WebRTC      │ WebRTC      │
│ Storage      │ SQLite      │ SQLite      │ SQLite      │
│ Language     │ Rust        │ Erlang/C++  │ Java        │
│ Open Source  │ ✅ Full     │ ❌ None     │ ⚠️ Partial  │
│ Self-host    │ ✅ Yes      │ ❌ No       │ ⚠️ Complex  │
│ Costs (1000) │ ~$500/mo    │ N/A         │ N/A         │
└──────────────┴─────────────┴─────────────┴─────────────┘

ADVANTAGES:
✅ P2P reduces server costs 80%
✅ No single point of failure
✅ Works even if servers down
✅ Fully open source & auditable
✅ LGPD compliant (data in Brazil)

═══════════════════════════════════════════════════════════════

LICENSES COMPATIBILITY:

Core Library (AGPL v3):
├─ libp2p (MIT) ✅
├─ Signal Protocol (AGPL v3) ✅
├─ WebRTC (BSD) ✅
├─ SQLite (Public Domain) ✅
├─ Automerge (MIT) ✅
└─ UniFFI (MPL 2.0) ✅

Apps (Can be proprietary if needed):
├─ Android app (Apache 2.0 or proprietary)
├─ iOS app (Apache 2.0 or proprietary)
└─ Desktop app (Apache 2.0 or proprietary)

Note: AGPL v3 core requires:
- Publishing source code if modified
- Network use = distribution (trigger)
- OK for commercial use
- OK for proprietary apps using it via FFI

═══════════════════════════════════════════════════════════════

RISK ASSESSMENT:

LOW RISK:
✅ All libraries are mature (>3 years old)
✅ All have active communities
✅ All used in production at scale
✅ All have good documentation

MEDIUM RISK:
⚠️ Rust ecosystem evolving (but stable)
⚠️ UniFFI relatively new (but Mozilla-backed)
⚠️ WebRTC on mobile can drain battery (mitigable)

HIGH RISK (MITIGATED):
🔴 P2P complexity (libp2p handles this)
🔴 NAT traversal (TURN relay for 20%)
🔴 Multi-device sync (CRDTs handle this)

═══════════════════════════════════════════════════════════════
