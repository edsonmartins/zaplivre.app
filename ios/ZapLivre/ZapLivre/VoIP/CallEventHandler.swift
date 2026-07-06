//
//  CallEventHandler.swift
//  ZapLivre
//
//  Created by ZapLivre Team
//

import Foundation

/// Bridges call lifecycle events from core to the app CallManager.
final class CallEventHandler: NSObject, FfiCallEventCallback {
    private weak var callManager: CallManager?

    init(callManager: CallManager) {
        self.callManager = callManager
    }

    func onIncomingCall(callId: String, fromPeerId: String) {
        DispatchQueue.main.async { [weak self] in
            self?.callManager?.handleIncomingCall(coreCallId: callId, fromPeerId: fromPeerId)
        }
    }

    func onCallStateChanged(callId: String, state: FfiCallState) {
        DispatchQueue.main.async { [weak self] in
            self?.callManager?.handleCallStateChanged(coreCallId: callId, state: state)
        }
    }

    func onCallEnded(callId: String, reason: FfiCallEndReason) {
        DispatchQueue.main.async { [weak self] in
            self?.callManager?.handleCallEnded(coreCallId: callId, reason: reason)
        }
    }
}
