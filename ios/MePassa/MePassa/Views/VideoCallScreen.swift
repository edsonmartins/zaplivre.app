//
//  VideoCallScreen.swift
//  MePassa
//
//  Created by MePassa Team
//  Copyright © 2026 MePassa. All rights reserved.
//

import SwiftUI
import AVFoundation

/// VideoCallScreen - UI for video call with local preview and remote video
struct VideoCallScreen: View {
    let callId: String
    let peerName: String
    let onHangup: () -> Void
    
    @StateObject private var cameraManager = CameraManager()
    @State private var videoEnabled = true
    @State private var isMuted = false
    @State private var callDuration = 0
    
    // Timer for call duration
    private let timer = Timer.publish(every: 1, on: .main, in: .common).autoconnect()
    
    var body: some View {
        ZStack {
            // Remote video (full screen)
            RemoteVideoView(callId: callId)
                .edgesIgnoringSafeArea(.all)
            
            // Local video preview (PiP - top right corner)
            if videoEnabled {
                LocalVideoPreview(cameraManager: cameraManager)
                    .frame(width: 120, height: 160)
                    .cornerRadius(12)
                    .overlay(
                        RoundedRectangle(cornerRadius: 12)
                            .stroke(Color.white.opacity(0.3), lineWidth: 2)
                    )
                    .padding()
                    .frame(maxWidth: .infinity, maxHeight: .infinity, alignment: .topTrailing)
            }
            
            // Controls overlay (bottom)
            VStack {
                Spacer()
                
                VStack(spacing: 16) {
                    // Peer name
                    Text(peerName)
                        .font(.title2)
                        .fontWeight(.medium)
                        .foregroundColor(.white)
                    
                    // Call duration
                    Text(formatDuration(callDuration))
                        .font(.body)
                        .foregroundColor(.white.opacity(0.8))
                    
                    // Control buttons
                    HStack(spacing: 20) {
                        // Video toggle
                        Button(action: toggleVideo) {
                            Image(systemName: videoEnabled ? "video.fill" : "video.slash.fill")
                                .font(.system(size: 24))
                                .frame(width: 56, height: 56)
                                .background(videoEnabled ? Color.blue : Color.red)
                                .foregroundColor(.white)
                                .clipShape(Circle())
                        }
                        
                        // Mute toggle
                        Button(action: toggleMute) {
                            Image(systemName: isMuted ? "mic.slash.fill" : "mic.fill")
                                .font(.system(size: 24))
                                .frame(width: 56, height: 56)
                                .background(isMuted ? Color.red : Color.blue)
                                .foregroundColor(.white)
                                .clipShape(Circle())
                        }
                        
                        // Switch camera
                        Button(action: switchCamera) {
                            Image(systemName: "arrow.triangle.2.circlepath.camera.fill")
                                .font(.system(size: 24))
                                .frame(width: 56, height: 56)
                                .background(Color.blue)
                                .foregroundColor(.white)
                                .clipShape(Circle())
                        }
                        
                        // Hangup
                        Button(action: hangup) {
                            Image(systemName: "phone.down.fill")
                                .font(.system(size: 28))
                                .frame(width: 72, height: 72)
                                .background(Color.red)
                                .foregroundColor(.white)
                                .clipShape(Circle())
                        }
                    }
                }
                .padding(.vertical, 24)
                .frame(maxWidth: .infinity)
                .background(
                    Color.black.opacity(0.5)
                        .edgesIgnoringSafeArea(.bottom)
                )
            }
        }
        .onAppear {
            startVideo()
        }
        .onDisappear {
            stopVideo()
        }
        .onReceive(timer) { _ in
            callDuration += 1
        }
    }
    
    // MARK: - Actions
    
    private func toggleVideo() {
        videoEnabled.toggle()

        if videoEnabled {
            startVideo()
            // Enable video track on WebRTC
            Task {
                do {
                    try await MePassaCore.shared.enableVideo(callId: callId, codec: .h264)
                } catch {
                    print("❌ Failed to enable video: \(error)")
                }
            }
        } else {
            stopVideo()
            // Disable video track on WebRTC
            Task {
                do {
                    try await MePassaCore.shared.disableVideo(callId: callId)
                } catch {
                    print("❌ Failed to disable video: \(error)")
                }
            }
        }
    }
    
    private func toggleMute() {
        isMuted.toggle()
        Task {
            do {
                try await MePassaCore.shared.toggleMute(callId: callId)
            } catch {
                print("❌ Failed to toggle mute: \(error)")
            }
        }
    }
    
    private func switchCamera() {
        cameraManager.switchCamera()
        // Notify FFI about camera switch
        Task {
            do {
                try await MePassaCore.shared.switchCamera(callId: callId)
            } catch {
                print("❌ Failed to switch camera: \(error)")
            }
        }
    }
    
    private func hangup() {
        stopVideo()
        Task {
            try? await MePassaCore.shared.hangupCall(callId: callId)
        }
        onHangup()
    }
    
    private func startVideo() {
        cameraManager.startCapture { sampleBuffer in

            // Extract pixel buffer from sample buffer
            guard let pixelBuffer = CMSampleBufferGetImageBuffer(sampleBuffer) else {
                return
            }

            // Get frame dimensions
            let width = CVPixelBufferGetWidth(pixelBuffer)
            let height = CVPixelBufferGetHeight(pixelBuffer)

            // Lock the pixel buffer for reading
            CVPixelBufferLockBaseAddress(pixelBuffer, .readOnly)
            defer {
                CVPixelBufferUnlockBaseAddress(pixelBuffer, .readOnly)
            }

            // Get raw pixel data
            guard let baseAddress = CVPixelBufferGetBaseAddress(pixelBuffer) else {
                return
            }

            let bytesPerRow = CVPixelBufferGetBytesPerRow(pixelBuffer)
            let dataSize = bytesPerRow * height
            let data = Data(bytes: baseAddress, count: dataSize)

            // Convert to UInt8 array and send via FFI
            let frameData = [UInt8](data)

            Task {
                do {
                    try await MePassaCore.shared.sendVideoFrame(
                        callId: self.callId,
                        frameData: frameData,
                        width: UInt32(width),
                        height: UInt32(height)
                    )
                } catch {
                    // Frame drop is acceptable
                }
            }
        }

        // Enable video track on WebRTC
        Task {
            do {
                try await MePassaCore.shared.enableVideo(callId: callId, codec: .h264)
            } catch {
                print("❌ Failed to enable video: \(error)")
            }
        }
    }
    
    private func stopVideo() {
        cameraManager.stopCapture()
    }
    
    // MARK: - Helpers
    
    private func formatDuration(_ seconds: Int) -> String {
        let hours = seconds / 3600
        let minutes = (seconds % 3600) / 60
        let secs = seconds % 60
        
        if hours > 0 {
            return String(format: "%d:%02d:%02d", hours, minutes, secs)
        } else {
            return String(format: "%02d:%02d", minutes, secs)
        }
    }
}

// MARK: - Local Video Preview

struct LocalVideoPreview: UIViewRepresentable {
    @ObservedObject var cameraManager: CameraManager
    
    func makeUIView(context: Context) -> UIView {
        let view = UIView()
        view.backgroundColor = .black
        
        let previewLayer = cameraManager.getPreviewLayer()
        previewLayer.frame = view.bounds
        view.layer.addSublayer(previewLayer)
        
        return view
    }
    
    func updateUIView(_ uiView: UIView, context: Context) {
        // Update layer frame on size change
        if let previewLayer = uiView.layer.sublayers?.first as? AVCaptureVideoPreviewLayer {
            DispatchQueue.main.async {
                previewLayer.frame = uiView.bounds
            }
        }
    }
}

// MARK: - Remote Video Rendering

struct RemoteVideoView: UIViewRepresentable {
    let callId: String

    func makeCoordinator() -> Coordinator {
        Coordinator(callId: callId)
    }

    func makeUIView(context: Context) -> UIView {
        let view = UIView()
        view.backgroundColor = .black

        // Create display layer
        let displayLayer = AVSampleBufferDisplayLayer()
        displayLayer.videoGravity = .resizeAspect
        displayLayer.frame = view.bounds
        view.layer.addSublayer(displayLayer)

        // Store in coordinator
        context.coordinator.displayLayer = displayLayer
        context.coordinator.videoHandler.setDisplayLayer(displayLayer)

        // Register callback
        Task {
            do {
                try await MePassaCore.shared.registerVideoFrameCallback(context.coordinator.videoHandler)
                print("✅ Video frame callback registered for call: \(callId)")
            } catch {
                print("❌ Failed to register video callback: \(error)")
            }
        }

        return view
    }

    func updateUIView(_ uiView: UIView, context: Context) {
        // Update layer frame on size change
        if let displayLayer = context.coordinator.displayLayer {
            DispatchQueue.main.async {
                displayLayer.frame = uiView.bounds
            }
        }
    }

    static func dismantleUIView(_ uiView: UIView, coordinator: Coordinator) {
        coordinator.videoHandler.release()
    }

    class Coordinator {
        let callId: String
        let videoHandler: VideoFrameHandler
        var displayLayer: AVSampleBufferDisplayLayer?

        init(callId: String) {
            self.callId = callId
            self.videoHandler = VideoFrameHandler(callId: callId)
        }
    }
}

// MARK: - Preview

#Preview {
    VideoCallScreen(
        callId: "test-call-id",
        peerName: "Test User",
        onHangup: {}
    )
}
