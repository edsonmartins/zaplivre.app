# Push Notifications - Checklist Final

## iOS (APNs)

- [ ] App ID `app.zaplivre.ios` com Push Notifications habilitado no Apple Developer.
- [ ] APNs Auth Key (`.p8`) criado e configurado no servidor.
- [ ] Entitlements configurados:
  - `ios/ZapLivre/ZapLivre.dev.entitlements`
  - `ios/ZapLivre/ZapLivre.prod.entitlements`
- [ ] App instalado em device físico e token APNs logado:
  - `🍎 APNs device token: ...`
- [ ] Token registrado no push server:
  - `✅ Device token registered with push server`
- [ ] Push de teste recebido (app fechado, background, foreground).
- [ ] Ao tocar a notificação, abre a conversa correta.

## Android (FCM)

- [ ] Projeto Firebase com FCM ativo.
- [ ] `google-services.json` presente no app.
- [ ] Token FCM logado:
  - `📱 FCM token obtained: ...`
- [ ] Token registrado no push server:
  - `✅ FCM token successfully registered with Push Server`
- [ ] Persistência de token funcionando (se `peerId` não existir, envia depois).
- [ ] Push de teste recebido (app fechado, background, foreground).
- [ ] Ao tocar a notificação, abre a conversa correta.

## End-to-end

- [ ] Push server aceita register APNs/FCM e envia para os dois.
- [ ] `peer_id` presente no payload e navegação funciona em ambos.
- [ ] Logs de erro ausentes em `/api/v1/send`.
