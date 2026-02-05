# Plano de Testes iOS (Fase 13)

Objetivo: validar o fluxo completo no iOS antes de encerrar a fase.

## 1) Build e inicializacao

- [ ] Build Debug em device fisico
- [ ] App inicia sem crash
- [ ] Peer ID criado e persistido

## 2) P2P

- [ ] iOS -> Android: enviar e receber mensagem
- [ ] iOS -> Desktop: enviar e receber mensagem
- [ ] Contador de peers > 0 quando online

## 3) Store & Forward

- [ ] Enviar mensagem com destinatario offline
- [ ] Destinatario recebe ao voltar online

## 4) Push (APNs)

- [ ] Token APNs gerado no device
- [ ] Token registrado no push server
- [ ] Push chega com app fechado
- [ ] Tap abre conversa correta

## 5) VoIP (CallKit)

- [ ] Chamada iOS <-> Android
- [ ] Chamada iOS <-> Desktop
- [ ] Audio bidirecional
- [ ] Mute e hangup funcionam

## 6) UX/estabilidade

- [ ] Navegacao entre telas (Conversations, Chat, Settings)
- [ ] Reabrir app e validar estado
- [ ] App em background por 10 min e retorno sem erro

## Criterio de encerramento

- [ ] Todos os itens acima passaram sem crash/erro critico
