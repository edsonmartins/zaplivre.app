# FFI Implementation Documentation - ZapLivre Core

## Visão Geral

A camada FFI (Foreign Function Interface) do zaplivre-core utiliza **UniFFI 0.31** para gerar bindings automáticos para Kotlin (Android) e Swift (iOS/macOS). Esta implementação resolve desafios complexos de threading impostos pela arquitetura do libp2p.

## Arquitetura

### Desafio Principal: libp2p::Swarm é `!Send + !Sync`

O `libp2p::Swarm` foi projetado para rodar em uma única thread async e não pode ser compartilhado entre threads. Isso criou um conflito com os requisitos do UniFFI, que exige que tipos expostos via FFI sejam `Send + Sync`.

### Solução: Arquitetura Baseada em Channels

Em vez de expor o `Client` diretamente através do FFI, implementamos uma arquitetura de comunicação por channels:

```
┌─────────────────┐
│  Kotlin/Swift   │
│    Bindings     │
└────────┬────────┘
         │ FFI calls
         ▼
┌─────────────────┐
│ ZapLivreClient   │  (Send + Sync - apenas contém String)
│  (FFI wrapper)  │
└────────┬────────┘
         │ mpsc::channel
         ▼
┌─────────────────┐
│  ClientHandle   │  (Sender global)
└────────┬────────┘
         │ Commands
         ▼
┌─────────────────┐
│  Client Task    │  (!Send - roda em LocalSet dedicada)
│  (run_client_   │
│   task)         │
└─────────────────┘
         │
         ▼
┌─────────────────┐
│  Client (Core)  │  (contém NetworkManager + Database)
└─────────────────┘
```

### Componentes

#### 1. ZapLivreClient (src/ffi/client.rs)

Struct thread-safe que:
- **Não** contém `Arc<Client>` diretamente
- Apenas armazena `data_dir: String`
- Envia comandos via channels para o Client real

**Por que é Send + Sync:** Contém apenas tipos primitivos (String).

#### 2. ClientHandle

Handle global estático que mantém um `mpsc::UnboundedSender<ClientCommand>`:
- Inicializado uma única vez (OnceLock)
- Permite enviar comandos de qualquer thread
- O Client roda em uma thread dedicada com `LocalSet`

#### 3. ClientCommand (enum)

Define todos os comandos possíveis:
```rust
enum ClientCommand {
    LocalPeerId { response: oneshot::Sender<String> },
    SendTextMessage { to: PeerId, content: String, response: ... },
    ListConversations { response: ... },
    // ... outros comandos
}
```

Cada comando inclui um `oneshot::Sender` para receber a resposta.

#### 4. run_client_task

Task assíncrona que:
- Roda em uma `LocalSet` (permite futures !Send)
- Processa comandos do channel
- Executa operações no Client
- Envia respostas de volta via oneshot channels

#### 5. Runtime Global

```rust
static RUNTIME: OnceLock<Arc<tokio::runtime::Runtime>> = OnceLock::new();
```

Runtime tokio compartilhada para executar operações assíncronas:
- Métodos síncronos (FFI) usam `runtime().block_on()`
- Client task roda em thread dedicada com `LocalSet`

## Refatorações Necessárias

### Database Thread-Safe

Para resolver problemas de `!Sync` do rusqlite::Connection:

```rust
pub struct Database {
    conn: Arc<Mutex<Connection>>, // Era: Connection
}
```

**Mudanças:**
- Connection agora é thread-safe via Mutex
- Método `conn()` retorna `MutexGuard<Connection>`
- Todos os métodos fazem lock antes de acessar
- Testes atualizados para usar `with_connection()` helper

## Arquivos Principais

### UDL Definition (src/zaplivre.udl)

Define a interface FFI declarativamente:

```webidl
interface ZapLivreClient {
    [Throws=ZapLivreFfiError]
    constructor(string data_dir);

    [Throws=ZapLivreFfiError]
    string local_peer_id();

    [Throws=ZapLivreFfiError, Async]
    void listen_on(string multiaddr);

    [Throws=ZapLivreFfiError, Async]
    string send_text_message(string to_peer_id, string content);

    // ... outros métodos
};
```

### FFI Types (src/ffi/types.rs)

Tipos FFI-safe:
- `ZapLivreFfiError` - enum de erros com mensagens
- `FfiMessage` - mensagem com todos os campos serializáveis
- `FfiConversation` - conversação
- `MessageStatus` - enum de status

Conversões automáticas via `From` traits:
```rust
impl From<crate::storage::Message> for FfiMessage { ... }
impl From<crate::utils::error::ZapLivreError> for ZapLivreFfiError { ... }
```

### Client Implementation (src/ffi/client.rs)

Implementação completa com:
- Inicialização do Client em thread dedicada
- Envio de comandos via channels
- Métodos síncronos (`local_peer_id`, `list_conversations`)
- Métodos assíncronos (`send_text_message`, `bootstrap`)

### Bootstrap Peers (produção)

Por padrão, o FFI adiciona peers de bootstrap públicos do IPFS em `core/src/ffi/client.rs`.
Para usar seus próprios bootstraps (ex: VPS), substitua a lista por seus peers:

```rust
let bootstrap_peers = vec![
    ("/ip4/<PUBLIC_IP>/tcp/4001", "12D3KooW..."),
    ("/ip4/<PUBLIC_IP>/tcp/4002", "12D3KooW..."),
];

for (addr_str, peer_id_str) in bootstrap_peers {
    if let (Ok(addr), Ok(peer_id)) = (
        addr_str.parse::<libp2p::Multiaddr>(),
        peer_id_str.parse::<libp2p::PeerId>(),
    ) {
        builder = builder.add_bootstrap_peer(peer_id, addr);
    }
}
```

## Fluxo de Execução

### Exemplo: Enviar Mensagem

```
1. Kotlin chama: client.sendTextMessage("peer123", "Hello")
   │
   ▼
2. FFI binding converte para Rust
   │
   ▼
3. ZapLivreClient::send_text_message()
   │
   ▼
4. Cria oneshot channel para resposta
   │
   ▼
5. Envia ClientCommand::SendTextMessage via mpsc
   │
   ▼
6. run_client_task recebe comando
   │
   ▼
7. Executa client.send_text_message() (async)
   │
   ▼
8. Envia resultado via oneshot
   │
   ▼
9. ZapLivreClient recebe resultado (await)
   │
   ▼
10. Retorna para Kotlin via FFI
```

## Build System

### Cargo.toml

```toml
[dependencies]
uniffi = { workspace = true }

[build-dependencies]
uniffi = { workspace = true, features = ["build"] }
```

### build.rs

```rust
// Gera scaffolding do UDL
uniffi::generate_scaffolding("src/zaplivre.udl")
    .expect("Failed to generate UniFFI scaffolding");
```

### src/lib.rs

```rust
// Include scaffolding NO CRATE ROOT (não em submodule!)
uniffi::include_scaffolding!("zaplivre");

// Re-exportar tipos FFI no crate root
pub use ffi::{
    FfiConversation, FfiMessage, ZapLivreClient,
    ZapLivreFfiError, MessageStatus,
};
```

## Gerando Bindings

### Manualmente (Recomendado)

Use o exemplo em `examples/generate_bindings.rs` após habilitar feature `bindgen`:

```toml
# Em Cargo.toml [dev-dependencies]
uniffi = { workspace = true, features = ["bindgen"] }
```

Então:
```bash
cargo run --example generate_bindings
```

Isso gera:
- `target/bindings/zaplivre.kt` (Kotlin)
- `target/bindings/zaplivre.swift` (Swift)

### Via uniffi-bindgen (futuro)

Quando disponível para 0.31:
```bash
uniffi-bindgen generate src/zaplivre.udl --language kotlin --out-dir target/bindings
uniffi-bindgen generate src/zaplivre.udl --language swift --out-dir target/bindings
```

## Testes

### Teste de Compilação

```bash
cargo build           # Dev build
cargo build --release # Release build
cargo clippy          # Linter
cargo test            # Testes unitários
```

### Verificar Scaffolding Gerado

```bash
ls ../target/debug/build/zaplivre-core-*/out/zaplivre.uniffi.rs
```

## Limitações e Trade-offs

### Pros ✅

1. **Respects libp2p's threading model** - !Send Client roda em LocalSet dedicada
2. **Type-safe** - UniFFI garante type-safety entre Rust/Kotlin/Swift
3. **Automatic bindings** - Menos código boilerplate
4. **Error handling** - Conversões automáticas de erros
5. **Database thread-safe** - Mutex permite acesso concurrent

### Cons ⚠️

1. **Channel overhead** - Cada operação envolve:
   - Alocação de oneshot channel
   - Serialização de comando
   - Context switch para client task
   - Deserialização de resposta

2. **Complexity** - Arquitetura mais complexa que FFI direto

3. **Single client** - Apenas um Client global (via OnceLock)
   - Múltiplos ZapLivreClient compartilham mesmo Client
   - data_dir do primeiro new() é usado

4. **Lifet ime management** - Client vive até o fim do programa

## Próximos Passos

### FASE 5 - 100% COMPLETA ✅

- [x] UniFFI 0.31 configurado
- [x] UDL definitions criadas
- [x] Database thread-safe (Mutex<Connection>)
- [x] Arquitetura de channels implementada
- [x] Compilação bem-sucedida
- [x] Feature bindgen habilitada
- [x] Bindings Kotlin gerados (80KB - uniffi/zaplivre/zaplivre.kt)
- [x] Bindings Swift gerados (47KB - zaplivre.swift + 26KB - zaplivreFFI.h)
- [x] Cross-compilation Android configurada (NDK 26.3.11579264)
- [x] libzaplivre_core.so compilada para Android ARM64 (6.3MB)
- [x] Cross-compilation iOS configurada
- [x] libzaplivre_core.a compilada para iOS ARM64 Device (96MB)
- [x] libzaplivre_core.a compilada para iOS Simulator ARM64 (96MB)
- [x] libzaplivre_core.a compilada para iOS Simulator x86_64 (95MB)

### FASE 6 - Android App (próximo)

1. Integrar bindings Kotlin
2. Copiar `libzaplivre_core.so` para `jniLibs/`
3. Criar ZapLivreService (foreground service)
4. Implementar UI básica (Jetpack Compose)

### Melhorias Futuras

1. **Multiple clients** - Permitir múltiplas instâncias
2. **Callbacks** - Eventos do Client para UI (message_received, etc.)
3. **Connection pooling** - r2d2 pool para SQLite
4. **Graceful shutdown** - Desligar Client task cleanly
5. **Benchmarks** - Medir overhead de channels

## Referências

- [UniFFI Documentation](https://mozilla.github.io/uniffi-rs/)
- [libp2p Threading Model](https://docs.rs/libp2p-swarm/)
- [Tokio LocalSet](https://docs.rs/tokio/latest/tokio/task/struct.LocalSet.html)
- [Rusqlite Thread Safety](https://docs.rs/rusqlite/latest/rusqlite/struct.Connection.html)

---

**Última atualização:** 2025-01-20
**Status:** ✅ FASE 5: 100% COMPLETA - Todos bindings e bibliotecas nativas gerados com sucesso!
