# FASE 8: Push Notifications - Guia de Testes End-to-End

**Data:** 2026-01-20
**Status:** Pronto para testes
**Componentes:** Desktop (Tauri), Android (FCM), Push Server (Rust)

---

## 📋 Visão Geral do Sistema

O sistema de push notifications implementado na FASE 8 consiste em 3 componentes principais:

1. **Desktop App (Tauri)**: Notificações locais quando mensagem chega
2. **Android App**: FCM (Firebase Cloud Messaging) para notificações push
3. **Push Server**: Servidor Rust que gerencia tokens e envia notificações FCM

---

## 🔧 Setup Inicial

### 1. Push Server

**Pré-requisitos:**
- PostgreSQL rodando (via Docker ou local)
- Firebase Server Key (obtido do Firebase Console)

**Passos:**

```bash
# 1. Iniciar PostgreSQL (via Docker)
cd /Users/edsonmartins/desenvolvimento/zaplivre
docker-compose up -d postgres

# 2. Configurar environment variables
cd server/push
cp .env.example .env

# 3. Editar .env e adicionar:
# DATABASE_URL=postgresql://zaplivre:zaplivre_dev_password@localhost:5432/zaplivre
# FCM_SERVER_KEY=<seu_fcm_server_key>

# 4. Executar Push Server
cargo run --release
# ou
../../target/release/zaplivre-push

# Servidor deve estar rodando em: http://localhost:8081
```

**Verificar:**
```bash
curl http://localhost:8081/health
# Deve retornar: OK
```

### 2. Firebase Setup (Android)

**Pré-requisitos:**
- Projeto Firebase criado
- `google-services.json` adicionado em `android/app/`
- FCM habilitado no projeto

**Verificação:**
```bash
# Verificar se google-services.json existe
ls android/app/google-services.json

# Se não existir, seguir FIREBASE_SETUP.md
cat android/FIREBASE_SETUP.md
```

### 3. Android App

**Build e Install:**
```bash
cd android
./gradlew clean assembleDebug installDebug

# Ou via Android Studio:
# - Open android/ folder
# - Run > Run 'app'
```

**Verificar logs FCM:**
```bash
adb logcat -s FCM ZapLivreService PushServerClient
```

### 4. Desktop App

**Executar:**
```bash
cd desktop
npm run tauri:dev
```

---

## 🧪 Testes Funcionais

### Teste 1: Verificar Health do Push Server

**Objetivo:** Confirmar que o Push Server está rodando

```bash
curl http://localhost:8081/health
```

**Resultado esperado:**
- Status 200 OK
- Body: "OK"

---

### Teste 2: Registro de Token FCM (Android)

**Objetivo:** Verificar que o Android app registra o FCM token no Push Server

**Passos:**

1. **Iniciar Android app** (via emulador ou dispositivo físico)
2. **Aguardar ZapLivreService inicializar** (5-10 segundos)
3. **Verificar logs:**

```bash
adb logcat | grep -E "(FCM|PushServerClient|ZapLivreService)"
```

**Resultado esperado nos logs:**
```
ZapLivreService: Initializing ZapLivreClient from service
ZapLivreService: 🔐 Getting FCM token...
ZapLivreService: 📱 FCM token obtained: AAAA...
ZapLivreService: 📤 Registering FCM token with Push Server...
PushServerClient: 📤 Registering token - peer_id: <peer_id>, device_id: <device_id>
PushServerClient: ✅ Token registered successfully
ZapLivreService: ✅ FCM token successfully registered with Push Server
```

4. **Verificar no banco de dados PostgreSQL:**

```bash
docker exec -it zaplivre-postgres psql -U zaplivre -d zaplivre

# No psql:
SELECT peer_id, platform, device_id, device_name, is_active, created_at
FROM push_tokens
ORDER BY created_at DESC
LIMIT 5;
```

**Resultado esperado:**
- Deve haver 1 registro com:
  - `platform = 'fcm'`
  - `is_active = true`
  - `device_id` = Android ID do dispositivo
  - `token` = FCM token

---

### Teste 3: Enviar Notificação Manual via Push Server

**Objetivo:** Testar envio de notificação push manualmente

**Pré-requisitos:**
- Android app rodando (pode estar em background/fechado)
- Token registrado (Teste 2 completo)

**Passos:**

1. **Obter peer_id do Android:**

```bash
adb logcat | grep "PeerId:"
# Ou verificar no banco de dados (Teste 2, passo 4)
```

2. **Enviar notificação via curl:**

```bash
curl -X POST http://localhost:8081/api/v1/send \
  -H "Content-Type: application/json" \
  -d '{
    "peer_id": "<peer_id_do_android>",
    "title": "Teste Manual",
    "body": "Esta é uma notificação de teste enviada via Push Server",
    "data": {
      "type": "test",
      "timestamp": "2026-01-20T14:00:00Z"
    }
  }'
```

**Resultado esperado:**

- **No Push Server (logs):**
```
📤 Send notification request - peer_id: <peer_id>, title: Teste Manual
📱 Found 1 active device(s)
  🔥 Sending FCM notification - title: Teste Manual, body_len: 58
  ✅ FCM notification sent to <device_id>
✅ Sent 1 notification(s), 0 failed
```

- **No Android (notificação):**
  - Notificação aparece na barra de status
  - Título: "Teste Manual"
  - Corpo: "Esta é uma notificação de teste..."
  - Som/vibração (depende das configurações do dispositivo)

- **Ao clicar na notificação:**
  - Android app abre

---

### Teste 4: Desktop Notifications (Local)

**Objetivo:** Verificar notificações locais no Desktop app

**Pré-requisitos:**
- Desktop app rodando
- Conversa ativa com algum peer

**Passos:**

1. **Iniciar Desktop app**
2. **Enviar mensagem de outro dispositivo** (ou simular)
3. **Aguardar atualização automática** (polling a cada 5 segundos)

**Resultado esperado:**
- Notificação desktop aparece com:
  - Título: "Nova mensagem"
  - Corpo: Preview da mensagem ou peer_id

**Nota:** Desktop usa notificações locais (Tauri API), não FCM.

---

### Teste 5: Fluxo Completo End-to-End (Android Offline)

**Objetivo:** Testar fluxo completo: mensagem offline → push → notificação → app acorda

**Cenário:**
- Device A (Android): App em background/fechado
- Device B (Desktop ou outro Android): Envia mensagem

**Passos:**

1. **Device A:**
   - Garantir que app está registrado (Teste 2)
   - Fechar app ou colocar em background
   - Desconectar Wi-Fi (simular offline) - **OPCIONAL**

2. **Device B:**
   - Enviar mensagem de texto para peer_id do Device A
   - Mensagem vai para Message Store (peer offline)

3. **Message Store (servidor):**
   - Detecta que Device A está offline
   - **Chama Push Server:** POST /api/v1/send
   - Push Server envia FCM para Device A

4. **Device A:**
   - Recebe notificação FCM
   - ZapLivreFirebaseMessagingService processa
   - Notificação aparece na barra de status
   - ZapLivreService é acordado (start)
   - App faz poll no Message Store
   - Mensagem é baixada e exibida

**Resultado esperado:**

- **Logs Device A:**
```
FCM: FCM message received from: <fcm_sender>
FCM: Notification - Title: Nova mensagem, Body: <preview>
FCM: ZapLivreService started to sync messages
ZapLivreService: Service start command received
ZapLivreService: <poll message store>
```

- **Notificação Device A:**
  - Título: "Nova mensagem"
  - Corpo: Preview da mensagem
  - Click abre o app na conversa

**Nota:** Este teste requer Message Store implementado (FASE futura).

---

### Teste 6: Múltiplos Dispositivos (Mesmo Peer)

**Objetivo:** Verificar que múltiplos devices do mesmo peer recebem notificações

**Pré-requisitos:**
- 2+ dispositivos Android com mesmo peer_id
- Ou: 1 Android real + 1 Emulador (peer_id diferente, mas simula)

**Passos:**

1. **Registrar Device 1 e Device 2:**
   - Iniciar app em ambos
   - Aguardar registro (Teste 2)

2. **Verificar banco de dados:**
```sql
SELECT device_id, device_name, token, is_active
FROM push_tokens
WHERE peer_id = '<peer_id>'
  AND is_active = true;
```

**Resultado esperado:**
- 2 registros com `device_id` diferentes
- Ambos com `is_active = true`

3. **Enviar notificação:**
```bash
curl -X POST http://localhost:8081/api/v1/send \
  -H "Content-Type: application/json" \
  -d '{
    "peer_id": "<peer_id>",
    "title": "Broadcast Test",
    "body": "Mensagem para todos os dispositivos"
  }'
```

**Resultado esperado:**
- Ambos dispositivos recebem notificação
- Push Server logs: "Sent 2 notification(s), 0 failed"

---

### Teste 7: Token Refresh (FCM)

**Objetivo:** Verificar que token é atualizado quando Firebase gera novo token

**Passos:**

1. **Forçar refresh do token** (Android):
```bash
# Via adb shell (pode não funcionar em todos devices)
adb shell am broadcast -a com.google.firebase.INSTANCE_ID_EVENT
```

2. **Ou desinstalar/reinstalar app:**
```bash
adb uninstall com.zaplivre
./gradlew installDebug
```

3. **Verificar logs:**
```bash
adb logcat | grep "New FCM token"
```

**Resultado esperado:**
- `ZapLivreFirebaseMessagingService.onNewToken()` é chamado
- Token é enviado ao Push Server via `/api/v1/register`
- Banco de dados é atualizado (mesmo peer_id + device_id)

---

### Teste 8: Token Inválido (Soft Delete)

**Objetivo:** Verificar que tokens inválidos são marcados como inativos

**Cenário:** Token FCM expirou ou device desinstalou o app

**Passos:**

1. **Simular token inválido** (criar registro fake no DB):
```sql
INSERT INTO push_tokens (peer_id, platform, device_id, token, device_name, app_version)
VALUES ('test_peer_invalid', 'fcm', 'invalid_device', 'INVALID_TOKEN_123', 'Test Device', '0.1.0');
```

2. **Enviar notificação:**
```bash
curl -X POST http://localhost:8081/api/v1/send \
  -H "Content-Type: application/json" \
  -d '{
    "peer_id": "test_peer_invalid",
    "title": "Test Invalid",
    "body": "Teste com token inválido"
  }'
```

3. **Verificar logs do Push Server:**
```
  ❌ FCM failed for invalid_device: <erro FCM>
  🔄 Marking token as inactive for invalid_device
```

4. **Verificar banco de dados:**
```sql
SELECT peer_id, device_id, is_active
FROM push_tokens
WHERE device_id = 'invalid_device';
```

**Resultado esperado:**
- Token marcado como `is_active = false`
- Não será mais usado em envios futuros

---

### Teste 9: Unregister Token

**Objetivo:** Testar desregistro de token (logout/desinstalação)

**Passos:**

1. **Obter peer_id e device_id de um device Android registrado**

2. **Chamar endpoint de unregister:**
```bash
curl -X DELETE http://localhost:8081/api/v1/unregister \
  -H "Content-Type: application/json" \
  -d '{
    "peer_id": "<peer_id>",
    "device_id": "<device_id>"
  }'
```

3. **Verificar banco de dados:**
```sql
SELECT peer_id, device_id, is_active, last_used_at
FROM push_tokens
WHERE peer_id = '<peer_id>'
  AND device_id = '<device_id>';
```

**Resultado esperado:**
- Token continua no banco (soft delete)
- `is_active = false`
- `last_used_at` atualizado

4. **Tentar enviar notificação:**
```bash
curl -X POST http://localhost:8081/api/v1/send \
  -H "Content-Type: application/json" \
  -d '{
    "peer_id": "<peer_id>",
    "title": "After Unregister",
    "body": "Esta notificação NÃO deve chegar"
  }'
```

**Resultado esperado:**
- Push Server retorna: "Sent 0 notification(s), 0 failed"
- Device não recebe notificação

---

### Teste 10: Performance e Latência

**Objetivo:** Medir latência do fluxo completo

**Passos:**

1. **Enviar notificação e medir tempo:**
```bash
time curl -X POST http://localhost:8081/api/v1/send \
  -H "Content-Type: application/json" \
  -d '{
    "peer_id": "<peer_id>",
    "title": "Performance Test",
    "body": "Teste de latência"
  }'
```

2. **Anotar timestamp quando notificação chega no Android**

**Métricas esperadas:**
- **Push Server → FCM:** < 500ms
- **FCM → Android device:** 1-3 segundos (depende da rede)
- **Total (end-to-end):** < 5 segundos

---

## 🐛 Troubleshooting

### Problema: Push Server não inicia

**Erro:** `Failed to connect to database`

**Solução:**
```bash
# Verificar se PostgreSQL está rodando
docker ps | grep postgres

# Se não estiver, iniciar:
docker-compose up -d postgres

# Verificar DATABASE_URL no .env
cat server/push/.env
```

---

### Problema: Android não recebe notificações

**Possíveis causas:**

1. **Token não registrado:**
   - Verificar logs: `adb logcat | grep PushServerClient`
   - Confirmar no DB: `SELECT * FROM push_tokens WHERE platform = 'fcm'`

2. **Push Server não está acessível:**
   - Android Emulator: usar `http://10.0.2.2:8081`
   - Dispositivo físico: usar IP da máquina (ex: `http://192.168.1.100:8081`)
   - Editar `PushServerClient.kt` para alterar `baseUrl`

3. **FCM Server Key inválida:**
   - Verificar .env do Push Server
   - Revalidar key no Firebase Console

4. **google-services.json ausente ou incorreto:**
   - Verificar: `ls android/app/google-services.json`
   - Rebuild: `./gradlew clean assembleDebug`

---

### Problema: Desktop notifications não aparecem

**Possíveis causas:**

1. **Permissões de notificação negadas:**
   - macOS: System Preferences → Notifications → <Desktop App>
   - Windows: Settings → Notifications → <Desktop App>

2. **Polling não está funcionando:**
   - Verificar logs do Tauri console
   - Aumentar frequência de polling (diminuir intervalo)

---

### Problema: "Token já existe mas não atualiza"

**Causa:** ON CONFLICT no SQL pode não estar funcionando

**Solução:**
```sql
-- Verificar constraint no banco
SELECT constraint_name, constraint_type
FROM information_schema.table_constraints
WHERE table_name = 'push_tokens';

-- Deve haver UNIQUE constraint em (peer_id, device_id)
```

---

## 📊 Checklist de Testes Completo

- [ ] **Setup**
  - [ ] PostgreSQL rodando
  - [ ] Push Server iniciado e healthy
  - [ ] Firebase configurado
  - [ ] Android app buildado
  - [ ] Desktop app rodando

- [ ] **Android Integration**
  - [ ] Token FCM registrado no startup
  - [ ] Token salvo no banco de dados
  - [ ] Notificação recebida (app background)
  - [ ] Notificação recebida (app fechado)
  - [ ] Click na notificação abre app

- [ ] **Desktop Integration**
  - [ ] Notificação local aparece (app aberto)
  - [ ] Notificação local aparece (app minimizado)

- [ ] **Push Server**
  - [ ] Health check retorna OK
  - [ ] Register endpoint funciona
  - [ ] Send endpoint funciona
  - [ ] Unregister endpoint funciona
  - [ ] Múltiplos devices suportados
  - [ ] Token inválido marcado inactive

- [ ] **End-to-End**
  - [ ] Fluxo completo offline → push → notificação
  - [ ] Latência < 5 segundos
  - [ ] Token refresh funciona
  - [ ] Soft delete funciona

---

## 🎯 Próximos Passos (Pós-FASE 8)

- [ ] Implementar Message Store integration (trigger push quando peer offline)
- [ ] APNs support para iOS (FASE 13)
- [ ] Rich notifications (imagem preview)
- [ ] Notification grouping
- [ ] Rate limiting no Push Server
- [ ] Analytics e métricas de delivery

---

**FASE 8 Testing Guide - Versão 1.0**
**Autor:** Claude + Edson Martins
**Data:** 2026-01-20
