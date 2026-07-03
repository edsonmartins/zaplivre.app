//
//  IncomingCallScreen.swift
//  MePassa
//
//  Created by MePassa Team
//  Copyright © 2026 MePassa. All rights reserved.
//

import SwiftUI

struct IncomingCallScreen: View {
    @EnvironmentObject var callManager: CallManager
    @State private var pulseAnimation = false

    var body: some View {
        ZStack {
            // Background
            Color.black
                .opacity(0.95)
                .ignoresSafeArea()

            VStack(spacing: 50) {
                Spacer()

                // Caller info
                VStack(spacing: 20) {
                    // Animated avatar
                    ZStack {
                        // Pulse effect
                        Circle()
                            .fill(Color.blue.opacity(0.3))
                            .frame(width: 180, height: 180)
                            .scaleEffect(pulseAnimation ? 1.2 : 1.0)
                            .opacity(pulseAnimation ? 0 : 1)

                        // Avatar
                        Circle()
                            .fill(Color.blue)
                            .frame(width: 140, height: 140)
                            .overlay(
                                Text(callManager.currentCall?.displayName.prefix(1).uppercased() ?? "?")
                                    .font(.system(size: 60, weight: .bold))
                                    .foregroundColor(.white)
                            )
                    }
                    .onAppear {
                        withAnimation(.easeInOut(duration: 1.5).repeatForever(autoreverses: false)) {
                            pulseAnimation = true
                        }
                    }

                    // Caller name
                    Text(callManager.currentCall?.displayName ?? "Unknown")
                        .font(.title)
                        .fontWeight(.semibold)
                        .foregroundColor(.white)

                    // Call type
                    HStack(spacing: 8) {
                        Image(systemName: "phone.fill")
                            .font(.caption)
                        Text("Chamada de voz ZapLivre")
                            .font(.subheadline)
                    }
                    .foregroundColor(.white.opacity(0.8))
                }

                Spacer()

                // Call actions
                HStack(spacing: 80) {
                    // Decline button
                    VStack(spacing: 12) {
                        Button(action: { callManager.endCall() }) {
                            Image(systemName: "phone.down.fill")
                                .font(.title2)
                                .foregroundColor(.white)
                                .frame(width: 70, height: 70)
                                .background(Color.red)
                                .clipShape(Circle())
                        }

                        Text("Recusar")
                            .font(.subheadline)
                            .foregroundColor(.white)
                    }

                    // Accept button
                    VStack(spacing: 12) {
                        Button(action: { callManager.answerCall() }) {
                            Image(systemName: "phone.fill")
                                .font(.title2)
                                .foregroundColor(.white)
                                .frame(width: 70, height: 70)
                                .background(Color.green)
                                .clipShape(Circle())
                        }

                        Text("Aceitar")
                            .font(.subheadline)
                            .foregroundColor(.white)
                    }
                }
                .padding(.bottom, 60)
            }
            .padding()
        }
    }
}

#Preview {
    IncomingCallScreen()
        .environmentObject({
            let manager = CallManager()
            manager.currentCall = Call(
                id: UUID(),
                peerId: "12D3KooW...",
                displayName: "Bob",
                isOutgoing: false
            )
            manager.callState = .ringing
            return manager
        }())
}
