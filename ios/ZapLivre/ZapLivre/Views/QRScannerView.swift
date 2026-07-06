//
//  QRScannerView.swift
//  ZapLivre
//
//  Created by ZapLivre Team
//  Copyright © 2026 ZapLivre. All rights reserved.
//
//  SwiftUI wrapper for QRScannerViewController

import SwiftUI
import AVFoundation

struct QRScannerView: View {
    @Environment(\.dismiss) var dismiss
    let onScan: (String) -> Void

    var body: some View {
        QRScannerRepresentable(onScan: onScan, onCancel: {
            dismiss()
        })
        .ignoresSafeArea()
    }
}

// MARK: - UIViewControllerRepresentable
struct QRScannerRepresentable: UIViewControllerRepresentable {
    let onScan: (String) -> Void
    let onCancel: () -> Void

    func makeUIViewController(context: Context) -> QRScannerViewController {
        let scanner = QRScannerViewController()
        scanner.delegate = context.coordinator
        return scanner
    }

    func updateUIViewController(_ uiViewController: QRScannerViewController, context: Context) {
        // No updates needed
    }

    func makeCoordinator() -> Coordinator {
        Coordinator(onScan: onScan, onCancel: onCancel)
    }

    class Coordinator: NSObject, QRScannerDelegate {
        let onScan: (String) -> Void
        let onCancel: () -> Void

        init(onScan: @escaping (String) -> Void, onCancel: @escaping () -> Void) {
            self.onScan = onScan
            self.onCancel = onCancel
        }

        func qrScanner(_ scanner: QRScannerViewController, didScanCode code: String) {
            onScan(code)
        }

        func qrScannerDidCancel(_ scanner: QRScannerViewController) {
            onCancel()
        }
    }
}

#Preview {
    QRScannerView { peerId in
        print("Scanned: \(peerId)")
    }
}
