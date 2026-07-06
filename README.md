# ZapLivre

> **Comunicação verdadeiramente híbrida: P2P quando possível, servidor quando necessário**

[![License: AGPL-3.0](https://img.shields.io/badge/License-AGPL%203.0-blue.svg)](https://www.gnu.org/licenses/agpl-3.0)
[![Rust](https://img.shields.io/badge/rust-%23000000.svg?style=flat&logo=rust&logoColor=white)](https://www.rust-lang.org/)
[![Status](https://img.shields.io/badge/status-v0.1.0--alpha%20em%20desenvolvimento-yellow)](https://github.com/integralltech/zaplivre)

## 🎯 Visão

**ZapLivre** é uma plataforma de mensagens instantâneas com arquitetura **HÍBRIDA P2P + Servidor**:

- **80% P2P direto:** Mensagens vão peer-to-peer (privacidade máxima, zero custo)
- **15% TURN relay:** Fallback quando NAT simétrico/firewall
- **5% Store & Forward:** Destinatário offline (PostgreSQL, TTL 14 dias)

### Diferencial

| | WhatsApp | Telegram | Signal | **ZapLivre** |
|---|---|---|---|---|
| **E2E por padrão** | ✅ | ❌ | ✅ | ✅ |
| **Sem telefone** | ❌ | ❌ | ❌ | ✅ |
| **P2P direto** | ❌ | ❌ | ❌ | ✅ (80%) |
| **VoIP integrado** | ✅ | ✅ | ✅ | ✅ |
| **Funciona offline** | ✅ | ✅ | ✅ | ✅ |
| **Self-hosting** | ❌ | ❌ | ❌ | ✅ |
| **Open source** | ❌ | ⚠️ | ✅ | ✅ |
| **Sem ban** | ❌ | ❌ | ❌ | ✅ |

**TL;DR:** Como WhatsApp (funciona sempre) + Melhor que WhatsApp (privado, sem ban, self-hosting).

---

## 🏗️ Arquitetura

```
┌──────────────────────────────────────────────────┐
│              ZAPLIVRE HÍBRIDO                      │
├──────────────────────────────────────────────────┤
│                                                   │
│  CENÁRIO 1: P2P Direto (80%)                     │
│  ────────────────────────────                    │
│  [Alice] ←────── P2P ──────→ [Bob]               │
│  • Zero custo servidor                           │
│  • Latência ~50ms                                │
│  • Privacidade máxima                            │
│                                                   │
│  CENÁRIO 2: TURN Relay (15%)                     │
│  ────────────────────────────                    │
│  [Alice] ──→ [TURN] ──→ [Bob]                    │
│  • NAT simétrico/Firewall                        │
│  • Ainda E2E encrypted                           │
│  • Latência ~200ms                               │
│                                                   │
│  CENÁRIO 3: Store & Forward (5%)                 │
│  ────────────────────────────────                │
│  [Alice] ──→ [Store] ··· [Bob offline]           │
│                │                                  │
│                └──→ [Bob] (quando online)        │
│  • TTL 14 dias                                   │
│  • Encrypted no servidor                         │
│  • Auto-delete após entrega                      │
│                                                   │
└──────────────────────────────────────────────────┘
```

---

## 📦 Stack Técnico

### Core (Rust)
- **libp2p:** Networking P2P (Kademlia DHT, GossipSub, Circuit Relay v2)
- **Signal Protocol:** E2E encryption (Double Ratchet, X3DH)
- **WebRTC:** VoIP chamadas de voz (webrtc-rs + Opus codec)
- **SQLite:** Storage local thread-safe
- **UniFFI 0.31:** FFI bindings (Rust → Kotlin/Swift)

### Apps
- **Android:** Kotlin + Jetpack Compose + Material3
- **iOS:** Swift + SwiftUI — design system ZapLivre (paleta de marca, bolhas com cauda, componentes reutilizáveis)
- **Desktop:** Tauri 2.0 (Rust + React + TypeScript)

> **Identidade visual:** o app adota uma linguagem inspirada nos mensageiros de mercado,
> mas com a marca ZapLivre — azul `#2F6BFF` + gradiente "spark" (raio-pássaro) no lugar do
> verde. O iOS já foi repaginado; o Android está em migração para o mesmo design system.

### Servidor (Self-hosted)
- **Bootstrap Nodes:** libp2p DHT + Kademlia (peer discovery)
- **TURN Relay:** coturn (NAT traversal para WebRTC)
- **Message Store:** PostgreSQL + Redis (offline delivery)
- **Push Notifications:** FCM (Android) + APNs (iOS)
- **Identity Server:** Username resolution (@alice → peer_id)

---

## 🚧 Progresso Atual

**Status:** v0.1.0-alpha — desenvolvimento ativo. Core e servidores maduros; apps
(iOS/Android/Desktop) com todas as áreas principais implementadas e em fase de
validação end-to-end. Veja o [Estado por plataforma](#-estado-por-plataforma) e o
[Estado por funcionalidade](#-estado-por-funcionalidade) mais abaixo.

### ✅ O que já está implementado

**FASE 1-5: Core Library (100%)** ✅
- ✅ Identity (Ed25519) + Crypto (Signal Protocol Double Ratchet)
- ✅ Network (libp2p: Kademlia DHT, GossipSub, mDNS, Identify)
- ✅ Storage (SQLite thread-safe, migrations, FTS5 search)
- ✅ Protocol (Protobuf) + Client API completa
- ✅ FFI Bindings (UniFFI 0.31: Kotlin + Swift)
- 📊 **~9.000 LoC**, 110+ testes passando

**Android App** 🔨 (implementado; runtime em estabilização)
- ✅ Jetpack Compose + Material3 + design system ZapLivre (14 telas)
- ✅ ZapLivreClientWrapper (singleton, coroutines)
- ✅ Foreground Service P2P + notificação persistente
- ✅ Chat 1:1, grupos, mídia, chamadas (telas)
- ⚠️ Rodou no Android pela 1ª vez em 07/2026 (3 bugs de runtime corrigidos); inicialização P2P em estabilização

**FASE 7: Desktop App MVP (100%)** ✅
- ✅ Tauri 2.0 + React 18 + TypeScript
- ✅ 3 views: Onboarding → Conversations → Chat
- ✅ FFI integration via Tauri commands
- ✅ System tray + menu contextual
- ✅ Cross-platform (DMG, MSI, AppImage)
- 📊 **~2.200 LoC**, 20 arquivos

**FASE 9: Bootstrap + DHT Server (100%)** ✅
- ✅ Kademlia DHT para peer discovery
- ✅ SQLite persistence (peers, records)
- ✅ Health check endpoint
- ✅ Docker + docker-compose
- 📊 **~700 LoC**, 6 arquivos

**FASE 10: P2P Relay + TURN Server (100%)** ✅
- ✅ libp2p Circuit Relay v2 (server + client)
- ✅ DCUtR hole punching automático
- ✅ coturn TURN server configurado
- ✅ TURN credentials service (HMAC-SHA1)
- ✅ Fallback automático (direto → hole punch → relay)
- 📊 **~1.650 LoC**, 18 arquivos

**FASE 11: Message Store (100%)** ✅
- ✅ PostgreSQL + Redis para store & forward
- ✅ API REST (store, retrieve, delete)
- ✅ TTL automático (14 dias)
- ✅ Encryption em repouso
- 📊 **~900 LoC**, 7 arquivos

**FASE 12: VoIP - Chamadas de Voz (100%)** ✅
- ✅ WebRTC integration (webrtc-rs + SDP + ICE)
- ✅ Opus codec (24kbps, 20ms frames)
- ✅ P2P signaling via libp2p
- ✅ Signaling server WebSocket (fallback) - `server/signaling`
- ✅ TURN client integration
- ✅ Android UI (CallScreen + IncomingCallScreen)
- ✅ Desktop UI (CallView + IncomingCallModal)
- ✅ Runtime permissions (RECORD_AUDIO, BLUETOOTH_CONNECT)
- ✅ CallAudioManager (Bluetooth auto-routing)
- ✅ Background calls (foreground service PHONE_CALL)
- ✅ Call history database (SQLite schema v2)
- 📊 **~4.600 LoC**, 24/24 tarefas completas

**FASE 13: iOS App (em progresso)** 🚧
- ✅ Xcode project setup (via xcodegen CLI)
- ✅ Swift + SwiftUI UI (Login, Conversations, Chat, Settings, Call) - 2.100+ LoC
- ✅ UniFFI bindings gerados (zaplivre.swift 2.357 LoC)
- ✅ VoIP integration com CallKit (CallManager 309 LoC)
- ✅ Audio I/O com AVAudioEngine (AudioManager 311 LoC)
- ✅ QR Scanner com AVFoundation (238 LoC)
- ✅ **Rust core compila para iOS** (conditional compilation #[cfg(feature = "voip")])
- ✅ **Library integrada** (libzaplivre_core_ios.a + libzaplivre_core_sim.a)
- ✅ **Build bem-sucedida:** xcodebuild -scheme ZapLivre build → BUILD SUCCEEDED!
- ✅ **Build pipeline automatizado** (build-all.sh, build-rust.sh, generate-bindings.sh)
- ✅ **Documentação completa** (README.md com guias de setup, arquitetura, troubleshooting)
- ⚠️ **Testes end-to-end:** pendente
- 📊 **~3.700 LoC Swift + 2.357 LoC bindings**, 11/11 tarefas de desenvolvimento completas

### 📊 Estado por plataforma

Linhas de código reais no repositório (fora os bindings UniFFI gerados):

| Plataforma | LoC | Estado |
|------------|-----|--------|
| **Core + servidores (Rust)** | ~32.800 | **Maduro** — 141 testes passando (identity, crypto, storage, network, voip) |
| **Android (Kotlin/Compose)** | ~18.700 | 14 telas + design system ZapLivre; rodou no Android pela 1ª vez em 07/2026 (3 bugs de runtime corrigidos); inicialização P2P ainda em estabilização |
| **iOS (Swift/SwiftUI)** | ~15.900 | 25 telas; roda no simulador; VoIP via CallKit; testes end-to-end pendentes |
| **Desktop (Tauri/React/TS)** | ~5.000 | **Funcional** — build multiplataforma + suíte Vitest |
| **Total** | **~72.400** | — |

### 📊 Estado por funcionalidade

O código de grupos, mídia e vídeo **já existe** nas plataformas (telas + FFI + core) — o
que falta na maioria dos casos é validação end-to-end, não implementação.

| Funcionalidade | Estado |
|----------------|--------|
| Identidade (Ed25519) + E2E (Signal Double Ratchet) | ✅ implementado e testado no core |
| Mensagens de texto 1:1 | ✅ implementado |
| Grupos (sender keys) | 🔨 telas nas 3 plataformas + FFI (`create_group`, `add_group_member`, `send_group_message`); validação E2E pendente |
| Mídia (imagem / vídeo / arquivo) | 🔨 pickers, galeria e viewer implementados |
| Chamadas de voz (WebRTC + Opus) | 🔨 implementado; validação cross-platform pendente |
| Videochamadas (H.264 + CameraX) | 🔨 telas + `enable_video`/`rtp_video` implementados; validação pendente |
| Push (FCM + APNs) | 🔨 servidor + clientes implementados; integração final pendente |
| Infra self-hosted (bootstrap/DHT, relay/TURN, store, signaling) | ✅ implementada |
| Multi-device sync | ⏳ planejado |

Legenda: ✅ maduro / testado · 🔨 implementado, validação end-to-end pendente · ⏳ planejado

### 🎯 Foco atual

- **iOS**: testes end-to-end no simulador, APNs, TestFlight
- **Android**: estabilizar a inicialização P2P em runtime, testes end-to-end no emulador/device
- **Cross-platform**: validar mensagens, grupos e VoIP entre Android ↔ iOS ↔ Desktop

---

## 🚀 Roadmap

### Mês 1-2: Setup & Fundação ✅
- [x] Estrutura do monorepo
- [x] Workspace Rust configurado
- [x] Core library completa
- [ ] CI/CD básico
- [ ] Landing page
- [ ] 50-100 beta testers

### Mês 3: Mensagens Básicas ✅
- [x] Core library (Identity + Crypto + Network + Storage)
- [x] Android MVP (mensagens texto)
- [x] Desktop MVP (Tauri)
- [x] Bootstrap + TURN + Store servers
- [ ] 10 beta testers trocando mensagens

### Mês 4: CHAMADAS DE VOZ ✅ **100% COMPLETO**
- [x] WebRTC integration
- [x] TURN relay
- [x] UI de chamadas (Android + Desktop)
- [x] Runtime permissions + Bluetooth
- [x] Qualidade validada
- **Próximo:** Testes cross-platform com beta testers

### iOS App 🚧
- [x] App iOS (Swift + SwiftUI, 25 telas)
- [x] CallKit + AVAudioEngine + QR Scanner
- [x] Build Rust core para iOS + library integrada
- [ ] Testes end-to-end no Simulator
- [ ] Testes em dispositivos físicos + TestFlight

### Grupos, Mídia e Vídeo 🔨 (implementados — validação pendente)
- [x] Grupos: telas + FFI (sender keys) nas 3 plataformas
- [x] Mídia: imagens, vídeos e arquivos (pickers, galeria, viewer)
- [x] Videochamadas 1:1: telas + `enable_video`/`rtp_video`
- [ ] Validação end-to-end (2 devices) de grupos, mídia e vídeo
- [ ] Chamadas em grupo (até 8)

### Ainda planejado ⏳
- [ ] Multi-device sync
- [ ] Integração final de push (FCM/APNs)

---

## 🛠️ Desenvolvimento

### Pré-requisitos

- **Rust:** 1.88+ (`rustup default stable`)
- **Node.js:** 18+ (para desktop app)
- **Android Studio:** Hedgehog+ (para Android app)
- **NDK:** 26.1.10909125+
- **Docker:** (para servidores)

### Build Rápido

```bash
# Core library
cd core
cargo build --release

# Android app (requer NDK)
cd android
./gradlew assembleDebug

# Desktop app
cd desktop
npm install
npm run tauri dev

# DMG (macOS)
cd ..
make dmg

# Servidores (Bootstrap + TURN + Store)
cd server
docker-compose up -d
```

### Testes

```bash
# Core tests (110+ testes)
cd core
cargo test --workspace

# Benchmarks
cargo bench

# Android (manual)
# Seguir BUILD_AND_TEST.md
```

**Documentação completa:** [BUILD_AND_TEST.md](BUILD_AND_TEST.md)

**DMG macOS:** veja `scripts/build-dmg.md` para instruções detalhadas.

---

## 📖 Documentação

### Guias Principais
- [**Plano de Execução**](EXECUCAO.md) - Fases detalhadas, progresso atual
- [**Build & Test Guide**](BUILD_AND_TEST.md) - Como buildar e testar VoIP
- [**Arquitetura Híbrida**](docs/architecture/01-hybrid-architecture.md) - Por que P2P + Servidor
- [**Tech Stack**](documentos/zaplivre_tech_stack_completo.md) - Bibliotecas e justificativas

### Por Componente
- **Android:** [BUILD_GUIDE.md](android/BUILD_GUIDE.md) | [TESTING.md](android/TESTING.md) | [README.md](android/README.md)
- **Desktop:** [README.md](desktop/README.md) | [ARCHITECTURE.md](desktop/ARCHITECTURE.md)
- **Core:** [FFI_IMPLEMENTATION.md](core/FFI_IMPLEMENTATION.md) | [FASE5_ARTIFACTS.md](core/FASE5_ARTIFACTS.md)
- **Servers:** READMEs em `server/bootstrap`, `server/store`, `server/push`

---

## 🤝 Contribuindo

Aceitamos contribuições! Veja [CONTRIBUTING.md](CONTRIBUTING.md) para detalhes.

**Áreas que precisamos:**
- 🦀 **Core Developers** (Rust: libp2p, WebRTC, crypto)
- 📱 **Mobile Developers** (Kotlin/Compose, Swift/SwiftUI)
- 🖥️ **Desktop Developers** (Tauri, React, TypeScript)
- 🎨 **Designers** (UI/UX para Android/iOS/Desktop)
- 📝 **Documentação** (technical writers)
- 🌍 **Tradutores** (i18n: pt-BR, en, es)
- 🧪 **QA Testers** (testes VoIP em dispositivos reais)

---

## 📊 Status Detalhado

**Versão:** 0.1.0-alpha (em desenvolvimento)

| Fase | Componente | Status | Progresso |
|------|------------|--------|-----------|
| 1-5 | Core (Rust) | ✅ Completo | 100% |
| 6 | Android App | ✅ Completo | 100% |
| 7 | Desktop App | ✅ Completo | 100% |
| 8 | Push Notifications | 🚧 Em progresso | 75% |
| 9 | Bootstrap + DHT | ✅ Completo | 100% |
| 10 | P2P Relay + TURN | ✅ Completo | 100% |
| 11 | Message Store | ✅ Completo | 100% |
| 12 | VoIP Calls | ✅ Completo | 100% |
| 13 | **iOS App** | 🚧 **Em progresso** | **85%** |
| 14 | Videochamadas | ⏳ Aguardando | 0% |
| 15 | Grupos | ⏳ Aguardando | 0% |
| 16 | Mídia & Polimento | ⏳ Aguardando | 0% |
| 17 | Multi-Device | ⏳ Aguardando | 0% |

**Progresso geral:** 11/19 fases (58%) | ~28.764 LoC (75% do estimado)

---

## 💰 Modelo de Negócio

**Open Source Core + Serviços Opcionais**

### Sempre gratuito:
- ✅ Código completo (AGPL v3)
- ✅ Apps (Android/iOS/Desktop)
- ✅ Documentação
- ✅ Relay comunitário (best-effort)

### Opções pagas (futuro):
- **ZapLivre Cloud Relay** ($5-20/mês): SLA 99.9%, suporte
- **Enterprise Self-Hosted:** Suporte técnico, instalação
- **Custom Development:** Features sob demanda

---

## 🎯 Milestone Crítico (Próximo)

**TESTE DECISIVO após FASE 12:**

Perguntar a 20+ beta testers:
> **"Você usaria ZapLivre como seu chat principal?"**

- **< 50% SIM:** ⛔ PARA TUDO e conserta VoIP
- **50-70% SIM:** ⚠️ Continua com cautela, itera feedback
- **> 70% SIM:** 🚀 Full speed para iOS (FASE 13)

*Chamadas de voz são deal-breaker. 87% dos brasileiros usam WhatsApp para chamadas.*

---

## 📜 Licença

[AGPL-3.0](LICENSE) - Este projeto é open source.

**IMPORTANTE:** AGPL impede forks fechados. Se você usar ZapLivre em um serviço, deve disponibilizar o código-fonte.

---

## 🙏 Agradecimentos

Construído com tecnologias open source incríveis:
- [**libp2p**](https://libp2p.io/) - Protocol Labs
- [**Signal Protocol**](https://signal.org/docs/) - Signal Foundation
- [**WebRTC**](https://webrtc.org/) - webrtc-rs
- [**Tauri**](https://tauri.app/)
- [**UniFFI**](https://mozilla.github.io/uniffi-rs/) - Mozilla
- E muitas outras...

---

## 📞 Contato

- **Website:** [zaplivre.app](https://zaplivre.app) *(em breve)*
- **GitHub:** [github.com/integralltech/zaplivre](https://github.com/integralltech/zaplivre)
- **Discord:** *(em breve)*
- **Email:** contato@integralltech.com.br

---

<div align="center">

**Feito com ❤️ por [IntegrallTech](https://integralltech.com.br)**

*"Não adianta ter privacidade perfeita se ninguém usar.*
*ZapLivre escolhe privacidade boa o suficiente + UX boa o suficiente = Adoção real."*

[![Star on GitHub](https://img.shields.io/github/stars/integralltech/zaplivre?style=social)](https://github.com/integralltech/zaplivre)

</div>
