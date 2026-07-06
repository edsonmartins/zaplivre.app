# ZapLivre - Tech Stack Completo e Bibliotecas Consagradas

## 🎯 Filosofia: "Use o que já funciona"

**Princípio:** Não reinventar a roda. Usar bibliotecas open source maduras, bem mantidas, com comunidade ativa.

**Critérios de seleção:**
- ✅ Open source (MIT, Apache 2.0, ISC preferidos)
- ✅ Bem mantido (commits recentes, issues respondidas)
- ✅ Battle-tested (usado em produção por empresas grandes)
- ✅ Documentação clara
- ✅ Comunidade ativa
- ✅ Bindings multiplataforma (quando aplicável)

---

## 📱 VISÃO GERAL DA ARQUITETURA

```
┌─────────────────────────────────────────────────────────────┐
│                    ZapLivre Platform                          │
├─────────────────────────────────────────────────────────────┤
│                                                              │
│  Clients                                                     │
│  ┌──────────┐  ┌──────────┐  ┌──────────┐                 │
│  │ Android  │  │   iOS    │  │ Desktop  │                 │
│  │          │  │          │  │ (Tauri)  │                 │
│  │ Kotlin   │  │  Swift   │  │ Rust+Web │                 │
│  └────┬─────┘  └────┬─────┘  └────┬─────┘                 │
│       │             │             │                         │
│       └─────────────┴─────────────┘                         │
│                     │                                       │
│              ┌──────▼──────┐                                │
│              │             │                                │
│              │ ZapLivre Core│  ← Rust library (FFI)         │
│              │  (Rust)     │    Shared logic              │
│              │             │                                │
│              └──────┬──────┘                                │
│                     │                                       │
│       ┌─────────────┼─────────────┐                        │
│       │             │             │                         │
│  ┌────▼────┐  ┌────▼────┐  ┌────▼────┐                   │
│  │libp2p   │  │ Signal  │  │ WebRTC  │                   │
│  │(Network)│  │(Crypto) │  │ (VoIP)  │                   │
│  └─────────┘  └─────────┘  └─────────┘                   │
│                                                              │
├─────────────────────────────────────────────────────────────┤
│                                                              │
│  Backend Services (Self-hosted)                             │
│  ┌────────────┐  ┌────────────┐  ┌────────────┐          │
│  │ Bootstrap  │  │  Message   │  │    TURN    │          │
│  │  Nodes     │  │   Store    │  │   Relay    │          │
│  │  (Rust)    │  │(PostgreSQL)│  │  (coturn)  │          │
│  └────────────┘  └────────────┘  └────────────┘          │
│                                                              │
└─────────────────────────────────────────────────────────────┘
```

---

## 🦀 CORE LIBRARY (Rust) - O Coração do Sistema

### Por que Rust?
- ✅ Performance nativa (C/C++ level)
- ✅ Memory safe (sem segfaults, use-after-free)
- ✅ Excelente para networking/crypto
- ✅ FFI fácil (C ABI) → funciona em Android/iOS/Desktop
- ✅ Ecossistema maduro de bibliotecas P2P/crypto
- ✅ Usado por: Discord, Cloudflare, AWS, Meta

### Estrutura do Core

```
zaplivre-core/
├── Cargo.toml
├── src/
│   ├── lib.rs
│   ├── network/      # libp2p
│   ├── crypto/       # Signal Protocol
│   ├── storage/      # SQLite
│   ├── voip/         # WebRTC
│   ├── sync/         # CRDTs
│   └── ffi/          # Bindings (uniffi)
```

---

## 🌐 1. NETWORKING P2P

### ⭐ libp2p (ESCOLHA PRINCIPAL)

**O que é:**
Biblioteca modular para redes P2P, criada pelo Protocol Labs (IPFS, Filecoin)

**Usado por:**
- IPFS (InterPlanetary File System)
- Polkadot (blockchain)
- Ethereum 2.0 (beacon chain)
- Substrate (blockchain framework)

**Linguagem:** Rust (rust-libp2p)
**Licença:** MIT
**Manutenção:** ⭐⭐⭐⭐⭐ (Protocol Labs, >100 contributors)

**Módulos que vamos usar:**

```toml
[dependencies]
libp2p = "0.53"
# Específicos:
libp2p-tcp = "0.41"          # TCP transport
libp2p-quic = "0.10"         # QUIC transport (melhor que TCP)
libp2p-noise = "0.44"        # Encryption (handshake)
libp2p-yamux = "0.45"        # Multiplexing
libp2p-kad = "0.45"          # Kademlia DHT (peer discovery)
libp2p-gossipsub = "0.46"    # Pub/sub (group messages)
libp2p-relay = "0.17"        # Circuit relay (NAT traversal)
libp2p-dcutr = "0.11"        # DCUtR (hole punching)
libp2p-identify = "0.44"     # Peer identification
```

**Por que libp2p:**
- ✅ Protocolo modular e extensível
- ✅ NAT traversal embutido (relay + DCUtR)
- ✅ DHT para peer discovery (Kademlia)
- ✅ Pub/sub para mensagens em grupo (GossipSub)
- ✅ Múltiplos transports (TCP, QUIC, WebSocket)
- ✅ Usado em produção por projetos grandes

**Alternativas consideradas:**
- ❌ Roll our own TCP/UDP → muita complexidade, bugs
- ⚠️ ZeroMQ → não é P2P puro, precisa broker
- ⚠️ nanomsg → menos features que libp2p

**Código exemplo:**

```rust
use libp2p::{
    identity, noise, tcp, yamux, PeerId, Transport,
    swarm::{NetworkBehaviour, SwarmBuilder},
};

// 1. Criar identity (keypair)
let local_key = identity::Keypair::generate_ed25519();
let local_peer_id = PeerId::from(local_key.public());

// 2. Setup transport (TCP + QUIC)
let transport = tcp::tokio::Transport::default()
    .upgrade(upgrade::Version::V1)
    .authenticate(noise::Config::new(&local_key)?)
    .multiplex(yamux::Config::default())
    .boxed();

// 3. Create swarm com behaviours
#[derive(NetworkBehaviour)]
struct ZapLivreBehaviour {
    kademlia: kad::Behaviour<MemoryStore>,
    gossipsub: gossipsub::Behaviour,
    relay: relay::client::Behaviour,
}

let swarm = SwarmBuilder::with_tokio_executor(
    transport,
    behaviour,
    local_peer_id
).build();

// 4. Listen e dial
swarm.listen_on("/ip4/0.0.0.0/tcp/0".parse()?)?;
swarm.dial(bootstrap_peer_addr)?;
```

---

## 🔐 2. CRIPTOGRAFIA E2E

### ⭐ Signal Protocol (Double Ratchet)

**O que é:**
Protocolo de criptografia E2E com forward secrecy e break-in recovery

**Usado por:**
- Signal (óbvio)
- WhatsApp
- Facebook Messenger (secret conversations)
- Google Messages (RCS)
- Skype (private conversations)

**Implementação Rust:** `libsignal-client`
**Licença:** AGPL v3
**Manutenção:** ⭐⭐⭐⭐⭐ (Signal Foundation)

```toml
[dependencies]
libsignal-client = "0.40"
```

**Componentes:**

```rust
use libsignal_protocol::{
    IdentityKeyStore, PreKeyStore, SignedPreKeyStore,
    SessionStore, SenderKeyStore,
    process_prekey_bundle, encrypt, decrypt,
};

// 1. Gerar identity key (por device)
let identity_keypair = KeyPair::generate(&mut rng);

// 2. Gerar prekeys (batch de 100)
let prekeys: Vec<PreKeyRecord> = (0..100)
    .map(|id| PreKeyRecord::new(PreKeyId::from(id), &mut rng))
    .collect();

// 3. Iniciar sessão (X3DH)
let session = process_prekey_bundle(
    &remote_address,
    &remote_registration_id,
    &remote_identity_key,
    &remote_prekey,
    &remote_signed_prekey,
    &remote_signature,
    &mut identity_store,
    &mut session_store,
)?;

// 4. Encrypt message
let ciphertext = encrypt(&plaintext, &remote_address, &session_store)?;

// 5. Decrypt message
let plaintext = decrypt(&ciphertext, &remote_address, &session_store)?;
```

**Por que Signal Protocol:**
- ✅ **Forward secrecy:** Chaves antigas não comprometem mensagens novas
- ✅ **Break-in recovery:** Se chave vaza, próxima mensagem restaura segurança
- ✅ **Async messaging:** Funciona offline (prekeys)
- ✅ **Battle-tested:** Bilhões de mensagens/dia no WhatsApp

**Grupos:** Sender Keys

```rust
use libsignal_protocol::group_cipher;

// Sender distribui group key via Signal Protocol 1:1
let group_cipher = GroupCipher::new(&group_store);
let ciphertext = group_cipher.encrypt(&plaintext)?;

// Receivers decriptam com mesma chave
let plaintext = group_cipher.decrypt(&ciphertext)?;
```

**Alternativa (mais simples):** MLS (Message Layer Security)

```toml
[dependencies]
openmls = "0.5"
```

- ✅ IETF standard (RFC 9420)
- ✅ Melhor para grupos grandes (>100 pessoas)
- ✅ Mais eficiente que Signal Protocol groups
- ⚠️ Mais novo, menos battle-tested
- **Recomendação:** Signal Protocol para MVP, MLS para futuro

---

## 🗄️ 3. STORAGE LOCAL

### ⭐ SQLite + rusqlite

**O que é:**
Database SQL embutida, serverless, single-file

**Usado por:**
- Literalmente TUDO: Android, iOS, Chrome, Firefox, Python
- WhatsApp, Telegram, Discord (apps desktop)
- >1 trillion databases em uso (estimativa)

**Rust binding:** `rusqlite`
**Licença:** Public Domain
**Manutenção:** ⭐⭐⭐⭐⭐ (SQLite + rusqlite muito ativo)

```toml
[dependencies]
rusqlite = { version = "0.31", features = ["bundled"] }
```

**Schema exemplo:**

```sql
-- messages.sql
CREATE TABLE messages (
    id TEXT PRIMARY KEY,
    conversation_id TEXT NOT NULL,
    sender_id TEXT NOT NULL,
    content BLOB NOT NULL,  -- encrypted
    timestamp INTEGER NOT NULL,
    delivered BOOLEAN DEFAULT 0,
    read BOOLEAN DEFAULT 0,
    FOREIGN KEY (conversation_id) REFERENCES conversations(id)
);

CREATE INDEX idx_messages_conversation 
ON messages(conversation_id, timestamp);

-- conversations.sql
CREATE TABLE conversations (
    id TEXT PRIMARY KEY,
    type TEXT NOT NULL,  -- 'direct' or 'group'
    name TEXT,
    last_message_timestamp INTEGER,
    unread_count INTEGER DEFAULT 0
);

-- contacts.sql
CREATE TABLE contacts (
    peer_id TEXT PRIMARY KEY,
    display_name TEXT,
    public_key BLOB NOT NULL,
    last_seen INTEGER
);
```

**Código Rust:**

```rust
use rusqlite::{Connection, params};

pub struct Database {
    conn: Connection,
}

impl Database {
    pub fn new(path: &str) -> Result<Self> {
        let conn = Connection::open(path)?;
        
        // Migrations
        conn.execute_batch(include_str!("../sql/schema.sql"))?;
        
        Ok(Self { conn })
    }
    
    pub fn insert_message(&self, msg: &Message) -> Result<()> {
        self.conn.execute(
            "INSERT INTO messages (id, conversation_id, sender_id, content, timestamp)
             VALUES (?1, ?2, ?3, ?4, ?5)",
            params![
                msg.id,
                msg.conversation_id,
                msg.sender_id,
                msg.content,
                msg.timestamp
            ],
        )?;
        Ok(())
    }
    
    pub fn get_conversation_messages(
        &self,
        conv_id: &str,
        limit: u32
    ) -> Result<Vec<Message>> {
        let mut stmt = self.conn.prepare(
            "SELECT id, conversation_id, sender_id, content, timestamp
             FROM messages
             WHERE conversation_id = ?1
             ORDER BY timestamp DESC
             LIMIT ?2"
        )?;
        
        let messages = stmt.query_map(params![conv_id, limit], |row| {
            Ok(Message {
                id: row.get(0)?,
                conversation_id: row.get(1)?,
                sender_id: row.get(2)?,
                content: row.get(3)?,
                timestamp: row.get(4)?,
            })
        })?
        .collect::<Result<Vec<_>, _>>()?;
        
        Ok(messages)
    }
}
```

**Features importantes:**

- ✅ **WAL mode** (Write-Ahead Logging): melhor concorrência
- ✅ **FTS5** (Full-Text Search): busca em mensagens
- ✅ **Encryption:** SQLCipher (opcional, para dados sensíveis)

```rust
// WAL mode
conn.execute_batch("PRAGMA journal_mode=WAL;")?;

// Full-text search
conn.execute_batch(r#"
    CREATE VIRTUAL TABLE messages_fts USING fts5(
        content,
        content='messages',
        content_rowid='id'
    );
"#)?;
```

**Alternativas:**
- ⚠️ sled: Pure Rust DB, mas menos maduro que SQLite
- ⚠️ RocksDB: Muito bom, mas overkill para chat
- ❌ PostgreSQL/MySQL: Precisa servidor, não é embedded

---

## 🔄 4. SINCRONIZAÇÃO MULTI-DEVICE

### ⭐ CRDTs (Conflict-free Replicated Data Types)

**Problema:** Usuário tem múltiplos devices (celular, desktop, tablet). Como sincronizar mensagens sem conflitos?

**Solução:** CRDTs - estruturas de dados que convergem automaticamente

**Implementação Rust:** `automerge`

```toml
[dependencies]
automerge = "0.5"
```

**O que é Automerge:**
- CRDT library criada pelo Ink & Switch
- Usado por: Figma (colaboração), Notion (sync)
- JSON-like data structure que auto-sincroniza

**Exemplo:**

```rust
use automerge::{Automerge, transaction::Transactable};

// Device 1: Cria documento
let mut doc1 = Automerge::new();
let mut tx = doc1.transaction();
tx.put(automerge::ROOT, "messages", vec![])?;
tx.commit();

// Sync para Device 2
let changes = doc1.get_changes(&[]);
let mut doc2 = Automerge::new();
doc2.apply_changes(changes)?;

// Device 1: Adiciona mensagem
let mut tx = doc1.transaction();
let messages = tx.get(automerge::ROOT, "messages")?;
tx.insert(&messages, 0, "Hello")?;
tx.commit();

// Device 2: Adiciona mensagem diferente (concorrente!)
let mut tx = doc2.transaction();
let messages = tx.get(automerge::ROOT, "messages")?;
tx.insert(&messages, 0, "World")?;
tx.commit();

// Merge! Sem conflitos
doc1.merge(&mut doc2)?;
doc2.merge(&mut doc1)?;

// Ambos devices agora têm: ["World", "Hello"]
// Ordem determinística baseada em timestamps/IDs
```

**Por que CRDTs:**
- ✅ Sem servidor de truth único
- ✅ Funciona offline
- ✅ Merge automático sem conflitos
- ✅ Cada device é um peer igual

**Alternativa:** Operational Transformation (OT)
- Usado por Google Docs
- Mais complexo de implementar
- Requer servidor central (não é P2P puro)

---

## 📞 5. VOIP / CHAMADAS

### ⭐ WebRTC

**Implementações:**

#### Rust: webrtc-rs

```toml
[dependencies]
webrtc = "0.9"
tokio = { version = "1", features = ["full"] }
```

**Exemplo:**

```rust
use webrtc::api::APIBuilder;
use webrtc::peer_connection::RTCPeerConnection;

// 1. Create API
let api = APIBuilder::new().build();

// 2. Create PeerConnection
let config = RTCConfiguration {
    ice_servers: vec![RTCIceServer {
        urls: vec!["stun:stun.zaplivre.app:3478".to_string()],
        ..Default::default()
    }],
    ..Default::default()
};

let peer_connection = api.new_peer_connection(config).await?;

// 3. Add audio track
let (track, _) = api.new_track(
    PayloadType::try_from(111)?,
    rand::random(),
    "audio".to_string(),
    "zaplivre-audio".to_string()
)?;

peer_connection.add_track(Arc::clone(&track)).await?;

// 4. Create offer
let offer = peer_connection.create_offer(None).await?;
peer_connection.set_local_description(offer).await?;

// 5. Exchange SDP via signaling server
// ...

// 6. Handle ICE candidates
peer_connection.on_ice_candidate(Box::new(move |candidate| {
    // Send to remote peer via signaling
    Box::pin(async move {
        send_ice_candidate(candidate).await;
    })
})).await;
```

#### JavaScript (Tauri Desktop):

```javascript
// Mais simples, mais maduro
const peerConnection = new RTCPeerConnection({
    iceServers: [{ urls: 'stun:stun.zaplivre.app:3478' }]
});

// Add local stream
const stream = await navigator.mediaDevices.getUserMedia({ audio: true });
stream.getTracks().forEach(track => {
    peerConnection.addTrack(track, stream);
});

// Create offer
const offer = await peerConnection.createOffer();
await peerConnection.setLocalDescription(offer);
```

**Recomendação:**
- ✅ **Desktop:** JavaScript WebRTC API (Tauri)
- ✅ **Mobile:** Native SDKs (melhor battery/performance)

#### Android: WebRTC Native SDK

```kotlin
// build.gradle.kts
dependencies {
    implementation("io.getstream:stream-webrtc-android:1.1.1")
}
```

#### iOS: WebRTC Framework

```swift
import WebRTC

let factory = RTCPeerConnectionFactory()
let config = RTCConfiguration()
config.iceServers = [RTCIceServer(urlStrings: ["stun:stun.zaplivre.app:3478"])]

let peerConnection = factory.peerConnection(
    with: config,
    constraints: RTCMediaConstraints(),
    delegate: self
)
```

### Codecs (Áudio):

**Opus** (OBRIGATÓRIO)

```toml
[dependencies]
opus = "0.3"
```

- ✅ Open source (royalty-free)
- ✅ Melhor qualidade/bitrate do mercado
- ✅ 6-510 kbps adaptativos
- ✅ Latência ultra-baixa (<20ms)
- ✅ Usado por: Discord, Zoom, WhatsApp

### TURN/STUN Server:

**coturn** (já mencionado)

```bash
# Docker
docker run -d \
  --network=host \
  -v /etc/coturn/turnserver.conf:/etc/coturn/turnserver.conf \
  coturn/coturn
```

---

## 📲 6. PUSH NOTIFICATIONS

### Android: UnifiedPush

**O que é:** 
Alternativa open source ao Google FCM
Usuário escolhe provider (ntfy, NextPush, etc)

```toml
[dependencies]
# No core, apenas HTTP client para enviar
reqwest = "0.11"
```

**Ou:** FCM (Firebase Cloud Messaging)
- ✅ Mais confiável
- ⚠️ Requer Google Play Services
- ⚠️ Dependência do Google

```kotlin
// Android (Kotlin)
dependencies {
    implementation("com.google.firebase:firebase-messaging:23.4.0")
}
```

### iOS: APNs (Apple Push Notification Service)

**Obrigatório para iOS**, não tem alternativa

```rust
// Rust client para enviar notificações
[dependencies]
a2 = "0.9"  // APNs client
```

---

## 🖥️ 7. UI FRAMEWORKS

### Android: Jetpack Compose

```kotlin
// build.gradle.kts
android {
    buildFeatures {
        compose = true
    }
    composeOptions {
        kotlinCompilerExtensionVersion = "1.5.8"
    }
}

dependencies {
    // Compose
    implementation("androidx.compose.ui:ui:1.6.0")
    implementation("androidx.compose.material3:material3:1.2.0")
    implementation("androidx.compose.ui:ui-tooling-preview:1.6.0")
    
    // Navigation
    implementation("androidx.navigation:navigation-compose:2.7.6")
    
    // ViewModel
    implementation("androidx.lifecycle:lifecycle-viewmodel-compose:2.7.0")
}
```

**Por que Compose:**
- ✅ Moderno, declarativo (como React/SwiftUI)
- ✅ Menos boilerplate que Views
- ✅ Hot reload
- ✅ Material Design 3 nativo

### iOS: SwiftUI

```swift
import SwiftUI

struct ConversationView: View {
    @StateObject var viewModel: ConversationViewModel
    
    var body: some View {
        VStack {
            ScrollView {
                LazyVStack {
                    ForEach(viewModel.messages) { message in
                        MessageRow(message: message)
                    }
                }
            }
            
            MessageInput(onSend: viewModel.sendMessage)
        }
        .navigationTitle("Conversa")
    }
}
```

### Desktop: Tauri 2.0

```toml
[dependencies]
tauri = "2.0"
```

**Frontend:** React ou Vue ou Svelte

```json
// package.json
{
  "dependencies": {
    "react": "^18.2.0",
    "react-dom": "^18.2.0",
    "@tauri-apps/api": "^2.0.0"
  }
}
```

**Por que Tauri:**
- ✅ Bundle pequeno (~3MB vs Electron ~100MB)
- ✅ Menos RAM (usa webview do sistema)
- ✅ Rust backend (integra com core)
- ✅ Hot reload
- ✅ Multi-platform (Windows, Mac, Linux)

---

## 🔗 8. FFI (BINDINGS MULTIPLATAFORMA)

### ⭐ UniFFI

**O que é:**
Gerador automático de bindings Rust → Kotlin/Swift/Python

**Criado por:** Mozilla
**Usado por:** Firefox, Glean SDK

```toml
[dependencies]
uniffi = "0.27"

[build-dependencies]
uniffi = { version = "0.27", features = ["build"] }
```

**Interface Definition (UDL):**

```idl
// zaplivre.udl
namespace zaplivre {
    ZapLivreClient create_client(string user_id);
};

interface ZapLivreClient {
    constructor(string user_id);
    
    [Throws=ZapLivreError]
    void send_message(string recipient, string text);
    
    sequence<Message> get_messages(string conversation_id);
    
    [Throws=ZapLivreError]
    void start_call(string recipient);
};

dictionary Message {
    string id;
    string sender;
    string content;
    u64 timestamp;
};

[Error]
enum ZapLivreError {
    "NetworkError",
    "CryptoError",
    "StorageError",
};
```

**Gera automaticamente:**

```kotlin
// Android (Kotlin) - gerado
val client = createClient("user123")
client.sendMessage("bob", "Hello!")
val messages = client.getMessages("conv456")
```

```swift
// iOS (Swift) - gerado
let client = createClient(userId: "user123")
try client.sendMessage(recipient: "bob", text: "Hello!")
let messages = client.getMessages(conversationId: "conv456")
```

**Por que UniFFI:**
- ✅ Mantém tipos sincronizados (Rust ↔ Kotlin ↔ Swift)
- ✅ Menos erro humano
- ✅ Integração fácil

**Alternativa:** `cbindgen` + manual JNI/C interop
- ⚠️ Muito mais trabalho manual
- ⚠️ Propenso a erros

---

## 🐳 9. BACKEND / INFRASTRUCTURE

### Bootstrap Nodes: Rust + libp2p

```rust
// Mesmo código do core, só roda 24/7
use libp2p::*;

#[tokio::main]
async fn main() -> Result<()> {
    let mut swarm = create_swarm()?;
    
    // Listen on multiple transports
    swarm.listen_on("/ip4/0.0.0.0/tcp/4001".parse()?)?;
    swarm.listen_on("/ip4/0.0.0.0/udp/4001/quic-v1".parse()?)?;
    
    // Event loop
    loop {
        match swarm.select_next_some().await {
            SwarmEvent::NewListenAddr { address, .. } => {
                println!("Listening on {}", address);
            }
            SwarmEvent::Behaviour(event) => {
                handle_event(event);
            }
            _ => {}
        }
    }
}
```

### Message Store: PostgreSQL + Redis

**PostgreSQL:**
- ✅ Store-and-forward messages (TTL 14 dias)
- ✅ Battle-tested, muito confiável
- ✅ Rust client: `tokio-postgres`

```toml
[dependencies]
tokio-postgres = "0.7"
deadpool-postgres = "0.12"  # Connection pooling
```

```sql
CREATE TABLE offline_messages (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    recipient_id TEXT NOT NULL,
    sender_id TEXT NOT NULL,
    encrypted_content BYTEA NOT NULL,
    created_at TIMESTAMP DEFAULT NOW(),
    expires_at TIMESTAMP DEFAULT NOW() + INTERVAL '14 days'
);

CREATE INDEX idx_offline_messages_recipient 
ON offline_messages(recipient_id, created_at);
```

**Redis:**
- ✅ Presence (online/offline)
- ✅ Rate limiting
- ✅ Cache

```toml
[dependencies]
redis = { version = "0.24", features = ["tokio-comp", "connection-manager"] }
```

```rust
// Presence
redis::cmd("SETEX")
    .arg(format!("presence:{}", user_id))
    .arg(300)  // 5 minutos
    .arg("online")
    .query_async(&mut conn)
    .await?;
```

### TURN Server: coturn

Já coberto anteriormente. Open source, battle-tested.

---

## 📦 10. DEPENDENCY MANAGEMENT

### Cargo.toml (Core library)

```toml
[package]
name = "zaplivre-core"
version = "0.1.0"
edition = "2021"

[dependencies]
# Networking
libp2p = { version = "0.53", features = ["tcp", "quic", "noise", "yamux", "kad", "gossipsub", "relay", "dcutr"] }
tokio = { version = "1", features = ["full"] }

# Crypto
libsignal-protocol = "0.1"
libsignal-client = "0.40"
rand = "0.8"
sha2 = "0.10"

# Storage
rusqlite = { version = "0.31", features = ["bundled"] }
automerge = "0.5"

# VoIP
webrtc = "0.9"
opus = "0.3"

# Serialization
serde = { version = "1", features = ["derive"] }
serde_json = "1"
bincode = "1"

# FFI
uniffi = "0.27"

# Utilities
anyhow = "1"
thiserror = "1"
log = "0.4"
tracing = "0.1"
uuid = { version = "1", features = ["v4", "serde"] }

[build-dependencies]
uniffi = { version = "0.27", features = ["build"] }
```

---

## 🧪 11. TESTING & QUALITY

### Unit Tests

```rust
#[cfg(test)]
mod tests {
    use super::*;
    
    #[tokio::test]
    async fn test_send_message() {
        let client = ZapLivreClient::new("alice").await.unwrap();
        client.send_message("bob", "test").await.unwrap();
        // assertions
    }
}
```

### Integration Tests

```toml
[dev-dependencies]
tokio-test = "0.4"
mockall = "0.12"  # Mocking
proptest = "1"     # Property-based testing
```

### Benchmarks

```toml
[dev-dependencies]
criterion = "0.5"

[[bench]]
name = "crypto_bench"
harness = false
```

```rust
use criterion::{black_box, criterion_group, criterion_main, Criterion};

fn encrypt_benchmark(c: &mut Criterion) {
    c.bench_function("encrypt_message", |b| {
        b.iter(|| {
            encrypt(black_box(&message))
        });
    });
}

criterion_group!(benches, encrypt_benchmark);
criterion_main!(benches);
```

---

## 🔧 12. BUILD & CI/CD

### GitHub Actions

```yaml
# .github/workflows/ci.yml
name: CI

on: [push, pull_request]

jobs:
  test:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4
      - uses: actions-rs/toolchain@v1
        with:
          toolchain: stable
      
      - name: Build
        run: cargo build --verbose
      
      - name: Test
        run: cargo test --verbose
      
      - name: Clippy
        run: cargo clippy -- -D warnings
      
      - name: Format check
        run: cargo fmt -- --check
  
  build-android:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4
      - uses: actions-rs/toolchain@v1
        with:
          toolchain: stable
          target: aarch64-linux-android
      
      - name: Build Android
        run: cargo build --target aarch64-linux-android
```

---

## 📊 COMPARAÇÃO COM CONCORRENTES

| Feature | ZapLivre | WhatsApp | Telegram | Signal |
|---------|---------|----------|----------|--------|
| **E2E Crypto** | Signal Protocol | Signal Protocol | MTProto | Signal Protocol |
| **Architecture** | P2P (libp2p) | Centralized | Centralized | Centralized |
| **VoIP** | WebRTC | WebRTC | WebRTC | WebRTC |
| **Storage** | SQLite | SQLite | SQLite | SQLite |
| **Open Source** | ✅ AGPL v3 | ❌ | ⚠️ Clients only | ✅ GPL v3 |
| **Self-hosting** | ✅ | ❌ | ❌ | ⚠️ Servers |

**Vantagens técnicas do ZapLivre:**
- ✅ P2P reduz custos de servidor (80% tráfego P2P)
- ✅ Sem ponto único de falha
- ✅ Funciona mesmo se servidores caírem
- ✅ Open source completo (apps + infra)

---

## 🎯 STACK FINAL RECOMENDADO

### Core (Rust)
```
libp2p (networking)
+ Signal Protocol (crypto)
+ WebRTC (voip)
+ SQLite (storage)
+ Automerge (sync)
+ UniFFI (ffi)
= zaplivre-core.so/.dylib/.dll
```

### Android (Kotlin)
```
Jetpack Compose (UI)
+ Kotlin Coroutines (async)
+ zaplivre-core (FFI)
+ FCM ou UnifiedPush (notifications)
= ZapLivre.apk
```

### iOS (Swift)
```
SwiftUI (UI)
+ Combine (reactive)
+ zaplivre-core (FFI)
+ APNs (notifications)
= ZapLivre.app
```

### Desktop (Tauri)
```
React/Vue (UI)
+ Tauri 2.0 (framework)
+ zaplivre-core (FFI)
+ JavaScript WebRTC (calls)
= ZapLivre.exe/.dmg/.appimage
```

### Backend (Rust)
```
libp2p (bootstrap nodes)
+ PostgreSQL (message store)
+ Redis (presence/cache)
+ coturn (TURN relay)
= Self-hosted infrastructure
```

---

## 📚 REFERÊNCIAS E DOCS

### Tutoriais Essenciais

**libp2p:**
- https://docs.libp2p.io/
- https://github.com/libp2p/rust-libp2p/tree/master/examples

**Signal Protocol:**
- https://signal.org/docs/
- https://github.com/signalapp/libsignal

**WebRTC:**
- https://webrtc.org/
- https://github.com/webrtc-rs/webrtc

**UniFFI:**
- https://mozilla.github.io/uniffi-rs/
- https://github.com/mozilla/uniffi-rs

**Tauri:**
- https://tauri.app/v2/
- https://github.com/tauri-apps/tauri

---

## ✅ CHECKLIST DE VALIDAÇÃO

Antes de começar desenvolvimento, validar:

- [ ] Todas as bibliotecas são open source? (preferencialmente MIT/Apache 2.0)
- [ ] Todas estão sendo mantidas ativamente? (commits nos últimos 3 meses)
- [ ] Todas têm documentação adequada?
- [ ] Todas têm comunidade ativa? (issues respondidas, Discord/forum)
- [ ] Todas são usadas em produção por projetos grandes?
- [ ] Todas têm testes adequados?
- [ ] Licenças são compatíveis entre si?
- [ ] Bindings funcionam em todas plataformas target (Android/iOS/Desktop)?

---

## 🎯 PRÓXIMOS PASSOS

1. **Proof of Concept (1 semana):**
   - [ ] Setup projeto Rust com libp2p
   - [ ] 2 peers conectam e trocam mensagem plaintext
   - [ ] Validar que libp2p funciona

2. **Crypto PoC (1 semana):**
   - [ ] Integrar Signal Protocol
   - [ ] 2 peers trocam mensagem E2E encrypted
   - [ ] Validar session management

3. **Storage PoC (3 dias):**
   - [ ] Setup SQLite
   - [ ] Salvar mensagens localmente
   - [ ] Query e display

4. **FFI PoC (1 semana):**
   - [ ] UniFFI setup
   - [ ] Gerar bindings Kotlin/Swift
   - [ ] Chamar core de Android/iOS

5. **VoIP PoC (2 semanas):**
   - [ ] WebRTC setup
   - [ ] Chamada de voz 1:1
   - [ ] Validar qualidade

**Timeline total PoCs:** ~6 semanas

Só depois de todos PoCs validados → começar apps completos.

---

**Este é o tech stack completo.** 

**Tudo open source, tudo battle-tested, tudo pronto pra usar.** 🚀
