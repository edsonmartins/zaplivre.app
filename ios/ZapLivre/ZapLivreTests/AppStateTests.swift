//
//  AppStateTests.swift
//  ZapLivreTests
//
//  Copyright © 2026 ZapLivre. All rights reserved.
//
//  Unit tests for AppState transitions using a fake ZapLivreCoreProtocol,
//  so no Rust FFI client is ever created.

import XCTest
@testable import ZapLivre

// MARK: - Fake core

final class FakeCore: ZapLivreCoreProtocol {
    var isInitialized = false
    var localPeerId: String?

    /// Peer ID the fake "discovers" when initialize() succeeds
    var peerIdAfterInitialize: String?
    /// If set, initialize() throws this error
    var initializeError: Error?
    /// Conversations returned by listConversations()
    var conversationsToReturn: [FfiConversationWrapper] = []
    /// If set, listConversations() throws this error
    var listConversationsError: Error?

    private(set) var initializeCallCount = 0
    private(set) var startListeningCallCount = 0
    private(set) var bootstrapCallCount = 0
    private(set) var listConversationsCallCount = 0
    private(set) var scanGroupKeysCallCount = 0

    func initialize() async throws {
        initializeCallCount += 1
        if let error = initializeError {
            throw error
        }
        isInitialized = true
        localPeerId = peerIdAfterInitialize
    }

    func startListening() async throws {
        startListeningCallCount += 1
    }

    func bootstrap() async throws {
        bootstrapCallCount += 1
    }

    func listConversations() async throws -> [FfiConversationWrapper] {
        listConversationsCallCount += 1
        if let error = listConversationsError {
            throw error
        }
        return conversationsToReturn
    }

    func scanGroupSenderKeyMessages() async {
        scanGroupKeysCallCount += 1
    }
}

// MARK: - Tests

final class AppStateTests: XCTestCase {
    private var core: FakeCore!
    private var appState: AppState!

    override func setUp() {
        super.setUp()
        core = FakeCore()
        appState = AppState(core: core)
    }

    override func tearDown() {
        appState.logout() // stops the auto-refresh timer
        appState = nil
        core = nil
        super.tearDown()
    }

    /// Spins the main run loop until `condition` is true or `timeout` elapses.
    /// AppState publishes its changes via the main queue, so the run loop
    /// must be pumped for them to land.
    @discardableResult
    private func waitUntil(timeout: TimeInterval = 3.0, _ condition: () -> Bool) -> Bool {
        let deadline = Date().addingTimeInterval(timeout)
        while Date() < deadline {
            if condition() { return true }
            RunLoop.current.run(until: Date().addingTimeInterval(0.02))
        }
        return condition()
    }

    // MARK: Initial state (onboarding)

    func testInitialStateIsUnauthenticated() {
        XCTAssertFalse(appState.isAuthenticated)
        XCTAssertNil(appState.currentUser)
        XCTAssertTrue(appState.conversations.isEmpty)
        XCTAssertTrue(appState.groups.isEmpty)
        XCTAssertNil(appState.pendingConversationPeerId)
    }

    // MARK: Onboarding -> ready (core reports identity)

    func testCoreReportedIdentityTransitionsToAuthenticated() {
        core.peerIdAfterInitialize = "12D3KooWFakePeerIdentityABCDEF"

        // Mirror ZapLivreApp.initializeZapLivreCore: initialize the core, then
        // log in with the peer ID it reports.
        let initDone = expectation(description: "core initialized")
        Task { [core = core!] in
            try? await core.initialize()
            try? await core.startListening()
            await MainActor.run {
                if let peerId = core.localPeerId {
                    self.appState.login(peerId: peerId)
                }
            }
            initDone.fulfill()
        }
        wait(for: [initDone], timeout: 3.0)

        XCTAssertTrue(core.isInitialized)
        XCTAssertTrue(appState.isAuthenticated)
        XCTAssertEqual(appState.currentUser?.peerId, "12D3KooWFakePeerIdentityABCDEF")
        XCTAssertEqual(appState.currentUser?.id, "12D3KooWFakePeerIdentityABCDEF")
        // login() starts auto-refresh, which loads conversations immediately
        XCTAssertTrue(waitUntil { self.core.listConversationsCallCount >= 1 })
    }

    // MARK: Init error keeps onboarding state

    func testCoreInitializationErrorStaysUnauthenticated() {
        core.initializeError = ZapLivreCoreError.storageError("disk full")

        let initDone = expectation(description: "core init attempted")
        Task { [core = core!] in
            // Mirror ZapLivreApp.initializeZapLivreCore's error path: on throw,
            // no login happens.
            do {
                try await core.initialize()
                XCTFail("initialize() should have thrown")
            } catch {
                // expected
            }
            initDone.fulfill()
        }
        wait(for: [initDone], timeout: 3.0)

        XCTAssertFalse(core.isInitialized)
        XCTAssertNil(core.localPeerId)
        XCTAssertFalse(appState.isAuthenticated)
        XCTAssertNil(appState.currentUser)
        XCTAssertEqual(core.listConversationsCallCount, 0)
    }

    // MARK: Logout

    func testLogoutResetsAllState() {
        appState.login(peerId: "peer-abc")
        appState.openConversation(peerId: "peer-xyz")
        XCTAssertTrue(appState.isAuthenticated)

        appState.logout()

        XCTAssertFalse(appState.isAuthenticated)
        XCTAssertNil(appState.currentUser)
        XCTAssertTrue(appState.conversations.isEmpty)
        XCTAssertTrue(appState.groups.isEmpty)
        XCTAssertNil(appState.pendingConversationPeerId)
    }

    // MARK: Pending conversation while unauthenticated

    func testOpenConversationBeforeLoginQueuesPeerWithoutLoading() {
        appState.openConversation(peerId: "peer-pending")

        XCTAssertEqual(appState.pendingConversationPeerId, "peer-pending")
        XCTAssertFalse(appState.isAuthenticated)
        // Must not hit the core while unauthenticated
        RunLoop.current.run(until: Date().addingTimeInterval(0.1))
        XCTAssertEqual(core.listConversationsCallCount, 0)
    }

    func testOpenConversationAfterLoginLoadsConversations() {
        appState.login(peerId: "peer-abc")
        let callsAfterLogin = waitUntil { self.core.listConversationsCallCount >= 1 }
        XCTAssertTrue(callsAfterLogin)

        appState.openConversation(peerId: "peer-target")

        XCTAssertEqual(appState.pendingConversationPeerId, "peer-target")
        XCTAssertTrue(waitUntil { self.core.listConversationsCallCount >= 2 })
    }

    // MARK: loadConversations mapping

    func testLoadConversationsMapsCoreConversationsAndFiltersMissingPeerIds() {
        core.conversationsToReturn = [
            FfiConversationWrapper(
                id: "conv-1",
                peerId: "12D3KooWLongPeerIdValue9999",
                displayName: "Alice",
                lastMessageId: "msg-1",
                lastMessageAt: Date(),
                unreadCount: 3
            ),
            FfiConversationWrapper(
                id: "conv-2",
                peerId: "12D3KooWAnotherPeer00001111",
                displayName: nil, // no name -> fallback to truncated peer ID
                lastMessageId: nil,
                lastMessageAt: nil,
                unreadCount: 0
            ),
            FfiConversationWrapper(
                id: "conv-3",
                peerId: nil, // no peer ID -> must be filtered out
                displayName: "Ghost",
                lastMessageId: nil,
                lastMessageAt: nil,
                unreadCount: 7
            )
        ]

        appState.loadConversations()

        XCTAssertTrue(waitUntil { self.appState.conversations.count == 2 })
        XCTAssertEqual(core.scanGroupKeysCallCount, 1)

        let first = appState.conversations[0]
        XCTAssertEqual(first.id, "conv-1")
        XCTAssertEqual(first.peerId, "12D3KooWLongPeerIdValue9999")
        XCTAssertEqual(first.displayName, "Alice")
        XCTAssertEqual(first.unreadCount, 3)

        let second = appState.conversations[1]
        XCTAssertEqual(second.id, "conv-2")
        // Fallback display name: first 12 chars of the peer ID + "..."
        XCTAssertEqual(second.displayName, "12D3KooWAnot...")
        XCTAssertEqual(second.unreadCount, 0)
    }

    func testLoadConversationsFailureLeavesStateUnchanged() {
        core.listConversationsError = ZapLivreCoreError.networkError("offline")

        appState.loadConversations()

        XCTAssertTrue(waitUntil { self.core.listConversationsCallCount >= 1 })
        // Give any (unexpected) publish a chance to land, then assert nothing changed
        RunLoop.current.run(until: Date().addingTimeInterval(0.2))
        XCTAssertTrue(appState.conversations.isEmpty)
        XCTAssertEqual(core.scanGroupKeysCallCount, 0)
    }
}
