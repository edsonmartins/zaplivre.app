//
//  VideoPlayerView.swift
//  ZapLivre
//
//  Created by ZapLivre Team
//  Copyright © 2026 ZapLivre. All rights reserved.
//

import SwiftUI
import AVKit

/// VideoPlayerView - Video player with playback controls
struct VideoPlayerView: View {
    let media: FfiMedia
    @Binding var showUI: Bool

    @StateObject private var playerViewModel = VideoPlayerViewModel()
    @State private var isLoading = true

    var body: some View {
        ZStack {
            Color.black

            if isLoading {
                ProgressView()
                    .progressViewStyle(.circular)
                    .tint(.white)
            } else if let player = playerViewModel.player {
                // Video player
                VideoPlayer(player: player) {
                    // Custom overlay (empty - we control it below)
                }
                .ignoresSafeArea()
                .onTapGesture {
                    showUI.toggle()
                }

                // Custom controls overlay
                if showUI {
                    VStack {
                        Spacer()

                        // Play/Pause button
                        Button(action: {
                            playerViewModel.togglePlayPause()
                        }) {
                            Image(systemName: playerViewModel.isPlaying ? "pause.circle.fill" : "play.circle.fill")
                                .font(.system(size: 64))
                                .foregroundColor(.white)
                                .shadow(radius: 10)
                        }

                        Spacer()

                        // Video info overlay
                        VStack(alignment: .leading, spacing: 8) {
                            if let fileName = media.fileName {
                                Text(fileName)
                                    .font(.body)
                                    .foregroundColor(.white)
                            }

                            HStack(spacing: 16) {
                                if let duration = media.durationSeconds {
                                    Text(formatDuration(duration))
                                        .font(.caption)
                                        .foregroundColor(.white.opacity(0.8))
                                }

                                if let width = media.width, let height = media.height {
                                    Text("\(width)x\(height)")
                                        .font(.caption)
                                        .foregroundColor(.white.opacity(0.8))
                                }

                                if let fileSize = media.fileSize {
                                    Text(formatFileSize(fileSize))
                                        .font(.caption)
                                        .foregroundColor(.white.opacity(0.8))
                                }
                            }
                        }
                        .frame(maxWidth: .infinity, alignment: .leading)
                        .padding()
                        .background(
                            LinearGradient(
                                colors: [Color.clear, Color.black.opacity(0.7)],
                                startPoint: .top,
                                endPoint: .bottom
                            )
                        )
                    }
                }
            } else {
                VStack(spacing: 12) {
                    Image(systemName: "exclamationmark.triangle")
                        .font(.system(size: 48))
                        .foregroundColor(.white.opacity(0.6))

                    Text("Erro ao carregar vídeo")
                        .foregroundColor(.white.opacity(0.8))
                }
            }
        }
        .onAppear {
            loadVideo()
        }
        .onDisappear {
            playerViewModel.cleanup()
        }
    }

    private func loadVideo() {
        Task {
            do {
                // Try local path first
                if let localPath = media.localPath {
                    let url = URL(fileURLWithPath: localPath)
                    if FileManager.default.fileExists(atPath: url.path) {
                        await MainActor.run {
                            playerViewModel.setupPlayer(url: url)
                            isLoading = false
                        }
                        return
                    }
                }

                // Download video to temp location
                let data = try await ZapLivreCore.shared.downloadMedia(mediaHash: media.mediaHash)
                let tempURL = FileManager.default.temporaryDirectory.appendingPathComponent("video_\(media.id).mp4")
                try data.write(to: tempURL)

                await MainActor.run {
                    playerViewModel.setupPlayer(url: tempURL)
                    isLoading = false
                }
            } catch {
                print("❌ Error loading video: \(error)")
                await MainActor.run {
                    isLoading = false
                }
            }
        }
    }
}

/// VideoPlayerViewModel - Manages AVPlayer state
class VideoPlayerViewModel: ObservableObject {
    @Published var player: AVPlayer?
    @Published var isPlaying = false

    private var timeObserver: Any?

    func setupPlayer(url: URL) {
        let playerItem = AVPlayerItem(url: url)
        player = AVPlayer(playerItem: playerItem)

        // Add observer for playback state
        NotificationCenter.default.addObserver(
            forName: .AVPlayerItemDidPlayToEndTime,
            object: playerItem,
            queue: .main
        ) { [weak self] _ in
            self?.isPlaying = false
            self?.player?.seek(to: .zero)
        }
    }

    func togglePlayPause() {
        guard let player = player else { return }

        if isPlaying {
            player.pause()
            isPlaying = false
        } else {
            player.play()
            isPlaying = true
        }
    }

    func cleanup() {
        player?.pause()
        player = nil

        if let observer = timeObserver {
            player?.removeTimeObserver(observer)
            timeObserver = nil
        }
    }

    deinit {
        cleanup()
    }
}

/// Format duration seconds to MM:SS
private func formatDuration(_ seconds: Int32) -> String {
    let mins = seconds / 60
    let secs = seconds % 60
    return String(format: "%d:%02d", mins, secs)
}

/// Format file size to human readable format
private func formatFileSize(_ bytes: Int64) -> String {
    let kb = Double(bytes) / 1024.0
    let mb = kb / 1024.0
    let gb = mb / 1024.0

    if gb >= 1.0 {
        return String(format: "%.1f GB", gb)
    } else if mb >= 1.0 {
        return String(format: "%.1f MB", mb)
    } else if kb >= 1.0 {
        return String(format: "%.1f KB", kb)
    } else {
        return "\(bytes) B"
    }
}
