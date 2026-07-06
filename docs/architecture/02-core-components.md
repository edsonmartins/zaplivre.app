# Core Library Components

## Overview

The `zaplivre-core` library is written in Rust and provides the foundational functionality for all ZapLivre applications (Android, iOS, Desktop). It exposes a unified API through FFI bindings (UniFFI).

## Module Structure

```
core/src/
├── lib.rs              # Public API exports
├── identity/           # Keypairs & Identity Management
├── crypto/             # E2E Encryption (Signal Protocol)
├── network/            # P2P Networking (libp2p)
├── storage/            # Local Persistence (SQLite)
├── sync/               # Multi-device Sync (CRDTs)
├── voip/               # Voice/Video Calls (WebRTC)
├── protocol/           # Message Protocol (Protobuf)
├── api/                # Client API & Events
└── utils/              # Utilities (logging, config, error)
```

---

## 1. Identity Module

**Purpose:** Manage user identity and cryptographic keypairs.

### Key Types:

```rust
pub struct Keypair {
    pub signing_key: SigningKey,    // Ed25519 for signatures
    pub identity_key: IdentityKey,   // X25519 for key agreement
}

pub struct Identity {
    pub peer_id: String,             // "zaplivre_[base58(pubkey)]"
    pub keypair: Keypair,
    pub prekeys: Vec<PreKey>,        // Pool of 100 one-time prekeys
}
```

### Responsibilities:

- Generate Ed25519 signing keypairs
- Generate X25519 identity and prekey pairs
- Derive unique Peer ID from public key
- Secure storage in device keychain/keystore
- Key rotation

### Storage:

- **Android**: EncryptedSharedPreferences + Android Keystore
- **iOS**: iOS Keychain
- **Desktop**: System keyring (keyring-rs)

---

## 2. Crypto Module

**Purpose:** End-to-end encryption using Signal Protocol.

### Components:

```rust
pub struct SignalSession {
    session_state: SessionState,
    ratchet: DoubleRatchet,
}

pub struct DoubleRatchet {
    root_key: RootKey,
    sending_chain: ChainKey,
    receiving_chain: ChainKey,
}
```

### Features:

- **X3DH**: Extended Triple Diffie-Hellman for initial key exchange
- **Double Ratchet**: Forward secrecy + break-in recovery
- **Sender Keys**: Efficient group encryption
- **Sealed Sender**: Hide sender identity in metadata

### Encryption Flow:

1. Alice fetches Bob's prekey bundle
2. X3DH generates shared secret
3. Initialize Double Ratchet with shared secret
4. Encrypt message: `encrypt(plaintext) → ciphertext`
5. Send encrypted payload over P2P or relay

### Libraries:

- `libsignal-protocol-rust` (official Signal implementation)
- `ed25519-dalek` (signing)
- `x25519-dalek` (key agreement)

---

## 3. Network Module

**Purpose:** P2P networking using libp2p.

### Transport Stack:

```
┌─────────────────────────────┐
│     Application Layer       │
│  (Messages, Calls, etc)     │
├─────────────────────────────┤
│    Protocols (GossipSub)    │
├─────────────────────────────┤
│    Multiplexing (Yamux)     │
├─────────────────────────────┤
│  Encryption (Noise XX)      │
├─────────────────────────────┤
│  Transport (TCP + QUIC)     │
└─────────────────────────────┘
```

### Key Components:

#### 3.1 Transport

```rust
pub struct ZapLivreTransport {
    tcp: TcpTransport,
    quic: QuicTransport,
    noise: NoiseConfig,
    yamux: YamuxConfig,
}
```

- **TCP**: Fallback transport
- **QUIC**: Primary transport (lower latency, built-in encryption)
- **Noise Protocol**: Authenticated encryption (XX pattern)
- **Yamux**: Stream multiplexing

#### 3.2 Discovery (DHT)

```rust
pub struct Discovery {
    kademlia: Kademlia,
    mdns: Mdns,           // Local network discovery
    bootstrap: Vec<Multiaddr>,
}
```

- **Kademlia DHT**: Distributed peer discovery
- **mDNS**: Local network discovery (same WiFi)
- **Bootstrap Nodes**: Initial entry points (3 public nodes)

#### 3.3 Routing

- **Circuit Relay v2**: Relay through intermediate peers
- **DCUtR**: Hole-punching for NAT traversal
- **AutoNAT**: Detect NAT type

#### 3.4 Messaging

- **RequestResponse**: 1:1 messages
- **GossipSub**: Group messages (pub/sub)
- **Identify**: Exchange peer info

---

## 4. Storage Module

**Purpose:** Local persistence using SQLite.

### Database Schema:

```sql
-- Messages
CREATE TABLE messages (
    id TEXT PRIMARY KEY,
    conversation_id TEXT NOT NULL,
    sender_peer_id TEXT NOT NULL,
    recipient_peer_id TEXT,
    content BLOB NOT NULL,        -- E2E encrypted
    message_type TEXT NOT NULL,
    timestamp INTEGER NOT NULL,
    status TEXT NOT NULL,         -- pending, sent, delivered, read
    is_incoming BOOLEAN NOT NULL
);

-- Conversations
CREATE TABLE conversations (
    id TEXT PRIMARY KEY,
    type TEXT NOT NULL,           -- direct, group
    name TEXT,
    participants TEXT NOT NULL,   -- JSON array
    last_message_id TEXT,
    unread_count INTEGER DEFAULT 0,
    updated_at INTEGER NOT NULL
);

-- Contacts
CREATE TABLE contacts (
    peer_id TEXT PRIMARY KEY,
    display_name TEXT,
    avatar_url TEXT,
    public_key BLOB NOT NULL,
    added_at INTEGER NOT NULL
);
```

### Features:

- **WAL Mode**: Write-Ahead Logging for concurrency
- **FTS5**: Full-text search for messages
- **Encryption at rest**: SQLCipher (optional)
- **Migrations**: Schema versioning

### API:

```rust
pub trait Storage {
    async fn save_message(&self, msg: Message) -> Result<()>;
    async fn get_conversation(&self, id: &str) -> Result<Conversation>;
    async fn search_messages(&self, query: &str) -> Result<Vec<Message>>;
}
```

---

## 5. Sync Module

**Purpose:** Multi-device synchronization using CRDTs.

### Technology:

- **Automerge**: CRDT library for JSON-like data
- **Conflict-free**: Multiple devices can edit simultaneously
- **Eventually consistent**: All devices converge to same state

### Sync Protocol:

1. Device A generates change
2. Change encoded as Automerge patch
3. Patch sent to all other devices (P2P or via sync server)
4. Each device applies patch to local state
5. Conflicts automatically resolved

### Data Synced:

- Message read receipts
- Conversation metadata
- Contact list
- User preferences

---

## 6. VoIP Module

**Purpose:** Voice and video calls using WebRTC.

### Components:

```rust
pub struct CallSession {
    peer_connection: PeerConnection,
    audio_track: AudioTrack,
    video_track: Option<VideoTrack>,
    ice_candidates: Vec<IceCandidate>,
}
```

### Call Flow:

1. **Signaling**: Exchange SDP offers/answers via libp2p
2. **ICE**: Gather candidates (STUN + TURN)
3. **Connection**: Establish WebRTC peer connection
4. **Media**: Stream audio/video

### Codecs:

- **Audio**: Opus (48kHz, adaptive bitrate)
- **Video**: H264/VP8 (adaptive resolution)

### Features:

- Echo cancellation
- Noise suppression
- Adaptive bitrate
- Simulcast (multiple quality streams)

---

## 7. Protocol Module

**Purpose:** Message serialization using Protocol Buffers.

### Message Types:

```protobuf
message Message {
  string id = 1;
  string sender = 2;
  string recipient = 3;
  int64 timestamp = 4;
  MessageContent content = 5;
}

message MessageContent {
  oneof content {
    TextContent text = 1;
    ImageContent image = 2;
    VideoContent video = 3;
    FileContent file = 4;
    CallContent call = 5;
  }
}
```

### Benefits:

- Compact binary format
- Forward/backward compatibility
- Language-agnostic

---

## 8. API Module

**Purpose:** Public client API exposed to applications.

### Main API:

```rust
pub struct Client {
    identity: Identity,
    network: Network,
    storage: Storage,
    crypto: CryptoManager,
}

impl Client {
    pub async fn send_text(&mut self, recipient: &str, text: String) -> Result<String>;
    pub async fn start_call(&mut self, recipient: &str) -> Result<CallSession>;
    pub async fn create_group(&mut self, name: String, participants: Vec<String>) -> Result<String>;
    pub fn on_event(&mut self, handler: EventHandler);
}
```

### Event System:

```rust
pub enum Event {
    MessageReceived(Message),
    MessageDelivered(String),  // message_id
    CallIncoming(CallInfo),
    ContactStatusChanged(String, Status),
}
```

---

## 9. Utils Module

### Logging:

- `tracing` crate for structured logging
- Log levels: TRACE, DEBUG, INFO, WARN, ERROR
- JSON output for production

### Configuration:

```rust
pub struct Config {
    pub data_dir: PathBuf,
    pub bootstrap_nodes: Vec<Multiaddr>,
    pub turn_servers: Vec<TurnServer>,
    pub enable_mdns: bool,
}
```

### Error Handling:

```rust
pub type Result<T> = std::result::Result<T, ZapLivreError>;

#[derive(Debug, thiserror::Error)]
pub enum ZapLivreError {
    #[error("Network error: {0}")]
    Network(String),

    #[error("Crypto error: {0}")]
    Crypto(String),

    #[error("Storage error: {0}")]
    Storage(#[from] rusqlite::Error),
}
```

---

## FFI Bindings (UniFFI)

### Definition:

```rust
// ffi/zaplivre.udl
namespace zaplivre {
    Client create_client(Config config);
};

interface Client {
    string send_text(string recipient, string text);
    void on_message_received(MessageCallback callback);
};

callback interface MessageCallback {
    void on_message(Message message);
};
```

### Generated Bindings:

- **Kotlin**: `com.zaplivre.bindings.ZapLivreCore`
- **Swift**: `import ZapLivreCore`

---

## Performance Considerations

- **Async/Await**: All I/O operations are async (tokio runtime)
- **Zero-copy**: Minimize allocations for large payloads
- **Connection pooling**: Reuse P2P connections
- **Lazy loading**: Load messages on-demand

---

**Next:** [Networking Details](03-networking.md)
