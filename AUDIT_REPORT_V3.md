# Auditoria Técnica V3 — ZapLivre

Data: 2026-01-28
Escopo: core (Rust), desktop (Tauri/React), iOS (Swift), Android (Kotlin), servidores.

## Resumo Executivo
A base está funcional para mensagens P2P 1:1 e persistência local, porém ainda há lacunas críticas em mídia, VoIP, grupos e integração completa de rede (relay/DCUtR, signaling). Existem também simplificações de segurança (Signal/X3DH reduzido, storage key estática) e pontos de produção ainda mockados (push, identidade, UI). Abaixo estão os gaps priorizados para correção.

## P0 — Bloqueadores (funcionalidade incompleta ou risco sistêmico)
- **Envio de mídia não vai para a rede**: métodos de imagem/áudio/documento/vídeo persistem apenas localmente e deixam TODOs para envio e armazenamento em disco. Isso quebra a experiência multi‑device e a consistência entre peers. (`core/src/api/client.rs`)
- **VoIP incompleto (stubs e pipeline parcial)**: integração de signaling, eventos e controle de áudio/vídeo ainda estão pendentes; FFI expõe funções stub quando feature desabilitada e a pipeline de vídeo é placeholder. (`core/src/voip/*`, `core/src/network/swarm.rs`, `core/src/ffi/*`, iOS/Android UI)
- **Grupos não implementados nos clientes**: telas de grupos no desktop/iOS/Android estão com TODOs e sem ligação com API real. (`desktop/src/views/GroupChatView.tsx`, `ios/ZapLivre/ZapLivre/Views/Group*`, `android/app/src/main/kotlin/com/zaplivre/ui/screens/group/*`)

## P1 — Alta prioridade (segurança, confiabilidade, produção)
- **Criptografia E2E simplificada**: X3DH usa apenas prekeys X25519 (sem identidade) e sem libsignal; precisa hardening para produção. (`core/src/crypto/signal.rs`)
- **Identidade e storage key não usam secure storage nativo**: iOS/Android ainda gravam identidade em arquivo no data_dir, não em Keychain/Keystore; risco de perda/roubo. (`ios/ZapLivre/ZapLivre/Core/ZapLivreCore.swift`, `android/.../ZapLivreClientWrapper.kt`, `core/src/identity/storage.rs`)
- **Push notifications com URL local e sem configuração real**: endpoint de push usa localhost e há erros de APS no iOS. (`ios/ZapLivre/ZapLivre/Core/PushNotificationManager.swift`)
- **Relay/DCUtR e signaling incompletos**: TODOs em swarm e integração de relay/upgrade e encaminhamento de sinais para VoIP ainda não feito. (`core/src/network/swarm.rs`)
- **Download de mídia limitado a local**: `download_media` só lê arquivo local e não solicita ao peer. (`core/src/api/client.rs`)

## P2 — Média prioridade (UX, consistência, dívida técnica)
- **Reações/forward sem broadcast**: ações são locais e não propagam pela rede. (`core/src/api/client.rs`)
- **Fallbacks e panics**: existem caminhos com `panic!` em testes/handlers que deveriam virar erro controlado. (`core/src/network/message_handler.rs`)
- **Mocks de UI e fluxos incompletos**: logout, limpeza de cache, ícones, etc. (`ios/ZapLivre/ZapLivre/Views/SettingsView.swift`, `android/.../SettingsScreen.kt`, `android/.../ZapLivreService.kt`)
- **Testes end‑to‑end ignorados**: testes de integração estão marcados como ignore. (`core/tests/message_integration.rs`)

## P3 — Baixa prioridade (polimento)
- **Warnings e imports não utilizados** em core/desktop (não bloqueia, mas gera ruído e risco de regressão).
- **Docs e scripts com placeholders** (ex.: URLs/keys). (`docs/APNS_SETUP_GUIDE.md`, `server/push/README.md`)

## Observações Arquiteturais
- O core já tem suporte de DHT/relay e E2E básico, mas falta a amarração completa dos fluxos (signaling VoIP, mídia, reações, grupos). Isso explica a instabilidade em reconexão e divergência de estado entre desktop/iOS.
- O storage encryption existe para mensagens, mas não cobre mídia nem garante gerenciamento de chaves por plataforma (Keychain/Keystore). Isso é um risco em produção.
- A camada de identidade possui `IdentityStorage` abstrato, mas iOS/Android ainda usam caminho de arquivo direto, não o storage seguro.

## Recomendações de Próximos Passos (ordem sugerida)
1. **Concluir mídia P2P**: salvar mídia em disco, anunciar e baixar via rede; atualizar UI para consumo real.
2. **Finalizar signaling VoIP**: enviar/receber sinais via swarm e integrar CallManager em todas plataformas.
3. **Implementar grupos**: API + UI (listagem, mensagens, membros) e testes básicos.
4. **Hardening de segurança**: substituir X3DH simplificado por libsignal‑protocol e mover identidade para Keychain/Keystore.
5. **Push em produção**: configurar endpoints reais (APNS/FCM), tratar tokens e abrir conversa por push.

---

Se quiser, posso transformar este relatório em backlog priorizado com épicos e tickets (com estimativas), ou já começar a atacar a lista P0/P1.
