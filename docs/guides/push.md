# Push Notifications (iOS + Android)

Guia rápido para configurar, registrar e testar push notifications no MePassa.

## 1) Visão geral

**iOS (APNs)**  
- Registro via APNs (device token)  
- Envio via Push Server (`/api/v1/register`)  

**Android (FCM)**  
- Token via FirebaseMessaging  
- Envio via Push Server (`/api/v1/register`)  

## 2) iOS (APNs)

### 2.1 Apple Developer

1. App ID: `app.mepassa.ios` com **Push Notifications** habilitado.  
2. Gerar **APNs Auth Key** (`.p8`).  
3. Anotar:
   - Key ID
   - Team ID
   - Bundle ID

### 2.2 Xcode (Signing & Capabilities)

- Target → Signing & Capabilities:
  - Team selecionado
  - Push Notifications habilitado
  - Background Modes: Remote notifications

Entitlements no repo:
- `ios/MePassa/MePassa.dev.entitlements`
- `ios/MePassa/MePassa.prod.entitlements`

### 2.3 Build e device físico

```bash
cd ios
xcodebuild -scheme MePassa -configuration Debug -destination 'platform=iOS,name=SEU_IPHONE' build
```

### 2.4 Token e registro

Logs esperados:
```
🍎 APNs device token: <token>
✅ Device token registered with push server
```

### 2.5 Teste de push (APNs)

Template no repo:
```
scripts/test-apns.sh
```

Uso:
```bash
APNS_KEY_PATH=... APNS_KEY_ID=... APNS_TEAM_ID=... APNS_BUNDLE_ID=... \
APNS_DEVICE_TOKEN=... APNS_ENV=development PEER_ID_DESTINO=... \
scripts/test-apns.sh
```

Critério de sucesso:
- Notificação aparece com app fechado, em background e em foreground.
- Ao tocar, abre a conversa correta.

## 3) Android (FCM)

### 3.1 Firebase

- Projeto Firebase com **FCM** ativo.
- `google-services.json` no app Android.

### 3.2 Token e registro

Logs esperados:
```
📱 FCM token obtained: <token>
✅ FCM token successfully registered with Push Server
```

### 3.3 Teste de push (FCM)

Envie um push pelo console do Firebase com payload:
```json
{
  "peer_id": "<PEER_ID_DESTINO>",
  "title": "Nova mensagem",
  "body": "Teste push Android"
}
```

Critério de sucesso:
- Notificação aparece com app fechado, em background e em foreground.
- Ao tocar, abre a conversa correta.

## 4) Endpoints do Push Server

- `POST /api/v1/register`
  - Payload:
    - `peer_id`
    - `platform` (`apns` ou `fcm`)
    - `device_id`
    - `token`
    - `device_name`
    - `app_version`

## 5) Checklist de validação final

- [ ] iOS: token APNs registrado no servidor  
- [ ] Android: token FCM registrado no servidor  
- [ ] Push abre conversa correta em ambos  
- [ ] Push funciona com app fechado  
