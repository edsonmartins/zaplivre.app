//
//  MessageEventHandler.swift
//  MePassa
//
//  EVT-02: eventos de mensagem do core -> UI (substitui o polling de 2-5s).
//  Recebe callbacks do FFI e propaga via NotificationCenter + AppState.
//

import Foundation

extension Notification.Name {
    /// Core inicializado fora do launch (fluxo da LoginView) - dispara o
    /// setup complementar no MePassaApp
    static let mePassaCoreStarted = Notification.Name("mePassaCoreStarted")
    static let mePassaMessageReceived = Notification.Name("mePassaMessageReceived")
    static let mePassaMessageStatusChanged = Notification.Name("mePassaMessageStatusChanged")
    static let mePassaTyping = Notification.Name("mePassaTyping")
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
                name: .mePassaMessageReceived,
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
                name: .mePassaMessageStatusChanged,
                object: nil,
                userInfo: userInfo
            )
        }
    }

    func onTyping(peerId: String, isTyping: Bool) {
        DispatchQueue.main.async {
            NotificationCenter.default.post(
                name: .mePassaTyping,
                object: nil,
                userInfo: ["peer_id": peerId, "is_typing": isTyping]
            )
        }
    }
}
