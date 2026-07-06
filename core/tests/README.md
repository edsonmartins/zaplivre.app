# Integration Tests - Core (Identity Client + Storage)

Testes de integração que validam o fluxo completo:
1. Identity Client registra username no Identity Server
2. Storage salva contato localmente com username
3. Client faz lookup de username
4. Storage armazena contato remoto
5. Queries em storage local

## Pré-requisitos

1. **Identity Server** rodando em `http://localhost:8080`
2. **PostgreSQL** e **Redis** disponíveis (para o Identity Server)

## Setup Rápido

```bash
# Terminal 1: Start Identity Server
cd server/identity
cargo run

# Terminal 2: Run integration tests
cd core
cargo test --test identity_integration --features integration-tests
```

## Testes Incluídos

### 1. `test_full_flow_register_and_store`
Fluxo completo de registro e armazenamento:
- Gera identidade
- Registra @username no Identity Server
- Armazena contato localmente
- Valida dados armazenados

### 2. `test_lookup_and_cache_locally`
Simula Bob procurando Alice:
- Alice registra @alice
- Bob faz lookup de @alice
- Bob salva Alice nos contatos locais (cache)
- Bob consulta cache local

### 3. `test_update_prekeys_and_refresh_cache`
Atualização de prekeys:
- Alice registra
- Alice atualiza prekeys
- Bob faz lookup (prekeys atualizadas)
- Bob atualiza cache local

### 4. `test_duplicate_username_local_and_remote`
Validação de username único em ambos os níveis:
- Alice registra @alice (servidor)
- Alice salva localmente
- Eve tenta registrar @alice no servidor → ERRO
- Eve tenta salvar @alice localmente → ERRO

### 5. `test_search_contacts_after_registration`
Busca de contatos:
- Registra múltiplos usuários
- Salva todos localmente
- Busca por username → encontra
- Busca por display_name → encontra

### 6. `test_health_check`
Verifica saúde do Identity Server via Client API.

## Como Rodar

### Rodar todos os testes de integração

```bash
cargo test --test identity_integration --features integration-tests
```

### Rodar teste específico

```bash
cargo test --test identity_integration --features integration-tests test_full_flow_register_and_store
```

## Exemplo de Output

```
running 6 tests
test integration_tests::test_full_flow_register_and_store ... ok
test integration_tests::test_lookup_and_cache_locally ... ok
test integration_tests::test_update_prekeys_and_refresh_cache ... ok
test integration_tests::test_duplicate_username_local_and_remote ... ok
test integration_tests::test_search_contacts_after_registration ... ok
test integration_tests::test_health_check ... ok

test result: ok. 6 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out
```

## Fluxo de Dados Testado

```
┌─────────────┐     register_username()    ┌──────────────────┐
│   Client    │ ────────────────────────> │ Identity Server  │
│  (Alice)    │                            │  (PostgreSQL)    │
└─────────────┘                            └──────────────────┘
      │
      │ insert_contact()
      ▼
┌─────────────┐
│   Storage   │
│  (SQLite)   │
└─────────────┘


┌─────────────┐     lookup_username()      ┌──────────────────┐
│   Client    │ ────────────────────────> │ Identity Server  │
│   (Bob)     │ <──────────────────────── │  (PostgreSQL)    │
└─────────────┘   prekey_bundle + peer_id  └──────────────────┘
      │
      │ insert_contact()
      ▼
┌─────────────┐
│   Storage   │  get_contact_by_username("alice")
│  (SQLite)   │ ──────────────────────────────────> (Cache Hit)
└─────────────┘
```

## Troubleshooting

### Erro: "connection refused"
```bash
# Start Identity Server
cd server/identity
cargo run
```

### Erro: "USERNAME_TAKEN" inesperado
```bash
# Limpar database de teste
psql zaplivre_identity -c "DELETE FROM usernames WHERE username LIKE 'alice_%';"

# Ou recriar database
dropdb zaplivre_identity
createdb zaplivre_identity
psql zaplivre_identity < server/identity/schema.sql
```

### Rate limit errors
```bash
# Limpar Redis
redis-cli FLUSHALL
```

## CI/CD

Adicionar ao GitHub Actions:

```yaml
integration-tests:
  runs-on: ubuntu-latest

  services:
    postgres:
      image: postgres:15
      env:
        POSTGRES_DB: zaplivre_identity
        POSTGRES_USER: zaplivre
        POSTGRES_PASSWORD: zaplivre

    redis:
      image: redis:7-alpine

  steps:
    - name: Start Identity Server
      run: cd server/identity && cargo run &

    - name: Run core integration tests
      run: cd core && cargo test --test identity_integration --features integration-tests
```
