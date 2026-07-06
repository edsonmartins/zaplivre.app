//
//  ImagePreviewView.swift
//  ZapLivre
//
//  Full-screen image preview with zoom and pan
//

import SwiftUI

/// Full-screen image preview with zoom and pan
struct ImagePreviewView: View {
    let imageUrl: String
    let fileName: String?
    let onDismiss: () -> Void
    let onDownload: (() -> Void)?

    @State private var scale: CGFloat = 1.0
    @State private var lastScale: CGFloat = 1.0
    @State private var offset: CGSize = .zero
    @State private var lastOffset: CGSize = .zero

    var body: some View {
        ZStack {
            Color.black.ignoresSafeArea()

            // Image with zoom and pan
            AsyncImage(url: URL(string: imageUrl)) { phase in
                switch phase {
                case .empty:
                    ProgressView()
                        .progressViewStyle(CircularProgressViewStyle(tint: .white))

                case .success(let image):
                    image
                        .resizable()
                        .aspectRatio(contentMode: .fit)
                        .scaleEffect(scale)
                        .offset(offset)
                        .gesture(
                            MagnificationGesture()
                                .onChanged { value in
                                    let delta = value / lastScale
                                    lastScale = value
                                    scale = min(max(scale * delta, 1), 5)
                                }
                                .onEnded { _ in
                                    lastScale = 1.0
                                    if scale < 1 {
                                        withAnimation {
                                            scale = 1
                                            offset = .zero
                                        }
                                    }
                                }
                        )
                        .gesture(
                            DragGesture()
                                .onChanged { value in
                                    if scale > 1 {
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

                case .failure:
                    VStack {
                        Image(systemName: "exclamationmark.triangle")
                            .font(.largeTitle)
                            .foregroundColor(.white)
                        Text("Failed to load image")
                            .foregroundColor(.white)
                    }

                @unknown default:
                    EmptyView()
                }
            }

            // Top bar
            VStack {
                HStack {
                    Button(action: onDismiss) {
                        Image(systemName: "xmark")
                            .font(.system(size: 20, weight: .medium))
                            .foregroundColor(.white)
                            .padding(12)
                            .background(Color.black.opacity(0.5))
                            .clipShape(Circle())
                    }

                    Spacer()

                    if let fileName = fileName {
                        Text(fileName)
                            .font(.headline)
                            .foregroundColor(.white)
                            .padding(.horizontal, 16)
                            .padding(.vertical, 8)
                            .background(Color.black.opacity(0.5))
                            .cornerRadius(8)
                    }

                    Spacer()

                    if let onDownload = onDownload {
                        Button(action: onDownload) {
                            Image(systemName: "arrow.down.circle")
                                .font(.system(size: 20, weight: .medium))
                                .foregroundColor(.white)
                                .padding(12)
                                .background(Color.black.opacity(0.5))
                                .clipShape(Circle())
                        }
                    } else {
                        Color.clear
                            .frame(width: 44, height: 44)
                    }
                }
                .padding(.horizontal)
                .padding(.top, 8)

                Spacer()
            }

            // Zoom indicator
            if scale > 1 {
                VStack {
                    Spacer()

                    Text("\(Int(scale * 100))%")
                        .font(.subheadline)
                        .foregroundColor(.white)
                        .padding(.horizontal, 16)
                        .padding(.vertical, 8)
                        .background(Color.black.opacity(0.7))
                        .cornerRadius(8)
                        .padding(.bottom, 80)
                }
            }

            // Reset zoom button
            if scale > 1 {
                VStack {
                    Spacer()

                    HStack {
                        Spacer()

                        Button(action: {
                            withAnimation {
                                scale = 1
                                offset = .zero
                                lastOffset = .zero
                            }
                        }) {
                            Image(systemName: "arrow.up.left.and.arrow.down.right")
                                .font(.system(size: 20))
                                .foregroundColor(.white)
                                .padding(16)
                                .background(Color.blue)
                                .clipShape(Circle())
                                .shadow(radius: 4)
                        }
                        .padding(.trailing)
                        .padding(.bottom)
                    }
                }
            }
        }
    }
}

/// Simple image preview without zoom (for quick view)
struct SimpleImagePreview: View {
    let imageUrl: String
    let onDismiss: () -> Void

    var body: some View {
        ZStack {
            Color.black.ignoresSafeArea()

            AsyncImage(url: URL(string: imageUrl)) { phase in
                switch phase {
                case .empty:
                    ProgressView()
                        .progressViewStyle(CircularProgressViewStyle(tint: .white))

                case .success(let image):
                    image
                        .resizable()
                        .aspectRatio(contentMode: .fit)

                case .failure:
                    Image(systemName: "photo.fill")
                        .font(.largeTitle)
                        .foregroundColor(.gray)

                @unknown default:
                    EmptyView()
                }
            }

            VStack {
                HStack {
                    Button(action: onDismiss) {
                        Image(systemName: "xmark")
                            .font(.system(size: 20, weight: .medium))
                            .foregroundColor(.white)
                            .padding(12)
                            .background(Color.black.opacity(0.5))
                            .clipShape(Circle())
                    }
                    .padding()

                    Spacer()
                }

                Spacer()
            }
        }
    }
}

// MARK: - Preview

struct ImagePreviewView_Previews: PreviewProvider {
    static var previews: some View {
        ImagePreviewView(
            imageUrl: "https://picsum.photos/800/1200",
            fileName: "sample_image.jpg",
            onDismiss: {},
            onDownload: {}
        )
    }
}
