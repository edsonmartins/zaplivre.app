# ZapLivre Identity Server

Identity Server para o sistema de mensagens ZapLivre. Gerencia o registro e lookup de @usernames mapeados para Peer IDs (libp2p).

## Arquitetura

- **Framework:** Axum (async Rust web framework)
- **Database:** PostgreSQL (armazenamento de usernames + prekey bundles)
- **Cache/Rate Limiting:** Redis
- **Crypto:** Ed25519 (verificação de assinaturas)

## API Endpoints

### POST /api/v1/register
Registra um novo username.

**Request:**
```json
{
  "username": "alice",
  "peer_id": "12D3KooW...",
  "public_key": "base64_encoded_ed25519_public_key",
  "prekey_bundle": {
    "identity_key": "base64_x25519_key",
    "signed_prekey_id": 1,
    "signed_prekey": "base64_x25519_key",
    "signed_prekey_signature": "base64_signature",
    "one_time_prekey": {
      "id": 1,
      "public_key": "base64_x25519_key"
    }
  },
  "signature": "base64_ed25519_signature",
  "timestamp": 1704067200
}
```

**Response (201 Created):**
```json
{
  "username": "alice",
  "peer_id": "12D3KooW...",
  "created_at": "2024-01-01T00:00:00Z"
}
```

**Errors:**
- `400 INVALID_USERNAME` - Username inválido (3-20 chars, lowercase alphanumeric + underscore)
- `409 USERNAME_TAKEN` - Username já registrado (retorna sugestões)
- `400 INVALID_SIGNATURE` - Assinatura inválida
- `429 RATE_LIMIT_EXCEEDED` - Limite de 5 registros/hora excedido

### GET /api/v1/lookup?username=alice
Busca informações de um username.

**Response (200 OK):**
```json
{
  "username": "alice",
  "peer_id": "12D3KooW...",
  "prekey_bundle": {
    "identity_key": "base64_x25519_key",
    "signed_prekey_id": 1,
    "signed_prekey": "base64_x25519_key",
    "signed_prekey_signature": "base64_signature",
    "one_time_prekey": {
      "id": 1,
      "public_key": "base64_x25519_key"
    }
  },
  "last_updated": "2024-01-01T00:00:00Z"
}
```

**Errors:**
- `404 USERNAME_NOT_FOUND` - Username não encontrado
- `429 RATE_LIMIT_EXCEEDED` - Limite de 100 lookups/hora excedido

### PUT /api/v1/prekeys
Atualiza os prekeys de um username (rotação de chaves).

**Request:**
```json
{
  "peer_id": "12D3KooW...",
  "prekey_bundle": {
    "identity_key": "base64_x25519_key",
    "signed_prekey_id": 2,
    "signed_prekey": "base64_x25519_key",
    "signed_prekey_signature": "base64_signature",
    "one_time_prekey": null
  },
  "signature": "base64_ed25519_signature",
  "timestamp": 1704067200
}
```

**Response (200 OK):**
```json
{
  "updated_at": "2024-01-01T00:10:00Z"
}
```

**Errors:**
- `404 USERNAME_NOT_FOUND` - Peer ID não encontrado
- `429 RATE_LIMIT_EXCEEDED` - Limite de 50 updates/hora excedido

### GET /health
Health check endpoint (sem rate limiting).

**Response (200 OK):**
```json
{
  "status": "healthy",
  "version": "0.1.0",
  "uptime_seconds": 12345,
  "database": {
    "status": "connected",
    "latency_ms": 1.23
  },
  "redis": {
    "status": "connected",
    "latency_ms": 0.45
  },
  "timestamp": "2024-01-01T00:00:00Z"
}
```

## Rate Limiting

Limites por IP:
- **Register:** 5 requests/hora
- **Lookup:** 100 requests/hora
- **Update Prekeys:** 50 requests/hora

Headers de resposta:
```
X-RateLimit-Limit: 100
X-RateLimit-Remaining: 95
```

## Validação de Username

- **Comprimento:** 3-20 caracteres
- **Formato:** Apenas lowercase (a-z), números (0-9) e underscore (_)
- **Regex:** `^[a-z0-9_]{3,20}$`
- **Exemplos válidos:** `alice`, `bob_123`, `user2024`
- **Exemplos inválidos:** `Al`, `Alice` (uppercase), `alice@` (caractere especial)

## Autenticação via Assinatura

Todos os requests de escrita (register, update prekeys) requerem assinatura Ed25519:

**Formato da mensagem assinada:**
```
register:{username}:{timestamp}
```

**Exemplo:**
```
register:alice:1704067200
```

**Validação:**
1. Timestamp deve estar dentro de ±5 minutos do horário atual
2. Assinatura verificada com a public_key fornecida
3. Public key deve ser Ed25519 (32 bytes)
4. Signature deve ser Ed25519 (64 bytes)

## Configuração

### Variáveis de Ambiente

Crie um arquivo `.env`:

```bash
# Database
DATABASE_URL=postgres://zaplivre:zaplivre@localhost/zaplivre_identity

# Redis
REDIS_URL=redis://localhost

# Server
BIND_ADDR=0.0.0.0:8080
```

### PostgreSQL Schema

```sql
CREATE TABLE usernames (
    id SERIAL PRIMARY KEY,
    username VARCHAR(20) UNIQUE NOT NULL,
    peer_id TEXT NOT NULL,
    public_key BYTEA NOT NULL,
    prekey_bundle JSONB NOT NULL,
    created_at TIMESTAMP NOT NULL DEFAULT NOW(),
    last_updated TIMESTAMP NOT NULL DEFAULT NOW()
);

CREATE INDEX idx_username ON usernames(username);
CREATE INDEX idx_peer_id ON usernames(peer_id);
```

## Desenvolvimento

### Requisitos

- Rust 1.92+
- PostgreSQL 15+
- Redis 7+

### Setup Local

1. Instalar PostgreSQL e Redis:
```bash
# macOS
brew install postgresql@15 redis

# Ubuntu
sudo apt install postgresql-15 redis-server
```

2. Criar database:
```bash
createdb zaplivre_identity
psql zaplivre_identity < schema.sql
```

3. Iniciar Redis:
```bash
redis-server
```

4. Configurar variáveis de ambiente:
```bash
cp .env.example .env
# Editar .env com suas configurações
```

5. Compilar e rodar:
```bash
cargo run
```

### Testes

```bash
# Unit tests
cargo test

# Integration tests (requer DB rodando)
DATABASE_URL=postgres://... cargo test --features integration-tests

# Coverage
cargo tarpaulin --out Html
```

## Deploy

### Docker Compose

```yaml
version: '3.8'

services:
  postgres:
    image: postgres:15
    environment:
      POSTGRES_DB: zaplivre_identity
      POSTGRES_USER: zaplivre
      POSTGRES_PASSWORD: zaplivre
    volumes:
      - ./schema.sql:/docker-entrypoint-initdb.d/schema.sql
      - postgres_data:/var/lib/postgresql/data

  redis:
    image: redis:7-alpine

  identity-server:
    build: .
    ports:
      - "8080:8080"
    environment:
      DATABASE_URL: postgres://zaplivre:zaplivre@postgres/zaplivre_identity
      REDIS_URL: redis://redis
      BIND_ADDR: 0.0.0.0:8080
    depends_on:
      - postgres
      - redis

volumes:
  postgres_data:
```

### Produção

Para produção, considere:

1. **TLS/HTTPS:** Usar nginx/Caddy como reverse proxy
2. **Monitoramento:** Prometheus + Grafana
3. **Logs:** Centralizar com ELK stack
4. **Backup:** PostgreSQL backup diário
5. **Scaling:** Load balancer + múltiplas instâncias
6. **Redis:** Redis Cluster para alta disponibilidade

## Segurança

- ✅ Todas as comunicações devem usar HTTPS em produção
- ✅ Assinaturas Ed25519 previnem spoofing de identidade
- ✅ Rate limiting previne abuse
- ✅ Prekey bundles permitem E2E encryption (X3DH)
- ✅ Timestamps previnem replay attacks (janela de 5 minutos)
- ✅ Usernames são públicos (não armazenar PII sensível)
- ⚠️ Public keys são públicas (necessário para descoberta de peers)

## Troubleshooting

### "Database error: connection refused"
- Verificar se PostgreSQL está rodando: `pg_isready`
- Verificar URL de conexão no `.env`
- Verificar firewall/network

### "Redis error: connection refused"
- Verificar se Redis está rodando: `redis-cli ping`
- Verificar URL de conexão no `.env`

### "Rate limit exceeded"
- Aguardar o período da janela (1 hora)
- Verificar IP real (X-Forwarded-For em produção)
- Considerar aumentar limites se necessário

## Referências

- [ADR 001: Username System](../../docs/architecture/decisions/001-username-system.md)
- [Identity Server OpenAPI Spec](../../docs/api/identity-server.yaml)
- [Signal Protocol X3DH](https://signal.org/docs/specifications/x3dh/)
- [Ed25519 Signatures](https://ed25519.cr.yp.to/)

## Licença

AGPL-3.0 - Ver [LICENSE](../../LICENSE) para detalhes.
