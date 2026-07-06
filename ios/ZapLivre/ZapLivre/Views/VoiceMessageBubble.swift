//
//  VoiceMessageBubble.swift
//  ZapLivre
//
//  Voice message bubble with playback controls
//

import SwiftUI
import AVFoundation

/// Voice message bubble with audio playback
struct VoiceMessageBubble: View {
    let audioURL: String
    let durationSeconds: Int?
    let isOwnMessage: Bool
    let timestamp: String

    @StateObject private var audioPlayer = AudioPlayerViewModel()

    var body: some View {
        HStack {
            if isOwnMessage {
                Spacer()
            }

            VStack(alignment: isOwnMessage ? .trailing : .leading, spacing: 4) {
                HStack(spacing: 12) {
                    // Play/Pause button
                    Button(action: {
                        if audioPlayer.isPlaying {
                            audioPlayer.pause()
                        } else {
                            if audioPlayer.currentPosition == 0 {
                                audioPlayer.play(url: URL(string: audioURL)!)
                            } else {
                                audioPlayer.resume()
                            }
                        }
                    }) {
                        Image(systemName: audioPlayer.isPlaying ? "pause.circle.fill" : "play.circle.fill")
                            .font(.system(size: 36))
                            .foregroundColor(isOwnMessage ? .white : .blue)
                    }

                    // Waveform and duration
                    VStack(alignment: .leading, spacing: 4) {
                        // Progress bar
                        ProgressView(value: audioPlayer.progress)
                            .tint(isOwnMessage ? .white : .blue)
                            .frame(width: 150)

                        // Duration text
                        Text("\(formatTime(audioPlayer.currentPosition)) / \(formatTime(durationSeconds ?? 0))")
                            .font(.caption)
                            .foregroundColor(isOwnMessage ? .white.opacity(0.9) : .secondary)
                    }
                }
                .padding(.horizontal, 12)
                .padding(.vertical, 8)
                .background(isOwnMessage ? Color.blue : Color(.systemGray5))
                .cornerRadius(16)

                // Timestamp
                Text(timestamp)
                    .font(.caption2)
                    .foregroundColor(.secondary)
            }

            if !isOwnMessage {
                Spacer()
            }
        }
        .onAppear {
            if let url = URL(string: audioURL) {
                audioPlayer.loadAudio(url: url, duration: TimeInterval(durationSeconds ?? 0))
            }
        }
    }

    private func formatTime(_ seconds: Int) -> String {
        let minutes = seconds / 60
        let secs = seconds % 60
        return String(format: "%02d:%02d", minutes, secs)
    }
}

/// Audio player view model
class AudioPlayerViewModel: NSObject, ObservableObject, AVAudioPlayerDelegate {
    @Published var isPlaying = false
    @Published var currentPosition = 0
    @Published var duration = 0
    @Published var progress: Double = 0.0

    private var audioPlayer: AVAudioPlayer?
    private var timer: Timer?

    func loadAudio(url: URL, duration: TimeInterval) {
        do {
            audioPlayer = try AVAudioPlayer(contentsOf: url)
            audioPlayer?.delegate = self
            audioPlayer?.prepareToPlay()
            self.duration = Int(audioPlayer?.duration ?? duration)
        } catch {
            print("Failed to load audio: \(error)")
        }
    }

    func play(url: URL) {
        do {
            audioPlayer = try AVAudioPlayer(contentsOf: url)
            audioPlayer?.delegate = self
            audioPlayer?.play()
            isPlaying = true
            startTimer()
        } catch {
            print("Failed to play audio: \(error)")
        }
    }

    func pause() {
        audioPlayer?.pause()
        isPlaying = false
        stopTimer()
    }

    func resume() {
        audioPlayer?.play()
        isPlaying = true
        startTimer()
    }

    func stop() {
        audioPlayer?.stop()
        audioPlayer?.currentTime = 0
        isPlaying = false
        currentPosition = 0
        progress = 0
        stopTimer()
    }

    private func startTimer() {
        timer = Timer.scheduledTimer(withTimeInterval: 0.1, repeats: true) { [weak self] _ in
            guard let self = self, let player = self.audioPlayer else { return }

            self.currentPosition = Int(player.currentTime)
            self.progress = player.currentTime / player.duration

            // Auto-stop when finished
            if !player.isPlaying {
                self.stop()
            }
        }
    }

    private func stopTimer() {
        timer?.invalidate()
        timer = nil
    }

    // MARK: - AVAudioPlayerDelegate

    func audioPlayerDidFinishPlaying(_ player: AVAudioPlayer, successfully flag: Bool) {
        stop()
    }
}

/// Compact voice message indicator
struct VoiceMessageIndicator: View {
    let durationSeconds: Int
    let isPlaying: Bool

    var body: some View {
        HStack(spacing: 8) {
            Image(systemName: isPlaying ? "pause.circle.fill" : "play.circle.fill")
                .font(.system(size: 16))
                .foregroundColor(.blue)

            Text(formatTime(durationSeconds))
                .font(.subheadline)
        }
    }

    private func formatTime(_ seconds: Int) -> String {
        let minutes = seconds / 60
        let secs = seconds % 60
        return String(format: "%02d:%02d", minutes, secs)
    }
}
