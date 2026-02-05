# Plano de Testes Android (Fase 6 + Push)

Objetivo: validar fluxo completo no Android antes de release.

## 1) Build e inicializacao

- [ ] Build Debug via Gradle
- [ ] App inicia sem crash
- [ ] Peer ID criado e persistido

## 2) P2P

- [ ] Android -> iOS: enviar e receber mensagem
- [ ] Android -> Desktop: enviar e receber mensagem
- [ ] Contador de peers > 0 quando online

## 3) Store & Forward

- [ ] Enviar mensagem com destinatario offline
- [ ] Destinatario recebe ao voltar online

## 4) Push (FCM)

- [ ] Token FCM gerado e persistido
- [ ] Token registrado no push server
- [ ] Push chega com app fechado
- [ ] Tap abre conversa correta

## 5) VoIP (audio)

- [ ] Chamada Android <-> iOS
- [ ] Chamada Android <-> Desktop
- [ ] Audio bidirecional
- [ ] Mute e hangup funcionam

## 6) UX/estabilidade

- [ ] Navegacao entre telas (Conversations, Chat, Settings)
- [ ] Reabrir app e validar estado
- [ ] App em background por 10 min e retorno sem erro

## Criterio de encerramento

- [ ] Todos os itens acima passaram sem crash/erro critico
