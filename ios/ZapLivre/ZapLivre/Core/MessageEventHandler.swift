//
//  MessageEventHandler.swift
//  ZapLivre
//
//  EVT-02: eventos de mensagem do core -> UI (substitui o polling de 2-5s).
//  Recebe callbacks do FFI e propaga via NotificationCenter + AppState.
//

import Foundation

extension Notification.Name {
    /// Core inicializado fora do launch (fluxo da LoginView) - dispara o
    /// setup complementar no ZapLivreApp
    static let zapLivreCoreStarted = Notification.Name("zapLivreCoreStarted")
    static let zapLivreMessageReceived = Notification.Name("zapLivreMessageReceived")
    static let zapLivreMessageStatusChanged = Notification.Name("zapLivreMessageStatusChanged")
    static let zapLivreTyping = Notification.Name("zapLivreTyping")
}

final class MessageEventHandler: NSObject, FfiMessageEventCallback {
    private weak var appState: AppState?

    init(appState: AppState) {
        self.appState = appState
    }

    func onMessageReceived(messageId: String, fromPeerId: String) {
        DispatchQueue.main.async { [weak self] in
            self?.appState?.loadConversations()
            NotificationCenter.default.post(
                name: .zapLivreMessageReceived,
                object: nil,
                userInfo: ["message_id": messageId, "from_peer_id": fromPeerId]
            )
        }
    }

    func onMessageStatusChanged(messageId: String, status: MessageStatus, peerId: String?) {
        DispatchQueue.main.async {
            var userInfo: [String: Any] = ["message_id": messageId]
            if let peerId = peerId {
                userInfo["peer_id"] = peerId
            }
            NotificationCenter.default.post(
                name: .zapLivreMessageStatusChanged,
                object: nil,
                userInfo: userInfo
            )
        }
    }

    func onTyping(peerId: String, isTyping: Bool) {
        DispatchQueue.main.async {
            NotificationCenter.default.post(
                name: .zapLivreTyping,
                object: nil,
                userInfo: ["peer_id": peerId, "is_typing": isTyping]
            )
        }
    }
}
