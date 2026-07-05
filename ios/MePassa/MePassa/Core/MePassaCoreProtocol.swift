//
//  MePassaCoreProtocol.swift
//  MePassa
//
//  Created by MePassa Team
//  Copyright © 2026 MePassa. All rights reserved.
//
//  Abstraction over MePassaCore so that AppState (and tests) can depend on
//  a protocol instead of the concrete FFI-backed singleton.

import Foundation

/// Surface of MePassaCore consumed by AppState / app bootstrap logic.
/// Kept intentionally small - extend only when a consumer needs it.
protocol MePassaCoreProtocol: AnyObject {
    /// True once the underlying Rust client has been created
    var isInitialized: Bool { get }

    /// Local libp2p peer ID (the user's identity), nil before initialization
    var localPeerId: String? { get }

    /// Initialize the core (creates the Rust client and loads the identity)
    func initialize() async throws

    /// Start listening for incoming P2P connections
    func startListening() async throws

    /// Connect to bootstrap nodes (DHT)
    func bootstrap() async throws

    /// List all conversations
    func listConversations() async throws -> [FfiConversationWrapper]

    /// Scan recent direct messages for group sender-key payloads
    func scanGroupSenderKeyMessages() async
}

extension MePassaCore: MePassaCoreProtocol {
    /// Protocol shim: methods with default parameter values cannot satisfy
    /// a parameterless protocol requirement, so forward explicitly.
    func scanGroupSenderKeyMessages() async {
        await scanGroupSenderKeyMessages(limitPerConversation: 50, minInterval: 30)
    }
}
