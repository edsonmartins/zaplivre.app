# Audit Report — ZapLivre Codebase

## Scope
- Core (Rust): networking, storage, identity, protocol, VoIP, FFI
- Desktop (Tauri/React)
- iOS (SwiftUI)
- Android (Kotlin)
- Server (identity, push, bootstrap)

## Method
- Static scan for TODOs, mocks, stubs, placeholders, missing integrations
- Review of core flows (identity → network → storage → UI) and failure modes
- Architecture sanity checks (state propagation, delivery semantics, reconnection strategy)

## Executive Summary
The codebase has strong foundations but several critical gaps block reliable messaging, delivery receipts, reconnection outside LAN, and end‑to‑end security. There is also no unified event propagation layer to push core events to UI clients, leading to stale UI state and reliance on polling or view reloads. These need to be addressed before scaling or deploying bootstrap infrastructure.

## Critical Findings (Must Fix)
1) ACKs not actually sent back to sender
- Impact: delivery status never advances, retry logic cannot converge, UI status inconsistent.
- Evidence: ACK is created but not sent: `core/src/network/swarm.rs#L496` (TODO “ACK is created but not sent back”).

2) No E2E encryption in message storage or network
- Impact: plaintext messages in transit and at rest; unacceptable for production.
- Evidence: `core/src/network/message_handler.rs#L184` (TODO for E2E), `core/Cargo.toml#L41` (signal protocol TODO).

3) No real event propagation from core to UI layers
- Impact: UI does not update when messages arrive; requires reload to see state.
- Evidence: `core/src/api/builder.rs#L121` (event channel TODO), `core/src/network/message_handler.rs#L29` (event_tx Option never wired).

4) Bootstrap/DHT peers not configured in clients
- Impact: reconnection fails outside LAN; DHT lookups fail without bootstrap.
- Evidence: `core/src/api/builder.rs#L130` uses add_bootstrap_peer but apps never set peers.

5) Identity lifecycle incomplete (iOS)
- Impact: users cannot restore identity; forced to create new keypairs leading to peer changes.
- Evidence: `ios/ZapLivre/ZapLivre/Core/ZapLivreCore.swift#L61` and `#L67` (import/export TODO).

## High Priority Findings
1) NAT traversal detection is placeholder
- Impact: connection reliability degraded on real networks.
- Evidence: `core/src/network/nat_detection.rs#L59`.

2) Group security incomplete
- Impact: group signatures not verified; trust model broken.
- Evidence: `core/src/group/manager.rs#L359`.

3) Media pipeline incomplete
- Impact: media handling inconsistent and partially stubbed.
- Evidence: `core/src/api/client.rs#L238-L461` (media send/receive TODOs).

4) VoIP/Video stubs in FFI and core
- Impact: iOS/Android UI shows features that are not implemented or partially stubbed.
- Evidence: `core/src/ffi/client.rs#L1261-L1347`, `core/src/voip/*.rs` TODOs.

## Medium Priority Findings
1) Message ordering and delivery semantics are not enforced in core
- Impact: UI ordering bugs; no clear source of truth for chronology.
- Evidence: no ordering guarantees beyond timestamps; UI sorts vary across platforms.

2) DHT publish/resolve lacks durability strategy
- Impact: addresses may expire or not be republished; reconnection can degrade.
- Evidence: DHT publish on listen only; no republish TTL or refresh loop.

3) Missing retry/delivery pipeline for outbound messages
- Impact: message send may fail without automatic retry or offline queue.
- Evidence: no queue logic in `core/src/api/client.rs` beyond immediate send.

4) Push notification integration (iOS)
- Impact: non‑production APNS URL and missing peer ID; push won’t route.
- Evidence: `ios/ZapLivre/ZapLivre/Core/PushNotificationManager.swift#L18` and `#L69`.

## Low Priority / UI & Docs
- Group screens and settings include multiple mocks/placeholders across iOS/Android/Desktop.
- Various UI placeholders and unimplemented navigation items.

## Architectural Gaps
1) Event Bus Missing
- A core event bus should stream message/connection status into UI clients.
- Currently, event_tx is unused and callbacks are not propagated in FFI apps.

2) Identity Persistence & Recovery
- Identity exists in core but is not exposed with robust backup/restore in iOS/Android.
- Without stable identity, peer IDs change and DHT entries become stale.

3) Trust & Encryption Model
- No Signal/Double‑Ratchet integration yet; storage is plaintext.
- Must be implemented before production.

4) Delivery Semantics & Acks
- ACK creation exists but transport response is missing. No read receipts flow.

5) Network Resilience Strategy
- Need explicit bootstrap nodes + periodic republish + relay fallback strategy.

## Recommended Fix Order (Discovery → Implementation)
1) **Core messaging correctness**
   - Send ACK responses over network.
   - Add event channel and deliver events to app layers.

2) **Identity stability**
   - Implement iOS/Android identity export/import.
   - Add “restore identity” flow to UI.

3) **Network robustness**
   - Configure bootstrap peers in apps.
   - Add DHT republish loop + TTL strategy.
   - Improve NAT detection / relay usage.

4) **Security**
   - Implement Signal protocol (E2E) and encrypted storage.

5) **Feature completeness**
   - Media send/receive pipeline.
   - Group signature verification.
   - VoIP integration (or hide UI until ready).

## Proposed Deliverables
- Patch series implementing ACK send + event bus.
- Identity backup/restore for iOS + Android.
- Bootstrap configuration + DHT republish strategy.
- Architecture doc update + threat model.

## Next Step
After addressing the above, proceed with bootstrap server deployment (`server/bootstrap`) and configure clients to connect to those nodes by default.
