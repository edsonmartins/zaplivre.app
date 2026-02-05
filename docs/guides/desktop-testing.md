# Plano de Testes Desktop (Fase 7)

Objetivo: validar fluxo completo no Desktop antes de release.

## 1) Build e inicializacao

- [ ] Build local (Tauri)
- [ ] App inicia sem crash
- [ ] Peer ID criado e persistido

## 2) P2P

- [ ] Desktop -> Android: enviar e receber mensagem
- [ ] Desktop -> iOS: enviar e receber mensagem
- [ ] Contador de peers > 0 quando online

## 3) Store & Forward

- [ ] Enviar mensagem com destinatario offline
- [ ] Destinatario recebe ao voltar online

## 4) VoIP (audio)

- [ ] Chamada Desktop <-> Android
- [ ] Chamada Desktop <-> iOS
- [ ] Audio bidirecional
- [ ] Mute e hangup funcionam

## 5) UX/estabilidade

- [ ] Navegacao entre telas (Conversations, Chat, Settings)
- [ ] Reabrir app e validar estado
- [ ] App em background por 10 min e retorno sem erro

## Criterio de encerramento

- [ ] Todos os itens acima passaram sem crash/erro critico
