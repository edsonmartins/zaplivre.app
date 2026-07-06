//
//  CallManager.swift
//  ZapLivre
//
//  Created by ZapLivre Team
//  Copyright © 2026 ZapLivre. All rights reserved.
//

import Foundation
import CallKit
import AVFoundation
import Combine

class CallManager: NSObject, ObservableObject {
    // MARK: - Published Properties
    @Published var currentCall: Call?
    @Published var callState: CallState = .idle
    @Published var isMuted = false
    @Published var isSpeakerOn = false

    // MARK: - CallKit
    private let callController = CXCallController()
    private var provider: CXProvider?

    // MARK: - Audio
    private let audioManager = AudioManager()
    private var audioSession: AVAudioSession {
        AVAudioSession.sharedInstance()
    }

    // MARK: - Configuration
    override init() {
        super.init()
        configureCallKit()
    }

    func configure() {
        // Additional configuration if needed
        print("📞 CallManager configured")
    }

    // MARK: - CallKit Configuration
    private func configureCallKit() {
        let configuration = CXProviderConfiguration(localizedName: "ZapLivre")
        configuration.supportsVideo = true
        configuration.maximumCallGroups = 1
        configuration.maximumCallsPerCallGroup = 1
        configuration.supportedHandleTypes = [.generic]

        // Audio
        configuration.ringtoneSound = "ringtone.caf"

        provider = CXProvider(configuration: configuration)
        provider?.setDelegate(self, queue: nil)

        print("✅ CallKit provider configured")
    }

    // MARK: - Outgoing Call
    func startCall(to peerId: String, displayName: String) {
        let handle = CXHandle(type: .generic, value: peerId)
        let startCallAction = CXStartCallAction(call: UUID(), handle: handle)

        let transaction = CXTransaction(action: startCallAction)

        callController.request(transaction) { [weak self] error in
            if let error = error {
                print("❌ Error requesting start call: \(error)")
                return
            }

            print("✅ Start call requested")
            self?.createCall(id: startCallAction.callUUID, peerId: peerId, displayName: displayName, isOutgoing: true)
        }
    }

    // MARK: - Incoming Call
    func reportIncomingCall(callId: UUID, peerId: String, displayName: String, coreCallId: String? = nil, completion: @escaping (Error?) -> Void) {
        let update = CXCallUpdate()
        update.remoteHandle = CXHandle(type: .generic, value: peerId)
        update.localizedCallerName = displayName
        update.supportsHolding = false
        update.supportsGrouping = false
        update.supportsUngrouping = false
        update.supportsDTMF = false
        update.hasVideo = true

        provider?.reportNewIncomingCall(with: callId, update: update) { [weak self] error in
            if let error = error {
                print("❌ Error reporting incoming call: \(error)")
                completion(error)
                return
            }

            print("✅ Incoming call reported")
            self?.createCall(id: callId, peerId: peerId, displayName: displayName, isOutgoing: false, coreCallId: coreCallId)
            completion(nil)
        }
    }

    // MARK: - Call Management
    private func createCall(id: UUID, peerId: String, displayName: String, isOutgoing: Bool, coreCallId: String? = nil) {
        let call = Call(
            id: id,
            peerId: peerId,
            displayName: displayName,
            isOutgoing: isOutgoing,
            coreCallId: coreCallId
        )

        DispatchQueue.main.async {
            self.currentCall = call
            self.callState = isOutgoing ? .connecting : .ringing
        }

        if isOutgoing {
            initiateWebRTCConnection(peerId: peerId)
        }
    }

    func answerCall() {
        guard let call = currentCall else { return }

        let answerAction = CXAnswerCallAction(call: call.id)
        let transaction = CXTransaction(action: answerAction)

        callController.request(transaction) { error in
            if let error = error {
                print("❌ Error answering call: \(error)")
                return
            }

            print("✅ Call answered")
        }
    }

    func endCall() {
        guard let call = currentCall else { return }

        let endCallAction = CXEndCallAction(call: call.id)
        let transaction = CXTransaction(action: endCallAction)

        callController.request(transaction) { [weak self] error in
            if let error = error {
                print("❌ Error ending call: \(error)")
                return
            }

            print("✅ Call ended")
            if let coreCallId = call.coreCallId {
                Task {
                    try? await ZapLivreCore.shared.hangupCall(callId: coreCallId)
                }
            }
            self?.cleanupCall()
        }
    }

    private func cleanupCall() {
        stopAudio()

        DispatchQueue.main.async {
            self.currentCall = nil
            self.callState = .idle
            self.isMuted = false
            self.isSpeakerOn = false
        }
    }

    // MARK: - Audio Controls
    func toggleMute() {
        isMuted.toggle()

        if isMuted {
            audioManager.mute()
        } else {
            audioManager.unmute()
        }

        print("🔇 Mute: \(isMuted)")
        if let coreCallId = currentCall?.coreCallId {
            Task {
                try? await ZapLivreCore.shared.toggleMute(callId: coreCallId)
            }
        }
    }

    func toggleSpeaker() {
        isSpeakerOn.toggle()

        do {
            try audioManager.enableSpeaker(isSpeakerOn)
            print("🔊 Speaker: \(isSpeakerOn)")
        } catch {
            print("❌ Error toggling speaker: \(error)")
        }
        if let coreCallId = currentCall?.coreCallId {
            Task {
                try? await ZapLivreCore.shared.toggleSpeaker(callId: coreCallId)
            }
        }
    }

    // MARK: - VoIP Event Handlers
    func handleMuteChanged(_ muted: Bool) {
        isMuted = muted
        if muted {
            audioManager.mute()
        } else {
            audioManager.unmute()
        }
        print("🔇 Mute updated from core: \(muted)")
    }

    func handleSpeakerChanged(_ enabled: Bool) {
        isSpeakerOn = enabled
        do {
            try audioManager.enableSpeaker(enabled)
        } catch {
            print("❌ Error applying speaker state: \(error)")
        }
        print("🔊 Speaker updated from core: \(enabled)")
    }

    func handleCameraSwitchRequested() {
        // Camera switching is handled by the UI layer (VideoCallScreen)
        print("📸 Camera switch requested from core")
    }

    // MARK: - Call Lifecycle Handlers
    func handleIncomingCall(coreCallId: String, fromPeerId: String) {
        if currentCall != nil {
            print("⚠️ Incoming call ignored - already in call")
            return
        }
        let callUuid = UUID()
        reportIncomingCall(callId: callUuid, peerId: fromPeerId, displayName: fromPeerId, coreCallId: coreCallId) { error in
            if let error = error {
                print("❌ Failed to report incoming call: \(error)")
            }
        }
    }

    func handleCallStateChanged(coreCallId: String, state: FfiCallState) {
        guard currentCall?.coreCallId == coreCallId else {
            return
        }

        let mappedState: CallState
        switch state {
        case .initiating, .connecting:
            mappedState = .connecting
        case .ringing:
            mappedState = .ringing
        case .active:
            mappedState = .connected
        case .ending, .ended:
            mappedState = .ended
        }

        // Iniciar o áudio quando a chamada de fato conecta (transição para ACTIVE)
        if mappedState == .connected && callState != .connected {
            startAudio()
        }

        callState = mappedState

        if mappedState == .ended {
            cleanupCall()
        }
    }

    func handleCallEnded(coreCallId: String, reason: FfiCallEndReason) {
        guard currentCall?.coreCallId == coreCallId else {
            return
        }

        print("📴 Call ended (\(reason))")
        callState = .ended
        cleanupCall()
    }

    func handleAudioFrame(coreCallId: String, data: Data, sampleRate: UInt32, channels: UInt32) {
        guard currentCall?.coreCallId == coreCallId else {
            return
        }
        if sampleRate != 48_000 || channels != 1 {
            print("⚠️ Unsupported audio format: \(sampleRate)Hz, channels=\(channels)")
            return
        }

        audioManager.playAudio(data)
    }

    // MARK: - WebRTC Integration
    private func initiateWebRTCConnection(peerId: String) {
        print("📞 Initiating WebRTC connection to \(peerId)...")
        Task {
            do {
                let coreCallId = try await ZapLivreCore.shared.startCall(to: peerId)
                DispatchQueue.main.async {
                    if var call = self.currentCall {
                        call.coreCallId = coreCallId
                        self.currentCall = call
                    }
                    // Não marcar .connected aqui: o peer ainda nem atendeu.
                    // O estado real (ACTIVE) chega via handleCallStateChanged,
                    // que também inicia o áudio.
                    self.callState = .connecting
                }
            } catch {
                DispatchQueue.main.async {
                    self.callState = .ended
                }
                print("❌ Failed to start core call: \(error)")
            }
        }
    }

    private func startAudio() {
        do {
            try audioSession.setCategory(.playAndRecord, mode: .voiceChat, options: [.allowBluetooth, .defaultToSpeaker])
            try audioSession.setActive(true)

            // TODO: Connect AVAudioEngine to WebRTC audio tracks
            // This will use AVAudioEngine for audio I/O similar to Android's CallAudioManager

            print("🎤 Audio session started")
        } catch {
            print("❌ Error starting audio session: \(error)")
        }
    }

    private func stopAudio() {
        do {
            try audioSession.setActive(false)
            print("🎤 Audio session stopped")
        } catch {
            print("❌ Error stopping audio session: \(error)")
        }
    }
}

// MARK: - CXProviderDelegate
extension CallManager: CXProviderDelegate {
    func providerDidReset(_ provider: CXProvider) {
        print("📞 Provider reset")
        cleanupCall()
    }

    func provider(_ provider: CXProvider, perform action: CXStartCallAction) {
        configureAudioSession()
        action.fulfill()
    }

    func provider(_ provider: CXProvider, perform action: CXAnswerCallAction) {
        guard let call = currentCall else {
            action.fail()
            return
        }

        configureAudioSession()

        if let coreCallId = call.coreCallId {
            Task {
                do {
                    try await ZapLivreCore.shared.acceptCall(callId: coreCallId)
                    print("✅ Accepted call via core")
                } catch {
                    print("❌ Failed to accept call via core: \(error)")
                }
            }
        }

        // O estado .connected chega do core (ACTIVE) via handleCallStateChanged;
        // aqui apenas ativamos o áudio junto do answer do CallKit.
        DispatchQueue.main.async {
            self.callState = .connecting
        }

        startAudio()
        action.fulfill()
    }

    func provider(_ provider: CXProvider, perform action: CXEndCallAction) {
        if let coreCallId = currentCall?.coreCallId {
            Task {
                try? await ZapLivreCore.shared.hangupCall(callId: coreCallId)
            }
        }
        cleanupCall()
        action.fulfill()
    }

    func provider(_ provider: CXProvider, perform action: CXSetMutedCallAction) {
        isMuted = action.isMuted
        if let coreCallId = currentCall?.coreCallId {
            Task {
                try? await ZapLivreCore.shared.toggleMute(callId: coreCallId)
            }
        }
        action.fulfill()
    }

    func provider(_ provider: CXProvider, didActivate audioSession: AVAudioSession) {
        print("🎤 Audio session activated")

        do {
            try audioManager.start()

            // Setup audio callback to send to WebRTC
            audioManager.onAudioCaptured = { [weak self] audioData in
                guard let self = self else { return }
                guard !self.isMuted else { return }
                guard let coreCallId = self.currentCall?.coreCallId else { return }

                let audioBytes = [UInt8](audioData)
                Task {
                    try? await ZapLivreCore.shared.sendAudioFrame(
                        callId: coreCallId,
                        audioData: audioBytes,
                        sampleRate: 48_000,
                        channels: 1
                    )
                }
            }

            print("✅ Audio I/O started")
        } catch {
            print("❌ Error starting audio: \(error)")
        }
    }

    func provider(_ provider: CXProvider, didDeactivate audioSession: AVAudioSession) {
        print("🎤 Audio session deactivated")
        audioManager.stop()
    }

    private func configureAudioSession() {
        do {
            try audioSession.setCategory(.playAndRecord, mode: .voiceChat)
            try audioSession.setActive(true)
        } catch {
            print("❌ Error configuring audio session: \(error)")
        }
    }
}

// MARK: - Models
struct Call: Identifiable {
    let id: UUID
    let peerId: String
    let displayName: String
    let isOutgoing: Bool
    var startTime: Date = Date()
    var coreCallId: String?
}

enum CallState {
    case idle
    case ringing
    case connecting
    case connected
    case ended
}
