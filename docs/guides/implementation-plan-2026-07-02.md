# Plano de Implementação — 2026-07-02

Fonte: `docs/guides/audit-2026-07-02.md` (auditoria completa de core, server, Android, iOS, desktop e build/testes).
Substitui o `implementation-plan.md` genérico anterior como plano vigente.

**Como usar:** cada tarefa tem ID estável (ex.: `CORE-03`), severidade da auditoria, arquivos, critério de aceite e estimativa. As fases são ordenadas por dependência — cada fase termina num **milestone testável**. Marque os checkboxes conforme concluir; use os IDs em commits (`fix(core): CORE-03 aguardar ConnectionEstablished`).

## Visão geral dos milestones

| Milestone | Entrega | Estimativa acumulada |
|---|---|---|
| **M1** — Build reproduzível | Qualquer dev builda tudo do zero; stack local sobe completa | ~3–4 dias |
| **M2** — Mensagens 1:1 testáveis | Texto+mídia A↔B confiável nas 4 plataformas, sem crash | +4–6 dias |
| **M3** — VoIP testável | Chamada de voz/vídeo ponta-a-ponta com callee funcionando | +3–5 dias |
| **M4** — Grupos testáveis | Protocolo de grupo robusto + UI ligada nas 3 plataformas | +5–8 dias |
| **M5** — Baseline de segurança | Sem plaintext silencioso, auth no backend, chaves protegidas | +5–7 dias |
| **M6** — Push + confiabilidade | APNs/FCM reais, store→push, retry/backoff | +3–5 dias |
| **M7** — Qualidade e polimento | Testes verdes e reativados, UX debt, P3s | +5–8 dias |

Total estimado: **~28–43 dias-pessoa**. M1+M2 (~7–10 dias) já destravam o alfa interno de mensagens.

---

## FASE 1 — Build e ambiente reproduzíveis (M1)

Objetivo: `git clone` → build funcionando em qualquer máquina; `make up` sobe o backend completo.

### Ambiente/toolchain
- [x] **ENV-01** (P0) ✅ 2026-07-02 — check de `protoc`/cargo/docker no `make setup` (target `check-prereqs`) + seção de pré-requisitos no `BUILD_AND_TEST.md`.
- [x] **ENV-02** (P2) ✅ 2026-07-02 — push migrado para sqlx 0.8 do workspace (`server/push/Cargo.toml`); `cargo check -p mepassa-push` OK, uma única árvore sqlx.

### Android build
- [x] **AND-01** (P0) ✅ 2026-07-02 — task Gradle `buildRustCore` roda `android/build-native.sh` antes do `preBuild` (só quando as `.so` faltam; force com `-PrebuildNative`); NDK resolvido via `ANDROID_NDK_HOME`/`local.properties`/mais novo do SDK (path hardcoded removido); check de `protoc` no script.
- [x] **AND-02** (P0) ✅ 2026-07-02 — `android/local.properties.example` criado; `abiFilters` agora inclui `x86_64` (emulador).

### iOS build
- [x] **IOS-01** (P0) ✅ 2026-07-02 — decisão: `.a` NÃO versionadas (grandes, geradas por máquina); `ios/.gitignore` corrigido (ignora `Libraries/` e documenta `build-rust.sh` como pré-requisito); `BUILD_AND_TEST.md` atualizado.
- [x] **IOS-02** (P0) ✅ 2026-07-02 — `project.yml` sincronizado: `CODE_SIGN_ENTITLEMENTS` (Debug=dev, Release=prod), `MESSAGE_STORE_URL`/`SIGNALING_SERVER_URL`, `NSLocalNetworkUsageDescription`+`NSBonjourServices` (`_p2p._udp`), `arm64`. *Pendente de verificação com `xcodegen generate` (não instalado nesta máquina).*
- [x] **IOS-03** (P2) ✅ 2026-07-02 — instruções unificadas para uniffi 0.31 (`generate-bindings.sh`, `pre-build-check.sh`); scripts legados movidos para `ios/legacy-scripts/` com README.

### Backend deploy
- [x] **SRV-01** (P0) ✅ 2026-07-02 — `server/identity/Dockerfile` criado; serviço `identity-server` (porta 8083) adicionado a `docker-compose.yml` e `stack.yml` (traefik `identity.associahub.com.br`).
- [x] **SRV-02** (P0) ✅ 2026-07-02 — Dockerfile do signaling copia o workspace completo; serviço `signaling-server` adicionado ao `docker-compose.yml` com healthcheck (`/health` existe); `curl` adicionado ao runtime.
- [x] **SRV-03** (P0) ✅ 2026-07-02 — compose e stack passam `APNS_KEY_PATH`/`APNS_KEY_ID`/`APNS_TEAM_ID`/`APNS_BUNDLE_ID`/`APNS_PRODUCTION`; diretório `server/push/certs/` criado com README; `*.p8` no `.gitignore`.
- [x] **SRV-04** (P0) ✅ 2026-07-02 — `db.rs` lê `DateTime<Utc>` (compatível com TIMESTAMPTZ do init.sql); `schema.sql` reescrito espelhando o init.sql; `cargo check -p mepassa-identity-server` OK.
- [x] **SRV-05** (P0) ✅ 2026-07-02 — `UNIQUE` removido de `push_tokens.peer_id` no `init.sql` (unique composto `(peer_id, device_id)` mantido). Bancos dev existentes: `make db-reset`.
- [x] **SRV-06** (P2) ✅ 2026-07-02 — `.env.example` reescrito e sincronizado (APNs, TURN_STATIC_SECRET, PEER_ID_SEED, RELAY_*, ENABLE_TTL_CLEANUP; `APNS_CERT_PASSWORD` removido; URLs locais default com produção comentada).
- [x] **SRV-07** (P2) ✅ 2026-07-02 — Makefile em `docker compose` V2; `db-reset` corrigido; `make health` cobre postgres/redis/store/push/turn-credentials/identity/bootstrap/signaling/coturn; novos `logs-identity`/`logs-signaling`.

**Extras descobertos e corrigidos durante a Fase 1 (2026-07-02):**
- **ENV-03** `cmake` também é pré-requisito (audiopus_sys/feature voip) — adicionado ao `check-prereqs` e docs; CMake 4.x quebra o build do opus vendorizado → política pinada em `.cargo/config.toml` (`CMAKE_POLICY_VERSION_MINIMUM=3.5`), válido para o workspace e para `desktop/src-tauri`.
- **ENV-04** `tokio-tungstenite` sem a feature `connect` — **o fallback de signaling do cliente (`core/src/voip/signaling_server.rs`) nunca compilou com a feature `voip`** (7 erros, invisíveis ao `cargo check` default porque voip é opcional). Corrigido em `core/Cargo.toml`; `cargo check -p mepassa-core --features voip` agora passa. Recomendação: incluir `--features voip` no CI (TST-04).
- **ENV-05** JDK: Gradle 8.5 exige Java 17–21 (o default da máquina era 25) — documentado em `local.properties.example`.
- **ENV-06** `desktop/dist` precisa existir para `tauri::generate_context!` — basta `npm run build` uma vez (documentar no onboarding de devs).

**Milestone M1:** dev novo builda core+server+desktop+android+ios seguindo docs; `make up` sobe postgres, redis, store, push, bootstrap, coturn, turn-credentials, identity e signaling. **Status: ✅ concluído em 2026-07-02** (pendências de validação: `docker compose build` dos novos serviços — docker indisponível neste shell — e `xcodegen generate` — não instalado).

---

## FASE 2 — Crashes e fluxos quebrados evidentes (M2, parte 1)

Objetivo: nenhum crash conhecido; wrappers deixam de mentir (stubs com sucesso fake).

### Android
- [x] **AND-03** (P0) ✅ 2026-07-02 — `connectedPeers.toInt()` no `getString` (`MePassaService.kt`); crash `IllegalFormatConversionException` eliminado.
- [x] **AND-04** (P0) ✅ 2026-07-02 — envs `MESSAGE_STORE_URL`/`SIGNALING_SERVER_URL` agora configuradas em `MePassaApplication.onCreate` (`configureCoreEnvironment()`), antes de qualquer `initialize()`; duplicação removida do service.
- [x] **AND-05** (P1) ✅ 2026-07-02 — core `ffi/client.rs`: `expect` no build do client substituído por saída graciosa da thread (comandos subsequentes retornam erro FFI controlado em vez de abort do processo).

### iOS
- [x] **IOS-04** (P0) ✅ 2026-07-02 — `sendDocumentMessage`/`sendVideoMessage` reais em `MePassaCore.swift` (chamadas FFI com conversão Data→[UInt8]). *Validar com xcodebuild após gerar as libs Rust.*
- [x] **IOS-05** (P1) ✅ 2026-07-02 — reações reais (`getMessageReactions`/`addReaction`/`removeReaction` via FFI síncrono).
- [x] **IOS-06** (P1) ✅ 2026-07-02 — `AppDelegate.pushManager` com `didSet` que atribui o delegate de `UNUserNotificationCenter` no momento da injeção.
- [x] **IOS-07** (P1) ✅ 2026-07-02 (antecipado na Fase 1) — `NSLocalNetworkUsageDescription` + `NSBonjourServices` (`_p2p._udp`) adicionados ao `Info.plist` e ao `project.yml`. Validar descoberta LAN em devices físicos no primeiro teste.

### Desktop
- [x] **DSK-01** (P1) ✅ 2026-07-02 — comando `switch_camera` criado em `commands.rs` e registrado no `generate_handler!`.
- [x] **DSK-02** (P2) ✅ 2026-07-02 — `formatTime` no ChatView multiplica por 1000 (created_at em segundos).
- [x] **DSK-03** (P2) ✅ 2026-07-02 — `localPeerIdRef` (ref) elimina a closure obsoleta do setInterval; `is_own_message` correto.
- [x] **DSK-04** (P2) ✅ 2026-07-02 — App.tsx já tinha tela de erro+Retry (auditoria superestimou); complemento: botão "Get Started" desabilitado com "Initializing..." enquanto não há peer ID. `tsc --noEmit` OK.

---

## FASE 3 — Núcleo de mensagens confiável (M2, parte 2)

Objetivo: mensagem 1:1 nunca se perde silenciosamente; entrega é observável.

- [x] **CORE-01** (P0) ✅ 2026-07-02 — `ensure_peer_connected_with` aguarda a conexão com deadline de 10s após o dial (poll de 200ms); os dois métodos unificados.
- [x] **CORE-02** (P0) ✅ 2026-07-02 — tabela `outbound_queue` (migration v5) + `storage/outbox.rs` (com testes) + worker no builder com backoff exponencial (5s→15min, purge em 14d). `DeliveryOutcome.queued` → status Pending.
- [x] **CORE-03** (P1) ✅ 2026-07-02 — mapa `pending_outbound` (request_id→mensagem, só Text/Encrypted) no swarm; `OutboundFailure` → `MessageHandler::requeue_failed_outbound` (enfileira + regride status para Pending); ACK remove do mapa.
- [ ] **CORE-04** (P1) **PARCIAL** — `run_network` corrigido (loop de poll; antes segurava o write-lock para sempre = deadlock). **Pendente:** processar inbound fora do lock — `handler.handle_incoming_message().await` (decrypt Signal + SQLite + fs) roda dentro do `poll_once` sob write-lock (`swarm.rs:642`); refactor invasivo, fazer com TST-02/03 no lugar. — 1,5d
- [x] **CORE-05** (P1) ✅ 2026-07-02 — `add_reaction`/`remove_reaction` async reais; `block_in_place`/`block_on` removidos (`broadcast_reaction`).
- [x] **CORE-06** (P1) ✅ 2026-07-02 (por decisão) — requisito de `LocalSet` para `ClientBuilder::build` formalizado e documentado (NetworkManager é `!Sync` pelo transport do Swarm, com ou sem voip); testes de builder/client rodam em `LocalSet` como o caminho FFI de produção.
- [x] **CORE-07** (P2) ✅ 2026-07-02 — `FfiMessageEventCallback` no UDL (received/status/typing, eventos thin com IDs) + `register_message_event_callback`; adapter para o `ClientEvent` interno; bindings Kotlin/Swift regenerados (uniffi 0.31.2). Destrava EVT-01/02/03.
- [x] **CORE-08** (P3) ✅ 2026-07-02 (commit da Fase 2) — `expect` no build → saída graciosa da thread.
- [ ] **CORE-09** (P2) Canais unbounded (`message_handler.rs:56`, `builder.rs:143`): definir bounds + política de descarte com log. *Adiado: mudar emit_event síncrono para bounded exige revisão dos call sites; baixo risco para alfa.* — 0,5d

**Milestone M2:** texto e mídia 1:1 A↔B nas 4 plataformas, com status correto (Sent→Delivered), sem crash e sem perda silenciosa. Roteiro de teste: `docs/guides/testing-manual.md`.

---

## FASE 4 — VoIP ponta-a-ponta (M3)

Objetivo: callee recebe a chamada em todas as plataformas; estados corretos.

- [x] **AND-06** (P0) ✅ 2026-07-02 — `FfiCallEventCallback` registrado no `initialize()`; eventos expostos como StateFlows; NavHost navega para IncomingCall. *Validação de compile pendente (sem Android SDK nesta máquina).*
- [x] **DSK-05** (P0) ✅ 2026-07-02 — `CallEventEmitter` no init_client emite `voip:incoming_call`/`call_state`/`call_ended`; `IncomingCallModal` renderizado no App com notificação; `call_ended` fecha modal/tela. tsc + cargo check OK.
- [x] **IOS-08** (P1) ✅ 2026-07-02 — `startVoiceCall()` via `callManager.startCall`. Telas órfãs CallScreen/IncomingCallScreen: manter decisão para depois (CallKit cobre a UI de chamada nativa).
- [x] **IOS-09** (P1) ✅ 2026-07-02 — `.connecting` até o core reportar ACTIVE (caller e CXAnswerCallAction); áudio inicia na transição real em `handleCallStateChanged`.
- [x] **AND-07** (P2) ✅ 2026-07-02 — CallScreen observa `callState`/`callEnded`; timer só conta em ACTIVE; encerramento remoto fecha a tela.
- [x] **CORE-10** (P2) ✅ 2026-07-02 — `pending_ice` buffer (cap 64/chamada) + `drain_pending_ice` ao registrar o peer.
- [x] **CORE-11** (P2) ✅ 2026-07-02 — sinais em voo rastreados (`pending_voip_signals`); `OutboundFailure` reenvia via canal fallback → servidor WebSocket (consumidor no `VoIPIntegration::spawn`).
- [x] **CORE-12** (P2) ✅ 2026-07-02 — canal CallEvent 128→1024; expect do Opus → erro; `FfiCall.video_enabled/video_codec` reais (novos campos em `Call`).
- [ ] **DSK-06** (P1) Captura e envio de vídeo local no desktop: comando `send_video_frame` + captura via `getUserMedia` no frontend. *Decisão 2026-07-02: desktop fica recepção-somente de vídeo nesta fase; UI já mostra placeholder no preview local.* — 2d
- [ ] **AND-08** (P1) Validar caminho de áudio `cpal` no Android em **device real** (não validável nesta máquina); se falhar, decidir: backend AAudio/Oboe via JNI ou envio de frames do Kotlin (`send_audio_frame` já existe no FFI). — 0,5d+
- [x] **CORE-13** (P3) ✅ 2026-07-02 — `enable_video` força fallback VP9→VP8 com warning.
- [x] **CORE-14** (P3) ✅ 2026-07-02 — `voip/pipeline.rs` e `voip/video_pipeline.rs` (código morto) removidos.

**Milestone M3:** chamada de voz iOS↔Android↔Desktop com atender/recusar/desligar/mute; vídeo onde suportado. Roteiro: `docs/guides/video-calls-checklist.md`.

---

## FASE 5 — Grupos robustos (M4)

Objetivo: grupo multi-dispositivo funcional e íntegro; sem hack de chave por mensagem de texto.

- [ ] **CORE-15** (P0) Transmitir contador na `EncryptedMessage` de grupo (`crypto/group.rs:32-36,139-178`): incluir `iteration`/índice da sender key; receptor avança a ratchet até o índice (com janela para out-of-order). *Aceite:* teste com perda/reordenação de mensagem no gossipsub continua decifrando.* — 1,5d
- [ ] **CORE-16** (P0) Protocolo in-band de grupo: mensagens de protocolo tipadas (protobuf) para invite/join/leave/membership-update e **distribuição de sender key via sessão 1:1 E2E** (substituindo o hack `mepassa-group-key:v1:` sobre texto). Validar remetente contra membership antes de aceitar seed. Atualizar `group/manager.rs:137-171` para processar membership recebida. — 3d
- [ ] **AND-09** (P1) Android: remover o parsing de `mepassa-group-key:` por string-matching (`MePassaClientWrapper.kt:589-619`) quando CORE-16 chegar; validar origem. — 0,5d
- [ ] **DSK-07** (P1) Desktop: idem — remover aceitação de sender key de qualquer peer (`ConversationsView.tsx:103-150`, `ChatView.tsx:73-127`) e ligar `join_group` ao fluxo de convite. — 0,5d
- [ ] **IOS-10** (P0) iOS: descomentar chamadas reais em `GroupListView.swift:102-119` (loadGroups) e `:237-255` (createGroup); ligar `leaveGroup` real (`GroupInfoView.swift:150-157`). *Aceite:* grupos criados aparecem e persistem.* — 0,5d
- [ ] **DSK-08** (P2) Desktop: botão Leave Group com onClick (`GroupChatView.tsx:354-356`); Group Info com lista de membros real. — 0,5d
- [ ] **AND-10** (P2) Android: lista de membros real no `GroupInfoScreen.kt:157-170`; adicionar `update_group` ao FFI (nome/descrição) e ligar edição (`:587-589`). — 1d
- [ ] **IOS-11** (P2) iOS: edição de grupo real (`GroupInfoView.swift:321-332`) usando o mesmo `update_group`. — 0,25d
- [ ] **CORE-17** (P2) Assinar mensagens de grupo no nível sender-key (hoje autenticação só no envelope `GroupMessage.sign`) — avaliar se o envelope basta e documentar a decisão. — 0,5d

**Milestone M4:** criar grupo no device A, convidar B e C, todos trocam mensagens; perda de mensagem não quebra o grupo. Teste automatizado de dessincronização incluído.

---

## FASE 6 — Baseline de segurança (M5)

Objetivo: apto a testes com dados reais. Nada de plaintext silencioso; backend não é um open relay.

### Core
- [ ] **SEC-01** (P1) Remover downgrade silencioso para plaintext (`client.rs:276-287,336-347,1452-1463,1604-1615`): falha de E2E ⇒ erro para o caller (ou flag explícita `allow_plaintext` default false). *Aceite:* teste que força falha de sessão não gera pacote em claro.* — 1d
- [ ] **SEC-02** (P1) Autorização de mídia: `build_media_chunks` (`message_handler.rs:502-513`) só serve chunks a peers participantes da conversa da mídia. — 0,5d
- [ ] **SEC-03** (P1) Integridade de mídia: verificar `media_hash` após remontagem dos chunks (`message_handler.rs:456-500`); cifrar chunks com a sessão E2E (ou chave de conteúdo derivada). — 1d
- [ ] **SEC-04** (P1) Persistir sessões Signal + identidades TOFU em SQLite cifrado (`crypto/signal.rs:182-186,366-386`). *Aceite:* restart não reseta pinning nem quebra sessões.* — 1,5d
- [ ] **SEC-05** (P1) Cifrar sender-key seeds no SQLite (`group/storage.rs:260-300`) com a storage key. — 0,5d
- [ ] **SEC-06** (P1) Eliminar `identity.key` plaintext: core aceitar identidade só via provider (env/callback já existe para Keychain/Keystore); não gravar arquivo na primeira execução (`builder.rs:104-110`); migração remove arquivos legados. Android: limpar `MEPASSA_IDENTITY_B64` do ambiente pós-init (`MePassaClientWrapper.kt:64`). — 1d
- [ ] **SEC-07** (P2) Prekeys: marcar Kyber OTP como usada (`signal.rs:355-362`); `peek_one_time_prekey` não reusar sempre a mesma OPK (`prekeys.rs:221-258`); persistir prekey pool para o bundle sobreviver a restart (`identity/storage.rs:158-165`). — 1d
- [ ] **SEC-08** (P2) Safety numbers/fingerprint para verificação de identidade (mitigar TOFU MITM) — pode ficar pós-alfa, registrar decisão. — 2d (opcional nesta fase)
- [ ] **CORE-18** (P2) Signaling client: exigir `wss://` por default; só aceitar `ws://` com flag explícita de dev (`signaling_server.rs:117-127`). — 0,25d

### Backend
- [ ] **SEC-09** (P1) Autenticação por assinatura Ed25519 (peer assina payload+timestamp) em: store GET/DELETE (`store/src/api.rs:61-113`), push register/send (`push/src/api/*`), turn-credentials (`handlers.rs:40-43`). Definir formato comum (header `X-MePassa-Signature`). — 2d
- [ ] **SEC-10** (P1) Identity: implementar verificação de assinatura no `PUT /prekeys` (`identity/handlers.rs:64-67`). — 0,5d
- [ ] **SEC-11** (P1) Signaling: exigir prova de posse do peer_id no `Register` (challenge assinado); validar `from_peer_id` contra a conexão; rate-limit e limite de payload. — 1d
- [ ] **SEC-12** (P1) Segredos: TURN secret via env no `turnserver.conf` (template + envsubst ou flag), remover default `mepassa_turn_dev_secret` de `config.rs:22`; chave do bootstrap de arquivo/env secreto em vez de `SHA256(seed pública)` (`bootstrap/main.rs:259-273`); remover defaults de credenciais de DB em código (`store/main.rs:32-37`). — 1d
- [ ] **SEC-13** (P1) coturn: `external-ip` via env/template (`turnserver.conf:11`), consumindo `TURN_EXTERNAL_IP` do .env. — 0,25d
- [ ] **SEC-14** (P2) Identity: assinatura de registro cobrir `peer_id`+`public_key` (`handlers.rs:130`); checar timestamp antes da verificação; erros de assinatura → 400, não 500. — 0,5d
- [ ] **SEC-15** (P2) Rate limit por IP real da conexão (fallback quando sem proxy) em vez de só `x-forwarded-for` (`rate_limit.rs:48-56`). — 0,5d
- [ ] **SEC-16** (P2) Restringir CORS nos serviços (`push`, `store`, `identity`, `turn-credentials`). — 0,25d
- [ ] **AND-11** (P1) Android: remover `usesCleartextTraffic="true"` (`AndroidManifest.xml:41`). — 0,1d

**Milestone M5:** pentest interno básico: peer não-autorizado não lê store alheio, não registra push alheio, não baixa mídia alheia, não sobe prekeys alheias; nenhum payload em claro observável no wire.

---

## FASE 7 — Push e entrega offline (M6)

- [ ] **PSH-01** (P1) Migrar FCM para HTTP v1 (OAuth2/service account) — a Legacy API (`fcm 0.9`, `fcm.rs:48`) foi desligada pelo Google. Falhar o startup se credencial ausente (hoje sobe com chave vazia e falha silenciosamente). — 1,5d
- [ ] **AND-12** (P1) Android: reativar plugin google-services (`build.gradle.kts:4`), adicionar `google-services.json` (e documentar como obter — `FIREBASE_SETUP.md`). *Aceite:* token FCM registrado no push-server.* — 0,5d
- [ ] **PSH-02** (P1) Integração store→push: quando mensagem entra no store e destinatário offline, disparar push (consumir o canal Redis já publicado em `store/api.rs:38-40`, ou chamada HTTP direta store→push). — 1d
- [ ] **PSH-03** (P1) iOS: implementar PushKit (`PKPushRegistry`) + report via CallKit para chamadas com app morto — obrigatório com background mode `voip` (risco de rejeição na App Store). — 1,5d
- [ ] **PSH-04** (P2) Store: resposta idempotente para duplicata (`database.rs:57-67` — `ON CONFLICT ... RETURNING` com `fetch_optional` + SELECT). — 0,25d
- [ ] **PSH-05** (P2) Purgar mensagens `delivered` antigas (hoje só `pending` expiradas — `database.rs:150-159`). — 0,25d
- [ ] **PSH-06** (P2) Navegação por push (abrir conversa/chamada ao tocar a notificação) — iOS `PushNotificationManager.swift:163-187` (depende de IOS-06), Android PendingIntent (`MePassaService.kt:149`). — 1d
- [ ] **PSH-07** (P3) Push: healthcheck no compose; env `PORT` vs `SERVER_PORT` unificada. — 0,25d

**Milestone M6:** device offline recebe push, abre o app e a mensagem chega via store; checklist `docs/guides/push-checklist.md` verde.

---

## FASE 8 — Eventos em vez de polling + fluxos de identidade (M7, parte 1)

Depende de CORE-07 (callback de mensagens no FFI).

- [ ] **EVT-01** (P2) Android: substituir polling (ChatScreen 2s, Conversations 5s, GroupChat 3s) por `FfiMessageEventCallback`; corrigir bug do `filtered.size > messages.size` (`ChatScreen.kt:314`) que esconde updates de status/deleção. — 1d
- [ ] **EVT-02** (P2) iOS: substituir timers (`MePassaApp.swift:173-182`, `ChatView.swift:442-447`) pelo callback. — 1d
- [ ] **EVT-03** (P2) Desktop: emitir eventos Tauri de mensagem a partir do callback; remover polling (ChatView 2s, Conversations 5s/30s, Group 3s/10s); de quebra elimina notificações duplicadas (`ChatView.tsx:132-147` + `ConversationsView.tsx:50-68`). — 1d

### Identidade: backup/restore funcionais
- [ ] **IDN-01** (P1) Definir fluxo de import ANTES do init do client: Android — onboarding pergunta "restaurar?" antes de `initialize()` (remover auto-init do `MainActivity.kt:61` ou condicioná-lo); resolver corrida do `OnboardingScreen.kt:43-50`. — 1d
- [ ] **IDN-02** (P1) iOS — mesmo problema: `MePassaApp.init()` inicializa antes da LoginView; suportar init adiado ou reinit para import (`MePassaCore.swift:79-81`). — 1d
- [ ] **AND-13** (P1) Android: ligar Settings/Profile/Search/MediaGallery/MediaViewer ao NavHost (`MePassaNavHost.kt`) — sem isso export de backup e troca de prekeys ficam inacessíveis. — 0,5d
- [ ] **DSK-09** (P2) Desktop: UI de backup/restore de identidade (export/import — comandos keychain já existem em `commands.rs:90-131`). — 1d

---

## FASE 9 — Higiene de testes e CI (M7, parte 2)

- [ ] **TST-01** (P1) Consertar compilação dos testes:
  - `core/src/media/image.rs:194,206` — imports `Cursor`/`ImageFormat` no módulo de teste;
  - `core/src/identity_client.rs:337` — teste com `?` retornar `Result`;
  - `core/tests/voip_integration.rs` + `message_integration.rs` — atualizar para a API atual do `Client`;
  - `server/identity/tests/integration_tests.rs` — ed25519-dalek 2.x (`SigningKey`) + base64 0.22 (`Engine`);
  - `server/bootstrap` — `tempfile` nas dev-dependencies.
  *Aceite:* `cargo test --workspace` compila 100%.* — 1,5d
- [ ] **TST-02** (P1) Reativar `test_end_to_end_message_exchange` (`message_integration.rs:131`) com verificação real de recepção (hoje `#[ignore]` e sem assert). — 1d
- [ ] **TST-03** (P2) Novos testes cobrindo os P0 corrigidos: race de conexão (CORE-01), retry offline (CORE-02), grupo com perda de mensagem (CORE-15), chunks de mídia com hash (SEC-03). — 2d
- [ ] **TST-04** (P2) CI (GitHub Actions): `cargo fmt --check`, `clippy -D warnings` (após TST-05), `cargo test --workspace`, `tsc --noEmit`, build Android debug. — 1d
- [ ] **TST-05** (P3) Zerar os 55 warnings do clippy (10× conversão inútil `SignalProtocolError`, base64 deprecado no store, `Arc` não-Send/Sync, etc.). — 1d
- [ ] **TST-06** (P3) Remover código morto: `crypto/ratchet.rs`, `crypto/session.rs` antigos (não compilam, órfãos), `group/sender_keys.rs` órfão, presença Redis não usada no store (`redis_client.rs:41-72`). — 0,5d

---

## FASE 10 — UX debt e polimento (M7, parte 3)

### Feature parity (FFI pronto, UI faltando)
- [ ] **UX-01** (P2) Android: ligar forward (`ChatScreen.kt:504-517` — FFI pronto) e envio de vídeo no chat (`:399,581-582`). — 1d
- [ ] **UX-02** (P2) Desktop: envio de mídia (expor comandos `send_image/voice/document_message` + UI de anexo); UI de reações e forward (hoje inexistentes). — 2d
- [ ] **UX-03** (P2) Desktop: ligar comandos órfãos — `connect_to_peer` (adicionar contato por multiaddr/QR) e `search_messages` (UI de busca). — 1d

### Settings/Profile (3 plataformas)
- [ ] **UX-04** (P2) Logout real: Android (`SettingsScreen.kt:294`), iOS (`SettingsView.swift:130` → chamar `AppState.logout()` existente). — 0,5d
- [ ] **UX-05** (P2) Cache/armazenamento real: cálculo de uso e limpeza de cache de imagem/vídeo (Android `SettingsScreen.kt:195-214`, iOS `SettingsView.swift:83-92`). — 1d
- [ ] **UX-06** (P3) Profile: avatar picker + salvar display name (Android `ProfileScreen.kt:107,142`; iOS `ProfileView.swift:46,69`); exibir nome em vez de peerId truncado nas conversas. — 1,5d
- [ ] **UX-07** (P3) Versão exibida vinda do build config (hoje "1.0.0 (Beta)" hardcoded divergindo de 0.1.0-alpha) — Android e iOS. — 0,1d
- [ ] **UX-08** (P3) Licenças/termos/privacidade (links reais ou remover entradas). — 0,25d

### Media viewer
- [ ] **UX-09** (P2) Android: MediaViewerScreen real (hoje stub declarado) — zoom, share, save. — 1d
- [ ] **UX-10** (P3) iOS: share/save no MediaViewerView (`:150,169`) + `NSPhotoLibraryAddUsageDescription`. — 0,5d

### Ciclo de vida / plataforma
- [ ] **AND-14** (P2) Foreground service: avaliar tipo (`dataSync` tem limite ~6h no Android 14+) — considerar `connectedDevice`/exemption de bateria; PendingIntent na notificação; ícone próprio; full-screen intent para chamada recebida em background. — 1d
- [ ] **DSK-10** (P3) HashRouter em vez de BrowserRouter (`main.tsx`); revisar `window.location.reload()`; limpar `voipState` do localStorage ao encerrar chamadas; remover plugin dialog não usado ou adicionar capability; guard no StrictMode double-init. — 0,5d
- [ ] **IOS-12** (P3) ~~`UIRequiredDeviceCapabilities` → `arm64`~~ (✅ feito na Fase 1); resta: `setBadgeCount` (API nova) e logs com `os.Logger` em vez de `print` com dados sensíveis. — 0,4d
- [ ] **AND-15** (P3) `processedGroupKeyMessageIds` persistido (hoje só memória); remover `println` de erros de mídia (`ChatScreen.kt:298,355`) por tratamento real. — 0,5d
- [ ] **UX-11** (P3) Desktop: preview real da última mensagem (`ConversationsView.tsx:291`); unificar idioma PT/BR da UI; remover botão share enganoso do QRCodeModal. — 0,5d

### Rede (pós-alfa, registrar)
- [ ] **NET-01** (P2) NAT detection real: adicionar AutoNAT (libp2p) ao behaviour em vez da heurística (`nat_detection.rs:60-106`). — 1,5d
- [ ] **NET-02** (P3) `ConnectionType` correto com eventos DCUtR (`swarm.rs:467-472`); contador de peers do bootstrap com `saturating_sub` (`bootstrap/main.rs:119-124`). — 0,5d
- [ ] **NET-03** (P3) Substituir busy-poll de 10ms do swarm por waker adequado (`swarm.rs:435-441`). — 1d
- [ ] **SRV-16** (P3) Identity: porta default sem conflito com store (8080); signaling: porta/log via env (`signaling/main.rs:42,52`); `uptime_seconds` usando `state.start_time`. — 0,5d

### Fora de escopo desta rodada (decidir e documentar)
- **SYNC-01** Multi-device sync (CRDT/Automerge) — `core/src/sync/` é placeholder. Decisão: adiar para pós-beta ou cortar do roadmap.
- **SFU-01** Chamadas em grupo (SFU vs mesh) — plano da Fase B do doc antigo. Adiar.
- **SEC-08** Safety numbers — se não entrar em M5, agendar para beta.

---

## Sequência recomendada e paralelização

```
Semana 1:  FASE 1 (M1)  ───────────────┐
Semana 2:  FASE 2 + FASE 3 (M2)        │  Fases 1–2 paralelizáveis por plataforma
Semana 3:  FASE 4 (M3) ‖ FASE 6-backend│  (1 dev core, 1 dev mobile, 1 dev backend)
Semana 4:  FASE 5 (M4) ‖ FASE 6-core   │
Semana 5:  FASE 7 (M6) ‖ FASE 9        │
Semana 6+: FASE 8 + FASE 10 (M7)       │
```

- **Trilha Core** (1 dev): CORE-01..09 → CORE-15/16 → SEC-01..08 → NET-*
- **Trilha Backend** (1 dev): SRV-* → SEC-09..16 → PSH-01/02/04/05
- **Trilha Apps** (1–2 devs): AND-*/IOS-*/DSK-* das fases 2, 4, 5 → EVT-* → UX-*

Regra de corte para o alfa: **M1 + M2 são obrigatórios**; M3/M4 entram no alfa se prontos, senão ficam atrás de feature flag; M5 é obrigatório antes de qualquer teste com usuários externos.
