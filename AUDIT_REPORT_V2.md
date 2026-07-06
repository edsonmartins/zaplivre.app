# Audit Report v2 — ZapLivre Codebase

## Scope
- Core (Rust): networking, storage, identity, protocol, VoIP, FFI
- Desktop (Tauri/React)
- iOS (SwiftUI)
- Android (Kotlin)
- Server (identity, push, bootstrap)

## Method
- Static scan for TODOs/mocks/stubs/placeholders
- Architecture review by flow (identity → networking → storage → UI)
- Cross‑app consistency review (features exposed vs implemented)

## Executive Summary
Despite recent fixes (ACK sending + event bus + identity import/export), the codebase still contains multiple critical gaps that block production readiness: end‑to‑end encryption is missing, bootstrap configuration is not owned by ZapLivre, NAT detection is placeholder, group signature verification is incomplete, and media send/receive pipeline is partially stubbed. VoIP remains largely stubbed across core + apps. These should be resolved before bootstrap server deployment.

## Critical Gaps (P0)
1) **E2E encryption missing**
- Impact: plaintext in transit/at rest; production blocker.
- Evidence: `core/src/network/message_handler.rs:191`, `core/Cargo.toml:41`.

2) **Bootstrap nodes not owned/configured**
- Impact: current FFI uses IPFS public bootstrap; not aligned with product/security.
- Evidence: `core/src/ffi/client.rs:726`.

3) **NAT detection placeholder**
- Impact: connectivity unreliable in real networks.
- Evidence: `core/src/network/nat_detection.rs:59`.

4) **Group signature verification missing**
- Impact: integrity and trust model for groups incomplete.
- Evidence: `core/src/group/manager.rs:359`.

5) **Media pipeline incomplete**
- Impact: send/download/store not fully implemented; breaks UX.
- Evidence: `core/src/api/client.rs:239, 246, 273, 333, 379, 455, 462`.

## High Priority (P1)
1) **VoIP/Video stubs** (core + FFI + apps)
- Evidence: `core/src/ffi/client.rs:1261-1347`, `core/src/voip/manager.rs:501+`, `core/src/voip/video_pipeline.rs:38/95`, `core/src/voip/integration.rs:328/340`, `ios/ZapLivre/ZapLivre/VoIP/CallManager.swift:115+`, `android/app/src/main/kotlin/com/zaplivre/ui/screens/call/VideoCallScreen.kt:256`.

2) **Relay integration incomplete**
- Evidence: `core/src/network/swarm.rs:262`.

3) **Push notifications not production‑ready (iOS)**
- Evidence: `ios/ZapLivre/ZapLivre/Core/PushNotificationManager.swift:18`.

## Medium Priority (P2)
1) **Reactions/forwarding not broadcast**
- Evidence: `core/src/api/client.rs:550, 575, 588`.

2) **Config placeholders**
- Evidence: `core/src/utils/config.rs:3`.

3) **Message ordering not standardized**
- Impact: inconsistent ordering between platforms.
- Evidence: no shared ordering spec; UI sorts vary.

## Low Priority (P3)
- Group UI placeholders in iOS/Android/Desktop.
- Profile/avatar and settings TODOs.

## Recommendations / Fix Order
1) **Security foundation**: implement Signal protocol (E2E) + encrypted storage.
2) **Network ownership**: replace IPFS bootstrap with ZapLivre bootstrap nodes; configurable list.
3) **Connectivity**: implement NAT detection + relay strategy.
4) **Media pipeline**: send/download/store and path management.
5) **VoIP**: either implement fully or hide features in UI.
6) **Push**: production push service setup.

## Next Step
Pick the first fix area and implement in priority order. Suggested first actions:
- Replace bootstrap list with ZapLivre nodes (config‑driven).
- Add NAT detection + relay reservation strategy.
- Begin E2E encryption integration plan.
