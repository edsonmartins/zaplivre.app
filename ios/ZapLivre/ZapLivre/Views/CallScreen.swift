//
//  CallScreen.swift
//  ZapLivre
//
//  Created by ZapLivre Team
//  Copyright © 2026 ZapLivre. All rights reserved.
//

import SwiftUI

struct CallScreen: View {
    @EnvironmentObject var callManager: CallManager
    @State private var callDuration: TimeInterval = 0
    @State private var timer: Timer?

    var body: some View {
        ZStack {
            // Background gradient
            LinearGradient(
                gradient: Gradient(colors: [Color.blue.opacity(0.8), Color.purple.opacity(0.6)]),
                startPoint: .topLeading,
                endPoint: .bottomTrailing
            )
            .ignoresSafeArea()

            VStack(spacing: 40) {
                Spacer()

                // Contact info
                VStack(spacing: 16) {
                    // Avatar
                    Circle()
                        .fill(Color.white.opacity(0.3))
                        .frame(width: 120, height: 120)
                        .overlay(
                            Text(callManager.currentCall?.displayName.prefix(1).uppercased() ?? "?")
                                .font(.system(size: 50, weight: .bold))
                                .foregroundColor(.white)
                        )

                    // Name
                    Text(callManager.currentCall?.displayName ?? "Unknown")
                        .font(.title)
                        .fontWeight(.semibold)
                        .foregroundColor(.white)

                    // Status
                    Text(callStatusText)
                        .font(.headline)
                        .foregroundColor(.white.opacity(0.9))
                }

                Spacer()

                // Call controls
                VStack(spacing: 30) {
                    // Audio controls
                    HStack(spacing: 50) {
                        // Mute button
                        CallControlButton(
                            icon: callManager.isMuted ? "mic.slash.fill" : "mic.fill",
                            isActive: callManager.isMuted,
                            action: { callManager.toggleMute() }
                        )

                        // Speaker button
                        CallControlButton(
                            icon: callManager.isSpeakerOn ? "speaker.wave.3.fill" : "speaker.fill",
                            isActive: callManager.isSpeakerOn,
                            action: { callManager.toggleSpeaker() }
                        )
                    }

                    // End call button
                    Button(action: { callManager.endCall() }) {
                        Image(systemName: "phone.down.fill")
                            .font(.title2)
                            .foregroundColor(.white)
                            .frame(width: 70, height: 70)
                            .background(Color.red)
                            .clipShape(Circle())
                    }
                }
                .padding(.bottom, 50)
            }
            .padding()
        }
        .onAppear {
            startTimer()
        }
        .onDisappear {
            stopTimer()
        }
    }

    private var callStatusText: String {
        switch callManager.callState {
        case .ringing:
            return "Chamando..."
        case .connecting:
            return "Conectando..."
        case .connected:
            return formatDuration(callDuration)
        case .ended:
            return "Chamada encerrada"
        case .idle:
            return ""
        }
    }

    private func startTimer() {
        timer = Timer.scheduledTimer(withTimeInterval: 1.0, repeats: true) { _ in
            if callManager.callState == .connected {
                callDuration += 1
            }
        }
    }

    private func stopTimer() {
        timer?.invalidate()
        timer = nil
    }

    private func formatDuration(_ duration: TimeInterval) -> String {
        let minutes = Int(duration) / 60
        let seconds = Int(duration) % 60
        return String(format: "%02d:%02d", minutes, seconds)
    }
}

struct CallControlButton: View {
    let icon: String
    let isActive: Bool
    let action: () -> Void

    var body: some View {
        Button(action: action) {
            Image(systemName: icon)
                .font(.title2)
                .foregroundColor(isActive ? .blue : .white)
                .frame(width: 60, height: 60)
                .background(isActive ? Color.white : Color.white.opacity(0.3))
                .clipShape(Circle())
        }
    }
}

#Preview {
    CallScreen()
        .environmentObject({
            let manager = CallManager()
            manager.currentCall = Call(
                id: UUID(),
                peerId: "12D3KooW...",
                displayName: "Alice",
                isOutgoing: true
            )
            manager.callState = .connected
            return manager
        }())
}
