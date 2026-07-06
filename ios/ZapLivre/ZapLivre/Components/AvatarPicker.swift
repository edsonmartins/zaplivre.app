//
//  AvatarPicker.swift
//  ZapLivre
//
//  Created by ZapLivre Team
//  Copyright © 2026 ZapLivre. All rights reserved.
//

import SwiftUI
import PhotosUI

/// AvatarPicker - Pick avatar from camera or photo library
struct AvatarPickerSheet: View {
    @Environment(\.dismiss) var dismiss
    let onImageSelected: (UIImage) -> Void

    @State private var showImagePicker = false
    @State private var showCamera = false

    var body: some View {
        NavigationView {
            List {
                Button(action: {
                    showCamera = true
                }) {
                    Label("Tirar foto", systemImage: "camera")
                }

                Button(action: {
                    showImagePicker = true
                }) {
                    Label("Escolher da galeria", systemImage: "photo")
                }
            }
            .navigationTitle("Foto de perfil")
            .navigationBarTitleDisplayMode(.inline)
            .toolbar {
                ToolbarItem(placement: .navigationBarTrailing) {
                    Button("Cancelar") {
                        dismiss()
                    }
                }
            }
        }
        .sheet(isPresented: $showCamera) {
            CameraImagePicker(sourceType: .camera) { image in
                onImageSelected(image)
                dismiss()
            }
        }
        .sheet(isPresented: $showImagePicker) {
            CameraImagePicker(sourceType: .photoLibrary) { image in
                onImageSelected(image)
                dismiss()
            }
        }
    }
}

/// CameraImagePicker - UIImagePickerController wrapper for camera/photos
struct CameraImagePicker: UIViewControllerRepresentable {
    let sourceType: UIImagePickerController.SourceType
    let onImagePicked: (UIImage) -> Void

    func makeUIViewController(context: Context) -> UIImagePickerController {
        let picker = UIImagePickerController()
        picker.sourceType = sourceType
        picker.delegate = context.coordinator
        return picker
    }

    func updateUIViewController(_ uiViewController: UIImagePickerController, context: Context) {}

    func makeCoordinator() -> Coordinator {
        Coordinator(self)
    }

    class Coordinator: NSObject, UIImagePickerControllerDelegate, UINavigationControllerDelegate {
        let parent: CameraImagePicker

        init(_ parent: CameraImagePicker) {
            self.parent = parent
        }

        func imagePickerController(_ picker: UIImagePickerController, didFinishPickingMediaWithInfo info: [UIImagePickerController.InfoKey : Any]) {
            if let image = info[.originalImage] as? UIImage {
                parent.onImagePicked(image)
            }
            picker.dismiss(animated: true)
        }

        func imagePickerControllerDidCancel(_ picker: UIImagePickerController) {
            picker.dismiss(animated: true)
        }
    }
}

#Preview {
    AvatarPickerSheet { _ in }
}
