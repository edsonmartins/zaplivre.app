//
//  DocumentPicker.swift
//  ZapLivre
//
//  Created by ZapLivre Team
//  Copyright © 2026 ZapLivre. All rights reserved.
//

import SwiftUI
import UniformTypeIdentifiers

/// DocumentPicker - Allows user to select files from device storage
///
/// Uses UIDocumentPickerViewController for file selection.
/// Supports all document types.
struct DocumentPicker: UIViewControllerRepresentable {
    @Environment(\.dismiss) var dismiss
    let onDocumentPicked: (URL) -> Void

    func makeUIViewController(context: Context) -> UIDocumentPickerViewController {
        // Create document picker for all file types
        let picker = UIDocumentPickerViewController(forOpeningContentTypes: [.item], asCopy: true)
        picker.delegate = context.coordinator
        picker.allowsMultipleSelection = false
        return picker
    }

    func updateUIViewController(_ uiViewController: UIDocumentPickerViewController, context: Context) {
        // No updates needed
    }

    func makeCoordinator() -> Coordinator {
        Coordinator(self)
    }

    class Coordinator: NSObject, UIDocumentPickerDelegate {
        let parent: DocumentPicker

        init(_ parent: DocumentPicker) {
            self.parent = parent
        }

        func documentPicker(_ controller: UIDocumentPickerViewController, didPickDocumentsAt urls: [URL]) {
            guard let url = urls.first else { return }

            // Start accessing security-scoped resource
            guard url.startAccessingSecurityScopedResource() else {
                print("❌ Failed to access security-scoped resource")
                parent.dismiss()
                return
            }

            defer {
                url.stopAccessingSecurityScopedResource()
            }

            // Notify parent
            parent.onDocumentPicked(url)
            parent.dismiss()
        }

        func documentPickerWasCancelled(_ controller: UIDocumentPickerViewController) {
            parent.dismiss()
        }
    }
}

/// DocumentPickerButton - Button that shows document picker
struct DocumentPickerButton: View {
    @State private var showingPicker = false
    let onDocumentPicked: (URL) -> Void
    let isEnabled: Bool

    init(isEnabled: Bool = true, onDocumentPicked: @escaping (URL) -> Void) {
        self.isEnabled = isEnabled
        self.onDocumentPicked = onDocumentPicked
    }

    var body: some View {
        Button(action: {
            showingPicker = true
        }) {
            Image(systemName: "paperclip")
                .font(.title2)
                .foregroundColor(isEnabled ? .blue : .gray)
        }
        .disabled(!isEnabled)
        .sheet(isPresented: $showingPicker) {
            DocumentPicker(onDocumentPicked: onDocumentPicked)
        }
    }
}
