//
//  MediaGalleryView.swift
//  ZapLivre
//
//  Created by ZapLivre Team
//  Copyright © 2026 ZapLivre. All rights reserved.
//

import SwiftUI

/// MediaGalleryView - Displays all media (images/videos) from a conversation
struct MediaGalleryView: View {
    let conversationId: String
    let peerName: String
    @Environment(\.dismiss) var dismiss

    @State private var mediaItems: [FfiMedia] = []
    @State private var isLoading = true
    @State private var selectedTab: MediaTab = .all
    @State private var selectedMedia: FfiMedia?
    @State private var showViewer = false

    // Grid layout
    private let columns = [
        GridItem(.flexible(), spacing: 2),
        GridItem(.flexible(), spacing: 2),
        GridItem(.flexible(), spacing: 2)
    ]

    var body: some View {
        NavigationView {
            VStack(spacing: 0) {
                // Tab picker
                Picker("Tipo de mídia", selection: $selectedTab) {
                    ForEach(MediaTab.allCases, id: \.self) { tab in
                        Text(tab.title).tag(tab)
                    }
                }
                .pickerStyle(.segmented)
                .padding()

                // Content
                if isLoading {
                    Spacer()
                    ProgressView()
                    Spacer()
                } else if mediaItems.isEmpty {
                    // Empty state
                    Spacer()
                    VStack(spacing: 12) {
                        Image(systemName: "photo.on.rectangle.angled")
                            .font(.system(size: 64))
                            .foregroundColor(.secondary)

                        Text("Nenhuma mídia")
                            .font(.title3)
                            .fontWeight(.medium)
                            .foregroundColor(.primary)

                        Text("As fotos e vídeos compartilhados aparecerão aqui")
                            .font(.body)
                            .foregroundColor(.secondary)
                            .multilineTextAlignment(.center)
                            .padding(.horizontal, 32)
                    }
                    .accessibilityIdentifier("mediagallery_empty")
                    Spacer()
                } else {
                    // Media grid
                    ScrollView {
                        LazyVGrid(columns: columns, spacing: 2) {
                            ForEach(mediaItems, id: \.id) { media in
                                MediaGridItem(media: media)
                                    .aspectRatio(1, contentMode: .fill)
                                    .clipped()
                                    .onTapGesture {
                                        selectedMedia = media
                                        showViewer = true
                                    }
                            }
                        }
                        .padding(2)
                    }
                    .accessibilityIdentifier("mediagallery_grid")
                }
            }
            .navigationTitle("Mídia - \(peerName)")
            .navigationBarTitleDisplayMode(.inline)
            .toolbar {
                ToolbarItem(placement: .navigationBarLeading) {
                    Button("Fechar") {
                        dismiss()
                    }
                }
            }
        }
        .onAppear {
            loadMedia()
        }
        .onChange(of: selectedTab) { _ in
            loadMedia()
        }
        .sheet(isPresented: $showViewer) {
            if let media = selectedMedia {
                MediaViewerView(
                    mediaItems: mediaItems,
                    initialIndex: mediaItems.firstIndex(where: { $0.id == media.id }) ?? 0
                )
            }
        }
    }

    private func loadMedia() {
        isLoading = true

        Task {
            do {
                let mediaType: FfiMediaType? = switch selectedTab {
                case .all: nil
                case .images: .image
                case .videos: .video
                }

                let media = try await ZapLivreCore.shared.getConversationMedia(
                    conversationId: conversationId,
                    mediaType: mediaType,
                    limit: 500
                )

                await MainActor.run {
                    mediaItems = media
                    isLoading = false
                }
            } catch {
                print("❌ Error loading media: \(error)")
                await MainActor.run {
                    isLoading = false
                }
            }
        }
    }
}

/// MediaGridItem - Single item in the media grid
struct MediaGridItem: View {
    let media: FfiMedia
    @State private var thumbnail: UIImage?

    var body: some View {
        ZStack {
            // Background
            Color.secondary.opacity(0.2)

            // Thumbnail
            if let thumbnail = thumbnail {
                Image(uiImage: thumbnail)
                    .resizable()
                    .scaledToFill()
            } else {
                ProgressView()
                    .progressViewStyle(.circular)
            }

            // Video overlay
            if media.mediaType == .video {
                VStack {
                    Spacer()

                    HStack {
                        Spacer()

                        // Duration badge
                        if let duration = media.durationSeconds {
                            Text(formatDuration(duration))
                                .font(.caption2)
                                .fontWeight(.semibold)
                                .foregroundColor(.white)
                                .padding(.horizontal, 6)
                                .padding(.vertical, 3)
                                .background(Color.black.opacity(0.7))
                                .cornerRadius(4)
                                .padding(4)
                        }
                    }
                }

                // Play icon
                Image(systemName: "play.circle.fill")
                    .font(.system(size: 40))
                    .foregroundColor(.white)
                    .shadow(radius: 4)
            }
        }
        .onAppear {
            loadThumbnail()
        }
    }

    private func loadThumbnail() {
        Task {
            do {
                // Try thumbnail path first
                if let thumbnailPath = media.thumbnailPath {
                    let url = URL(fileURLWithPath: thumbnailPath)
                    if let data = try? Data(contentsOf: url),
                       let image = UIImage(data: data) {
                        await MainActor.run {
                            thumbnail = image
                        }
                        return
                    }
                }

                // Download from media hash
                let data = try await ZapLivreCore.shared.downloadMedia(mediaHash: media.mediaHash)

                // For videos, use thumbnail data if available
                // For images, use the full data
                let imageData: Data
                if media.mediaType == .video {
                    // Try to extract thumbnail from video
                    // For now, just show a placeholder
                    imageData = data
                } else {
                    imageData = data
                }

                if let image = UIImage(data: imageData) {
                    await MainActor.run {
                        thumbnail = image
                    }
                }
            } catch {
                print("❌ Error loading thumbnail: \(error)")
            }
        }
    }
}

/// Media tab options
enum MediaTab: CaseIterable {
    case all
    case images
    case videos

    var title: String {
        switch self {
        case .all: return "Todas"
        case .images: return "Fotos"
        case .videos: return "Vídeos"
        }
    }
}

/// Format duration seconds to MM:SS
private func formatDuration(_ seconds: Int32) -> String {
    let mins = seconds / 60
    let secs = seconds % 60
    return String(format: "%d:%02d", mins, secs)
}
