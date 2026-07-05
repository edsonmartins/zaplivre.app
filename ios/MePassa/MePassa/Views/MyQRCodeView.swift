//
//  MyQRCodeView.swift
//  MePassa
//
//  Created by MePassa Team
//  Copyright © 2026 MePassa. All rights reserved.
//

import SwiftUI
import CoreImage.CIFilterBuiltins

struct MyQRCodeView: View {
    @Environment(\.dismiss) var dismiss
    @EnvironmentObject var appState: AppState

    var body: some View {
        NavigationView {
            VStack(spacing: 30) {
                Text("Meu QR Code")
                    .font(.title2)
                    .fontWeight(.bold)

                // QR Code
                if let peerId = appState.currentUser?.peerId {
                    Image(uiImage: generateQRCode(from: peerId))
                        .interpolation(.none)
                        .resizable()
                        .scaledToFit()
                        .frame(width: 250, height: 250)
                        .padding()
                        .background(Color.white)
                        .cornerRadius(16)
                        .shadow(radius: 10)
                        .accessibilityIdentifier("qr_image")
                } else {
                    Rectangle()
                        .fill(Color.secondary.opacity(0.2))
                        .frame(width: 250, height: 250)
                        .cornerRadius(16)
                }

                // Peer ID
                VStack(spacing: 8) {
                    Text("Peer ID")
                        .font(.caption)
                        .foregroundColor(.secondary)

                    Text(appState.currentUser?.peerId ?? "")
                        .font(.footnote)
                        .padding()
                        .background(Color.secondary.opacity(0.1))
                        .cornerRadius(8)
                }
                .padding(.horizontal)

                // Share button
                Button(action: sharePeerId) {
                    HStack {
                        Image(systemName: "square.and.arrow.up")
                        Text("Compartilhar")
                            .fontWeight(.semibold)
                    }
                    .frame(maxWidth: .infinity)
                    .padding()
                    .background(Color.blue)
                    .foregroundColor(.white)
                    .cornerRadius(12)
                }
                .padding(.horizontal)

                Spacer()
            }
            .padding()
            .navigationTitle("Meu Perfil")
            .navigationBarTitleDisplayMode(.inline)
            .toolbar {
                ToolbarItem(placement: .navigationBarTrailing) {
                    Button("Fechar") {
                        dismiss()
                    }
                    .accessibilityIdentifier("qr_close")
                }
            }
        }
    }

    private func generateQRCode(from string: String) -> UIImage {
        let context = CIContext()
        let filter = CIFilter.qrCodeGenerator()
        filter.message = Data(string.utf8)
        filter.correctionLevel = "M"

        if let outputImage = filter.outputImage {
            let transform = CGAffineTransform(scaleX: 10, y: 10)
            let scaledImage = outputImage.transformed(by: transform)

            if let cgImage = context.createCGImage(scaledImage, from: scaledImage.extent) {
                return UIImage(cgImage: cgImage)
            }
        }

        return UIImage(systemName: "qrcode") ?? UIImage()
    }

    private func sharePeerId() {
        guard let peerId = appState.currentUser?.peerId else { return }

        let activityVC = UIActivityViewController(
            activityItems: ["Meu ZapLivre Peer ID: \(peerId)"],
            applicationActivities: nil
        )

        if let windowScene = UIApplication.shared.connectedScenes.first as? UIWindowScene,
           let rootViewController = windowScene.windows.first?.rootViewController {
            rootViewController.present(activityVC, animated: true)
        }
    }
}

#Preview {
    MyQRCodeView()
        .environmentObject(AppState())
}
