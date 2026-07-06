//
//  VoiceRecordButton.swift
//  ZapLivre
//
//  Voice record button with hold-to-record functionality
//

import SwiftUI

/// Voice record button with hold-to-record
struct VoiceRecordButton: View {
    @ObservedObject var viewModel: VoiceRecorderViewModel
    let onVoiceMessageRecorded: (URL) -> Void

    @State private var isPressing = false

    var body: some View {
        ZStack {
            if viewModel.isRecording {
                // Recording UI
                HStack(spacing: 12) {
                    // Cancel button
                    Button(action: {
                        viewModel.cancelRecording()
                    }) {
                        Image(systemName: "xmark")
                            .font(.system(size: 20))
                            .foregroundColor(.red)
                    }

                    // Duration indicator
                    HStack(spacing: 8) {
                        // Pulsing red dot
                        Circle()
                            .fill(Color.red)
                            .frame(width: 10, height: 10)
                            .scaleEffect(isPressing ? 1.2 : 1.0)
                            .animation(
                                Animation.easeInOut(duration: 0.6).repeatForever(autoreverses: true),
                                value: isPressing
                            )
                            .onAppear { isPressing = true }
                            .onDisappear { isPressing = false }

                        Text(viewModel.formatDuration(viewModel.recordingDuration))
                            .font(.body)
                            .foregroundColor(.primary)

                        Text("Recording...")
                            .font(.caption)
                            .foregroundColor(.secondary)
                    }
                    .padding(.horizontal, 16)
                    .padding(.vertical, 8)
                    .background(Color.red.opacity(0.1))
                    .cornerRadius(20)

                    // Stop/Send button
                    Button(action: {
                        if let url = viewModel.stopRecording() {
                            // Only send if recording is longer than 0.5s
                            if viewModel.recordingDuration > 0.5 {
                                onVoiceMessageRecorded(url)
                            }
                        }
                    }) {
                        Image(systemName: "checkmark")
                            .font(.system(size: 20))
                            .foregroundColor(.white)
                            .frame(width: 40, height: 40)
                            .background(Color.blue)
                            .clipShape(Circle())
                    }
                }
                .transition(.move(edge: .trailing).combined(with: .opacity))
            } else {
                // Mic button (press and hold to record)
                Button(action: {}) {
                    Image(systemName: "mic.fill")
                        .font(.title2)
                        .foregroundColor(.blue)
                }
                .simultaneousGesture(
                    LongPressGesture(minimumDuration: 0.1)
                        .onEnded { _ in
                            viewModel.startRecording()
                        }
                )
                .transition(.move(edge: .trailing).combined(with: .opacity))
            }
        }
        .animation(.easeInOut(duration: 0.2), value: viewModel.isRecording)
    }
}

/// Simple voice record button (tap to start recording)
struct SimpleVoiceRecordButton: View {
    @ObservedObject var viewModel: VoiceRecorderViewModel
    let onVoiceMessageRecorded: (URL) -> Void

    var body: some View {
        Button(action: {
            if viewModel.isRecording {
                if let url = viewModel.stopRecording() {
                    if viewModel.recordingDuration > 0.5 {
                        onVoiceMessageRecorded(url)
                    }
                }
            } else {
                viewModel.startRecording()
            }
        }) {
            Image(systemName: viewModel.isRecording ? "stop.circle.fill" : "mic.circle.fill")
                .font(.system(size: 32))
                .foregroundColor(viewModel.isRecording ? .red : .blue)
        }
    }
}

/// Voice recording overlay (full-width)
struct VoiceRecordingOverlay: View {
    let isRecording: Bool
    let duration: TimeInterval
    let onCancel: () -> Void
    let onSend: () -> Void

    var body: some View {
        if isRecording {
            HStack {
                Button("Cancel", action: onCancel)
                    .foregroundColor(.red)

                Spacer()

                HStack(spacing: 8) {
                    Circle()
                        .fill(Color.red)
                        .frame(width: 8, height: 8)

                    Text(formatDuration(duration))
                        .font(.headline)
                }

                Spacer()

                Button("Send", action: onSend)
            }
            .padding()
            .background(Color(.systemGray6))
            .transition(.move(edge: .bottom))
        }
    }

    private func formatDuration(_ duration: TimeInterval) -> String {
        let minutes = Int(duration) / 60
        let seconds = Int(duration) % 60
        return String(format: "%02d:%02d", minutes, seconds)
    }
}
