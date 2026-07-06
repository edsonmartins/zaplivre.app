//
//  ImagePicker.swift
//  ZapLivre
//
//  Image picker using PHPickerViewController (iOS 14+)
//

import SwiftUI
import PhotosUI

/// SwiftUI wrapper for PHPickerViewController
struct ImagePicker: UIViewControllerRepresentable {
    @Binding var selectedImages: [UIImage]
    @Environment(\.presentationMode) var presentationMode

    var maxSelection: Int = 10

    func makeUIViewController(context: Context) -> PHPickerViewController {
        var configuration = PHPickerConfiguration()
        configuration.filter = .images
        configuration.selectionLimit = maxSelection
        configuration.preferredAssetRepresentationMode = .current

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
        let parent: ImagePicker

        init(_ parent: ImagePicker) {
            self.parent = parent
        }

        func picker(_ picker: PHPickerViewController, didFinishPicking results: [PHPickerResult]) {
            parent.presentationMode.wrappedValue.dismiss()

            guard !results.isEmpty else { return }

            var images: [UIImage] = []
            let group = DispatchGroup()

            for result in results {
                group.enter()

                result.itemProvider.loadObject(ofClass: UIImage.self) { object, error in
                    defer { group.leave() }

                    if let image = object as? UIImage {
                        images.append(image)
                    } else if let error = error {
                        print("Error loading image: \(error.localizedDescription)")
                    }
                }
            }

            group.notify(queue: .main) {
                self.parent.selectedImages = images
            }
        }
    }
}

/// Button to trigger image picker
struct ImagePickerButton: View {
    @Binding var selectedImages: [UIImage]
    @State private var showingPicker = false

    var maxSelection: Int = 10
    var iconOnly: Bool = true

    var body: some View {
        Button(action: {
            showingPicker = true
        }) {
            if iconOnly {
                Image(systemName: "photo.on.rectangle")
                    .font(.system(size: 22))
                    .foregroundColor(.blue)
            } else {
                Label("Select Images", systemImage: "photo.on.rectangle")
            }
        }
        .sheet(isPresented: $showingPicker) {
            ImagePicker(selectedImages: $selectedImages, maxSelection: maxSelection)
        }
    }
}

/// Inline image picker (for embedding in views)
struct InlineImagePicker: View {
    @Binding var selectedImages: [UIImage]
    @State private var showingPicker = false

    var maxSelection: Int = 10

    var body: some View {
        Button(action: {
            showingPicker = true
        }) {
            HStack {
                Image(systemName: "photo.on.rectangle.angled")
                    .font(.system(size: 20))

                Text("Add Photos")
                    .font(.body)

                if !selectedImages.isEmpty {
                    Spacer()

                    Text("\(selectedImages.count)")
                        .font(.caption)
                        .foregroundColor(.white)
                        .padding(.horizontal, 8)
                        .padding(.vertical, 4)
                        .background(Color.blue)
                        .clipShape(Capsule())
                }
            }
            .padding()
            .frame(maxWidth: .infinity)
            .background(Color.blue.opacity(0.1))
            .cornerRadius(12)
        }
        .sheet(isPresented: $showingPicker) {
            ImagePicker(selectedImages: $selectedImages, maxSelection: maxSelection)
        }
    }
}
