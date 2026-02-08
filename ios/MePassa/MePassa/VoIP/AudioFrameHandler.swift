//
//  AudioFrameHandler.swift
//  MePassa
//
//  Created by MePassa Team
//

import Foundation

/// Bridges remote audio frames from core to the app CallManager.
final class AudioFrameHandler: NSObject, FfiAudioFrameCallback {
    private weak var callManager: CallManager?

    init(callManager: CallManager) {
        self.callManager = callManager
    }

    func onAudioFrame(callId: String, data: [UInt8], sampleRate: UInt32, channels: UInt32) {
        let pcmData = Data(data)
        DispatchQueue.main.async { [weak self] in
            self?.callManager?.handleAudioFrame(
                coreCallId: callId,
                data: pcmData,
                sampleRate: sampleRate,
                channels: channels
            )
        }
    }
}
