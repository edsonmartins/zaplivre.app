# Integration Tests - Identity Server

Testes de integração para o Identity Server que validam:
- Registro de username único
- Lookup de username
- Username duplicado (erro 409)
- Rate limiting (anti-spam)
- Validação de username
- Health check

## Pré-requisitos

1. **PostgreSQL** rodando e disponível
2. **Redis** rodando e disponível
3. **Identity Server** rodando em `http://localhost:8080`

## Setup

### 1. Iniciar PostgreSQL e Redis

```bash
# macOS (Homebrew)
brew services start postgresql@15
brew services start redis

# Ubuntu
sudo systemctl start postgresql
sudo systemctl start redis-server

# Docker Compose
docker-compose up -d postgres redis
```

### 2. Criar database

```bash
createdb zaplivre_identity
psql zaplivre_identity < schema.sql
```

### 3. Iniciar Identity Server

```bash
# Terminal 1
cd server/identity
cargo run

# Aguardar:
# Identity Server listening on 0.0.0.0:8080
```

## Rodando os Testes

### Rodar todos os testes de integração

```bash
# Terminal 2
cd server/identity
cargo test --test integration_tests -- --test-threads=1
```

**IMPORTANTE:** Use `--test-threads=1` para evitar race conditions com rate limiting.

### Rodar testes específicos

```bash
# Teste de registro
cargo test --test integration_tests test_register_username_success

# Teste de lookup
cargo test --test integration_tests test_lookup_username_success

# Teste de duplicata (409)
cargo test --test integration_tests test_register_duplicate_username

# Teste de rate limiting
cargo test --test integration_tests test_rate_limiting_register

# Health check
cargo test --test integration_tests test_health_check
```

## Testes Incluídos

### 1. `test_health_check`
Verifica se o servidor está saudável:
- Status "healthy"
- PostgreSQL conectado
- Redis conectado

### 2. `test_register_username_success`
Registra um username e verifica:
- HTTP 200
- Username retornado corretamente
- Peer ID retornado corretamente

### 3. `test_lookup_username_success`
Registra e depois busca um username:
- HTTP 200
- Username encontrado
- Peer ID correto
- PreKey bundle retornado

### 4. `test_register_duplicate_username`
Tenta registrar mesmo username duas vezes:
- Primeiro registro: HTTP 200
- Segundo registro: HTTP 409 (Conflict)
- Error code: "USERNAME_TAKEN"
- Sugestões de username alternativo retornadas

### 5. `test_lookup_nonexistent_username`
Busca um username que não existe:
- HTTP 404 (Not Found)
- Error code: "USERNAME_NOT_FOUND"

### 6. `test_invalid_username_format`
Tenta registrar username inválido (maiúsculas):
- HTTP 400 (Bad Request)
- Error code: "INVALID_USERNAME"

### 7. `test_rate_limiting_register`
Tenta registrar 6 usernames rapidamente:
- Primeiros 5: podem passar (limite de 5/hora)
- 6º: HTTP 429 (Too Many Requests)

### 8. `test_rate_limit_headers`
Verifica headers de rate limiting:
- `X-RateLimit-Limit: 5`
- `X-RateLimit-Remaining: <N>`

## Troubleshooting

### Erro: "connection refused"
**Problema:** Identity Server não está rodando.
**Solução:** Iniciar o servidor em outro terminal: `cargo run`

### Erro: "database error"
**Problema:** PostgreSQL não está rodando ou database não existe.
**Solução:**
```bash
brew services start postgresql@15
createdb zaplivre_identity
psql zaplivre_identity < schema.sql
```

### Erro: "redis error"
**Problema:** Redis não está rodando.
**Solução:**
```bash
brew services start redis
```

### Testes falhando por rate limiting
**Problema:** Testes anteriores consumiram quota de rate limiting.
**Solução:**
1. Aguardar 1 hora OU
2. Limpar Redis: `redis-cli FLUSHALL` OU
3. Usar `--test-threads=1` para executar sequencialmente

### Rate limit ainda ativo
```bash
# Verificar keys do Redis
redis-cli KEYS "ratelimit:*"

# Limpar rate limits
redis-cli FLUSHALL

# Ou deletar keys específicas
redis-cli DEL "ratelimit:/api/v1/register:unknown"
```

## CI/CD

Para rodar em CI/CD:

```yaml
# .github/workflows/identity-server-tests.yml
name: Identity Server Tests

on: [push, pull_request]

jobs:
  integration-tests:
    runs-on: ubuntu-latest

    services:
      postgres:
        image: postgres:15
        env:
          POSTGRES_DB: zaplivre_identity
          POSTGRES_USER: zaplivre
          POSTGRES_PASSWORD: zaplivre
        options: >-
          --health-cmd pg_isready
          --health-interval 10s
          --health-timeout 5s
          --health-retries 5

      redis:
        image: redis:7-alpine
        options: >-
          --health-cmd "redis-cli ping"
          --health-interval 10s
          --health-timeout 5s
          --health-retries 5

    steps:
      - uses: actions/checkout@v3

      - name: Setup Rust
        uses: actions-rs/toolchain@v1
        with:
          toolchain: stable

      - name: Setup database
        run: |
          psql -h localhost -U zaplivre -d zaplivre_identity < server/identity/schema.sql
        env:
          PGPASSWORD: zaplivre

      - name: Start Identity Server
        run: |
          cd server/identity
          cargo run &
          sleep 5
        env:
          DATABASE_URL: postgres://zaplivre:zaplivre@localhost/zaplivre_identity
          REDIS_URL: redis://localhost

      - name: Run integration tests
        run: |
          cd server/identity
          cargo test --test integration_tests -- --test-threads=1
```

## Output Esperado

```
running 8 tests
test test_health_check ... ok
test test_register_username_success ... ok
test test_lookup_username_success ... ok
test test_register_duplicate_username ... ok
test test_lookup_nonexistent_username ... ok
test test_invalid_username_format ... ok
test test_rate_limiting_register ... ok
test test_rate_limit_headers ... ok

test result: ok. 8 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out
```
