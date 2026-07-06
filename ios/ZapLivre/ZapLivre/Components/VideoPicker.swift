//
//  VideoPicker.swift
//  ZapLivre
//
//  Created by ZapLivre Team
//  Copyright © 2026 ZapLivre. All rights reserved.
//

import SwiftUI
import PhotosUI
import AVFoundation

/// VideoPicker - Allows user to select videos from photo library
///
/// Uses PHPickerViewController for video selection.
/// Extracts video metadata (duration, dimensions, thumbnail).
struct VideoPicker: UIViewControllerRepresentable {
    @Environment(\.dismiss) var dismiss
    let onVideoPicked: (VideoInfo) -> Void

    func makeUIViewController(context: Context) -> PHPickerViewController {
        var configuration = PHPickerConfiguration(photoLibrary: .shared())
        configuration.filter = .videos
        configuration.selectionLimit = 1

        let picker = PHPickerViewController(configuration: configuration)
        picker.delegate = context.coordinator
        return picker
    }

    func updateUIViewController(_ uiViewController: PHPickerViewController, context: Context) {
        // No updates needed
    }

    func makeCoordinator() -> Coordinator {
        Coordinator(self)
    }

    class Coordinator: NSObject, PHPickerViewControllerDelegate {
        let parent: VideoPicker

        init(_ parent: VideoPicker) {
            self.parent = parent
        }

        func picker(_ picker: PHPickerViewController, didFinishPicking results: [PHPickerResult]) {
            guard let result = results.first else {
                parent.dismiss()
                return
            }

            // Load video as file
            result.itemProvider.loadFileRepresentation(forTypeIdentifier: UTType.movie.identifier) { url, error in
                guard let url = url, error == nil else {
                    print("❌ Error loading video: \(String(describing: error))")
                    DispatchQueue.main.async {
                        self.parent.dismiss()
                    }
                    return
                }

                // Copy to temp location (security-scoped resource)
                let tempURL = FileManager.default.temporaryDirectory.appendingPathComponent(url.lastPathComponent)
                do {
                    if FileManager.default.fileExists(atPath: tempURL.path) {
                        try FileManager.default.removeItem(at: tempURL)
                    }
                    try FileManager.default.copyItem(at: url, to: tempURL)

                    // Extract metadata
                    if let videoInfo = self.extractVideoInfo(from: tempURL) {
                        DispatchQueue.main.async {
                            self.parent.onVideoPicked(videoInfo)
                            self.parent.dismiss()
                        }
                    } else {
                        DispatchQueue.main.async {
                            self.parent.dismiss()
                        }
                    }
                } catch {
                    print("❌ Error copying video: \(error)")
                    DispatchQueue.main.async {
                        self.parent.dismiss()
                    }
                }
            }
        }

        private func extractVideoInfo(from url: URL) -> VideoInfo? {
            let asset = AVAsset(url: url)

            // Get duration
            let duration = CMTimeGetSeconds(asset.duration)

            // Get dimensions
            guard let track = asset.tracks(withMediaType: .video).first else {
                return nil
            }
            let size = track.naturalSize
            let width = Int(size.width)
            let height = Int(size.height)

            // Generate thumbnail
            let thumbnailData = generateThumbnail(from: asset)

            // Get file info
            let fileName = url.lastPathComponent
            let fileSize = (try? FileManager.default.attributesOfItem(atPath: url.path)[.size] as? Int64) ?? 0

            return VideoInfo(
                url: url,
                fileName: fileName,
                fileSize: fileSize,
                durationSeconds: Int(duration),
                width: width,
                height: height,
                thumbnailData: thumbnailData
            )
        }

        private func generateThumbnail(from asset: AVAsset) -> Data? {
            let imageGenerator = AVAssetImageGenerator(asset: asset)
            imageGenerator.appliesPreferredTrackTransform = true

            do {
                let cgImage = try imageGenerator.copyCGImage(at: .zero, actualTime: nil)
                let image = UIImage(cgImage: cgImage)

                // Compress to JPEG
                return image.jpegData(compressionQuality: 0.8)
            } catch {
                print("❌ Error generating thumbnail: \(error)")
                return nil
            }
        }
    }
}

/// VideoInfo - Video metadata
struct VideoInfo {
    let url: URL
    let fileName: String
    let fileSize: Int64
    let durationSeconds: Int
    let width: Int
    let height: Int
    let thumbnailData: Data?
}

/// VideoPickerButton - Button that shows video picker
struct VideoPickerButton: View {
    @State private var showingPicker = false
    let onVideoPicked: (VideoInfo) -> Void
    let isEnabled: Bool

    init(isEnabled: Bool = true, onVideoPicked: @escaping (VideoInfo) -> Void) {
        self.isEnabled = isEnabled
        self.onVideoPicked = onVideoPicked
    }

    var body: some View {
        Button(action: {
            showingPicker = true
        }) {
            Image(systemName: "video")
                .font(.title2)
                .foregroundColor(isEnabled ? .blue : .gray)
        }
        .disabled(!isEnabled)
        .sheet(isPresented: $showingPicker) {
            VideoPicker(onVideoPicked: onVideoPicked)
        }
    }
}
