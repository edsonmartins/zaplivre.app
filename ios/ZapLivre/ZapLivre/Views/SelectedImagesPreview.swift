//
//  SelectedImagesPreview.swift
//  ZapLivre
//
//  Preview of selected images before sending
//

import SwiftUI

/// Preview of selected images before sending
struct SelectedImagesPreview: View {
    let selectedImages: [UIImage]
    let onRemoveImage: (Int) -> Void
    let onSendImages: () -> Void

    var body: some View {
        if selectedImages.isEmpty {
            EmptyView()
        } else {
            VStack(spacing: 0) {
                Divider()

                VStack(spacing: 12) {
                    // Header with count and send button
                    HStack {
                        Text("\(selectedImages.count) image\(selectedImages.count > 1 ? "s" : "") selected")
                            .font(.subheadline)
                            .foregroundColor(.secondary)

                        Spacer()

                        Button(action: onSendImages) {
                            HStack(spacing: 6) {
                                Image(systemName: "paperplane.fill")
                                    .font(.system(size: 14))
                                Text("Send")
                                    .font(.subheadline.weight(.medium))
                            }
                            .foregroundColor(.white)
                            .padding(.horizontal, 16)
                            .padding(.vertical, 8)
                            .background(Color.blue)
                            .cornerRadius(20)
                        }
                    }
                    .padding(.horizontal)
                    .padding(.top, 12)

                    // Horizontal scrollable thumbnails
                    ScrollView(.horizontal, showsIndicators: false) {
                        HStack(spacing: 8) {
                            ForEach(selectedImages.indices, id: \.self) { index in
                                SelectedImageThumbnail(
                                    image: selectedImages[index],
                                    onRemove: { onRemoveImage(index) }
                                )
                            }
                        }
                        .padding(.horizontal)
                    }
                    .frame(height: 90)
                }
                .padding(.bottom, 12)
                .background(Color(.systemGray6))
            }
        }
    }
}

/// Single thumbnail in the selected images preview
struct SelectedImageThumbnail: View {
    let image: UIImage
    let onRemove: () -> Void

    var body: some View {
        ZStack(alignment: .topTrailing) {
            Image(uiImage: image)
                .resizable()
                .aspectRatio(contentMode: .fill)
                .frame(width: 80, height: 80)
                .clipped()
                .cornerRadius(8)

            Button(action: onRemove) {
                Image(systemName: "xmark")
                    .font(.system(size: 12, weight: .bold))
                    .foregroundColor(.white)
                    .frame(width: 24, height: 24)
                    .background(Color.black.opacity(0.6))
                    .clipShape(Circle())
            }
            .offset(x: 4, y: -4)
        }
    }
}

/// Compact indicator for chat input area
struct CompactSelectedImagesIndicator: View {
    let selectedCount: Int
    let onClear: () -> Void
    let onView: () -> Void

    var body: some View {
        if selectedCount == 0 {
            EmptyView()
        } else {
            Button(action: onView) {
                HStack(spacing: 8) {
                    Image(systemName: "photo.fill")
                        .font(.system(size: 16))

                    Text("\(selectedCount)")
                        .font(.subheadline.weight(.medium))

                    Button(action: onClear) {
                        Image(systemName: "xmark")
                            .font(.system(size: 12))
                    }
                    .buttonStyle(PlainButtonStyle())
                }
                .foregroundColor(.white)
                .padding(.horizontal, 12)
                .padding(.vertical, 8)
                .background(Color.blue)
                .cornerRadius(16)
            }
        }
    }
}

/// Full-screen preview of all selected images
struct SelectedImagesFullPreview: View {
    @Binding var selectedImages: [UIImage]
    @Environment(\.dismiss) var dismiss

    var body: some View {
        Text("Image Preview")
            .onAppear {
                dismiss()
            }
    }
}

// MARK: - Preview

struct SelectedImagesPreview_Previews: PreviewProvider {
    static var previews: some View {
        VStack {
            Spacer()

            SelectedImagesPreview(
                selectedImages: [UIImage(systemName: "photo")!, UIImage(systemName: "photo")!],
                onRemoveImage: { _ in },
                onSendImages: {}
            )
        }
    }
}
