# Plano de Implementacao (Gaps da Auditoria)

## Objetivo
Fechar divergencias entre especificacoes e implementacao atual.

## Fase A - Fundacao Criptografia e Sync
- A1 Integrar Signal Protocol (libsignal) no core e ajustar FFI.
- A2 Definir e implementar armazenamento seguro de sessions/keys no SQLite.
- A3 Implementar CRDTs (automerge) e sync multi-device no core.
- A4 Expor APIs de sync via FFI e consumir nos apps.
- A5 Atualizar docs de arquitetura e guias com o estado real.

## Fase B - VoIP/Video e Grupos
- B1 Definir estrategia de chamadas em grupo (SFU vs mesh).
- B2 Se SFU: selecionar stack (mediasoup/Janus), criar servico e integracao.
- B3 Implementar signaling para group calls (server ou p2p) e APIs no core.
- B4 Atualizar apps (iOS/Android/Desktop) para group calls e UX.

## Fase C - Push e Confiabilidade
- C1 Completar checklist de push prod (APNs/FCM) e corrigir falhas.
- C2 Revisar tokens, rotacao, retry/backoff e logs no push-server.
- C3 Documentar processo de deploy e troubleshoot do push.

## Fase D - QA e Polimento
- D1 Rodar testes hibridos (scripts + manual) e registrar resultados.
- D2 Fechar TODOs de UI/fluxo em chat, grupos e settings.
- D3 Atualizar README e ios/README com status real.

## Status
- Data:
- Responsavel:
- Observacoes:
