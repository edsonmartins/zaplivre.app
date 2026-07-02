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
- [x] **CORE-04** (P1) ✅ 2026-07-02 — COMPLETO: inbound (requests E gossip de grupo) agora é coletado no poll e processado FORA do write-lock (`Client::poll_network_once` → `process_inbound_request`); ACK/chunks reencaixados com reaquisição curta. Validado pelos E2E (mensagem + retry).
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

- [x] **CORE-15** (P0) ✅ 2026-07-02 — reestruturado além do planejado: derivação **stateless** de message keys por (seed, counter) com counter no wire; resolve perda, reordenação E restart (a seed persistida sozinha dessincronizava após restart); counter persistido por sender (migration v6, guarda de replay); mesma seed re-recebida preserva counter. Testes: perda/replay/restart/preservação. Trade-off de FS documentado no módulo.
- [x] **CORE-16** (P0) ✅ 2026-07-02 — `GroupControlEnvelope` (invite/sender_key/member_added/member_removed/leave) via mensagens 1:1 E2E (padrão ReactionEnvelope); `add_group_member` envia invite+member_added automaticamente; convidado entra, subscreve topic e responde sender_key a todos; validações anti-spoofing (sender_key só de membros, membership só de admins — métodos `remote_*`); orquestração na task de eventos do builder. *Nota: envelope com seed sem sessão E2E ainda cai em plaintext com warning — SEC-01 endurece.*
- [x] **AND-09** (P1) ✅ 2026-07-02 — hack removido do wrapper e telas; mantido só filtro de exibição para mensagens legadas.
- [x] **DSK-07** (P1) ✅ 2026-07-02 — varredura/parsing/envio manuais removidos; join agora é automático via invite do core.
- [x] **IOS-10** (P0) ✅ 2026-07-02 (antecipado) — loadGroups/createGroup/leaveGroup reais via MePassaCore (mocks removidos). *Validar com xcodebuild no primeiro build iOS.*
- [x] **DSK-08** (P2) ✅ 2026-07-02 — Leave Group funcional + lista de membros no modal (novos comandos tauri).
- [x] **AND-10** (P2) ✅ 2026-07-02 — lista de membros real (Você/Admin) + edição via novo `update_group` no FFI.
- [x] **IOS-11** (P2) ✅ 2026-07-02 — `saveChanges` real via `updateGroup`.
- [x] **CORE-17** (P2) ✅ 2026-07-02 (por decisão) — a assinatura Ed25519 do envelope externo (`GroupMessage.sign`, verificada contra a chave pública do contato em `handle_gossipsub_message`) cobre autenticidade e integridade do payload cifrado; assinatura adicional no nível sender-key seria redundante para o alfa. Reavaliar se sender keys forem compartilhadas fora do gossipsub assinado.

**Milestone M4:** criar grupo no device A, convidar B e C, todos trocam mensagens; perda de mensagem não quebra o grupo. Teste automatizado de dessincronização incluído.

---

## FASE 6 — Baseline de segurança (M5)

Objetivo: apto a testes com dados reais. Nada de plaintext silencioso; backend não é um open relay.

### Core
- [x] **SEC-01** (P1) ✅ 2026-07-02 — falha de criptografia E2E **aborta o envio** em todos os caminhos (texto, mídia inline, forward, reação, group control) via `prepare_outgoing_payload` unificado; sem sessão (peer sem bundle) plaintext continua com warning alto; `MEPASSA_REQUIRE_E2E=true` proíbe também esse caso (default off — troca de prekeys ainda não é automática nos apps).
- [x] **SEC-02** (P1) ✅ 2026-07-02 — chunks só servidos a peers da conversa da mídia (sender/recipient da mensagem dona).
- [x] **SEC-03** (P1) ✅ 2026-07-02 — hash (puro ou salted) verificado na remontagem; mismatch descarta o arquivo. *E2E dos chunks em si continua pendente (Noise de transporte cobre o wire).*
- [x] **SEC-04** (P1) ✅ 2026-07-02 — sessões Signal (cifradas com storage key) e identidades TOFU persistidas em SQLite (migration v7), restauradas no startup. Restart não reseta pinning nem sessões.
- [x] **SEC-05** (P1) ✅ 2026-07-02 — seeds cifradas em repouso (AES-GCM), fallback de leitura para formato legado; preservação de counter movida para Rust.
- [x] **SEC-06** (P1) **PARCIAL** ✅ 2026-07-02 — `identity.key` legado apagado quando a identidade vem do secure storage; Android limpa `MEPASSA_IDENTITY_B64` do ambiente após o build. *Pendente: eliminar a escrita do arquivo na PRIMEIRA execução (exige API de export no FFI para a migração das plataformas).*
- [ ] **SEC-07** (P2) Prekeys: persistir o pool (bundle muda a cada restart) e rotação de OPK. *Nota 2026-07-02: `mark_kyber_pre_key_used` é no-op sobre a kyber last-resort (reutilizável por design); o fix real de OPK exige pool server-side no identity server.* — 1d
- [ ] **SEC-08** (P2) Safety numbers/fingerprint — **adiado para beta** (decisão 2026-07-02).
- [x] **CORE-18** (P2) ✅ 2026-07-02 — URL sem esquema assume `wss://`; `ws://`/`http://` explícitos geram warning.

### Backend
- [x] **SEC-09** (P1) **PARCIAL (store completo)** ✅ 2026-07-02 — message store exige Ed25519 em POST/GET/DELETE (headers `x-mepassa-peer/ts/sig`, chave extraída do peer ID); GET/DELETE restritos ao peer autenticado; core assina as 3 chamadas. *Pendente: push register/send e turn-credentials (exigem API de assinatura no FFI para os apps chamarem — os apps fazem essas chamadas, não o core).*
- [x] **SEC-10** (P1) ✅ 2026-07-02 — `PUT /prekeys` verifica assinatura contra a chave pública registrada.
- [x] **SEC-11** (P1) ✅ 2026-07-02 — Register assinado (prova de posse do peer ID); relay só de conexões registradas com `from_peer_id` forçado ao peer autenticado; limite de 64KB; porta via env.
- [x] **SEC-12** (P1) ✅ 2026-07-02 — TURN secret via linha de comando do compose (fora do conf; stack exige env); bootstrap prefere chave aleatória persistida (`PEER_ID_SEED` mantém compat com warning); store sem credenciais default embutidas.
- [x] **SEC-13** (P1) ✅ 2026-07-02 — `external-ip` via `TURN_EXTERNAL_IP` na linha de comando do coturn.
- [x] **SEC-14** (P2) ✅ 2026-07-02 — assinatura de registro cobre username+peer_id+public_key+timestamp (core+servidor); timestamp antes; 400 em erro de assinatura.
- [x] **SEC-15** (P2) ✅ 2026-07-02 — IP real da conexão (ConnectInfo); x-forwarded-for só de proxy em rede privada.
- [x] **SEC-16** (P2) ✅ 2026-07-02 — CORS permissivo removido dos 4 serviços.
- [x] **AND-11** (P1) ✅ 2026-07-02 — `usesCleartextTraffic` removido.

**Milestone M5:** pentest interno básico: peer não-autorizado não lê store alheio, não registra push alheio, não baixa mídia alheia, não sobe prekeys alheias; nenhum payload em claro observável no wire.

---

## FASE 7 — Push e entrega offline (M6)

- [x] **PSH-01** (P1) ✅ 2026-07-02 — FCM HTTP v1 (OAuth2/service account, JWT RS256, cache de token); crate `fcm 0.9` removido; FCM opcional como o APNs (sem chave vazia silenciosa); `FCM_SERVICE_ACCOUNT_PATH` no compose/stack.
- [x] **AND-12** (P1) ✅ 2026-07-02 — plugin google-services aplicado **condicionalmente** (build funciona sem `google-services.json`, com warning); basta colocar o arquivo em `android/app/` para habilitar. *Validar token registrado no push-server no primeiro teste em device.*
- [x] **PSH-02** (P1) ✅ 2026-07-02 — `PushNotifier` no store dispara push via push-server ao armazenar mensagem offline (fire-and-forget; conteúdo nunca vai no push); `PUSH_SERVER_URL` no compose.
- [ ] **PSH-03** (P1) iOS PushKit + report CallKit — **pendente de campo**: exige credenciais APNs voip (.p8 + entitlement), extensão do push-server para push type `voip` (token separado, plataforma nova no schema) e device físico para validar. Fazer junto do primeiro ciclo de testes iOS.
- [x] **PSH-04** (P2) ✅ 2026-07-02 — duplicata idempotente (fetch_optional + SELECT do registro existente).
- [x] **PSH-05** (P2) ✅ 2026-07-02 — purge de mensagens `delivered` >7 dias no job de TTL.
- [x] **PSH-06** (P2) ✅ 2026-07-02 (Android) — o fluxo PendingIntent→MainActivity→NavHost já existia; corrigido para abrir a conversa do REMETENTE (`sender_peer_id`). iOS: delegate corrigido na Fase 2; navegação ao tocar depende do teste em device (mesmo ciclo do PSH-03).
- [x] **PSH-07** (P3) ✅ 2026-07-02 (feito na Fase 1) — healthcheck no compose e `PORT` unificada.

**Milestone M6:** device offline recebe push, abre o app e a mensagem chega via store; checklist `docs/guides/push-checklist.md` verde.

---

## FASE 8 — Eventos em vez de polling + fluxos de identidade (M7, parte 1)

Depende de CORE-07 (callback de mensagens no FFI).

- [x] **EVT-01** (P2) ✅ 2026-07-02 — SharedFlow de eventos no wrapper; Chat/Conversations coletam eventos (polling vira safety net de 30s); bug do `size >` removido. *Grupos continuam com polling (eventos de grupo têm canal próprio não exposto — futuro).*
- [x] **EVT-02** (P2) ✅ 2026-07-02 — `MessageEventHandler` (novo arquivo — rodar xcodegen) via NotificationCenter; timers 2s/5s → safety net 30s.
- [x] **EVT-03** (P2) ✅ 2026-07-02 — eventos Tauri `message:received/status/typing`; polling 2s/5s → safety net 30s; notificações duplicadas eliminadas.

### Identidade: backup/restore funcionais
- [x] **IDN-01** (P1) ✅ 2026-07-02 — auto-init (MainActivity e service) condicionado à existência de identidade; primeira execução decide criar/restaurar no Onboarding, que inicia o service ao concluir.
- [x] **IDN-02** (P1) ✅ 2026-07-02 — guard `hasExistingIdentity` no launch; pós-init extraído (`completeCoreSetup`) e disparado pela LoginView via `.mePassaCoreStarted` — criar/restaurar funciona sem reiniciar o app.
- [x] **AND-13** (P1) ✅ 2026-07-02 — Settings/Profile/Search no NavHost com ícones na barra de conversas (backup e prekeys acessíveis). MediaGallery/Viewer ficam para UX-09.
- [ ] **DSK-09** (P2) Desktop: UI de backup/restore de identidade (comandos keychain já existem). — 1d

---

## FASE 9 — Higiene de testes e CI (M7, parte 2)

- [x] **TST-01** (P1) ✅ 2026-07-02 — TODOS os testes compilam: lib (Fase 3), message_integration reescrita p/ API atual, voip_integration gateada por feature, identity server migrado (dalek 2.x, base64 0.22, assinatura SEC-14, porta 8083, #[ignore] p/ infra viva), tempfile no bootstrap, exemplos obsoletos removidos. **`cargo test --workspace`: 178 passed / 0 failed / 11 ignored.**
- [x] **TST-02** (P1) ✅ 2026-07-02 — teste E2E REAL: dois Clients completos por TCP local (LocalSet + drivers de rede), assert de recepção no B e de Delivered (ACK) no A. Roda em ~5s e valida CORE-01/02/03 de quebra (a entrega passou pela fila de retry no próprio teste).
- [x] **TST-03** (P2) ✅ 2026-07-02 — CORE-01 coberto pelo E2E (envio logo após dial); CORE-02: novo `tests/reliability.rs` (peer offline → fila → worker entrega + ACK quando o peer sobe, 5s); CORE-15: já coberto (loss/replay/restart em crypto::group); SEC-03: 2 testes de integridade de chunks (aceita hash válido, rejeita adulterado e apaga o .part). Novas suítes incluídas no CI.
- [x] **TST-04** (P2) ✅ 2026-07-02 — `.github/workflows/ci.yml`: check workspace + `--features voip`, testes core (lib + suítes funcionais) e servers, typecheck desktop. fmt/clippy entram após TST-05.
- [x] **TST-05** (P3) **PARCIAL** ✅ 2026-07-02 — `clippy --fix` + base64 deprecado corrigido + allows anotados: 55 → ~15 warnings (restantes: naming `from_str`, too-many-args, `Arc` !Send intencional do FFI).
- [x] **TST-06** (P3) ✅ 2026-07-02 — 1223 linhas de crypto órfã removidas (ratchet/session/sender_keys) + 2 exemplos obsoletos; presença Redis mantida com `#[allow(dead_code)]` anotado (integração futura com push).

---

## FASE 10 — UX debt e polimento (M7, parte 3)

### Feature parity (FFI pronto, UI faltando)
- [x] **UX-01** (P2) **PARCIAL** ✅ 2026-07-02 — forward com seletor de conversas implementado. *Envio de vídeo no chat ainda pendente.*
- [x] **UX-02** (P2) **PARCIAL** ✅ 2026-07-02 — anexo de arquivos funcional (comando `send_file_message`: imagens comprimidas, resto documento, cap 50MB; dialog plugin habilitado). *Reações/forward na UI desktop ainda pendentes.*
- [ ] **UX-03** (P2) Desktop: ligar comandos órfãos — `connect_to_peer` (adicionar contato por multiaddr/QR) e `search_messages` (UI de busca). — 1d

### Settings/Profile (3 plataformas)
- [x] **UX-04** (P2) ✅ 2026-07-02 — logout destrutivo com aviso explícito nas duas plataformas (apaga identidade segura + dados locais).
- [x] **UX-05** (P2) ✅ 2026-07-02 — uso de armazenamento calculado de verdade + limpeza de caches funcional (Android e iOS).
- [ ] **UX-06** (P3) Profile: avatar picker + salvar display name (Android `ProfileScreen.kt:107,142`; iOS `ProfileView.swift:46,69`); exibir nome em vez de peerId truncado nas conversas. — 1,5d
- [x] **UX-07** (P3) ✅ 2026-07-02 — versão vem de `BuildConfig.VERSION_NAME` / `CFBundleShortVersionString`.
- [ ] **UX-08** (P3) Licenças/termos/privacidade (links reais ou remover entradas). — 0,25d

### Media viewer
- [ ] **UX-09** (P2) Android: MediaViewerScreen real (hoje stub declarado) — zoom, share, save. — 1d
- [ ] **UX-10** (P3) iOS: share/save no MediaViewerView (`:150,169`) + `NSPhotoLibraryAddUsageDescription`. — 0,5d

### Ciclo de vida / plataforma
- [ ] **AND-14** (P2) **PARCIAL** — ✅ PendingIntent e ícone do app na notificação do service. Pendente: tipo do service (limite ~6h do `dataSync` no Android 14+), exemption de bateria e full-screen intent para chamada em background. — 0,5d
- [ ] **DSK-10** (P3) HashRouter em vez de BrowserRouter (`main.tsx`); revisar `window.location.reload()`; limpar `voipState` do localStorage ao encerrar chamadas; remover plugin dialog não usado ou adicionar capability; guard no StrictMode double-init. — 0,5d
- [ ] **IOS-12** (P3) ~~`UIRequiredDeviceCapabilities` → `arm64`~~ (✅ feito na Fase 1); resta: `setBadgeCount` (API nova) e logs com `os.Logger` em vez de `print` com dados sensíveis. — 0,4d
- [ ] **AND-15** (P3) `processedGroupKeyMessageIds` persistido (hoje só memória); remover `println` de erros de mídia (`ChatScreen.kt:298,355`) por tratamento real. — 0,5d
- [ ] **UX-11** (P3) Desktop: preview real da última mensagem (`ConversationsView.tsx:291`); unificar idioma PT/BR da UI; remover botão share enganoso do QRCodeModal. — 0,5d

### Rede (pós-alfa, registrar)
- [ ] **NET-01** (P2) NAT detection real: adicionar AutoNAT (libp2p) ao behaviour em vez da heurística (`nat_detection.rs:60-106`). — 1,5d
- [ ] **NET-02** (P3) `ConnectionType` correto com eventos DCUtR (`swarm.rs:467-472`); contador de peers do bootstrap com `saturating_sub` (`bootstrap/main.rs:119-124`). — 0,5d
- [ ] **NET-03** (P3) Substituir busy-poll de 10ms do swarm por waker adequado (`swarm.rs:435-441`). — 1d
- [ ] **SRV-16** (P3) Identity: porta default sem conflito com store (8080); signaling: porta/log via env (`signaling/main.rs:42,52`); `uptime_seconds` usando `state.start_time`. — 0,5d

### Cobertura de testes de UI (adicionado 2026-07-02)
- [x] **UIT-01** Desktop: 12 testes de tela (vitest + Testing Library + mockIPC do Tauri) cobrindo os 5 fluxos historicamente quebrados, incluindo o anti-regressão do "callee cego" (modal de chamada no evento) e comandos inexistentes (mock explode). `npm test` + CI.
- [x] **UIT-02** Mobile: flows Maestro black-box em `e2e/maestro/` (onboarding, navegação anti-órfã, enviar mensagem, backup, grupo) — rodar contra emulador/simulador com o app real (ver README).
- [ ] **UIT-03** Fluxos de 2 dispositivos (receber mensagem/chamada no mobile): manter no roteiro manual; automatizar depois do primeiro ciclo de testes se valer o custo.

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
