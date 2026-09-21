import AVFoundation
import Foundation
import StreamWebRTC

@MainActor
final class NativeWebRtcSession: NSObject, ObservableObject {
    @Published private(set) var localVideoTrack: RTCVideoTrack?
    @Published private(set) var remoteVideoTrack: RTCVideoTrack?

    private static let factory: RTCPeerConnectionFactory = {
        RTCInitializeSSL()
        return RTCPeerConnectionFactory(
            encoderFactory: RTCDefaultVideoEncoderFactory(),
            decoderFactory: RTCDefaultVideoDecoderFactory()
        )
    }()

    private let callId: String
    private let remotePeerId: String
    private var peerConnection: RTCPeerConnection!
    private var videoCapturer: RTCCameraVideoCapturer?
    private var localAudioTrack: RTCAudioTrack?
    private var signalObserver: NSObjectProtocol?
    private var setupTask: Task<Void, Never>?
    private var pendingSignals: [Notification] = []
    private var pendingCandidates: [RTCIceCandidate] = []
    private var usingFrontCamera = true

    init(callId: String, remotePeerId: String) {
        self.callId = callId
        self.remotePeerId = remotePeerId
        super.init()
    }

    /// The peer connection is created after the ICE servers are known. Without
    /// them only host candidates exist and a call between two phones on mobile
    /// data (CGNAT) never connects.
    func start() {
        configureAudioSession()

        // Listen before anything else: these notifications are not replayed, so
        // an offer arriving while the ICE servers are being fetched would be
        // lost. `handle` parks signals until the peer connection exists.
        signalObserver = NotificationCenter.default.addObserver(
            forName: .zapLivreWebRtcSignal,
            object: nil,
            queue: .main
        ) { [weak self] notification in
            Task { @MainActor in self?.handle(notification) }
        }

        setupTask = Task { @MainActor [weak self] in
            let iceServers = await ZapLivreCore.shared.iceServers().map { server in
                RTCIceServer(
                    urlStrings: server.urls,
                    username: server.username,
                    credential: server.credential
                )
            }
            guard let self, !Task.isCancelled else { return }
            if iceServers.isEmpty {
                print("⚠️ No ICE servers available: call limited to the local network")
            }
            self.connect(iceServers: iceServers)
        }
    }

    private func connect(iceServers: [RTCIceServer]) {
        let configuration = RTCConfiguration()
        configuration.iceServers = iceServers
        configuration.sdpSemantics = .unifiedPlan
        configuration.continualGatheringPolicy = .gatherContinually
        let constraints = RTCMediaConstraints(
            mandatoryConstraints: nil,
            optionalConstraints: ["DtlsSrtpKeyAgreement": "true"]
        )
        peerConnection = Self.factory.peerConnection(
            with: configuration,
            constraints: constraints,
            delegate: self
        )

        let audioSource = Self.factory.audioSource(with: RTCMediaConstraints(
            mandatoryConstraints: nil,
            optionalConstraints: nil
        ))
        let audioTrack = Self.factory.audioTrack(with: audioSource, trackId: "audio-\(callId)")
        localAudioTrack = audioTrack
        peerConnection.add(audioTrack, streamIds: ["stream-\(callId)"])

        let videoSource = Self.factory.videoSource()
        let capturer = RTCCameraVideoCapturer(delegate: videoSource)
        videoCapturer = capturer
        let videoTrack = Self.factory.videoTrack(with: videoSource, trackId: "video-\(callId)")
        localVideoTrack = videoTrack
        peerConnection.add(videoTrack, streamIds: ["stream-\(callId)"])
        startCapture(front: true)

        let localPeerId = ZapLivreCore.shared.localPeerId ?? ""
        if localPeerId < remotePeerId {
            createOffer()
        }

        // Signals that arrived while the connection was being set up
        let parked = pendingSignals
        pendingSignals.removeAll()
        parked.forEach { handle($0) }
    }

    func stop() {
        setupTask?.cancel()
        setupTask = nil
        pendingSignals.removeAll()
        if let signalObserver { NotificationCenter.default.removeObserver(signalObserver) }
        signalObserver = nil
        videoCapturer?.stopCapture()
        localVideoTrack?.isEnabled = false
        localAudioTrack?.isEnabled = false
        peerConnection?.close()
        peerConnection = nil
        localVideoTrack = nil
        remoteVideoTrack = nil
        deactivateAudioSession()
    }

    func setVideoEnabled(_ enabled: Bool) {
        localVideoTrack?.isEnabled = enabled
    }

    func setAudioEnabled(_ enabled: Bool) {
        localAudioTrack?.isEnabled = enabled
    }

    func switchCamera() {
        usingFrontCamera.toggle()
        startCapture(front: usingFrontCamera)
    }

    private func startCapture(front: Bool) {
        guard let capturer = videoCapturer else { return }
        let devices = RTCCameraVideoCapturer.captureDevices()
        let desiredPosition: AVCaptureDevice.Position = front ? .front : .back
        guard let device = devices.first(where: { $0.position == desiredPosition }) ?? devices.first,
              let format = RTCCameraVideoCapturer.supportedFormats(for: device)
                .sorted(by: { dimensions($0).width < dimensions($1).width })
                .last else { return }
        let fps = min(24, Int(format.videoSupportedFrameRateRanges.first?.maxFrameRate ?? 24))
        capturer.startCapture(with: device, format: format, fps: fps)
    }

    private func dimensions(_ format: AVCaptureDevice.Format) -> CMVideoDimensions {
        CMVideoFormatDescriptionGetDimensions(format.formatDescription)
    }

    private func createOffer() {
        let constraints = RTCMediaConstraints(
            mandatoryConstraints: [
                "OfferToReceiveAudio": "true",
                "OfferToReceiveVideo": "true"
            ],
            optionalConstraints: nil
        )
        peerConnection.offer(for: constraints) { [weak self] sdp, error in
            guard let self, let sdp, error == nil else {
                print("❌ iOS WebRTC offer failed: \(error?.localizedDescription ?? "unknown")")
                return
            }
            self.setLocal(sdp) {
                try await ZapLivreCore.shared.sendWebRtcOffer(
                    callId: self.callId,
                    sdp: sdp.sdp
                )
            }
        }
    }

    private func handle(_ notification: Notification) {
        guard peerConnection != nil else {
            pendingSignals.append(notification)
            return
        }
        guard let values = notification.userInfo,
              values["callId"] as? String == callId,
              let rawKind = values["kind"] as? String,
              let kind = WebRtcSignalKind(rawValue: rawKind) else { return }

        switch kind {
        case .offer:
            guard let sdp = values["sdp"] as? String else { return }
            setRemote(RTCSessionDescription(type: .offer, sdp: sdp)) { [weak self] in
                self?.createAnswer()
            }
        case .answer:
            guard let sdp = values["sdp"] as? String else { return }
            setRemote(RTCSessionDescription(type: .answer, sdp: sdp))
        case .ice:
            guard let candidate = values["candidate"] as? String else { return }
            let ice = RTCIceCandidate(
                sdp: candidate,
                sdpMLineIndex: Int32((values["sdpMLineIndex"] as? UInt16) ?? 0),
                sdpMid: values["sdpMid"] as? String
            )
            if peerConnection.remoteDescription == nil {
                pendingCandidates.append(ice)
            } else {
                peerConnection.add(ice)
            }
        }
    }

    private func createAnswer() {
        let constraints = RTCMediaConstraints(
            mandatoryConstraints: [
                "OfferToReceiveAudio": "true",
                "OfferToReceiveVideo": "true"
            ],
            optionalConstraints: nil
        )
        peerConnection.answer(for: constraints) { [weak self] sdp, error in
            guard let self, let sdp, error == nil else {
                print("❌ iOS WebRTC answer failed: \(error?.localizedDescription ?? "unknown")")
                return
            }
            self.setLocal(sdp) {
                try await ZapLivreCore.shared.sendWebRtcAnswer(
                    callId: self.callId,
                    sdp: sdp.sdp
                )
            }
        }
    }

    private func setLocal(
        _ sdp: RTCSessionDescription,
        send: @escaping () async throws -> Void
    ) {
        peerConnection.setLocalDescription(sdp) { error in
            guard error == nil else {
                print("❌ iOS WebRTC setLocal failed: \(error!.localizedDescription)")
                return
            }
            Task {
                do { try await send() }
                catch { print("❌ iOS WebRTC SDP signaling failed: \(error)") }
            }
        }
    }

    private func setRemote(_ sdp: RTCSessionDescription, completion: @escaping () -> Void = {}) {
        peerConnection.setRemoteDescription(sdp) { [weak self] error in
            guard let self, error == nil else {
                print("❌ iOS WebRTC setRemote failed: \(error?.localizedDescription ?? "unknown")")
                return
            }
            self.pendingCandidates.forEach { self.peerConnection.add($0) }
            self.pendingCandidates.removeAll()
            completion()
        }
    }

    private func configureAudioSession() {
        let session = AVAudioSession.sharedInstance()
        do {
            try session.setCategory(.playAndRecord, mode: .videoChat, options: [.allowBluetooth, .defaultToSpeaker])
            try session.setActive(true)
        } catch {
            print("❌ WebRTC audio session setup failed: \(error)")
        }
    }

    private func deactivateAudioSession() {
        try? AVAudioSession.sharedInstance().setActive(false, options: .notifyOthersOnDeactivation)
    }
}

extension NativeWebRtcSession: RTCPeerConnectionDelegate {
    nonisolated func peerConnection(_ peerConnection: RTCPeerConnection, didChange stateChanged: RTCSignalingState) {}
    nonisolated func peerConnection(_ peerConnection: RTCPeerConnection, didAdd stream: RTCMediaStream) {}
    nonisolated func peerConnection(_ peerConnection: RTCPeerConnection, didRemove stream: RTCMediaStream) {}
    nonisolated func peerConnectionShouldNegotiate(_ peerConnection: RTCPeerConnection) {}
    nonisolated func peerConnection(_ peerConnection: RTCPeerConnection, didChange newState: RTCIceConnectionState) {
        print("iOS WebRTC ICE \(callId): \(newState.rawValue)")
    }
    nonisolated func peerConnection(_ peerConnection: RTCPeerConnection, didChange newState: RTCIceGatheringState) {}
    nonisolated func peerConnection(_ peerConnection: RTCPeerConnection, didRemove candidates: [RTCIceCandidate]) {}
    nonisolated func peerConnection(_ peerConnection: RTCPeerConnection, didOpen dataChannel: RTCDataChannel) {}

    nonisolated func peerConnection(_ peerConnection: RTCPeerConnection, didGenerate candidate: RTCIceCandidate) {
        let callId = callId
        Task {
            do {
                try await ZapLivreCore.shared.sendWebRtcIceCandidate(
                    callId: callId,
                    candidate: candidate.sdp,
                    sdpMid: candidate.sdpMid,
                    sdpMLineIndex: UInt16(clamping: candidate.sdpMLineIndex)
                )
            } catch {
                print("❌ iOS WebRTC ICE signaling failed: \(error)")
            }
        }
    }

    nonisolated func peerConnection(
        _ peerConnection: RTCPeerConnection,
        didStartReceivingOn transceiver: RTCRtpTransceiver
    ) {
        guard let track = transceiver.receiver.track as? RTCVideoTrack else { return }
        Task { @MainActor in self.remoteVideoTrack = track }
    }
}
