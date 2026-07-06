//
//  VoipEventHandler.swift
//  ZapLivre
//
//  Created by ZapLivre Team
//

import Foundation

/// Bridges VoIP control events from core to the app CallManager.
final class VoipEventHandler: NSObject, FfiVoipEventCallback {
    private weak var callManager: CallManager?

    init(callManager: CallManager) {
        self.callManager = callManager
    }

    func onMuteChanged(callId: String, isMuted: Bool) {
        DispatchQueue.main.async { [weak self] in
            self?.callManager?.handleMuteChanged(isMuted)
        }
    }

    func onSpeakerphoneChanged(callId: String, enabled: Bool) {
        DispatchQueue.main.async { [weak self] in
            self?.callManager?.handleSpeakerChanged(enabled)
        }
    }

    func onCameraSwitchRequested(callId: String) {
        DispatchQueue.main.async { [weak self] in
            self?.callManager?.handleCameraSwitchRequested()
        }
    }
}
