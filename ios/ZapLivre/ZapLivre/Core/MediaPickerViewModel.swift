//
//  MediaPickerViewModel.swift
//  ZapLivre
//
//  ViewModel for managing media selection and upload
//

import SwiftUI
import Combine

/// Upload state
enum UploadState: Equatable {
    case idle
    case uploading(current: Int, total: Int)
    case success
    case error(String)
}

/// Media picker view model
class MediaPickerViewModel: ObservableObject {
    @Published var selectedImages: [UIImage] = []
    @Published var uploadState: UploadState = .idle

    private let zapLivreCore: ZapLivreCore

    init(zapLivreCore: ZapLivreCore = .shared) {
        self.zapLivreCore = zapLivreCore
    }

    /// Add images to selection
    func addImages(_ images: [UIImage]) {
        selectedImages.append(contentsOf: images)
    }

    /// Remove image at index
    func removeImage(at index: Int) {
        guard index >= 0 && index < selectedImages.count else { return }
        selectedImages.remove(at: index)
    }

    /// Clear all selected images
    func clearSelection() {
        selectedImages.removeAll()
    }

    /// Upload images to peer
    func uploadImages(to peerId: String, quality: CGFloat = 0.85) {
        guard !selectedImages.isEmpty else { return }

        uploadState = .uploading(current: 0, total: selectedImages.count)

        Task {
            do {
                for (index, image) in selectedImages.enumerated() {
                    try await uploadSingleImage(image, to: peerId, quality: quality)

                    await MainActor.run {
                        uploadState = .uploading(current: index + 1, total: selectedImages.count)
                    }
                }

                await MainActor.run {
                    uploadState = .success
                    clearSelection()
                }
            } catch {
                await MainActor.run {
                    uploadState = .error(error.localizedDescription)
                }
            }
        }
    }

    /// Upload a single image with compression (via FFI)
    private func uploadSingleImage(_ image: UIImage, to peerId: String, quality: CGFloat) async throws {
        // Convert UIImage to JPEG data (pre-compression before FFI)
        guard let imageData = image.jpegData(compressionQuality: quality) else {
            throw MediaError.compressionFailed
        }

        // Generate unique filename
        let fileName = "image_\(Int(Date().timeIntervalSince1970)).jpg"

        // Convert quality to 0-100 scale for FFI
        let qualityPercent = UInt32(quality * 100)

        // Call FFI method to send image with additional compression in Rust
        let messageId = try await zapLivreCore.sendImageMessage(
            to: peerId,
            imageData: imageData,
            fileName: fileName,
            quality: qualityPercent
        )

        print("✅ Image sent successfully: \(messageId)")
    }

    /// Reset upload state
    func resetUploadState() {
        uploadState = .idle
    }

    /// Get compressed JPEG data from UIImage
    func getCompressedJPEG(from image: UIImage, quality: CGFloat = 0.8) -> Data? {
        return image.jpegData(compressionQuality: quality)
    }

    /// Get thumbnail from UIImage
    func getThumbnail(from image: UIImage, size: CGSize = CGSize(width: 200, height: 200)) -> UIImage? {
        let renderer = UIGraphicsImageRenderer(size: size)
        return renderer.image { _ in
            image.draw(in: CGRect(origin: .zero, size: size))
        }
    }
}

/// Media-related errors
enum MediaError: LocalizedError {
    case compressionFailed
    case uploadFailed
    case invalidImage

    var errorDescription: String? {
        switch self {
        case .compressionFailed:
            return "Failed to compress image"
        case .uploadFailed:
            return "Failed to upload image"
        case .invalidImage:
            return "Invalid image format"
        }
    }
}

/// Media item metadata
struct MediaItem: Identifiable {
    let id = UUID()
    let image: UIImage
    let fileName: String?
    let fileSize: Int64?
}
