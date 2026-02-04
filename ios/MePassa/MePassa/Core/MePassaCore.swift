//
//  MePassaCore.swift
//  MePassa
//
//  Created by MePassa Team
//  Copyright © 2026 MePassa. All rights reserved.
//
//  Swift wrapper around UniFFI generated bindings
//  This provides a cleaner API for SwiftUI views

import Foundation
// Note: No need to import mepassa - the generated Swift code (mepassa.swift)
// is part of the same target. The bridging header already imports mepassaFFI.h

/// Swift wrapper for MePassa Core FFI
class MePassaCore: ObservableObject {
    // Shared singleton instance
    static let shared: MePassaCore = {
        let documentsPath = FileManager.default.urls(for: .documentDirectory, in: .userDomainMask)[0]
        let dataDir = documentsPath.appendingPathComponent("mepassa_data").path
        return MePassaCore(dataDir: dataDir)
    }()

    private var client: MePassaClient?

    private let dataDir: String
    @Published var isInitialized = false
    @Published var localPeerId: String?

    init(dataDir: String) {
        self.dataDir = dataDir
    }

    // MARK: - Initialization

    /// Initialize the MePassa core library
    func initialize() async throws {
        print("📱 MePassa Core initializing at: \(dataDir)")

        if let storeUrl = Bundle.main.object(forInfoDictionaryKey: "MESSAGE_STORE_URL") as? String {
            if !storeUrl.trimmingCharacters(in: .whitespacesAndNewlines).isEmpty {
                setenv("MESSAGE_STORE_URL", storeUrl, 1)
            }
        }

        setIdentityEnvFromKeychain()

        client = try await MePassaClient(dataDir: dataDir)
        localPeerId = try await client?.localPeerId()

        persistIdentityToKeychainIfNeeded()

        DispatchQueue.main.async {
            self.isInitialized = true
        }

        print("✅ MePassa Core initialized with peer ID: \(localPeerId ?? "unknown")")
    }

    // MARK: - Identity Management

    /// Generate new identity (keypair)
    func generateNewIdentity() async throws -> String {
        // This is done during initialization
        // The peer ID is derived from the public key
        return try await client?.localPeerId() ?? ""
    }

    /// Import existing identity from backup
    func importIdentity(backup: String) async throws {
        if client != nil {
            throw MePassaCoreError.storageError("Import requires app restart")
        }

        guard let data = Data(base64Encoded: backup.trimmingCharacters(in: .whitespacesAndNewlines)) else {
            throw MePassaCoreError.storageError("Invalid backup data")
        }

        try KeychainStore.saveIdentity(data)
        removeIdentityFileIfExists()

        let dbPath = databasePath()
        if FileManager.default.fileExists(atPath: dbPath) {
            try FileManager.default.removeItem(atPath: dbPath)
        }
    }

    /// Export current identity for backup
    func exportIdentity() async throws -> String {
        if let data = try KeychainStore.loadIdentity() {
            return data.base64EncodedString()
        }
        let keyPath = identityKeyPath()
        let data = try Data(contentsOf: URL(fileURLWithPath: keyPath))
        return data.base64EncodedString()
    }

    /// Export prekey bundle (JSON) for sharing with peers
    func exportPrekeyBundle() async throws -> String {
        return try await client?.getPrekeyBundleJson() ?? ""
    }

    /// Store a peer's prekey bundle (JSON) for E2E encryption
    func storePeerPrekeyBundle(peerId: String, bundleJson: String) throws {
        try client?.setContactPrekeyBundle(peerId: peerId, prekeyBundleJson: bundleJson)
    }

    // MARK: - Networking

    /// Start listening on default multiaddrs
    func startListening() async throws {
        try await client?.listenOn(multiaddr: "/ip4/0.0.0.0/tcp/0")
        try await client?.listenOn(multiaddr: "/ip6/::/tcp/0")
        print("📡 Started listening on P2P network")
    }

    private func identityKeyPath() -> String {
        return URL(fileURLWithPath: dataDir)
            .appendingPathComponent("identity.key")
            .path
    }

    private func setIdentityEnvFromKeychain() {
        do {
            if let data = try KeychainStore.loadIdentity() {
                let b64 = data.base64EncodedString()
                setenv("MEPASSA_IDENTITY_B64", b64, 1)
                return
            }
        } catch {
            print("⚠️ Failed to read identity from Keychain: \(error)")
        }
        unsetenv("MEPASSA_IDENTITY_B64")
    }

    private func persistIdentityToKeychainIfNeeded() {
        do {
            if try KeychainStore.loadIdentity() == nil {
                let keyPath = identityKeyPath()
                if FileManager.default.fileExists(atPath: keyPath) {
                    let data = try Data(contentsOf: URL(fileURLWithPath: keyPath))
                    try KeychainStore.saveIdentity(data)
                }
            }

            // Remove file-based identity to avoid plaintext storage
            removeIdentityFileIfExists()
        } catch {
            print("⚠️ Failed to persist identity in Keychain: \(error)")
        }
    }

    private func removeIdentityFileIfExists() {
        let keyPath = identityKeyPath()
        if FileManager.default.fileExists(atPath: keyPath) {
            try? FileManager.default.removeItem(atPath: keyPath)
        }
    }

    private func databasePath() -> String {
        return URL(fileURLWithPath: dataDir)
            .appendingPathComponent("mepassa.db")
            .path
    }

    /// Connect to bootstrap nodes
    func bootstrap() async throws {
        try await client?.bootstrap()
        print("🌐 Connected to bootstrap nodes")
    }

    /// Connect to a specific peer
    func connectToPeer(peerId: String, multiaddr: String) async throws {
        try await client?.connectToPeer(peerId: peerId, multiaddr: multiaddr)
        print("🔗 Connecting to peer: \(peerId)")
    }

    /// Get count of connected peers
    func connectedPeersCount() async throws -> Int {
        return try await Int(client?.connectedPeersCount() ?? 0)
    }

    // MARK: - Messaging

    /// Send text message to peer
    func sendMessage(to peerId: String, content: String) async throws -> String {
        let messageId = try await client?.sendTextMessage(toPeerId: peerId, content: content) ?? ""
        print("📨 Sent message to \(peerId): \(content)")
        return messageId
    }

    /// Send image message to peer (with compression in Rust core)
    func sendImageMessage(to peerId: String, imageData: Data, fileName: String, quality: UInt32 = 85) async throws -> String {
        let imageBytes = [UInt8](imageData)
        let messageId = try await client?.sendImageMessage(
            toPeerId: peerId,
            imageData: imageBytes,
            fileName: fileName,
            quality: quality
        ) ?? ""
        print("📷 Sent image to \(peerId): \(fileName)")
        return messageId
    }

    /// Send voice message to peer
    func sendVoiceMessage(to peerId: String, audioData: Data, fileName: String, durationSeconds: Int32) async throws -> String {
        let audioBytes = [UInt8](audioData)
        let messageId = try await client?.sendVoiceMessage(
            toPeerId: peerId,
            audioData: audioBytes,
            fileName: fileName,
            durationSeconds: durationSeconds
        ) ?? ""
        print("🎤 Sent voice message to \(peerId): \(fileName) (\(durationSeconds)s)")
        return messageId
    }

    /// Get messages for a conversation
    func getMessages(for peerId: String, limit: Int? = nil) async throws -> [FfiMessageWrapper] {
        let messages = try await client?.getConversationMessages(
            peerId: peerId,
            limit: limit.map { UInt32($0) },
            offset: nil
        ) ?? []

        return messages.map { FfiMessageWrapper(ffi: $0) }
    }

    /// Get conversation messages (alias for getMessages)
    func getConversationMessages(peerId: String, limit: UInt32?, offset: UInt32?) async throws -> [FfiMessageWrapper] {
        let messages = try await client?.getConversationMessages(
            peerId: peerId,
            limit: limit,
            offset: offset
        ) ?? []

        return messages.map { FfiMessageWrapper(ffi: $0) }
    }

    /// Delete message
    func deleteMessage(messageId: String) async throws {
        guard let client = client else {
            throw MePassaCoreError.notInitialized
        }

        try await client.deleteMessage(messageId: messageId)
    }

    /// Forward message
    func forwardMessage(messageId: String, toPeerId: String) async throws -> String {
        guard let client = client else {
            throw MePassaCoreError.notInitialized
        }

        return try await client.forwardMessage(messageId: messageId, toPeerId: toPeerId)
    }

    /// Get all conversations
    func listConversations() async throws -> [FfiConversationWrapper] {
        let conversations = try await client?.listConversations() ?? []
        return conversations.map { FfiConversationWrapper(ffi: $0) }
    }

    /// Mark conversation as read
    func markAsRead(peerId: String) async throws {
        try await client?.markConversationRead(peerId: peerId)
        print("✅ Marked conversation as read: \(peerId)")
    }

    /// Get media for a conversation
    func getConversationMedia(conversationId: String, mediaType: FfiMediaType?, limit: UInt32?) async throws -> [FfiMedia] {
        guard let client = client else {
            throw MePassaCoreError.notInitialized
        }

        return try await client.getConversationMedia(conversationId: conversationId, mediaType: mediaType, limit: limit)
    }

    /// Download media by hash
    func downloadMedia(mediaHash: String) async throws -> Data {
        guard let client = client else {
            throw MePassaCoreError.notInitialized
        }

        let bytes = try await client.downloadMedia(mediaHash: mediaHash)
        return Data(bytes)
    }

    /// Search messages
    func searchMessages(query: String, limit: UInt32?) async throws -> [FfiMessageWrapper] {
        guard let client = client else {
            throw MePassaCoreError.notInitialized
        }

        let messages = try await client.searchMessages(query: query, limit: limit)
        return messages.map { FfiMessageWrapper(ffi: $0) }
    }

    /// Get message reactions (stub for now)
    func getMessageReactions(messageId: String) async throws -> [FfiReaction] {
        // TODO: Implement when reaction support is added to FFI
        return []
    }

    /// Add reaction (stub for now)
    func addReaction(messageId: String, emoji: String) async throws {
        // TODO: Implement when reaction support is added to FFI
        print("⚠️ addReaction not yet implemented")
    }

    /// Remove reaction (stub for now)
    func removeReaction(messageId: String, emoji: String) async throws {
        // TODO: Implement when reaction support is added to FFI
        print("⚠️ removeReaction not yet implemented")
    }

    /// Send document message (stub for now)
    func sendDocumentMessage(to peerId: String, fileData: Data, fileName: String, mimeType: String) async throws -> String {
        // TODO: Implement when document support is added to FFI
        print("⚠️ sendDocumentMessage not yet implemented")
        return UUID().uuidString
    }

    /// Send video message (stub for now)
    func sendVideoMessage(toPeerId peerId: String, videoData: Data, fileName: String, width: Int32, height: Int32, durationSeconds: Int32, thumbnailData: Data?) async throws -> String {
        // TODO: Implement when video message support is added to FFI
        print("⚠️ sendVideoMessage not yet implemented")
        return UUID().uuidString
    }

    // MARK: - VoIP Calls

    /// Start voice call
    func startCall(to peerId: String) async throws -> String {
        let callId = try await client?.startCall(toPeerId: peerId) ?? ""
        print("📞 Starting call to: \(peerId)")
        return callId
    }

    /// Accept incoming call
    func acceptCall(callId: String) async throws {
        try await client?.acceptCall(callId: callId)
        print("✅ Accepted call: \(callId)")
    }

    /// Reject incoming call
    func rejectCall(callId: String, reason: String? = nil) async throws {
        try await client?.rejectCall(callId: callId, reason: reason)
        print("❌ Rejected call: \(callId)")
    }

    /// Hang up active call
    func hangupCall(callId: String) async throws {
        try await client?.hangupCall(callId: callId)
        print("📴 Hung up call: \(callId)")
    }

    /// Toggle mute status
    func toggleMute(callId: String) async throws {
        try await client?.toggleMute(callId: callId)
        print("🔇 Toggled mute for call: \(callId)")
    }

    /// Toggle speakerphone
    func toggleSpeaker(callId: String) async throws {
        try await client?.toggleSpeakerphone(callId: callId)
        print("🔊 Toggled speaker for call: \(callId)")
    }

    // MARK: - Groups (FASE 15)

    /// Create a new group
    func createGroup(name: String, description: String?) async throws -> FfiGroupWrapper {
        let group = try await client?.createGroup(name: name, description: description)
        print("👥 Created group: \(name)")
        return FfiGroupWrapper(ffi: group!)
    }

    /// Join an existing group
    func joinGroup(groupId: String, groupName: String) async throws {
        try await client?.joinGroup(groupId: groupId, groupName: groupName)
        print("✅ Joined group: \(groupName)")
    }

    /// Leave a group
    func leaveGroup(groupId: String) async throws {
        try await client?.leaveGroup(groupId: groupId)
        print("👋 Left group: \(groupId)")
    }

    /// Add member to group (admin only)
    func addGroupMember(groupId: String, peerId: String) async throws {
        try await client?.addGroupMember(groupId: groupId, peerId: peerId)
        print("➕ Added member to group \(groupId): \(peerId)")
    }

    /// Remove member from group (admin only)
    func removeGroupMember(groupId: String, peerId: String) async throws {
        try await client?.removeGroupMember(groupId: groupId, peerId: peerId)
        print("➖ Removed member from group \(groupId): \(peerId)")
    }

    /// Get all groups
    func getGroups() async throws -> [FfiGroupWrapper] {
        let groups = try await client?.getGroups() ?? []
        return groups.map { FfiGroupWrapper(ffi: $0) }
    }

    /// Get group messages
    func getGroupMessages(groupId: String, limit: Int? = nil) async throws -> [FfiMessageWrapper] {
        let messages = try client?.getGroupMessages(
            groupId: groupId,
            limit: limit.map { UInt32($0) },
            offset: nil
        ) ?? []
        return messages.map { FfiMessageWrapper(ffi: $0) }
    }

    /// Send message to group
    func sendGroupMessage(groupId: String, content: String) async throws -> String {
        return try await client?.sendGroupMessage(groupId: groupId, content: content) ?? ""
    }

    // MARK: - Video Methods (FASE 14)

    /// Enable video for an active call
    /// - Parameters:
    ///   - callId: Call identifier
    ///   - codec: Video codec to use (H264, VP8, VP9)
    func enableVideo(callId: String, codec: FfiVideoCodec = .h264) async throws {
        guard let client = client else {
            throw MePassaCoreError.notInitialized
        }

        try await client.enableVideo(callId: callId, codec: codec)
        print("📹 Video enabled for call: \(callId) with codec: \(codec)")
    }

    /// Disable video for an active call
    /// - Parameter callId: Call identifier
    func disableVideo(callId: String) async throws {
        guard let client = client else {
            throw MePassaCoreError.notInitialized
        }

        try await client.disableVideo(callId: callId)
        print("🚫 Video disabled for call: \(callId)")
    }

    /// Send video frame to remote peer
    /// - Parameters:
    ///   - callId: Call identifier
    ///   - frameData: Raw frame data (pre-encoded H.264/VP8 NALUs)
    ///   - width: Frame width in pixels
    ///   - height: Frame height in pixels
    func sendVideoFrame(callId: String, frameData: [UInt8], width: UInt32, height: UInt32) async throws {
        guard let client = client else {
            throw MePassaCoreError.notInitialized
        }

        try await client.sendVideoFrame(callId: callId, frameData: frameData, width: width, height: height)
        // Don't log every frame - too noisy
    }

    /// Switch camera (front/back) during video call
    /// - Parameter callId: Call identifier
    func switchCamera(callId: String) async throws {
        guard let client = client else {
            throw MePassaCoreError.notInitialized
        }

        try await client.switchCamera(callId: callId)
        print("📷 Camera switched for call: \(callId)")
    }

    /// Register callback for receiving remote video frames (FASE 14)
    ///
    /// The callback will be invoked on a background thread whenever a remote
    /// video frame is received during an active video call.
    ///
    /// - Parameter callback: Implementation of FfiVideoFrameCallback protocol
    func registerVideoFrameCallback(_ callback: FfiVideoFrameCallback) async throws {
        guard let client = client else {
            throw MePassaCoreError.notInitialized
        }

        try await client.registerVideoFrameCallback(callback: callback)
        print("✅ Video frame callback registered")
    }

    /// Register callback for VoIP control events (mute/speaker/camera)
    func registerVoipEventCallback(_ callback: FfiVoipEventCallback) async throws {
        guard let client = client else {
            throw MePassaCoreError.notInitialized
        }

        try await client.registerVoipEventCallback(callback: callback)
        print("✅ VoIP event callback registered")
    }

    /// Register callback for call lifecycle events (incoming/state/ended)
    func registerCallEventCallback(_ callback: FfiCallEventCallback) async throws {
        guard let client = client else {
            throw MePassaCoreError.notInitialized
        }

        try await client.registerCallEventCallback(callback: callback)
        print("✅ Call event callback registered")
    }
}

// MARK: - Wrapper Types

/// Swift wrapper for FfiMessage (from UniFFI)
struct FfiMessageWrapper: Identifiable {
    let id: String
    let conversationId: String
    let senderPeerId: String
    let recipientPeerId: String?
    let content: String?
    let messageType: String
    let createdAt: Date
    let status: MessageStatus

    init(id: String, conversationId: String, senderPeerId: String, recipientPeerId: String?, content: String?, messageType: String, createdAt: Date, status: MessageStatus) {
        self.id = id
        self.conversationId = conversationId
        self.senderPeerId = senderPeerId
        self.recipientPeerId = recipientPeerId
        self.content = content
        self.messageType = messageType
        self.createdAt = createdAt
        self.status = status
    }

    init(ffi: FfiMessage) {
        self.id = ffi.messageId
        self.conversationId = ffi.conversationId
        self.senderPeerId = ffi.senderPeerId
        self.recipientPeerId = ffi.recipientPeerId
        self.content = ffi.contentPlaintext
        self.messageType = ffi.messageType
        self.createdAt = Date(timeIntervalSince1970: TimeInterval(ffi.createdAt) / 1000.0)
        self.status = ffi.status
    }
}

/// Swift wrapper for FfiConversation (from UniFFI)
struct FfiConversationWrapper: Identifiable {
    let id: String
    let peerId: String?
    let displayName: String?
    let lastMessageId: String?
    let lastMessageAt: Date?
    let unreadCount: Int

    init(id: String, peerId: String?, displayName: String?, lastMessageId: String?, lastMessageAt: Date?, unreadCount: Int) {
        self.id = id
        self.peerId = peerId
        self.displayName = displayName
        self.lastMessageId = lastMessageId
        self.lastMessageAt = lastMessageAt
        self.unreadCount = unreadCount
    }

    init(ffi: FfiConversation) {
        self.id = ffi.id
        self.peerId = ffi.peerId
        self.displayName = ffi.displayName
        self.lastMessageId = ffi.lastMessageId
        self.lastMessageAt = ffi.lastMessageAt.map { Date(timeIntervalSince1970: TimeInterval($0) / 1000.0) }
        self.unreadCount = Int(ffi.unreadCount)
    }
}

/// Swift wrapper for FfiGroup (from UniFFI)
struct FfiGroupWrapper: Identifiable {
    let id: String
    let name: String
    let description: String?
    let avatarHash: String?
    let creatorPeerId: String
    let memberCount: Int
    let isAdmin: Bool
    let createdAt: Date

    init(id: String, name: String, description: String?, avatarHash: String?, creatorPeerId: String, memberCount: Int, isAdmin: Bool, createdAt: Date) {
        self.id = id
        self.name = name
        self.description = description
        self.avatarHash = avatarHash
        self.creatorPeerId = creatorPeerId
        self.memberCount = memberCount
        self.isAdmin = isAdmin
        self.createdAt = createdAt
    }

    init(ffi: FfiGroup) {
        self.id = ffi.id
        self.name = ffi.name
        self.description = ffi.description
        self.avatarHash = ffi.avatarHash
        self.creatorPeerId = ffi.creatorPeerId
        self.memberCount = Int(ffi.memberCount)
        self.isAdmin = ffi.isAdmin
        self.createdAt = Date(timeIntervalSince1970: TimeInterval(ffi.createdAt))
    }
}

// MARK: - Errors

enum MePassaCoreError: LocalizedError {
    case notInitialized
    case notImplemented(String)
    case networkError(String)
    case storageError(String)
    case cryptoError(String)

    var errorDescription: String? {
        switch self {
        case .notInitialized:
            return "MePassa Core not initialized"
        case .notImplemented(let feature):
            return "Feature not yet implemented: \(feature)"
        case .networkError(let message):
            return "Network error: \(message)"
        case .storageError(let message):
            return "Storage error: \(message)"
        case .cryptoError(let message):
            return "Crypto error: \(message)"
        }
    }
}

// MARK: - Helper Extensions

extension MessageStatus {
    var displayText: String {
        switch self {
        case .pending: return "Pending"
        case .sent: return "Sent"
        case .delivered: return "Delivered"
        case .read: return "Read"
        case .failed: return "Failed"
        }
    }
}
