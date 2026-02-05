# Testes Manuais (por plataforma)

Checklist detalhado para validacao manual antes de release.

## iOS

### Basico
- [ ] Login/onboarding concluido sem erros
- [ ] Peer ID exibido e salvo
- [ ] Lista de conversas carrega sem crash

### P2P
- [ ] Enviar mensagem com ambos online (iOS -> outro)
- [ ] Receber mensagem com ambos online (outro -> iOS)
- [ ] Contador de peers > 0 quando conectado

### Store & Forward
- [ ] Enviar mensagem para destinatario offline
- [ ] Destinatario recebe ao voltar online

### Push (APNs)
- [ ] Token APNs gerado no device
- [ ] Token registrado no push server
- [ ] Notificacao chega com app fechado
- [ ] Tap abre conversa correta

### VoIP (audio)
- [ ] Chamada de voz iOS <-> Android
- [ ] Chamada de voz iOS <-> Desktop
- [ ] Audio funciona nos dois sentidos

## Android

### Basico
- [ ] Login/onboarding concluido sem erros
- [ ] Peer ID exibido e salvo
- [ ] Lista de conversas carrega sem crash

### P2P
- [ ] Enviar mensagem com ambos online (Android -> outro)
- [ ] Receber mensagem com ambos online (outro -> Android)
- [ ] Contador de peers > 0 quando conectado

### Store & Forward
- [ ] Enviar mensagem para destinatario offline
- [ ] Destinatario recebe ao voltar online

### Push (FCM)
- [ ] Token FCM gerado
- [ ] Token registrado no push server
- [ ] Notificacao chega com app fechado
- [ ] Tap abre conversa correta

### VoIP (audio)
- [ ] Chamada de voz Android <-> iOS
- [ ] Chamada de voz Android <-> Desktop
- [ ] Audio funciona nos dois sentidos

## Desktop

### Basico
- [ ] Login/onboarding concluido sem erros
- [ ] Peer ID exibido e salvo
- [ ] Lista de conversas carrega sem crash

### P2P
- [ ] Enviar mensagem com ambos online (Desktop -> outro)
- [ ] Receber mensagem com ambos online (outro -> Desktop)
- [ ] Contador de peers > 0 quando conectado

### Store & Forward
- [ ] Enviar mensagem para destinatario offline
- [ ] Destinatario recebe ao voltar online

### VoIP (audio)
- [ ] Chamada de voz Desktop <-> Android
- [ ] Chamada de voz Desktop <-> iOS
- [ ] Audio funciona nos dois sentidos
