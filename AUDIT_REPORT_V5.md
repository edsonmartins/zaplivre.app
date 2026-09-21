# Auditoria Técnica V5 — ZapLivre

**Data:** 2026-09-20
**Base:** branch `main` @ `bb4e14d`
**Escopo:** core (Rust), server (bootstrap/identity/push/store/signaling/turn-credentials/coturn/postgres/monitoring), apps (android/ios/desktop), e2e, CI/CD, devops.
**Método:** cinco frentes de análise estática independentes (criptografia do core; rede e mensageria do core; backend; apps móveis; desktop + infra/processo), instruídas a não confiar no `AUDIT_REPORT_V4.md` nem no `PLANO_HOMOLOGACAO.md` e a conferir tudo contra o código. Em paralelo, verificação prática na máquina: `cargo fmt --check`, `cargo clippy --workspace --all-targets -- -D warnings`, `cargo test --workspace`.

As cinco frentes foram concluídas. Além da leitura de código, as frentes de apps e de infra inspecionaram binários (`.so`/`.a`), consultaram o histórico do CI (`gh run list/view`), rodaram `docker compose config` e sondaram DNS/HTTP dos endpoints públicos. A frente de backend e a de infra leram também o repositório irmão `zaplivre-devops` (deploy de produção).

**Legenda de evidência:** ✅ = conferido diretamente no código/na máquina pelo consolidador; os demais itens foram verificados pela frente responsável por leitura de código. "Não executado" = depende de runtime e não foi rodado.

---

## 1. Veredito

**NÃO está pronto para homologação.** E, no estado atual, **não é uma alternativa crível ao WhatsApp** — a distância é arquitetural, não de polimento.

| Área | Nota | Veredito |
|---|---|---|
| Criptografia e segurança do core | 4/10 | Não pronto |
| Rede, mensageria e FFI do core | 3/10 | Não pronto |
| Serviços de backend | 4/10 | Não pronto |
| App Android | 4,5/10 | Não pronto |
| App iOS | 3,5/10 | Não pronto |
| App Desktop | 3,5/10 | Não pronto |
| Entrega, infra e processo | 3/10 | Não há ambiente de homologação funcionando |

Os motivos, em ordem de gravidade:

0. **O ambiente para o qual os apps apontam está fora do ar.** `identity.zaplivre.app/health` responde HTTP 502 e `dht1`/`dht2.zaplivre.app` não têm registro DNS (conferido em 2026-09-20). Um testador que instale qualquer app hoje não registra username, não faz bootstrap e não usa store-and-forward.
1. **A `main` não passa em nenhum dos três gates do próprio projeto** (fmt, clippy, testes). Dois erros de compilação impedem rodar a suíte do workspace. No CI, a `main` está vermelha há 5 commits (desde 11/08), e como o fmt é o primeiro step, clippy e testes nem executam — os commits de verificação de identidade e de transparência de chaves nunca passaram por teste em CI. Os jobs de E2E aparecem verdes por `continue-on-error`, mas falham.
2. **Qualquer nó da rede pode se passar por qualquer contato.** O remetente declarado na mensagem não é comparado com o peer autenticado, e mensagens em plaintext são sempre aceitas na recepção. Isso vale também para comandos de administração de grupo.
3. **As funcionalidades de segurança recém-adicionadas não entregam a garantia que prometem.** O safety number e a transparência de chaves não detectam um servidor de identidade malicioso, porque a identidade Signal não é ligada à identidade verificável.
4. **A infraestrutura P2P, que é o núcleo do produto, não funciona fora de LAN.** O relay é código morto no cliente, o bootstrap fica em modo cliente no Kademlia, o TURN anuncia um hostname interno do docker, e as chaves privadas dos nós de bootstrap de produção são deriváveis de seeds públicas.
5. **A entrega offline — na prática o caminho principal — perde e duplica mensagens por desenho**, e grupos não têm entrega confiável.

O que é real e bom: E2E 1:1 com libsignal (PQXDH + Double Ratchet, Kyber1024 obrigatório), envio de texto que falha fechado, persistência cifrada de sessões, mensagens de grupo assinadas, backend com SQL parametrizado, assinatura Ed25519 com body-hash, segredos obrigatórios e comparação constant-time. A base é séria; a integração em volta dela é que está aberta.

O `PLANO_HOMOLOGACAO.md` marca como concluídos itens que o código não sustenta (CI verde, relay/NAT, sem plaintext, rotação de sender key). A conclusão "pronto para homologação" registrada em 09/08 não se sustenta para a `main` de hoje.

---

## 2. Evidências práticas (rodadas na máquina) ✅

| # | Verificação | Resultado | Evidência |
|---|---|---|---|
| V1 | `cargo fmt --all -- --check` | **Falha** | 39 diffs em 11 arquivos: `core/src/ffi/client.rs` (12), `core/src/api/client.rs` (7), `core/src/network/message_handler.rs` (4), `core/src/voip/{webrtc,integration,signaling,video,rtp_video}.rs`, `server/identity/src/{db,transparency}.rs`, `core/examples/signaling_probe.rs` |
| V2 | `cargo clippy --workspace --all-targets -- -D warnings` | **Falha** | `server/identity/src/transparency.rs:24` — `explicit_counter_loop` |
| V3 | `cargo test --workspace` | **Não compila** | `server/push/src/apns.rs:341` — `missing field content_available in initializer of apns::Aps` (E0063) |
| V4 | idem, sem o crate de push | **Não compila** | `core/examples/signaling_probe.rs:90` — `no method named encode found for GeneralPurpose` (E0599, falta importar o trait `Engine`) |
| V5 | idem, `--lib --bins --tests`, sem push | Passa | core unit 148; `message_integration` 3; `p2p_messaging` 2; `plaintext_integration` 1; `relay_integration` 16; `reliability` 4; turn-credentials 6; store 3 (+2 ignorados); bootstrap 1; identity 1 |
| V6 | Cobertura efetiva | **Buracos** | `voip_integration`: 0 testes executados; `identity_integration`: 0 (atrás de feature); identity server: 8 de integração todos `#[ignore]`; **signaling: 0 testes**; push: não compila em teste |

Nenhum teste falhou no que compila, mas VoIP, fluxo de identidade com servidor, signaling e push ficam sem verificação automatizada. `relay_integration` só testa structs — o relay nunca é exercitado de verdade.

---

## 3. P0 — Bloqueadores de homologação

### 3.1 Processo / build

**P0-A. `main` vermelha nos três gates (V1–V4), e o CI não segura.** ✅ `AGENTS.md` e `CONTRIBUTING.md` exigem fmt, clippy e testes limpos. `gh run list --workflow ci.yml`: falha em `07a8135` (11/08), `ebe696c`, `9b27c92`, `6b91e41` e `bb4e14d` (15/08); último verde `5b70107` (09/08). O fmt é o primeiro step, então clippy e todos os testes ficam pulados. A branch `main` não tem proteção e recebe push direto. O Android CI também falha em "Run unit tests" desde 14/08. É o P0 nº 1 da V4, reincidente.
*Correção:* corrigir `apns.rs:341`, importar `base64::Engine` em `signaling_probe.rs`, ajustar o laço em `transparency.rs:24`, rodar `cargo fmt` (trabalho de horas); separar o fmt em job próprio para não mascarar os testes; proteger `main` com checks obrigatórios.

**P0-A2. E2E vermelho no CI, mascarado como verde.** ✅ `.github/workflows/e2e-android.yml:33` e `e2e-ios.yml:34` têm `continue-on-error: true` no nível do job. No run iOS `31900789799` (commit `6b91e41`), **7 de 8 flows falharam** (só `05_backup_identidade` passou); no run Android `31900789816` o step falhou com `adb: device offline` e nenhum flow produziu veredito. O "10/10 Android e 8/8 iOS verdes" do plano vem de execuções locais de 8–10/08, sem artefato, anteriores às mudanças de verificação de identidade. Lint e ktlint também têm `continue-on-error`.
*Correção:* remover o `continue-on-error`, estabilizar o emulador, separar os flows `2dev_*` (o `config.yaml` inclui 12 flows de dois devices, impossíveis num emulador só).

**P0-A3. Não existe ambiente de homologação funcionando.** ✅ Sondagem em 2026-09-20: `identity`, `store`, `push`, `signal` e `turn.zaplivre.app` resolvem para 31.97.240.217 e respondem **HTTP 502** (Caddy no ar, backends não); `dht1` e `dht2.zaplivre.app` **não têm registro DNS**; portas 4001, 4002 e 3478/tcp fechadas ou filtradas. Os três apps fixam essas URLs. O ambiente estar fora do ar sem que ninguém tenha sido alertado é a prova prática da ausência de monitoramento.
*Correção:* subir o stack, criar os registros DNS, abrir as portas, e criar um ambiente de homologação separado (`*.hml.zaplivre.app`) com URL configurável nos apps.

**P0-A4. Schema de produção não é provisionado e não existem migrações.** `server/postgres/MIGRATIONS.md:4` declara "Status: Planejado"; não há `sqlx::migrate!` nem pastas `migrations/`; `store` e `push` não criam tabelas; o compose do devops usa um Postgres compartilhado (`bluevix-postgres`) sem montar `init.sql`. O plano marca B5 como `[x] Feito` quando só houve documentação. Um deploy limpo deixa `usernames`, `offline_messages` e `push_tokens` inexistentes → 500 em register e store.
*Correção:* implementar `sqlx::migrate!` com `0001_init.sql`.

### 3.2 Segurança do core

**P0-B. Falsificação de remetente.** ✅
`validate_message` (`core/src/network/message_handler.rs:319-350`) só checa que `sender_peer_id` não está vazio. A comparação com o peer autenticado pelo Noise existe apenas no sync de prekeys (linha 182). O receptor aceita `Payload::Text` em plaintext sempre, mesmo com `ZAPLIVRE_ALLOW_PLAINTEXT` desligado, e não sinaliza à UI que a mensagem não é cifrada.
*Ataque:* qualquer nó libp2p conecta em Bob e envia `Message{sender_peer_id=<Alice>, Text{"mudei de número, faz um pix…"}}`; a mensagem entra na conversa de Alice como legítima. O mesmo vale para `MediaOffer`, reações, read receipts, typing e ACK (dá para marcar mensagens alheias como Failed ou Read).
*Correção:* rejeitar quando `message.sender_peer_id != from_peer`; recusar `Text`, `MediaOffer` e envelopes de controle fora de `Encrypted`, salvo com a variável de dev; no caminho offline, confiar só no remetente provado pela sessão Signal. É o item mais barato e de maior impacto de toda a auditoria.

**P0-C. Controle de grupo aceito em plaintext com remetente forjável.**
`message_handler.rs:364-369` emite `GroupControl` a partir de Text plaintext com `from_peer_id = message.sender_peer_id`; as checagens `is_admin`/`is_group_member` (`api/builder.rs:592-602`, `group/manager.rs:382,415`) operam sobre esse valor. Forjando o PeerId do admin, o atacante remove ou adiciona membros, ou substitui a sender key de um membro (DoS do grupo). Agravante: `invite` de qualquer peer faz auto-join, e criador e membros são ditados pelo remetente (`builder.rs:499-525`).
*Correção:* a do P0-B, mais aceitar controle de grupo só dentro de `handle_encrypted_message`, exigir prova de admin e consentimento do usuário para convite.

**P0-D. Mídia acima de 512 KiB sai sem E2E Signal.**
`api/client.rs:35,1101-1142,1238-1267,1369-1398,1518-1547` enviam `MediaOffer` em plaintext; os `MediaChunk` também são plaintext de aplicação (`message_handler.rs:763-777`). Com o peer offline, a oferta (nome do arquivo, mime, tamanho, hash) vai em claro para o store (`client.rs:789-793`). Vídeo, documento e foto grande dependem só do Noise do libp2p: sem PQ, sem ligação com a identidade Signal. Viola o contrato "nunca plaintext por padrão".
*Correção:* oferta e pedido dentro da sessão Signal, chave AES-GCM aleatória por arquivo, transferir só ciphertext (modelo de anexos do Signal/WhatsApp).

**P0-E. Identidade Signal não ligada à identidade verificável.**
A chave de identidade Signal é gerada separada do Ed25519 (`identity/storage.rs:293-299`). O fingerprint cobre só o Ed25519 (`client.rs:77-92,2929-2939`), que já é auto-certificado pelo PeerId. A assinatura de `register` não cobre o bundle (`identity_client.rs:294-298`), a de `update_prekeys` cobre só `peer_id:timestamp` (`:392-393`), e o cliente grava o bundle vindo do servidor sem verificação (`client.rs:593-622`).
*Ataque:* um identity server malicioso ou comprometido entrega um bundle próprio; o primeiro contato offline sofre MITM completo. Os usuários comparam safety numbers, os valores batem, e a UI mostra "verificado" sobre uma sessão interceptada.
*Correção:* assinar com Ed25519 a identidade Signal (de preferência o bundle inteiro) e verificar no cliente contra o PeerId; incluir a identidade Signal dos dois lados no safety number; persistir "verificado" e emitir evento na troca de chave.

**P0-F. DoS remoto por alocação não limitada no codec.** ✅
`core/src/network/messaging.rs:49-55` (e o par de leitura de request) faz `vec![0u8; len]` com `len` u32 lido da rede, sem teto. Qualquer peer conectado envia `FF FF FF FF` e o app tenta alocar 4 GiB → OOM-kill.
*Correção:* teto de 1–2 MiB e leitura incremental.

### 3.3 Confiabilidade do core

**P0-G. Relay e NAT traversal são código morto no produto.** ✅
`api/builder.rs:199` constrói o cliente com `NetworkManager::new`, que chama `with_relay(None, None)` (`network/swarm.rs:75`). `with_relay` só é chamado em `core/tests/relay_integration.rs:23`. Sem circuito de relay, o DCUtR nunca atua.
*Cenário:* dois celulares em 4G com CGNAT — o caso dominante no Brasil — nunca conectam direto. Todo o 1:1 cai no store (que tem os defeitos de P0-H e P0-I), grupos não funcionam (dependem de gossipsub direto), mídia >512 KB não funciona. Os testes em LAN via mDNS mascaram o problema.
*Correção:* configurar relay com os bootstraps, reservar slot, publicar `/p2p-circuit` no DHT, adicionar AutoNAT server.

**P0-H. Mensagem entregue aparece como "Failed".**
`deliver_message` grava no store **e** enfileira na outbox (`client.rs:785-812`). Na recepção não há dedup por `message_id` antes do decrypt (`message_handler.rs:446`). B busca no store; quando os dois conectam, `wake_pending_outbound` (`swarm.rs:591`) reenvia o mesmo ciphertext; o libsignal recusa a duplicata, B responde ACK Error e A marca como Failed (`message_handler.rs:245`). Acontece com toda mensagem entregue via store-and-forward.
*Correção:* dedup idempotente por `message_id` respondendo ACK Received; ACK de entrega pelo store; não tratar duplicata como falha.

**P0-I. Perda silenciosa de mensagens vindas do store.**
`handle_incoming_message` sempre retorna `Ok(ack)`, mesmo em falha (`message_handler.rs:133-139`), então o `if let Err` em `client.rs:1026` nunca dispara e o ID é apagado do servidor (`client.rs:1035-1063`). Falha de decrypt apaga a mensagem para sempre, sem aviso. Além disso o fetch roda só dentro de `bootstrap()` (`client.rs:2061-2070`), um lote de 100, sem paginação, sem fetch periódico e sem gatilho de push.
*Correção:* inspecionar `ack.status`, apagar só sucesso, paginar, expor `fetch_offline()` acionável por push.

**P0-J. Pool de one-time prekeys com 1 chave, sem reposição e sem persistir consumo.** ✅ (tamanho do pool)
`api/builder.rs:177` chama `init_prekey_pool(1)`. `replenish_prekeys`, `rotate_signed_prekey` e `update_prekeys` nunca são chamados em produção. `get_bundle` usa `peek` (`identity/prekeys.rs:324,359`) e entrega a mesma OTP a todos; `remove_pre_key` só altera memória (`crypto/signal.rs:415-426`); o snapshot só é salvo no primeiro boot (`builder.rs:178-187`). No servidor, `lookup` devolve o mesmo `one_time_prekey` a todos (`server/identity/src/db.rs:221-241`).
*Cenário:* A e C baixam o bundle de B; A envia primeiro e consome a OTP; a primeira mensagem de C falha no decrypt e, com P0-I, some. Após restart a OTP reaparece (reuso). Signed prekey e Kyber ficam estáticas para sempre. As duas frentes do core chegaram a isso de forma independente.
*Correção:* pool de ~100, consumo persistido, reposição, rotação periódica de SPK/Kyber, pop atômico no servidor, reset de sessão em erro.

**P0-K. Grupos sem entrega confiável.**
Publish via gossipsub sem fila nem retry (o próprio comentário admite, `client.rs:2752-2761`), sem store-and-forward de grupo, sem dial para membros. A verificação de assinatura exige `contact.public_key`, vazia para contatos criados por username (`client.rs:265,294`; `group/manager.rs:625-636`) → mensagens desse membro são descartadas. Falha de decrypt grava `plaintext=None` para sempre (`manager.rs:645-656`). Envelope de sender key vai sem outbox (`client.rs:821-884`).
*Cenário:* grupo de 5 com 2 offline — esses 2 nunca recebem. Quem entra enquanto outro está offline vê mensagens em branco para sempre.
*Correção:* fan-out 1:1 sobre o pipeline com store, como Signal e WhatsApp.

**P0-L. Fila de comandos da FFI é serial e o HTTP não tem timeout.**
`run_client_task_arc` faz `while let Some(cmd) = recv().await { … .await }` (`ffi/client.rs:442`); `reqwest::Client::new()` sem timeout (`client.rs:143`, `builder.rs:338`, `voip/turn.rs:42`). Envio para peer offline segura a fila ~5 s; `download_media` até ~45 s; com o store pendurado, trava sem prazo. Durante isso `list_conversations`, `get_messages` e `send_audio_frame` bloqueiam → UI congelada (ANR) e áudio de chamada cortado.
*Correção:* `spawn_local` por comando, leituras de DB fora da fila, timeouts em todo HTTP, `spawn_blocking` para CPU/IO.

### 3.4 Backend / infraestrutura

**P0-M. Chaves privadas dos bootstrap/relay nodes de produção são públicas.** ✅
`stack.yml:125,163` e `docker-compose.yml:109` definem `PEER_ID_SEED=bootstrap-1/2`; a chave é `SHA256(seed)` (`server/bootstrap/src/main.rs:332-347`), e o próprio código loga "INSECURE" nesse caminho. A frente de backend derivou os peer IDs a partir das seeds e obteve exatamente os IDs pinados no cliente (`core/src/ffi/client.rs:1118-1119`).
*Ataque:* qualquer pessoa com acesso ao repositório se apresenta na rede como dht1/dht2, sem precisar de DNS hijack: eclipse da descoberta, interceptação de circuitos relay, DoS.
*Correção:* chave aleatória persistida (o código já suporta), publicar os novos peer IDs nos apps **antes** de distribuir builds de homologação. Depois disso, a troca exige release de cliente.

**P0-N. Bootstrap em Kademlia client-mode; relay devolve reserva sem endereços.** (não executado)
`server/bootstrap/src/{main,behaviour}.rs` nunca chamam `add_external_address` nem `set_mode(Server)`. libp2p-kad 0.45 inicia em `Mode::Client` e só vira Server com endereço externo confirmado; libp2p-relay 0.17 monta os `addrs` da reserva a partir de `external_addresses` — com lista vazia o cliente falha com `NoAddressesInReservation`.
*Consequência:* mesmo corrigindo P0-G no cliente, o servidor não atende queries DHT nem concede reservas úteis.
*Correção:* env `EXTERNAL_ADDR`, `swarm.add_external_address`, `kademlia.set_mode(Some(Mode::Server))`, teste de integração contra o binário real.

**P0-O. TURN inutilizável em produção.** ✅ (default no código)
`server/turn-credentials/src/config.rs:28` — `TURN_HOST` tem default `"coturn"`, hostname interno do docker. As frentes de backend e de infra, de forma independente, constataram que nenhum compose passa `TURN_HOST` ao container (só existe como comentário em `.env.example:20`). O serviço entregaria `turn:coturn:3478`, que não resolve. Resíduo do P1-6 da V4.
Mais grave: **nenhum cliente consome o TURN.** `TurnCredentialsClient` nunca é instanciado fora de testes; o caminho Rust usa só `stun:stun.l.google.com:19302` (`core/src/voip/manager.rs:169-170`), e o WebRTC nativo dos apps é criado sem ICE servers (ver M3, seção 7). O README:161 afirma "✅ TURN client integration". Chamadas entre redes com CGNAT ou 4G falham, e o IP dos usuários vaza para o Google.
*Correção:* tornar `TURN_HOST` obrigatório, falhando no boot se ausente ou igual a `coturn`; buscar credenciais efêmeras assinadas nos clientes e popular os `iceServers`.

**P0-P. Healthchecks de produção chamam binários ausentes da imagem.** (não executado)
`devops/Dockerfile.server` instala só `ca-certificates` e roda como uid 65532; `stack.yml:97,137,175,211,280,309` usa `curl`. Containers ficam `unhealthy` permanentemente (ciclo de restart no Swarm). Provável: volume `/app/data` nasce root-owned e o uid 65532 não cria `dht.db`.
*Correção:* subcomando `--healthcheck` no binário (ou curl na imagem) e `chown` de `/app/data` no Dockerfile.

---

## 4. P1 — Antes de beta/produção

### 4.1 Criptografia
| # | Gap | Evidência |
|---|---|---|
| C1 | **Key transparency não resiste a servidor malicioso:** raiz, `previous_hash` e log vêm do mesmo servidor na mesma consulta; sem raiz assinada, pinning, persistência, prova de consistência ou gossip (split-view trivial). Cobre só `peer_id`+Ed25519, já auto-certificado; não cobre username nem bundle Signal. Custo O(N) com laço controlado pelo servidor, sem teto (DoS) | `core/src/api/client.rs:2941-3019` |
| C2 | **Sem tratamento de troca de identidade:** falha fechado, mas sem API para aceitar nova chave, sem evento para UI, sem reset de sessão. Reinstalar ou corromper o banco quebra a conversa para sempre | `crypto/signal.rs:356-368` |
| C3 | **Identidade regenerada em silêncio:** erro ao ler `identity.key` gera keypair novo (que nem é salvo); o usuário perde peer ID, sessões e a storage key sem nenhum erro | `api/builder.rs:116-124` |
| C4 | **Grupos abaixo do padrão Signal/WhatsApp:** chave por mensagem = HKDF(seed, counter) sem ratchet → vazar a seed expõe todo o histórico; membro novo decifra mensagens anteriores capturadas (tópico gossipsub previsível); sem rotação na entrada; rotação na remoção é best-effort, sem epoch nem ack; AES-GCM sem AAD; counter monotônico descarta fora de ordem; overflow de `counter + 1` | `crypto/group.rs:50-64,141-190`, `builder.rs:617-626,751-765` |
| C5 | **Proteção em repouso parcial:** SQLite sem SQLCipher; contatos, grafo de conversas, texto de grupo (indexado no FTS), nomes de arquivo, mídia e miniaturas em claro. Storage key derivada deterministicamente da chave de identidade, sem Keystore/Secure Enclave | `core/Cargo.toml:60`, `storage/database.rs:47-56`, `client.rs:2735`, `identity/storage.rs:153-160` |
| C6 | **`handle_media_chunk` sem validação:** chunks aceitos sem oferta/pedido; `media_hash` vira nome de arquivo sem sanitização (path traversal, escreve `*.part` fora de `media/tmp`); `offset` i64 do remetente usado em `seek` (arquivo esparso gigante, enche o disco); hash só no último chunk | `message_handler.rs:630-709` |
| C7 | **Sinalização VoIP de fallback não autenticada fim a fim:** o servidor pode trocar fingerprints DTLS no SDP e interceptar chamadas; só o registro é assinado | `voip/signaling_server.rs:54-104` |
| C8 | **"Auditoria" do fork libsignal não é auditoria e tem erros factuais:** afirma que PQXDH não está ativo (o código exige Kyber1024); afirma "pin exato" (é caret); sem diff contra upstream, sem identificação de quem publica o `-syft`, checklist todo aberto. A raiz de confiança da criptografia é um crate beta de terceiro não auditado | `docs/AUDIT_LIBSIGNAL_SYFT.md`, `core/Cargo.toml:42` |

### 4.2 Rede, mensageria e FFI
| # | Gap | Evidência |
|---|---|---|
| N1 | **Polling ativo do swarm:** ≥15 wakeups/s permanentes + heartbeat gossipsub de 1 s, mDNS, Kademlia, ping. Incompatível com Doze, App Standby e background no iOS | `swarm.rs:544-551`, `ffi/client.rs:1200-1215` |
| N2 | **Android sem transporte DNS** ✅: o `cfg` cobre desktop e iOS; os bootstraps são `/dns4/…` → esperado `MultiaddrNotSupported`. P2-9 da V4 continua aberto | `network/transport.rs:64-79` |
| N3 | **Singleton FFI irrecuperável:** `new` sempre retorna Ok (falha só grava log); `OnceLock` impede logout/troca de conta/restore sem matar o processo; `expect` atravessa a FFI | `ffi/client.rs:88,1177-1187,1256` |
| N4 | **Transferência de mídia frágil:** arquivo inteiro em RAM, N requests de uma vez, sem garantia de ordem (chunk `is_last` antes dos demais → hash falha), timeout fixo de 10 s, sem resumo, exige remetente online; envio duplicado 4× | `message_handler.rs:745-780`, `client.rs:1073-1727,2912-2914` |
| N5 | **VoIP:** WS conecta uma vez no `build()`, sem timeout nem reconexão (morre na troca Wi-Fi↔4G); `TurnCredentialsClient` nunca instanciado — o caminho Rust usa só STUN do Google; sem timeout de ring; `call_history` nunca escrita | `voip/integration.rs:75-89`, `voip/manager.rs:151,169,998` |
| N6 | **Outbox:** entrada removida após `send_request`, sem esperar ACK; requeue depende de estado em RAM (crash perde a mensagem); só Text/Encrypted rastreados; worker serial com head-of-line de até ~100 s; cifra e avança o ratchet antes de persistir | `builder.rs:382-411`, `swarm.rs:454-460`, `client.rs:462-498` |
| N7 | **DHT sem validação e com teto de escala:** registro `zaplivre:addr:<peer>` sem assinatura (envenenável); `MemoryStore` default de 1024 registros em RAM → `put_record` rejeitado acima de ~1000 usuários; protocolo `/ipfs/kad/1.0.0` com bootstraps IPFS públicos como fallback | `swarm.rs:323-350`, `server/bootstrap/src/behaviour.rs:40`, `ffi/client.rs:1123-1128` |
| N8 | **Panic na task de rede mata o loop em silêncio:** `JoinHandle` descartado; o app para de receber sem erro visível | `ffi/client.rs:1187,1233` |

### 4.3 Backend
| # | Gap | Evidência |
|---|---|---|
| S1 | **Identity não vincula `peer_id` à `public_key`:** aceita qualquer string. Atacante registra um username com o `peer_id` da vítima (público na DHT) e a própria chave → a vítima nunca mais se registra (DoS permanente, não há delete), e `lookup?peer_id=` passa a devolver a chave do atacante | `server/identity/src/handlers.rs:25-35`, `db.rs:89-99` |
| S2 | **Assinaturas do identity não cobrem o bundle e não têm anti-replay:** `PUT /prekeys` capturado pode ser reenviado com bundle arbitrário por 5 min. O README afirma o contrário | `handlers.rs:31-34,88`, `README.md:181-182` |
| S3 | **Sem exclusão nem recuperação de conta (LGPD art. 18):** nenhum endpoint de delete, rename ou rotação; `unregister` do push só marca `is_active=false`. Perder o device = perder o username para sempre | `server/push/src/api/unregister.rs:30-31,58` |
| S4 | **Store sem resistência a spam/Sybil:** autenticação = posse de qualquer keypair; 120 req/min por peer ID gratuito; sem limite por IP, por destinatário ou global. Cada store dispara push visível → spam de notificação para qualquer `peer_id`. `PEER_REQUESTS` nunca remove chaves (memory leak) | `server/store/src/auth.rs:9-19,98-109`, `api.rs:72` |
| S5 | **Metadados expostos:** `sender_peer_id` vai no `data` do push → Google e Apple veem o grafo social. Store loga em INFO remetente, destinatário, id e tamanho. Bootstrap loga IPs; coturn em `verbose` com `peer_id` no username (liga IP a identidade). Mensagens entregues retidas 7 dias (o "delete" é UPDATE) | `push_notifier.rs:51-54`, `fcm.rs:164-166`, `database.rs:90-96,156,197-203` |
| S6 | **Signaling:** race na reconexão — a conexão antiga, ao cair, faz `remove(&peer_id)` sem checar o dono e apaga o registro novo (Wi-Fi→4G deixa o peer inalcançável para chamadas; vai aparecer na homologação de VoIP). Sem limite de conexões, timeout de registro, ping ou idle timeout. Estado in-memory (sem 2 réplicas). **Zero testes** | `server/signaling/src/main.rs:122,144-149,181-206` |
| S7 | **Parâmetros do relay semanticamente errados:** `max_circuit_bytes` recebe o valor de bytes/segundo (1 MB total por circuito) e `max_circuit_duration` = 120 s → mídia via relay falha | `server/bootstrap/src/behaviour.rs:69-70` |
| S8 | **APNs/FCM:** `InvalidProviderToken` (erro da chave do servidor) é tratado como token inválido e desativa tokens de usuários — uma `.p8` errada desativa todo o iOS. No FCM v1 o código procura as strings legadas; tokens mortos nunca são desativados | `server/push/src/api/send.rs:124-125,166-180` |
| S9 | **Rate-limit do identity inadequado:** por IP (CGNAT bloqueia legítimos; IPv6 /64 dá registros ilimitados); `GET`+`INCR`+`EXPIRE` não atômico (falha deixa chave sem TTL → IP bloqueado para sempre); endpoints de transparency sem limite; dump completo do diretório possível | `rate_limit.rs:24-36,95-136` |
| S10 | **TURN como relay aberto:** qualquer keypair obtém credencial; quotas em 0; `allow-loopback-peers` ativo e sem `denied-peer-ip` para IPv6 (SSRF em host com IPv6); sem `turns:` em TCP 443 | `server/coturn/turnserver.conf`, `turn-credentials/src/config.rs:63` |
| S11 | **Observabilidade e operação:** nenhum serviço expõe `/metrics`; `/health` do identity retorna `uptime_seconds` sempre 0 (`handlers.rs:124,133`); nenhum serviço axum tem graceful shutdown; migrações só existem como documento "Planejado"; um banco e um role com `GRANT ALL` para os três serviços | `server/postgres/init.sql:231-243` |
| S12 | **Anti-replay e rate-limit process-local** em store, push, turn e signaling: restart zera a janela; impede HA e rolling deploy. P1-4 da V4, aceito como limitação | `store/src/auth.rs:20-21` e análogos |

---

## 5. P2 — Média prioridade

- **Sem zeroização:** não há dependência `zeroize`; seeds e `storage_key` copiados por vários structs; `SenderKey`/`GroupSession` derivam `Debug` com a seed (`crypto/group.rs:84,201`).
- **Identidade por variável de ambiente** (`ZAPLIVRE_IDENTITY_B64`, `builder.rs:831-850`); `identity.key` em plaintext sem 0600 na primeira execução.
- **Conteúdo em claro no log:** `tracing::debug!("Received text: {}")` (`message_handler.rs:354`); SDP e IPs em INFO (`swarm.rs:498-503,836-841`).
- **`ZAPLIVRE_ALLOW_PLAINTEXT` lida em runtime em qualquer build** — deveria ser eliminada em release por `cfg`.
- **`encrypt_for_storage` sem AAD**; seeds legadas de 32 bytes em plaintext ainda aceitas (`group/storage.rs:320-324`).
- **Ordenação de mensagens:** `created_at` com resolução de segundo e hora local de recepção; timestamp do remetente descartado; `ORDER BY` sem desempate estável (`storage/messages.rs:165,198`).
- **Busca quebrada para 1:1:** o FTS indexa `content_plaintext`, e texto 1:1 grava `None` (`messages.rs:398`, `client.rs:493-494`). Trigger de UPDATE do FTS incorreto.
- **Migrações do core sem transação** (`storage/migrations.rs:83-99`).
- **Contatos poluídos:** PrekeyBundleSync enviado a todo peer conectado, inclusive bootstraps, criando um contato para cada (`swarm.rs:590-599`).
- **God-objects:** `api/client.rs` 3158 linhas, `ffi/client.rs` 2750, `message_handler.rs` 1332, `swarm.rs` 1031.
- **Backend:** `/health` do store expõe `pending_messages` sem auth; `/api/v1/send` do push roteado publicamente; `AppError::Database` devolve texto do sqlx ao cliente; `DATABASE_URL` do identity com credenciais default; `message_id` UNIQUE global permite suprimir mensagem alheia (deveria ser `(sender, message_id)`); `limit` negativo → 500; Dockerfiles por serviço rodam como root; imagens `:latest` não pinadas; Trivy não bloqueante.
- **Doc drift:** README do identity descreve errado a cobertura da assinatura; store e turn-credentials sem README.

---

## 6. É uma alternativa ao WhatsApp?

**Hoje não.** Inventário de capacidades no core:

| Capacidade | Estado |
|---|---|
| Texto 1:1 | Implementado, com P0 de spoofing e de confiabilidade |
| Mídia (imagem/vídeo/documento) | Parcial — >512 KB exige remetente online, sem E2E Signal, sem resumo |
| Notas de voz | Parcial — mesmo pipeline de mídia |
| Grupos | Parcial e frágil — sem entrega offline, sem retry, convite auto-aceito |
| Chamadas áudio/vídeo | Parcial — TURN não ligado no caminho Rust, sem timeout de ring, sem histórico |
| Recibo de entrega | Parcial — só no caminho P2P |
| Recibo de leitura | Stub — só recepção, nenhum código envia |
| Digitando | Stub — só recepção |
| Reações | 1:1 sem outbox; ausente em grupos |
| Responder (reply) | Stub — perdido no caminho cifrado |
| Encaminhar | Parcial — só texto, com prefixo literal |
| Busca | Quebrada para 1:1 |
| Descoberta de contatos | Parcial — só por @username, sem agenda/telefone |
| Presença / visto por último | Ausente |
| Editar mensagem | Ausente |
| Apagar para todos | Ausente (só soft delete local) |
| Mensagens temporárias | Ausente |
| Status/stories | Ausente |
| **Multi-device** | **Ausente** — `sync/mod.rs` é placeholder, `device_id` fixo em 1 |
| **Backup e restore** | **Ausente** |
| **Bloqueio** | **Ausente** — qualquer peer envia mensagem, convite e chunk |
| Denúncia / anti-abuso | Ausente |

**Riscos arquiteturais do desenho P2P, pelo que o código faz:**

1. **A entrega offline depende de um servidor** que foi tratado como "5% fallback" e é, na prática, o caminho principal. O sistema acaba centralizado sem as garantias de um centralizado (sem ACK, dedup, paginação ou gatilho de push).
2. **Celular não mantém swarm libp2p vivo em background.** iOS suspende, Android aplica Doze. Sem push acordando um fetch dedicado, não há entrega com o app fechado — e o polling de 15 wakeups/s drena bateria.
3. **Grupos via gossipsub** exigem membros online simultaneamente e conectados entre si. Grupos de centenas são inviáveis nesse desenho.
4. **A identidade é o PeerId — uma chave por instalação.** Multi-device, backup e restore exigem redesenho e não têm começo no código.
5. **Mídia P2P por pull** exige o remetente online no download; o padrão do mercado é blob cifrado em CDN.
6. **Metadados:** bootstrap, DHT e relay expõem o IP do usuário aos contatos e a qualquer nó.
7. **Rede aberta sem bloqueio nem anti-abuso.**
8. **Escala:** DHT em memória com teto de 1024 registros, 2 bootstraps, sem sharding.

Um **beta fechado de texto 1:1 entre usuários técnicos** é plausível depois dos P0. Um produto que o público geral adote no lugar do WhatsApp exige: entrega offline de primeira classe acionada por push, grupos por fan-out 1:1, anexos cifrados em servidor, multi-device, backup, bloqueio e anti-abuso.

---

## 7. Apps móveis (Android / iOS)

**Android 4,5/10 · iOS 3,5/10 — nenhum dos dois está pronto.** Hoje são um alfa para demonstração em LAN. Um ciclo em LAN com APK gerado pelo CI seria viável; fora da LAN, não.

### 7.1 Matriz de funcionalidades

**R** = real, **P** = parcial, **M** = mock/cosmético, **A** = ausente.

| Capacidade | Android | iOS | Evidência |
|---|---|---|---|
| Onboarding (criar/restaurar identidade) | R | R | `OnboardingScreen.kt:134-269`, `LoginView.swift:172-254` |
| Username | R (obrigatório) | P (opcional) | `ZapLivreNavHost.kt:85`; `LoginView.swift:156` |
| Iniciar conversa por username | R | **A** (`lookupUsername` nunca chamado) | `ConversationsScreen.kt:282` |
| Iniciar conversa por QR (scanner) | **A** (só exibe o QR) | R | `PeerQrCode.kt`; `NewChatView.swift:173` |
| Lista de contatos / agenda | A | A (`NSContactsUsageDescription` declarada sem uso) | `Info.plist:35` |
| Nome do contato na lista | P (cai para peerId truncado) | **A** (sempre `peerId.prefix(12)…`) | `ConversationsScreen.kt:310`; `ConversationsView.swift:121` |
| Chat 1:1 texto, status, paginação | R | R | `ChatViewModel.kt:127`; `ChatView.swift` |
| Imagem, vídeo, documento | P (sem compressão; OOM) | P (sem compressão) | `ChatScreen.kt:430-454` |
| Nota de voz | R | R | `VoiceRecordButton.kt`; `AudioRecorder.swift` |
| Reações, apagar, encaminhar, busca | R | R | wrappers e FFI |
| Responder/citar, editar, temporárias, stickers, localização | A | A | sem FFI e sem UI |
| Grupos | P (só texto) | P (só texto) | `GroupChatScreen.kt`, `GroupChatView.swift` |
| Chamada de áudio | P (sem toque, sem Telecom) | P (CallKit só em foreground) | `CallScreen.kt:69`; `CallManager.swift:322-347` |
| Chamada de vídeo | P (WebRTC **sem ICE servers**) | P (idem) | `NativeWebRtcSession.kt:77`; `.swift:36` |
| Chamada recebida com app fechado | A | **A** (sem PushKit) | sem `PKPushRegistry` |
| Notificação de mensagem | P (só via FCM, ver M5) | P (APNs não provisionado) | `NotificationHelper` tem um único chamador |
| Configurações | P (3 de 5 toggles funcionam) | **M** (5 toggles em `@State`) | `SettingsView.swift:13-17` |
| Privacidade (leitura, visto por último) | **M** | **M** | sem consumidor |
| Número de segurança | P (sem alerta de troca de chave) | R | `ChatScreen.kt:125-127,662`; `ChatView.swift:719-797` |
| Bloqueio, arquivar, silenciar, trava biométrica | A | A | sem FFI |
| Perfil | M (nome só local; avatar TODO) | M | `ProfileScreen.kt:119,158`; `ProfileView.swift:15,46` |
| Backup | P (só a chave, Base64 sem senha) | P (idem) | `ZapLivreClientWrapper.kt:273`; `ZapLivreCore.swift:114` |
| Logout | R | **P/bug** (ver 7.3) | `SettingsScreen.kt:226-231`; `ZapLivreApp.swift:167` |
| Relatório de crash | A | A | — |
| i18n | P (108 `Text("…")` hardcoded vs 17 `stringResource`) | A (sem `.strings`) | — |
| Acessibilidade | P (18 `contentDescription = null`) | fraca (2 `accessibilityLabel`) | — |

### 7.2 P0 dos apps

**M1. Libs nativas locais defasadas em relação aos bindings; no Android o core não inicializa.** Os bindings Kotlin/Swift foram regenerados em 14/08 (152 símbolos UniFFI). `jniLibs/arm64-v8a/libzaplivre_core.so` é de 12/08 e não tem 4 funções (`identity_fingerprint`, `contact_identity_fingerprint`, `contact_transparency_proof`, `get_conversation_messages_before`); a `x86_64` é de 08/08 e não tem 11. `app/build.gradle.kts:142-144` só recompila a lib se o arquivo não existir. Um `assembleDebug` nesta máquina reutiliza a lib velha → `UnsatisfiedLinkError` na criação do client; `ZapLivreClientWrapper.kt:259` captura só `Exception`, e o app cai na tela "Criar identidade" (efeito inferido, não executado). As `.a` do iOS são anteriores a `9b27c92` e `6b91e41` e não contêm essas correções. As arquiteturas dos ELF estão corretas — o item 12 da V4 está corrigido nesse aspecto.
*Correção:* `onlyIf` por mtime/hash de `core/`; gerar o APK de homologação só via CI com `cargo ndk` limpo e incluir arm64 (hoje o CI compila só x86_64, `e2e-android.yml:73`).

**M2. Android não resolve `/dns4`, e o default do app é `/dns4`.** ✅ (`transport.rs`) `core/src/network/transport.rs:64-79` exclui o transporte DNS para Android; `app/build.gradle.kts:61` define `/dns4/dht1.zaplivre.app/tcp/4001` como default e ainda sobrescreve o do core, perdendo o `dht2`. Todo dial de bootstrap falha com `MultiaddrNotSupported`: Android sem DHT e sem relay fora da LAN. Soma-se a P0-A3 (os registros DNS nem existem).
*Correção:* `dns::tokio::Transport::custom(...)` também no Android.

**M3. Videochamadas sem STUN/TURN.** ✅ `NativeWebRtcSession.kt:77` usa `RTCConfiguration(emptyList())`; `NativeWebRtcSession.swift:36` usa `RTCConfiguration()` sem `iceServers`. Só há candidatos host: vídeo em 4G ou CGNAT nunca conecta, e a infra de coturn fica sem uso (ver P0-O).

**M4. E2E vermelho no CI, mascarado** — ver P0-A2.

**M5. Notificações de mensagem no Android falham nos dois cenários comuns.** (a) `showMessageNotification` só é chamado pelo serviço FCM (`ZapLivreFirebaseMessagingService.kt:48`); mensagem que chega por P2P com o app em background só emite no `SharedFlow`, sem notificação. (b) O payload FCM carrega bloco `notification` (`server/push/src/fcm.rs:155-161`); com o app em background ou morto o Android exibe a bandeja sozinho e **não chama `onMessageReceived`**, então nada é baixado até o usuário abrir o app. E o toque na notificação entrega o extra `sender_peer_id`, mas `MainActivity.kt:167` lê `peer_id` — a conversa não abre.
*Correção:* FCM só com `data` e prioridade alta; notificação local a partir do callback do core; ler `sender_peer_id` no intent.

**M6. Chamadas não tocam com o app fechado.** iOS: `voip` e `PushKit.framework` declarados (`Info.plist:54`, `project.yml:167`), mas **nenhum `PKPushRegistry`**, e o servidor de push não envia VoIP; declarar `voip` sem usar PushKit é risco de rejeição na App Store. Android: sem `ConnectionService`, notificação de chamada sem toque em loop nem ações de atender/recusar (`ZapLivreService.kt:180-189`), sem `showWhenLocked`/`turnScreenOn`.

**M7. APNs real não provisionado** (C5 do plano, aberto e admitido). Sem isso o iOS não recebe push algum fora do foreground. Agravado por S8.

### 7.3 P1 dos apps

| # | Gap | Evidência |
|---|---|---|
| MA1 | **OOM ao enviar mídia no Android:** a FFI usa `List<UByte>` boxed; o app lê o vídeo inteiro (até 100 MB) e chama `.toUByteArray().toList()`. Um vídeo de 20–30 MB vira centenas de MB de heap; `OutOfMemoryError` não é `Exception` → crash. Sem compressão em nenhuma plataforma | `zaplivre.kt:2156-2168`, `ChatScreen.kt:430-454` |
| MA2 | **Logout quebrado no iOS:** apaga o Keychain mas não encerra o client, não apaga `zaplivre.db` nem a mídia, não desregistra o push. Restaurar backup em seguida falha ("Import requires app restart"); criar identidade nova mantém o DB do usuário anterior. O Android faz logout certo, mas nunca chama `unregisterToken` | `ZapLivreApp.swift:167-176`, `ZapLivreCore.swift:96-98`, `PushServerClient.kt:106` |
| MA3 | **Descoberta de contato assimétrica:** iOS sem busca por username; Android sem scanner de QR. Android→iOS só funciona se o usuário iOS registrou username (opcional no iOS) | grep vazio em `ios/`; `PeerQrCode.kt` |
| MA4 | **Toggles de privacidade sem efeito** nas duas plataformas ("Confirmações de leitura", "Última visualização"); no iOS os 5 toggles resetam ao reabrir a tela. UI enganosa | `SettingsView.swift:13-17` |
| MA5 | **Banco local em claro; no iOS entra em backup:** dados em `Documents/zaplivre_data` sem `isExcludedFromBackup` nem `FileProtection` → vão para iCloud/iTunes. Keychain sem `kSecAttrAccessible` | `ZapLivreCore.swift:19-20`, `KeychainStore.swift:36-53` |
| MA6 | **Android não alerta troca de chave do contato** (o iOS alerta). E `transparencyVerified = proof.isNotBlank()` exibe "incluída no log" para qualquer string não vazia | `ChatScreen.kt:126-127`, `ChatView.swift:787` |
| MA7 | **Sem relatório de crash e sem build de release:** Android sem `signingConfigs`, `versionCode 1` / `0.1.0-alpha`; nenhum workflow roda `assembleRelease` nem `archive`; regras ProGuard nunca exercitadas. iOS sem `PrivacyInfo.xcprivacy` nem `ITSAppUsesNonExemptEncryption` | `build.gradle.kts` |
| MA8 | **iOS sem ciclo de vida:** sem `scenePhase`, `beginBackgroundTask` nem reconexão no foreground; core só inicializa em `.onAppear`; num wake por push silencioso `connectToPeer` roda sem client; `completionHandler(.newData)` chamado imediatamente | `ZapLivreApp.swift:44-60`, `PushNotificationManager.swift:174-182`, `AppDelegate.swift:53-54` |
| MA9 | **Erros de envio de mídia engolidos no Android** (`// TODO: Show error to user`). Resíduo do P1-10 da V4 | `ChatScreen.kt:346,386` |

**P2:** `POST_NOTIFICATIONS` não pedida na primeira sessão; sem exceção de otimização de bateria nem `BOOT_COMPLETED`; `appVersion` hardcoded; iOS ainda envia seed de sender-key por DM de texto legado (`GroupInfoView.swift:246`) e faz polling a cada 30 s; "Exportar/Importar prekeys" exposto ao usuário final; `fatalError` em `AudioManager.swift:53`; harness `design_preview` no build de produção; sem `FLAG_SECURE`; testes mínimos (Android 18 JVM + 7 instrumentados; iOS 15; nada cobre chamadas, push ou mídia); `MESSAGE_STORE_URL` duplicada em `project.yml:48,79`.

**Sem problema (verificado):** `google-services.json` e `local.properties` nunca foram commitados (`git log --all` vazio) — o P1-12 da V4 não procede; `allowBackup=false`; identidade em `EncryptedSharedPreferences` / Keychain; sem cleartext nem exceções de ATS; domínios `*.zaplivre.app` consistentes entre plataformas; ícones presentes.

## 8. Desktop + entrega, infra e processo

### 8.1 Desktop — 3,5/10

Chat 1:1 de texto, imagem e arquivo e os grupos básicos são reais e bem ligados ao core; CSP e capabilities são razoáveis; a chave de identidade fica no Keychain. O resto:

| Área | Status | Evidência |
|---|---|---|
| Onboarding / username / restaurar identidade | Real (backup é Base64 cru da chave privada, sem senha, via clipboard) | `OnboardingView.tsx:30,49`; `commands.rs:917-953` |
| Chat 1:1 texto, busca, reações, encaminhar | Real | `ChatView.tsx:129-147,304` |
| Paginação de histórico | Ausente (limite fixo de 100) | `ChatView.tsx:172` |
| Apagar mensagem | Ausente (sem comando Tauri) | `main.rs:34-84` |
| Enviar imagem / arquivo | Real | `commands.rs:957-1010` |
| Enviar vídeo | Parcial (vai como documento) | `commands.rs:995-1007` |
| Gravar e enviar voz | Ausente | — |
| Receber vídeo | **Quebrado** (blob `video/mp4` renderizado em `<img>`) | `ChatView.tsx:459-470` |
| Receber voz | Provavelmente quebrado (CSP sem `media-src`; não executado) | `VoiceMessageBubble.tsx:72`; `tauri.conf.json:29` |
| Receber documento | Ausente (sem abrir nem salvar) | `ChatView.tsx:488` |
| Grupos: criar, listar, texto, adicionar, sair | Real, com polling de 3 s/10 s | `GroupChatView.tsx:47,137,157` |
| Grupos: remover membro, editar, entrar por convite | Ausente na UI (comando existe, nunca invocado) | — |
| **Chamada de voz** | **Só sinalização, sem nenhum áudio** (core compilado sem `voip_audio`; sem `send_audio_frame`) | `src-tauri/Cargo.toml:31`; `CallView.tsx:40-70` |
| Chamada de vídeo | Só recepção; "Camera: On" é apenas um rótulo | `VideoCallView.tsx:134-135,302-305` |
| **Verificação de identidade / safety number** | **Ausente** (Android e iOS têm; o desktop não expõe) | nada em `desktop/` |
| Settings | Placebo (5 toggles em `useState`; o próprio arquivo admite) | `SettingsView.tsx:11,17-21,120` |
| Configurar servidores / bootstrap | Ausente | `main.rs:22-30` |
| Tray "com menu contextual" (README:130) | Só o ícone | `tauri.conf.json` |
| Auto-update | Ausente | — |
| Bundle | Só macOS | `tauri.conf.json:40` |
| Testes | Vitest com mock do Tauri; nenhum teste do Rust `src-tauri` | `src/test/tauriMock.ts` |

**P0 do desktop**
- **D1. Chamadas sem áudio** (acima). Numa chamada Desktop↔Android a conexão estabelece e o desktop não captura nem reproduz som. Faltam também `NSMicrophoneUsageDescription`/`NSCameraUsageDescription` e entitlements (`tauri.conf.json:57` tem `entitlements: null`). *Correção:* ligar `voip_audio` ao `CallManager`, ou ocultar os botões de chamada na homologação.
- **D2. Não existe processo de release.** ✅ (lockfile) O crate `src-tauri` é workspace isolado e nunca é compilado no CI (o job `desktop` só roda `tsc` e vitest). `desktop/src-tauri/Cargo.lock` é ignorado por `.gitignore:3` → build não reproduzível. Sem assinatura nem notarização; nome do DMG fixo em `_x64`; sem updater nem release publicado. Um DMG não assinado é bloqueado pelo Gatekeeper, e a cada build não assinado o ACL do Keychain muda — o que leva a D3.

**P1 do desktop**
- **D3. Perda silenciosa de identidade.** `commands.rs:162-166` trata erro do Keychain igual a "sem identidade"; o core gera uma nova e `commands.rs:212-215` **sobrescreve** a entrada. O testador clica "Negar" no prompt do Keychain e perde peer ID e conta. *Correção:* abortar em qualquer `Err` que não seja `NoEntry`.
- **D4. Backup de identidade não cifrado** (`commands.rs:917-922`), via clipboard.

**P2 do desktop:** plugin shell com `open: true` sem uso (`tauri.conf.json:76`); `send_file_message(file_path)` e `import_identity_backup(data_dir)` aceitam caminhos arbitrários do webview; `get_group_sender_key_seed` e `export_identity_backup` expõem segredos ao JS; `dataDir` por concatenação de strings (`App.tsx:53`); 18 `console.log` e logs `debug` fixos em release.

### 8.2 Entrega, infra e processo — 3/10

Os P0 desta área estão na seção 3.1 (P0-A a P0-A4) e 3.4 (P0-M a P0-P). O restante:

| # | Gap | Evidência |
|---|---|---|
| I1 | **Licenciamento inconsistente** ✅ (ausência do arquivo): repositório **público** sem arquivo `LICENSE`, embora `README.md:413` aponte para ele; manifests declaram AGPL-3.0; depende de republicação de terceiros do libsignal (AGPL-3.0-only). Sem o texto da licença a distribuição não atende à AGPL; distribuição por App Store/TestFlight de código AGPL de terceiros tem tensão conhecida com os termos da Apple. **Não é parecer jurídico** — obter um antes do TestFlight | `gh api` → `license: null` |
| I2 | **Repositório público agrava P0-M:** as seeds dos bootstraps estão legíveis por qualquer pessoa. O `.env.example` do devops manda usar seeds fortes, mas aí o peer ID deixa de bater com o que está fixo no cliente e o dial falha | `stack.yml:125,163` |
| I3 | **Sem gate de supply chain:** sem `cargo audit`/`cargo deny`, `npm audit`, dependabot ou assinatura de imagem; Trivy com `exit-code: '0'`; actions fixadas por tag. Versões a conferir com `cargo audit` (advisories citados de memória pela frente, **não verificados por ferramenta**): `sqlx 0.8.0`, `ring 0.16.20`, `rustls 0.21.12`, `hyper 0.14`, `libp2p 0.53.2`, `rsa 0.9.10`, `libcrux-ml-kem 0.0.2` | `build-server-images.yml`, `Cargo.lock` |
| I4 | **Sem CD, monitoramento, alertas ou backup automatizado:** deploy manual via `scripts/deploy.sh`; rollback = trocar tag à mão, sem tratar schema; Prometheus/Grafana só no profile dev, sem regras de alerta; `BACKUP_AND_SECRETS.md` diz "automatize com cron" e não há script; Postgres e Redis compartilhados com outro produto (`bluevix_*`) | `zaplivre-devops/` |
| I5 | **Dois caminhos de deploy divergentes** (`stack.yml` Swarm × compose do devops), um deles com healthchecks impossíveis (P0-P) | — |
| I6 | **Faltam os documentos para testadores externos:** sem política de privacidade, termos, documentação LGPD, threat model, `SECURITY.md` ou runbook de incidentes. O serviço coleta username, chave pública, tokens de push e IP | `git ls-files` vazio para esses termos |
| I7 | **O plano de homologação não é um plano de testes:** `PLANO_HOMOLOGACAO.md` é lista de correções, sem casos de teste, critérios de aceite, matriz de devices, fluxo de reporte ou critério de saída. `FASE_8_TESTING_GUIDE.md` cobre só push, usa `FCM_SERVER_KEY` legado e tem 0 de 33 itens marcados. `docs/guides/desktop-testing.md` tem 37 linhas, todas em `[ ]` | — |
| I8 | **CI não roda os testes de `zaplivre-identity-server` nem de `zaplivre-bootstrap`** | `ci.yml`, step "Server tests" |

**P2 de infra:** compose de dev publica `5432` com senha default e Grafana `admin/admin`; `stack.yml` põe Postgres e Redis na `traefik-network`, publica 8080-8086 direto no host e passa segredos por env; imagens `coturn`/`prometheus`/`grafana`/`blackbox` em `:latest`; higiene do repositório (`AppIcons.zip` de 1,4 MB, `.claude/settings.local.json` com caminhos locais, dezenas de `FASE_*`/`PROGRESS`/`EXECUCAO`/`AUDIT_V1-V4` na raiz); `ISSUES_BACKLOG.md` sem atualização desde 05/07.

**README exagera:** "FASE 7 Desktop 100% — 3 views" (são 8); "System tray + menu contextual"; "TURN client integration ✅"; o plano afirma "CI verde desde então", o que é falso.

**Positivo (verificado):** pipeline publica as 6 imagens, bootstrap incluso, com SBOM e Trivy; secrets obrigatórios com `:?` (conferido com `docker compose config`); healthchecks de dev corrigidos; **nenhum segredo na árvore nem no histórico do git**.

---

## 9. Status dos itens da V4

| Item V4 | Status | Nota |
|---|---|---|
| V1–V5 / 2.1-1: branch vermelha (teste, clippy, fmt) | **Reincidente** | Outros erros, mesmo problema: `main` falha nos três gates |
| 2.1-2 / V6: "nunca plaintext por padrão" | **Parcial** | Corrigido no envio de texto. Mídia >512 KiB ignora a política (P0-D) e a recepção aceita plaintext (P0-B) |
| 2.2-3: bootstrap + coturn em produção | **Parcial** | Serviços existem no devops, mas ficam não-funcionais (P0-N, P0-O, P0-P) |
| 2.2-4: imagem do bootstrap no pipeline | **Corrigido** | `build-server-images.yml:35-36` |
| 2.2-5: bootstrap hardcoded legado | **Parcial** | Domínios trocados para `zaplivre.app` + override por env; peer IDs deriváveis de seed pública (P0-M); IPFS público segue como fallback |
| 2.2-6: divergência de domínio | **Corrigido** | `*.zaplivre.app` consistente em Android, iOS e desktop |
| 2.2-7: APNs em produção | **Aberto** | C5 do plano, admitido. Falta também PushKit (M6); novo bug que desativa tokens iOS (S8) |
| 2.2-8: CD, monitoramento, backup | **Aberto/Parcial** | `server/monitoring/` só no profile dev, só probes blackbox, sem alertas; produção sem nada; backup só em documento (I4). O ambiente em 502 sem alerta é a prova |
| 2.3-9: sync multi-device é stub | **Aberto (adiado)** | Só a documentação foi ajustada |
| 2.3-10a: rotação de sender key | **Parcial** | Implementada, mas best-effort, sem epoch/ack, sem rotação na entrada (C4) |
| 2.3-10b: seed em plaintext | **Corrigido no envio; aberto na recepção** | Controle de grupo plaintext forjável é aceito (P0-C) |
| 2.3-11: fork beta do libsignal | **Aberto** | Documento com erros factuais, checklist não cumprido (C8) |
| 2.4-12: `.so` x86_64 errado | **Parcial** | Arquitetura correta; as duas `.so` locais estão defasadas em relação aos bindings (M1) |
| 2.4-13: lib de device iOS, xcodegen, TODO de áudio | **Corrigido** | `project.yml:47`, `CallManager.swift:322-347`. A lib segue gitignorada e 2 commits atrás do core |
| 2.4-14: Settings e bundle do desktop | **Parcial** | Settings é placebo; macOS-only assumido e documentado |
| P1-1: healthchecks de dev (Redis/coturn) | **Corrigido** | Por leitura; não executado |
| P1-2: `/api/stats` sem auth | **Parcial** | Protegido, mas `/health` vaza o mesmo contador |
| P1-3: comparação não-constante | **Corrigido** | `subtle` em push e store |
| P1-4: estado process-local | **Aberto (aceito)** | Agravado pelo leak de `PEER_REQUESTS` |
| P1-5: testes do identity driftados | **Parcial** | Campos Kyber adicionados; todos `#[ignore]`, fora do CI |
| P1-6: `turns:` sem TLS / `TURN_HOST` | **Parcial** | `turns:` condicionado a TLS; `TURN_HOST` segue sem configuração |
| P1-8: migração de schema | **Aberto** | Só documento "Planejado"; marcado `[x]` indevidamente no plano (P0-A4) |
| P1-9: Maestro nunca executado em device | **Parcial, não verificável** | Alegação local sem artefato; no CI falha e é mascarado (P0-A2) |
| P1-10: envio offline engolido | **Parcial** | Agora persiste como Pending; surgiram P0-H e P0-I |
| P1-11: 7 testes VoIP `#[ignore]` | **Aberto** | O "9/9 ok" do plano foi execução manual |
| P1-12: `google-services.json` commitado | **Não procede** | Nunca esteve no histórico do git |
| P2-3: sem `androidTest` | **Corrigido** | Cobertura mínima (7 testes) |
| P2-4: toggles não persistem | **Parcial** | Android persiste, 2 de 5 sem efeito; iOS e desktop são cosméticos |
| P2-6: gates de fmt e clippy | **Corrigido (e falhando)** | Os gates existem; a `main` não passa neles |
| P2-7: scan, SBOM, `latest` | **Parcial** | Trivy não bloqueante; sem assinatura de imagem |
| P2-9: DNS transport no Android | **Aberto** | N2 |

---

## 10. Caminho mínimo para abrir homologação

Ordenado por custo/impacto.

1. **Deixar a `main` verde** (P0-A), remover os `continue-on-error` dos E2E (P0-A2), separar o fmt em job próprio e proteger a branch com checks obrigatórios.
2. **Trancar a recepção:** `sender_peer_id == from_peer`; recusar plaintext e controle de grupo fora da sessão Signal (P0-B, P0-C); teto no codec (P0-F); sanitizar `handle_media_chunk` (C6).
3. **Trocar as chaves dos bootstraps** por aleatórias persistidas e republicar os peer IDs **antes** de distribuir qualquer build (P0-M). Com o repositório público, as chaves atuais devem ser consideradas comprometidas.
4. **Subir um ambiente de homologação de verdade** (P0-A3): stack no ar, DNS de `dht1`/`dht2`, portas abertas, migrações com `sqlx::migrate!` (P0-A4), healthchecks compatíveis com a imagem (P0-P), um único caminho de deploy (I5), e alerta quando um `/health` cair.
5. **Fazer a infraestrutura P2P funcionar fora de LAN:** relay ligado no cliente (P0-G), bootstrap em modo Server com endereço externo (P0-N), parâmetros do relay (S7), transporte DNS no Android (M2/N2), `TURN_HOST` obrigatório e ICE servers populados nos três clientes (P0-O, M3). Validar com dois celulares em 4G, não em Wi-Fi.
6. **Tornar o store caminho de primeira classe:** dedup, ACK de entrega, apagar só sucesso, paginação, `fetch_offline()` por push (P0-H, P0-I); ciclo de vida de prekeys no cliente e no servidor (P0-J).
7. **Notificação e chamada com o app fechado:** FCM só com `data` e notificação local (M5), PushKit + push de chamada + Telecom/`CallStyle` (M6), APNs provisionado (M7) e o bug do `InvalidProviderToken` corrigido (S8).
8. **Builds de homologação reproduzíveis:** APK só via CI com `cargo ndk` limpo e arm64 (M1); DMG assinado e notarizado com lockfile versionado (D2); corrigir a perda silenciosa de identidade no desktop (D3) e o logout do iOS (MA2); paridade mínima de descoberta de contato (MA3).
9. **Ligar a identidade Signal ao Ed25519** e incluí-la no safety number (P0-E); vincular `peer_id` à chave no identity server (S1); assinar o bundle (S2); expor a verificação no desktop.
10. **Mídia grande dentro do E2E** com chave por arquivo (P0-D); corrigir o OOM de mídia no Android (MA1).
11. **Grupos por fan-out 1:1** (P0-K).
12. **Concorrência e timeouts na FFI** (P0-L); corrigir a race do signaling (S6) e o áudio do desktop (D1) antes de homologar VoIP — ou ocultar chamadas no desktop.
13. **Escrever o plano de testes** (I7) e os documentos mínimos para testadores externos: LICENSE, política de privacidade, termos, aviso LGPD (I1, I6).

Estimativa das frentes: itens 1–4 em dias; 5–9 em 3–6 semanas; 10–12 são refatorações de semanas cada. Os itens 1–3 fecham os ataques triviais e são o melhor retorno de toda a lista.

Um recorte menor é defensável mais cedo: **homologação interna de texto 1:1 e mídia pequena, só Android + iOS, sem chamadas e sem grupos**, depois dos itens 1–8. Isso permite validar o que já é real enquanto as refatorações de mídia, grupos e FFI seguem.

Para produção pública ainda faltam transparência assinada (C1), anti-abuso (S4, S9, S10), exclusão de conta/LGPD (S3), banco cifrado (C5, MA5), observabilidade (S11), gate de supply chain (I3), avaliação real do fork libsignal (C8), e as capacidades ausentes da seção 6.

---

## 11. Correções aplicadas

### Lote 1 — branch `fix/audit-v5-p0` (2026-09-20)

Verificado na máquina após o lote: `cargo fmt --all --check` limpo, `cargo clippy --workspace --all-targets -- -D warnings` limpo, `cargo test --workspace` verde sem exclusões (core unit 155, era 148).

| Item | Estado | O que mudou |
|---|---|---|
| P0-A (gates) | **Corrigido** | `apns.rs` (campo `content_available` no teste), `signaling_probe.rs` (trait `Engine` + `Result` de `sign`), `transparency.rs` (laço), `cargo fmt`. Lints que só apareciam com `--all-targets` também zerados |
| P0-A (CI) | **Parcial** | `ci.yml`: fmt em job próprio (não pula mais clippy e testes), clippy com `--all-targets`, testes de identity e bootstrap incluídos. **Falta:** proteção da branch `main` (configuração do GitHub) |
| P0-B (remetente forjado) | **Corrigido** | `validate_message` exige `sender_peer_id == from_peer`. Plaintext `Text` recusado na recepção por padrão; `MessageHandler::allow_plaintext` liga o downgrade de dev, fixado na construção do cliente a partir de `ZAPLIVRE_ALLOW_PLAINTEXT` |
| P0-C (controle de grupo em plaintext) | **Parcial** | Fechado pelo P0-B: envelope de controle só chega por sessão Signal e de remetente autenticado. **Falta:** consentimento para `invite` e prova de admin |
| P0-F (DoS no codec) | **Corrigido** | `MAX_FRAME_BYTES` = 4 MiB checado antes de alocar; `chunk_size` do requisitante limitado a 4 KiB–1 MiB |
| C6 (`handle_media_chunk`) | **Corrigido** | Hash precisa ser SHA-256 hex; chunk só de quem ofertou a mídia; janela de escrita limitada ao tamanho ofertado (teto de 512 MiB) |
| P0-H (duplicata vira Failed) | **Parcial** | Dedup idempotente por `message_id` + remetente antes do decrypt, respondendo ACK Received. **Falta:** ACK de entrega pelo store e limpeza da cópia no servidor quando o P2P chega primeiro |
| P0-I (perda silenciosa no store) | **Parcial** | O fetch só apaga do servidor o que teve ACK Received. **Falta:** paginação, `fetch_offline()` acionável por push. Limitação: mensagem permanentemente indecifrável fica no store até o TTL de 14 dias |

Testes novos: remetente forjado, plaintext recusado por padrão, duplicata, prefixo de 4 GiB no codec, hash com path traversal, chunk de peer que não ofertou, offset fora do tamanho.

Dívida encontrada e não tratada: `cargo clippy -p zaplivre-core --features voip -- -D warnings` falha (imports não usados e lints em `core/src/voip/`). O CI só roda `cargo check` com essa feature.

### Lote 2 — branch `fix/audit-v5-p0` (2026-09-20)

Verificado após o lote: fmt limpo, `cargo clippy --workspace --all-targets -- -D warnings` limpo, `cargo test --workspace` verde (core unit 156), `cargo ndk -t arm64-v8a check -p zaplivre-core` compila.

| Item | Estado | O que mudou |
|---|---|---|
| P0-J (prekeys) | **Parcial** | O bundle estático não carrega mais one-time prekey: a mesma cópia é servida a todos os iniciadores (servidor e QR), então uma chave de uso único quebrava o segundo contato e era reutilizada após restart. PQXDH é definido para bundle sem OTP. O pool passa a ser persistido a cada mutação (`persist_prekey_pool`). Teste: dois iniciadores com o mesmo bundle. **Falta:** pool de OTPs no identity server com pop atômico por lookup; rotação periódica de SPK/Kyber mantendo a anterior; republicação; reset de sessão em erro |
| P0-G (relay morto) | **Corrigido no cliente** | O builder usa o primeiro bootstrap como relay; o slot é reservado assim que o cliente conecta nele (sem esperar veredito de NAT); o endereço `/p2p-circuit` é publicado no DHT quando a reserva é aceita; o relay não recebe PrekeyBundleSync. Teste de integração novo `relay_circuit.rs` com relay real: peer que não escuta em porta nenhuma é alcançado pelo circuito. **Limitação:** todos os clientes usam o primeiro bootstrap como relay; **não validado em 4G real** |
| P0-N (bootstrap client-mode) | **Corrigido, não executado em deploy** | `EXTERNAL_ADDRS` (novo) → `add_external_address`; Kademlia forçado em `Mode::Server`; aviso no log se ausente. `stack.yml` e `docker-compose.yml` atualizados |
| S7 (limites do relay) | **Corrigido** | `RELAY_MAX_BYTES_PER_SEC` (que na prática cortava o circuito em 1 MB no total) substituída por `RELAY_MAX_CIRCUIT_BYTES` (64 MiB) e `RELAY_MAX_CIRCUIT_SECS` (30 min) |
| P0-O (TURN_HOST) | **Parcial** | `TURN_HOST` obrigatório; `coturn` é recusado no boot; `stack.yml` exige a variável. **Falta:** os clientes consumirem o TURN (M3, N5) e o compose do repositório `zaplivre-devops` definir a variável |
| M2 / N2 (DNS no Android) | **Corrigido** | Android usa o mesmo transporte DNS explícito do iOS; default do app inclui `dht2` |

**Atenção para o deploy:** o bootstrap agora espera `EXTERNAL_ADDRS` e o `turn-credentials` não sobe sem `TURN_HOST`. O repositório `zaplivre-devops` precisa das duas variáveis antes do próximo deploy.

### Lote 3 — branch `fix/audit-v5-p0` (2026-09-20)

Verificado após o lote: fmt, `clippy --workspace --all-targets -D warnings`, `cargo test --workspace` e `cargo check --features voip` limpos.

| Item | Estado | O que mudou |
|---|---|---|
| P0-L (HTTP sem timeout) | **Corrigido** | `utils::http::client()` com connect 5 s / request 15 s, usado no message store, no worker de grupo e no cliente TURN; conexão do WebSocket de signaling limitada a 10 s (rodava dentro do `build()`) |
| P0-L (fila serial da FFI) | **Corrigido, não medido em device** | Três pistas: *Immediate* (leituras de DB, controle de chamada, frames de áudio/vídeo) nunca espera a rede; *Ordered* (tudo que cifra ou envia) continua estritamente em ordem — duas cifragens concorrentes para o mesmo peer bifurcariam o ratchet; *Background* (download de mídia). **Falta:** `spawn_blocking` para compressão de imagem e IO de arquivo |
| C3 (identidade regenerada em silêncio) | **Corrigido no caminho de arquivo** | `identity.key` ilegível e falha ao gravar a chave nova agora são erro de build do cliente. **Falta:** o mesmo para o caminho de Keychain do desktop (D3) e recusar identidade nova sobre um banco existente, que depende de corrigir antes o logout do iOS (MA2) |

### Lote 4 — branch `fix/audit-v5-p0` (2026-09-21)

Verificado após o lote: fmt, `clippy --workspace --all-targets -D warnings` (também com `--features integration-tests`), `cargo test --workspace` verde (core unit 157). O crate `desktop/src-tauri` não pôde ser verificado até o fim: `tauri::generate_context!` exige `desktop/dist` (frontend buildado), ausente nesta árvore.

| Item | Estado | O que mudou |
|---|---|---|
| S1 (peer_id não vinculado à chave) | **Corrigido** | O identity server exige que `peer_id` seja o peer ID libp2p da chave que assinou o registro (`PEER_ID_MISMATCH`). Testes de integração do servidor e do core passam a derivar peer IDs reais (`Keypair::libp2p_peer_id`). Validado contra servidor real local junto com o P0-E (6/6) |
| S6 (signaling) | **Corrigido em (a)–(e)** | Remoção do peer só pelo dono da conexão (corrige a race Wi-Fi→4G); prazo de 15 s para registrar; uma identidade por conexão; limite de frame aplicado na camada WebSocket; lock de roteamento liberado antes do `send`. Primeiro teste do crate. **Falta:** limite global de conexões, anti-spam de chamadas, estado compartilhado para 2 réplicas |
| S8 (tokens de push) | **Corrigido** | `InvalidProviderToken`/`ExpiredProviderToken` não desativam mais tokens de device; FCM v1 `UNREGISTERED` passa a desativar |
| P0-E (identidade Signal não ligada) | **Corrigido no core** | O bundle leva `signal_identity_signature`: assinatura Ed25519 (a chave do peer ID) sobre a chave de identidade Signal. `verify_belongs_to(peer_id)` é **obrigatória** antes de `process_prekey_bundle`, para qualquer origem do bundle (identity server, QR, sync P2P). SPK e Kyber já são assinados pela identidade Signal, então o bundle inteiro encadeia até o peer ID. Com isso o safety number (Ed25519) passa a cobrir a sessão Signal. Campo propagado no DTO do identity client, no modelo do servidor, no Android e no desktop. Testes: bundle de terceiro, identidade Signal trocada e bundle sem assinatura são recusados. **Quebra de compatibilidade:** bundles registrados e QRs gerados antes deste commit deixam de ser aceitos. O identity server valida o mesmo vínculo em `register` e `PUT /prekeys` (`INVALID_PREKEY_BUNDLE`), o que também limita o replay do S2 a bundles genuínos do próprio dono. **Validado ponta a ponta** em 2026-09-21: identity server local (Postgres e Redis descartáveis) + `cargo test -p zaplivre-core --test identity_integration --features integration-tests` → 6/6. **Falta:** persistir "verificado" e evento de troca de chave (C2); expor verificação no desktop |

### Lote 5 — branch `fix/audit-v5-store` (2026-09-21)

Verificado após o lote: fmt, `clippy --workspace --all-targets -D warnings` e `cargo test --workspace` verdes.

| Item | Estado | O que mudou |
|---|---|---|
| P0-I (busca offline) | **Corrigido no core** | `fetch_offline_messages()` é pública, devolve quantas mensagens processou e pagina em lotes de 100 enquanto houver lote cheio e progresso. Falha ao limpar a caixa no servidor interrompe o laço (o próximo fetch recebe de novo e a recepção responde como duplicata). Erro HTTP do store deixa de ser tratado como "caixa vazia". A caixa passa a ser consultada a cada 30 s com o app aberto — antes só era lida no `bootstrap()`, então o que caía no store durante a sessão ficava lá até o próximo lançamento. `bootstrap()` não segura mais o lock do `NetworkManager` durante o HTTP. **Falta:** expor `fetch_offline_messages` na UDL (exige regenerar bindings e libs nativas) e acionar por push nos apps |
| P2 store (`limit` negativo → 500) | **Corrigido** | `clamp(1, 1000)` |
| Cobertura | **Novo** | `core/tests/offline_mailbox.rs`: primeiro teste ponta a ponta do store-and-forward, com store HTTP falso e cifragem Signal real — mensagem deixada no store com o destinatário offline, drenagem, payload indecifrável permanece no servidor, segunda drenagem não duplica |

### Lote 6 — branch `fix/audit-v5-hardening` (2026-09-21)

Verificado: fmt, `clippy --workspace --all-targets -D warnings`, `cargo test --workspace` verdes (core unit 159).

| Item | Estado | O que mudou |
|---|---|---|
| CI (pós-merge do #6) | **Corrigido** | O clippy do CI é mais novo que o local e acusava `large_const_arrays` no scaffolding gerado pelo UniFFI; permitido no nível do crate. `setup-android` pedia o pacote `tools`, removido do `sdkmanager`; trocado por `platform-tools`. Confirmado no CI real: job "Rust (core + servers)" verde no PR #7. O conserto do Android **não foi confirmado** (os workflows de Android só disparam com mudanças em `android/`) |
| P2 — conteúdo em claro no log | **Corrigido** | `Received text` loga só o tamanho |
| P2 — seed em `Debug` | **Corrigido** | `SenderKey` tem `Debug` manual com a seed redigida (`GroupSession` deriva `Debug` sobre ele) |
| C4 — overflow do counter de grupo | **Corrigido** | `checked_add` antes do decrypt; counter `u64::MAX` é recusado sem mover a guarda de replay |
| P2 — `registration_id` zero | **Corrigido** | Faixa 1..=16380 |
| P2 — `ZAPLIVRE_ALLOW_PLAINTEXT` em release | **Corrigido** | A chave só existe em builds debug (`cfg!(debug_assertions)`); nenhum app, script ou workflow dependia dela |

Dívida mantida: `cargo clippy -p zaplivre-core --features voip --all-targets -- -D warnings` tem ~27 lints, vários não mecânicos.

### Lote 7 — branch `feat/audit-v5-media-e2e` (2026-09-21)

Verificado: fmt, `clippy --workspace --all-targets -D warnings`, `cargo check --features voip`, `cargo test --workspace` verdes (core unit 163).

| Item | Estado | O que mudou |
|---|---|---|
| P0-D (mídia >512 KiB fora do E2E) | **Corrigido no core** | Novo `media::transfer`: o arquivo é selado com AES-256-GCM sob chave aleatória por arquivo (AAD de domínio). A oferta (`MediaOfferEnvelope`: chave, nonce, metadados, miniatura) viaja **dentro da sessão Signal**, pelo mesmo caminho do texto — inclusive store e outbox, então nome, tamanho e tipo do arquivo não ficam mais em claro no message store. A mídia passa a ser identificada pelo SHA-256 do blob **selado**; os chunks carregam só ciphertext. O receptor confere o hash, abre o blob e grava apenas o plaintext. O remetente não guarda cópia selada: recria o blob de forma determinística a partir do arquivo local e da oferta guardada cifrada em repouso. `Payload::MediaOffer` em plaintext não é mais enviado nem aceito. Mídia sem oferta não é servida |
| N4 (chunk `is_last` fora de ordem) | **Corrigido** | A conclusão do download é "todos os bytes recebidos", não "chegou o chunk marcado como último"; o `.part` não é mais truncado no offset 0; re-pedidos não contam em dobro |
| Bug pré-existente (hash de conteúdo × hash de armazenamento) | **Eliminado** | A oferta anunciava `content_hash`, mas o registro local usava um hash salgado quando o mesmo arquivo era reenviado; o pedido do receptor caía no registro de outra mensagem. Com chave aleatória por envio, cada envio tem hash próprio |
| Cobertura | **Novo** | `media::transfer` (4 testes: ida e volta, hash não relacionado ao plaintext, blob adulterado/de outra oferta, chave fora do `Debug`); testes de chunk adaptados ao blob selado, com chegada fora de ordem; `core/tests/media_transfer.rs`: dois clientes reais, documento de 1,5 MiB, da oferta cifrada ao download, no CI |

**Compatibilidade:** quebra o protocolo de mídia grande com versões anteriores (ofertas em plaintext são recusadas). Não muda a UDL nem o esquema do banco: os apps continuam chamando `download_media(media_hash)` com o hash do registro de mídia.

**Falta:** o remetente ainda precisa estar online no momento do download (modelo pull P2P; o padrão de mercado é blob cifrado em servidor); arquivo inteiro em memória para selar/abrir; timeout fixo de 10 s por tentativa em `download_media`; sem retomada de download.

### Lote 8 — branch `feat/audit-v5-group-fanout` (2026-09-21)

Verificado: fmt, `clippy --workspace --all-targets -D warnings`, `cargo check --features voip`, `cargo test --workspace` verdes (core unit 164).

| Item | Estado | O que mudou |
|---|---|---|
| P0-K (grupos sem entrega confiável) | **Corrigido no core** | A mensagem de grupo (mesma estrutura: conteúdo sob sender key + assinatura Ed25519) é entregue a **cada membro pelo pipeline 1:1**: sessão Signal, message store para quem está offline, outbox com retry. Não é mais publicada no GossipSub, que só alcançava membros online e diretamente conectados naquele instante, sem retry, e cujo tópico expunha group id, remetente e horário a qualquer assinante. Status `Sent` quando ao menos um membro recebeu ou está enfileirado |
| Ordem convite → chave → mensagem | **Tratado** | A mensagem de grupo passa pela mesma fila ordenada dos envelopes de controle (`MessageEvent::GroupMessage`); tratada na hora, ela ultrapassaria o convite e seria descartada como grupo desconhecido |
| Mensagem que chega antes da sender key | **Corrigido** | Fica guardada cifrada e é decifrada quando a chave chega (`decrypt_pending_messages`); antes ficava em branco para sempre |
| C4 — assinatura dependia de `contact.public_key` | **Corrigido** | A chave de verificação vem do peer ID do remetente; mensagens de membros cujo contato foi criado sem chave pública (username, QR) eram todas descartadas. No fan-out, o remetente declarado tem de ser o peer autenticado do canal 1:1 |
| Controle de grupo e reações sem outbox | **Corrigido** | `deliver_message_with` grava no store **e** enfileira retry local; antes, sem store (ou com ele fora), convite, sender key, remoção e reação eram descartados |
| P0-H (duplicata) — solução definitiva | **Corrigido** | Dedup persistente por id de mensagem no fio (`processed_messages`, migração 9, escopo por remetente, retenção de 14 dias). O dedup anterior só cobria mensagens gravadas sob o próprio id; fan-out de grupo, reações e controle passavam duas vezes pelo Signal, falhavam, e ficavam no store falhando a cada fetch |
| Cobertura | **Novo** | `core/tests/group_fanout.rs`: três clientes reais + store falso; membro online recebe por P2P e membro que nunca esteve online recebe convite, sender key e mensagem numa drenagem da caixa, sem duplicar. No CI. Store falso extraído para `core/tests/common` |

**Compatibilidade:** mensagens de grupo de versões anteriores (GossipSub) ainda são aceitas na recepção, mas não são mais emitidas. Migração 9 do banco local (aditiva).

**Falta:** custo O(membros) por mensagem no remetente (aceitável para grupos pequenos/médios; é o modelo do Signal/WhatsApp, que amortizam com sender keys + fan-out no servidor); remover a assinatura de tópicos GossipSub, que ainda expõe o group id aos peers conectados; mídia em grupo; rotação de sender key com epoch/ack (C4); consentimento para convite (P0-C).

### Merge da pilha (2026-09-21)

PRs #7, #8, #9 e #10 mergeados na `main`, cada um com todos os checks verdes. **CI de Rust da `main` verde pela primeira vez desde 09/08** (`bdee82a`).

### Lote 9 — branch `feat/apps-offline-push` (2026-09-21) — frente dos apps

Verificado: gates de Rust verdes; `./gradlew :app:compileDebugKotlin` e `:app:testDebugUnitTest` (18/18) com JDK 17. **Não verificado:** execução em device/emulador; build iOS (as mudanças no iOS são só os bindings gerados, aditivas).

| Item | Estado | O que mudou |
|---|---|---|
| P0-I (restante) — FFI | **Corrigido** | `fetch_offline_messages()` exposta na UDL e no cliente FFI (pista ordenada). Bindings Kotlin e Swift regenerados com `uniffi-bindgen 0.31.2`; o gerador reproduziu os arquivos versionados e o diff é só o método novo |
| M5 (b) — push Android não acordava o app | **Corrigido no servidor** | FCM v1 passa a ser **só de dados** (`title`/`body` viajam em `data`, prioridade alta). Com bloco `notification`, o Android desenhava a bandeja sozinho e, em background ou com o app morto, não chamava `onMessageReceived` |
| M5 (a) — mensagem em background sem notificação | **Corrigido** | `ZapLivreService` observa `messageEvents` e notifica quando a UI não está na tela (`AppVisibility`), sem conteúdo da mensagem. Cobre P2P e drenagem da caixa |
| M5 — drenagem ao receber push | **Corrigido** | O serviço FCM chama `fetchOfflineMessages()` quando o client já está pronto (o `start()` do service não refaz o bootstrap se ele já roda) |
| M5 — toque na notificação | **Tolerante** | `MainActivity` aceita `peer_id` e `sender_peer_id`. Com push só de dados a notificação é sempre montada pelo app, que já usava `peer_id` |
| Android CI — "Run unit tests" vermelho desde 14/08 | **Corrigido** | 7 testes do `ChatViewModelTest` esperavam `getConversationMessages(peer, null, null)`; o ViewModel passou a paginar (`50u`) |

**Observação de ambiente:** o JDK padrão desta máquina (25.0.2) quebra o Gradle do projeto; os builds Android foram feitos com `JAVA_HOME` apontando para o Temurin 17.

**Falta nesta frente:** iOS usar `fetchOfflineMessages` no wake por push e ter ciclo de vida (MA8); PushKit + push de chamada (M6); Telecom/`CallStyle` no Android; ICE servers a partir do `turn-credentials` (M3); APK só via CI com `cargo ndk` limpo e arm64 (M1); OOM de mídia por `List<UByte>` (MA1); logout do iOS (MA2); paridade de descoberta de contato (MA3).

### Lote 10 — branch `feat/apps-turn-ice` (2026-09-21)

Verificado: gates de Rust verdes (fmt, clippy `--all-targets`, `check --features voip`, `cargo test --workspace`); Android `:app:compileDebugKotlin` (JDK 17); **iOS `xcodebuild` para o simulador: BUILD SUCCEEDED**, com as libs nativas reconstruídas a partir do core atual (`ios/build-rust.sh`). **Validado contra o `turn-credentials` real** rodando localmente: `cargo test -p zaplivre-core --test ice_servers -- --ignored` → request assinado aceito, credenciais devolvidas. **Não verificado:** chamada real entre dois aparelhos em rede móvel (depende do coturn no ar com `TURN_HOST` público).

| Item | Estado | O que mudou |
|---|---|---|
| M3 / P0-O — clientes não consumiam o TURN | **Corrigido** | `Client::ice_servers()` (sem depender da feature `voip`): request assinado ao `turn-credentials`, mesmo esquema do message store; devolve STUN no **nosso** host (o coturn responde STUN) + TURN com credenciais efêmeras. Exposto na FFI (`ice_servers()` → `FfiIceServer`), na pista de background para não esperar envios enfileirados. URL por `TURN_CREDENTIALS_URL` (Android `BuildConfig`, iOS `Info.plist`/`project.yml`), default `https://turn.zaplivre.app` |
| Vídeo — Android | **Corrigido** | `NativeWebRtcSession` busca os ICE servers (teto de 5 s) antes de criar a `PeerConnection`; antes era `RTCConfiguration(emptyList())` |
| Vídeo — iOS | **Corrigido** | Idem. O observer de sinais é registrado **antes** da busca e os sinais são guardados até a conexão existir: as notificações do iOS não têm replay e uma oferta que chegasse durante a busca seria perdida |
| N5 — áudio (WebRTC do core) só com STUN do Google | **Corrigido** | `start_call`/`accept_call` entregam credenciais TURN frescas ao `CallManager` (best-effort); `build_turn_config` usa STUN no nosso host em vez do Google, que ficava sabendo quem liga para quem |
| MA8 (parte) — iOS no wake por push | **Parcial** | O handler de push drena a caixa offline (`fetchOfflineMessages`); antes só discava o remetente, o que não traz a mensagem que está no store. **Falta:** inicializar o core no wake em background e o ciclo de vida (`scenePhase`, `beginBackgroundTask`) |

**Deploy:** depende de `TURN_HOST` público no `turn-credentials` (P0-O) e do coturn alcançável; o default do app aponta para `https://turn.zaplivre.app`.

### Lote 11 — branch `fix/apps-media-oom-logout` (2026-09-21)

Verificado: gates de Rust verdes; Android `:app:compileDebugKotlin` + `:app:testDebugUnitTest` (18/18, JDK 17); iOS `xcodebuild` para o simulador com **BUILD SUCCEEDED** contra libs nativas reconstruídas. **Não verificado:** execução em device (envio de vídeo grande, fluxo de logout).

| Item | Estado | O que mudou |
|---|---|---|
| MA1 — OOM ao enviar mídia no Android | **Corrigido** | Os cinco métodos de mídia da FFI (`send_image/voice/document/video_message`, `download_media`) passaram de `sequence<u8>` para `bytes`. No Kotlin, `List<UByte>` (um objeto por byte: vídeo de 25 MB → centenas de MB de heap → `OutOfMemoryError`, que não é `Exception` e derrubava o app) vira `ByteArray`; no Swift, `Data`. Bindings regenerados e chamadores atualizados nas duas plataformas. **Falta:** os frames de chamada (`send_audio_frame`, `send_video_frame` e callbacks) seguem como `sequence<u8>` — é custo de desempenho, não crash; o app ainda lê o arquivo inteiro para a memória e não comprime |
| MA2 — logout do iOS | **Corrigido** | Desregistra o push (enquanto a identidade existe: o request é assinado), apaga identidade do Keychain, banco, mídia e preferências, e encerra o processo após voltar à tela inicial. O core Rust não pode ser recriado no mesmo processo (`OnceLock`): por isso restaurar backup logo após o logout falhava com "Import requires app restart", e o banco do usuário anterior ficava para o próximo. **Risco conhecido:** encerrar o processo no iOS é desaconselhado pela Apple; a alternativa exige tornar o core reinicializável (N3) |
| MA2 — logout do Android sem `unregisterToken` | **Corrigido** | Desregistra o push antes de apagar a identidade, com teto de 5 s |

### Lote 12 — branch `fix/apps-e2e-fresh-install` (2026-09-21)

Verificado **localmente com Maestro 2.4.0** num simulador iPhone 17 Pro, com o app assinado ad-hoc exatamente como no `e2e-ios.yml`: **suíte iOS 8/8, em duas rodadas seguidas** (no CI da `main` eram 7/8 falhando). Build iOS e gates de Rust verdes. **Não verificado:** a suíte Android (sem emulador local nesta sessão).

| Item | Estado | O que mudou |
|---|---|---|
| P0-A2 / hipótese do relatório (Keychain sobrevive ao `clearState`) | **Confirmado e corrigido** | O Keychain sobrevive à desinstalação; o diretório de dados, não. O app reinstalado achava a identidade, pulava o onboarding e abria "logado" sobre um banco vazio — no CI isso derrubava 7 dos 8 flows, e **em produção a chave privada ficava no aparelho depois de o usuário remover o app**. Uma identidade sem o diretório de dados ao lado é descartada no `didFinishLaunching`, só quando os dados protegidos estão disponíveis (lançamento em background antes do primeiro desbloqueio não conta como "diretório ausente"). A conta volta por backup exportado. Na primeira tentativa a checagem estava no `App.init` e não disparava; a verificação no simulador pegou isso |
| Validação de Peer ID no iOS | **Corrigido** | Só checava o prefixo: um ID malformado abria uma conversa em que tudo falhava depois, sem explicação. Agora exige Ed25519 libp2p (`12D3KooW` + base58, 52 caracteres); IDs `Qm…` (RSA) são recusados porque não carregam a chave Ed25519 que o protocolo usa para verificar o bundle do contato |
| Flake do diálogo de notificações | **Corrigido** | O diálogo de permissão podia surgir depois do passo opcional e cobrir o botão de onboarding; o setup comum o dispensa de novo |
| E2E Android | **Parcial, não verificado** | Os 12 flows `2dev_*` exigem dois devices e entravam na execução de um emulador só (`*.yml`); um deles nem era YAML válido (`release:true`) e abortava o parse. O `config.yaml` passa a rodar só os flows numerados; o YAML foi corrigido. O `adb: device offline` do emulador no CI não foi tratado |

**Observação:** os workflows de E2E só disparam em PRs contra `main`/`develop`; num PR empilhado eles não rodam.

### Merge da pilha dos apps (2026-09-21)

PRs #11, #12, #13 e #14 mergeados na `main`. Em #11 só falhavam os dois jobs de Maestro, as falhas antigas corrigidas pelo #14; os demais checks estavam verdes, inclusive "Build Android".

### Lote 13 — branch `feat/ios-username-lookup` (2026-09-21)

Verificado: iOS `xcodebuild` (assinatura ad-hoc) com BUILD SUCCEEDED; **suíte Maestro iOS 8/8** sem regressão e um flow ad-hoc confirmando a recusa de username inválido; Android `:app:compileDebugKotlin` + `:app:testDebugUnitTest` **22/22** (4 novos). **Não verificado:** busca de username com sucesso no iOS (exige identity server alcançável; o de produção responde 502) e leitura de QR com câmera real no Android (sem emulador/aparelho nesta sessão).

| Item | Estado | O que mudou |
|---|---|---|
| MA3 — iOS sem busca por username | **Corrigido** | O campo da nova conversa aceita `@usuário` ou Peer ID. Username (formato do servidor, `^[a-z0-9_]{3,20}$`) é resolvido no identity server e o bundle é guardado; o core verifica que ele pertence ao peer ID (P0-E) antes de qualquer sessão |
| MA3 — Android sem leitor de QR | **Corrigido, não validado com câmera** | `QrScannerDialog` com CameraX + zxing (dependências que o app já tinha) e `ContactQrCode.parse` para os três formatos (JSON v1 do iOS, `peerId@multiaddr`, só peer ID), com peer ID Ed25519 validado por inteiro; com endereço, disca o contato |

Com isso os dois apps conseguem se encontrar pelos dois caminhos: username e QR.

### Lote 14 — E2E no CI e build nativo do Android (2026-09-21)

**CI da `main` após o merge de #11–#14:** CI, iOS CI e imagens verdes. **E2E iOS no CI: 7/8** (antes 1/8). O `02_navegacao_telas` falhou: a captura do CI mostra que, com a lista de configurações rolada, o gesto de fechar só a trouxe de volta ao topo. Corrigido no PR #16 (repete o gesto enquanto a folha estiver aberta; 3/3 no simulador). **E2E Android no CI:** ainda abortava no parse — ver abaixo.

| Item | Estado | O que mudou |
|---|---|---|
| E2E Android — suíte nunca executava | **Corrigido (PR #16)** | O Maestro faz o parse de todo `.yml` do diretório antes de filtrar, e `2dev_envia_audio` usava campos inexistentes (`duration`, `release`, `desc`, `timeout`): **nenhum flow Android jamais produziu veredito**. Flow corrigido (todos passam no `maestro check-syntax`) e flows de dois aparelhos movidos para `two-devices/`. As linhas `adb: device offline` são só a sondagem do boot. **Execução da suíte não verificada localmente** (sem emulador); o PR dispara a primeira execução real |
| M1 — `.so` local defasada | **Corrigido** | A task `buildRustCore` declara o core como entrada e as `.so` como saída; antes só rodava quando faltava alguma `.so`, e o script compila só arm64 por padrão — a x86_64 de emulador, uma vez presente, nunca mais era refeita. **Verificado:** roda na 1ª vez, UP-TO-DATE sem mudança, ignora só `touch`, recompila quando o conteúdo do core muda |
| M1 — APK de homologação | **Corrigido** | O Android CI publica o `app-debug.apk` do commit como artefato (todas as ABIs, libs recém-compiladas), retido por 30 dias |
