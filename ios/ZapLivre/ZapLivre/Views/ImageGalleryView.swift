//
//  ImageGalleryView.swift
//  ZapLivre
//
//  Grid gallery for displaying images in a conversation
//

import SwiftUI

/// Gallery image data model
struct GalleryImage: Identifiable {
    let id: Int64
    let url: String
    let thumbnailUrl: String?
    let width: Int?
    let height: Int?
    let fileName: String?
}

/// Image gallery grid view
struct ImageGalleryView: View {
    let images: [GalleryImage]
    let onImageTap: (GalleryImage) -> Void

    private let columns = [
        GridItem(.flexible(), spacing: 4),
        GridItem(.flexible(), spacing: 4),
        GridItem(.flexible(), spacing: 4)
    ]

    var body: some View {
        if images.isEmpty {
            EmptyGalleryPlaceholder()
        } else {
            ScrollView {
                LazyVGrid(columns: columns, spacing: 4) {
                    ForEach(images) { image in
                        ImageThumbnail(image: image)
                            .onTapGesture {
                                onImageTap(image)
                            }
                    }
                }
                .padding(4)
            }
        }
    }
}

/// Single thumbnail in the gallery
struct ImageThumbnail: View {
    let image: GalleryImage

    var body: some View {
        AsyncImage(url: URL(string: image.thumbnailUrl ?? image.url)) { phase in
            switch phase {
            case .empty:
                ProgressView()
                    .frame(maxWidth: .infinity)
                    .aspectRatio(1, contentMode: .fill)
                    .background(Color.gray.opacity(0.2))

            case .success(let loadedImage):
                loadedImage
                    .resizable()
                    .aspectRatio(contentMode: .fill)
                    .frame(maxWidth: .infinity)
                    .aspectRatio(1, contentMode: .fill)
                    .clipped()

            case .failure:
                Image(systemName: "photo.fill")
                    .foregroundColor(.gray)
                    .frame(maxWidth: .infinity)
                    .aspectRatio(1, contentMode: .fill)
                    .background(Color.gray.opacity(0.2))

            @unknown default:
                EmptyView()
            }
        }
        .cornerRadius(4)
    }
}

/// Empty state placeholder
struct EmptyGalleryPlaceholder: View {
    var body: some View {
        VStack(spacing: 12) {
            Image(systemName: "photo.on.rectangle")
                .font(.system(size: 60))
                .foregroundColor(.gray.opacity(0.5))

            Text("No images yet")
                .font(.title3)
                .foregroundColor(.secondary)

            Text("Share photos to see them here")
                .font(.subheadline)
                .foregroundColor(.secondary.opacity(0.7))
        }
        .frame(maxWidth: .infinity, maxHeight: .infinity)
        .padding(32)
    }
}

/// Full gallery screen with navigation bar
struct ImageGalleryScreen: View {
    let conversationName: String
    let images: [GalleryImage]
    let onImageTap: (GalleryImage) -> Void
    @Environment(\.dismiss) var dismiss

    var body: some View {
        NavigationView {
            ImageGalleryView(images: images, onImageTap: onImageTap)
                .navigationTitle(conversationName)
                .navigationBarTitleDisplayMode(.inline)
                .toolbar {
                    ToolbarItem(placement: .navigationBarLeading) {
                        Button("Back") {
                            dismiss()
                        }
                    }

                    ToolbarItem(placement: .principal) {
                        VStack(spacing: 2) {
                            Text(conversationName)
                                .font(.headline)
                            Text("\(images.count) photos")
                                .font(.caption)
                                .foregroundColor(.secondary)
                        }
                    }
                }
        }
    }
}

// MARK: - Preview

struct ImageGalleryView_Previews: PreviewProvider {
    static var previews: some View {
        let sampleImages = (1...12).map { i in
            GalleryImage(
                id: Int64(i),
                url: "https://picsum.photos/400/400?random=\(i)",
                thumbnailUrl: "https://picsum.photos/200/200?random=\(i)",
                width: 400,
                height: 400,
                fileName: "image\(i).jpg"
            )
        }

        ImageGalleryView(images: sampleImages) { image in
            print("Tapped image: \(image.id)")
        }
    }
}
