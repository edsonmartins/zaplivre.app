//
//  ZapLivreApp.swift
//  ZapLivre
//
//  Created by ZapLivre Team
//  Copyright © 2026 ZapLivre. All rights reserved.
//

import SwiftUI
import CallKit

@main
struct ZapLivreApp: App {
    @UIApplicationDelegateAdaptor(AppDelegate.self) var appDelegate
    @StateObject private var appState = AppState()
    @StateObject private var callManager = CallManager()
    @StateObject private var pushManager = PushNotificationManager()
    @State private var didStartExistingIdentity = false

    init() {
        // Setup CallKit
        setupCallKit()
    }

    /// The Keychain survives uninstalling the app; the data directory does not.
    /// A reinstalled app therefore found an identity, skipped onboarding and
    /// opened "logged in" over an empty database, and the private key stayed on
    /// the device after the user had removed the app. An identity with no data
    /// directory next to it belongs to a previous install and is discarded; the
    /// account can still be brought back from an exported backup.
    ///
    /// Called from `didFinishLaunching`: inside `App.init` the protected-data
    /// flag is not reliable yet, and the UI has not looked for an identity.
    static func discardIdentityOrphanedByReinstall() {
        // Never decide this while files are locked (background launch before
        // the first unlock): a directory we cannot see is not a missing one.
        guard UIApplication.shared.isProtectedDataAvailable,
              let docs = FileManager.default.urls(for: .documentDirectory, in: .userDomainMask).first
        else { return }

        let dataDir = docs.appendingPathComponent("zaplivre_data")
        guard !FileManager.default.fileExists(atPath: dataDir.path),
              (try? KeychainStore.loadIdentity()) ?? nil != nil
        else { return }

        print("🧹 Discarding identity left in the Keychain by a previous install")
        try? KeychainStore.deleteIdentity()
    }

    static func hasExistingIdentity() -> Bool {
        if (try? KeychainStore.loadIdentity()) ?? nil != nil {
            return true
        }
        if let docs = FileManager.default.urls(for: .documentDirectory, in: .userDomainMask).first {
            let legacyKey = docs.appendingPathComponent("zaplivre_data/identity.key")
            if FileManager.default.fileExists(atPath: legacyKey.path) {
                return true
            }
        }
        return false
    }

    var body: some Scene {
        WindowGroup {
            ContentView()
                .environmentObject(appState)
                .environmentObject(callManager)
                .environmentObject(pushManager)
                .onAppear {
                    // Connect AppDelegate with PushManager
                    appDelegate.pushManager = pushManager
                    pushManager.appState = appState

                    // Request push notification permissions
                    pushManager.requestAuthorization()

                    // StateObject só possui identidade estável depois que a
                    // view entra na hierarquia. Inicializar no App.init()
                    // atualizava uma instância transitória de AppState: o core
                    // fazia login, mas ContentView permanecia no onboarding.
                    if !didStartExistingIdentity && Self.hasExistingIdentity() {
                        didStartExistingIdentity = true
                        initializeZapLivreCore()
                    } else if !didStartExistingIdentity {
                        print("ℹ️ No identity yet - LoginView will handle create/restore")
                    }
                }
                // IDN-02: quando a LoginView cria/restaura a identidade
                // (primeira execução), completar o setup (bootstrap, handlers,
                // push) que normalmente roda no launch
                .onReceive(NotificationCenter.default.publisher(for: .zapLivreCoreStarted)) { _ in
                    completeCoreSetup()
                }
        }
    }

    private func initializeZapLivreCore() {
        print("📱 Initializing ZapLivre Core...")

        Task {
            do {
                try await ZapLivreCore.shared.initialize()
                print("✅ ZapLivre Core initialized successfully")

                // Start listening for incoming P2P connections
                try await ZapLivreCore.shared.startListening()
                print("✅ ZapLivre Core listening for connections")

                if let peerId = ZapLivreCore.shared.localPeerId {
                    await MainActor.run {
                        appState.login(peerId: peerId)
                    }
                }
            } catch {
                print("❌ Failed to initialize ZapLivre Core: \(error)")
                return
            }

            completeCoreSetup()
        }
    }

    /// Pós-init compartilhado entre o launch normal e o fluxo da LoginView:
    /// bootstrap DHT, registro de callbacks e push
    private func completeCoreSetup() {
        Task {
            do {
                try await ZapLivreCore.shared.bootstrap()
                print("✅ ZapLivre Core bootstrapped")

                await MainActor.run {
                    pushManager.refreshRegistration()
                }

                let handler = VoipEventHandler(callManager: callManager)
                appState.voipEventHandler = handler
                try await ZapLivreCore.shared.registerVoipEventCallback(handler)

                let callHandler = CallEventHandler(callManager: callManager)
                appState.callEventHandler = callHandler
                try await ZapLivreCore.shared.registerCallEventCallback(callHandler)
                try ZapLivreCore.shared.registerWebRtcSignalingCallback(WebRtcSignalingBridge.shared)

                let audioHandler = AudioFrameHandler(callManager: callManager)
                appState.audioFrameHandler = audioHandler
                try await ZapLivreCore.shared.registerAudioFrameCallback(audioHandler)

                // EVT-02: eventos de mensagem substituem o polling das views
                let messageHandler = MessageEventHandler(appState: appState)
                appState.messageEventHandler = messageHandler
                try await ZapLivreCore.shared.registerMessageEventCallback(messageHandler)
            } catch {
                print("❌ Failed to complete core setup: \(error)")
            }
        }
    }
    
    private func setupCallKit() {
        // Configure CallKit provider
        callManager.configure()
    }
}

/// App-wide state management
class AppState: ObservableObject {
    @Published var isAuthenticated = false
    @Published var currentUser: User?
    @Published var conversations: [Conversation] = []
    @Published var groups: [ChatGroup] = []

    private var refreshTimer: Timer?
    private let core: ZapLivreCoreProtocol
    var voipEventHandler: VoipEventHandler?
    var callEventHandler: CallEventHandler?
    var audioFrameHandler: AudioFrameHandler?
    var messageEventHandler: MessageEventHandler?

    init(core: ZapLivreCoreProtocol = ZapLivreCore.shared) {
        self.core = core
    }

    func login(peerId: String) {
        self.isAuthenticated = true
        self.currentUser = User(id: peerId, username: nil, peerId: peerId)
        UserDefaults.standard.set(peerId, forKey: "local_peer_id")
        print("✅ Logged in as: \(peerId)")

        // Start auto-refresh when logged in
        startAutoRefresh()
    }

    func logout() {
        self.isAuthenticated = false
        self.currentUser = nil
        self.conversations = []
        self.groups = []
        self.pendingConversationPeerId = nil

        // Stop auto-refresh when logged out
        stopAutoRefresh()
    }

    @Published var pendingConversationPeerId: String?

    func openConversation(peerId: String) {
        if !isAuthenticated {
            pendingConversationPeerId = peerId
            return
        }

        pendingConversationPeerId = peerId
        loadConversations()
    }

    /// Load conversations from ZapLivreCore
    func loadConversations() {
        Task {
            do {
                let convs = try await core.listConversations()
                await core.scanGroupSenderKeyMessages()

                // Convert FFI conversations to local model
                await MainActor.run {
                    self.conversations = convs.compactMap { ffiConv in
                        // Only include conversations that have a peer_id
                        guard let peerId = ffiConv.peerId else { return nil }

                        let displayName: String
                        if let name = ffiConv.displayName {
                            displayName = name
                        } else {
                            displayName = String(peerId.prefix(12)) + "..."
                        }

                        return Conversation(
                            id: ffiConv.id,
                            peerId: peerId,
                            displayName: displayName,
                            lastMessage: nil, // FFI doesn't include message text, only ID
                            unreadCount: Int(ffiConv.unreadCount)
                        )
                    }

                    print("✅ Loaded \(self.conversations.count) conversations")
                }
            } catch {
                print("❌ Failed to load conversations: \(error)")
            }
        }
    }

    /// EVT-02: atualizações chegam por eventos do core (MessageEventHandler);
    /// o timer é apenas um safety net lento
    private func startAutoRefresh() {
        // Load immediately
        loadConversations()

        refreshTimer?.invalidate()
        refreshTimer = Timer.scheduledTimer(withTimeInterval: 30.0, repeats: true) { [weak self] _ in
            self?.loadConversations()
        }
    }

    /// Stop auto-refresh timer
    private func stopAutoRefresh() {
        refreshTimer?.invalidate()
        refreshTimer = nil
    }
}

/// Temporary models (will be replaced by UniFFI generated types)
struct User: Identifiable {
    let id: String
    let username: String?
    let peerId: String
}

struct Conversation: Identifiable {
    let id: String
    let peerId: String
    let displayName: String
    let lastMessage: String?
    let unreadCount: Int
}

struct ChatGroup: Identifiable {
    let id: String
    let name: String
    let description: String?
    let memberCount: Int
    let isAdmin: Bool
    let createdAt: Date
}
