//
//  MediaViewerView.swift
//  MePassa
//
//  Created by MePassa Team
//  Copyright © 2026 MePassa. All rights reserved.
//

import SwiftUI
import Photos

/// MediaViewerView - Fullscreen media viewer with zoom and swipe
struct MediaViewerView: View {
    let mediaItems: [FfiMedia]
    let initialIndex: Int

    @Environment(\.dismiss) var dismiss
    @State private var currentIndex: Int
    @State private var showUI = true

    init(mediaItems: [FfiMedia], initialIndex: Int) {
        self.mediaItems = mediaItems
        self.initialIndex = initialIndex
        _currentIndex = State(initialValue: initialIndex)
    }

    var currentMedia: FfiMedia? {
        guard currentIndex < mediaItems.count else { return nil }
        return mediaItems[currentIndex]
    }

    var body: some View {
        ZStack {
            Color.black.ignoresSafeArea()

            // Media pager (TabView for swipe)
            TabView(selection: $currentIndex) {
                ForEach(Array(mediaItems.enumerated()), id: \.element.id) { index, media in
                    Group {
                        if media.mediaType == .image {
                            ZoomableImageView(media: media, showUI: $showUI)
                        } else if media.mediaType == .video {
                            VideoPlayerView(media: media, showUI: $showUI)
                        } else {
                            VStack {
                                Spacer()
                                Text("Tipo de mídia não suportado")
                                    .foregroundColor(.white)
                                Spacer()
                            }
                        }
                    }
                    .tag(index)
                }
            }
            .tabViewStyle(.page(indexDisplayMode: .never))

            // Top bar overlay
            if showUI {
                VStack {
                    HStack {
                        Button(action: { dismiss() }) {
                            Image(systemName: "xmark")
                                .font(.title2)
                                .foregroundColor(.white)
                                .padding()
                                .background(Color.black.opacity(0.5))
                                .clipShape(Circle())
                        }

                        Spacer()

                        VStack(alignment: .trailing, spacing: 2) {
                            if let media = currentMedia {
                                Text(media.fileName ?? "Mídia")
                                    .font(.headline)
                                    .foregroundColor(.white)

                                Text("\(currentIndex + 1) de \(mediaItems.count)")
                                    .font(.caption)
                                    .foregroundColor(.white.opacity(0.8))
                            }
                        }

                        Spacer()

                        Menu {
                            Button {
                                shareMedia()
                            } label: {
                                Label("Compartilhar", systemImage: "square.and.arrow.up")
                            }

                            Button {
                                downloadMedia()
                            } label: {
                                Label("Salvar", systemImage: "arrow.down.circle")
                            }
                        } label: {
                            Image(systemName: "ellipsis")
                                .font(.title2)
                                .foregroundColor(.white)
                                .padding()
                                .background(Color.black.opacity(0.5))
                                .clipShape(Circle())
                        }
                    }
                    .padding()
                    .background(
                        LinearGradient(
                            colors: [Color.black.opacity(0.6), Color.clear],
                            startPoint: .top,
                            endPoint: .bottom
                        )
                    )

                    Spacer()

                    // Page indicator (bottom)
                    if mediaItems.count > 1 {
                        HStack(spacing: 6) {
                            ForEach(0..<min(mediaItems.count, 10), id: \.self) { index in
                                Circle()
                                    .fill(index == currentIndex ? Color.white : Color.white.opacity(0.3))
                                    .frame(width: 6, height: 6)
                            }
                        }
                        .padding()
                        .background(
                            LinearGradient(
                                colors: [Color.clear, Color.black.opacity(0.4)],
                                startPoint: .top,
                                endPoint: .bottom
                            )
                        )
                    }
                }
            }
        }
        .statusBar(hidden: !showUI)
        .onTapGesture {
            withAnimation {
                showUI.toggle()
            }
        }
    }

    private func shareMedia() {
        guard let media = currentMedia else { return }

        Task {
            do {
                let data = try await MePassaCore.shared.downloadMedia(mediaHash: media.mediaHash)
                let fileName = media.fileName ?? "media_\(media.id)"
                let tempURL = FileManager.default.temporaryDirectory.appendingPathComponent(fileName)
                try data.write(to: tempURL)

                await MainActor.run {
                    let activityVC = UIActivityViewController(
                        activityItems: [tempURL],
                        applicationActivities: nil
                    )
                    // Apresentar a partir da cena ativa (SwiftUI sem host próprio)
                    if let scene = UIApplication.shared.connectedScenes.first as? UIWindowScene,
                       let rootVC = scene.windows.first?.rootViewController {
                        var top = rootVC
                        while let presented = top.presentedViewController {
                            top = presented
                        }
                        // iPad: precisa de sourceView para o popover
                        activityVC.popoverPresentationController?.sourceView = top.view
                        top.present(activityVC, animated: true)
                    }
                }
            } catch {
                print("❌ Error sharing media: \(error)")
            }
        }
    }

    private func downloadMedia() {
        guard let media = currentMedia else { return }

        Task {
            do {
                let data = try await MePassaCore.shared.downloadMedia(mediaHash: media.mediaHash)

                // Save to Photos
                if media.mediaType == .image, let image = UIImage(data: data) {
                    UIImageWriteToSavedPhotosAlbum(image, nil, nil, nil)
                    print("✅ Image saved to Photos")
                } else if media.mediaType == .video {
                    // Save video to temp file then to Photos
                    let tempURL = FileManager.default.temporaryDirectory.appendingPathComponent("video_\(media.id).mp4")
                    try data.write(to: tempURL)

                    try await PHPhotoLibrary.shared().performChanges {
                        PHAssetChangeRequest.creationRequestForAssetFromVideo(atFileURL: tempURL)
                    }
                    print("✅ Video saved to Photos")
                }
            } catch {
                print("❌ Error downloading media: \(error)")
            }
        }
    }
}

/// ZoomableImageView - Image with pinch-to-zoom support
struct ZoomableImageView: View {
    let media: FfiMedia
    @Binding var showUI: Bool

    @State private var image: UIImage?
    @State private var isLoading = true
    @State private var scale: CGFloat = 1.0
    @State private var lastScale: CGFloat = 1.0
    @State private var offset: CGSize = .zero
    @State private var lastOffset: CGSize = .zero

    var body: some View {
        ZStack {
            if isLoading {
                ProgressView()
                    .progressViewStyle(.circular)
                    .tint(.white)
            } else if let image = image {
                Image(uiImage: image)
                    .resizable()
                    .scaledToFit()
                    .scaleEffect(scale)
                    .offset(offset)
                    .gesture(
                        MagnificationGesture()
                            .onChanged { value in
                                let delta = value / lastScale
                                lastScale = value
                                scale = max(1.0, min(scale * delta, 5.0))
                            }
                            .onEnded { _ in
                                lastScale = 1.0
                                if scale < 1.0 {
                                    withAnimation {
                                        scale = 1.0
                                        offset = .zero
                                    }
                                }
                            }
                    )
                    .gesture(
                        DragGesture()
                            .onChanged { value in
                                if scale > 1.0 {
                                    offset = CGSize(
                                        width: lastOffset.width + value.translation.width,
                                        height: lastOffset.height + value.translation.height
                                    )
                                }
                            }
                            .onEnded { _ in
                                lastOffset = offset
                            }
                    )
                    .onTapGesture(count: 2) {
                        withAnimation {
                            if scale > 1.0 {
                                scale = 1.0
                                offset = .zero
                                lastOffset = .zero
                            } else {
                                scale = 2.0
                            }
                        }
                    }
            } else {
                VStack(spacing: 12) {
                    Image(systemName: "exclamationmark.triangle")
                        .font(.system(size: 48))
                        .foregroundColor(.white.opacity(0.6))

                    Text("Erro ao carregar imagem")
                        .foregroundColor(.white.opacity(0.8))
                }
            }
        }
        .frame(maxWidth: .infinity, maxHeight: .infinity)
        .onAppear {
            loadImage()
        }
    }

    private func loadImage() {
        Task {
            do {
                // Try local path first
                if let localPath = media.localPath {
                    let url = URL(fileURLWithPath: localPath)
                    if let data = try? Data(contentsOf: url),
                       let img = UIImage(data: data) {
                        await MainActor.run {
                            image = img
                            isLoading = false
                        }
                        return
                    }
                }

                // Download from media hash
                let data = try await MePassaCore.shared.downloadMedia(mediaHash: media.mediaHash)

                if let img = UIImage(data: data) {
                    await MainActor.run {
                        image = img
                        isLoading = false
                    }
                }
            } catch {
                print("❌ Error loading image: \(error)")
                await MainActor.run {
                    isLoading = false
                }
            }
        }
    }
}
