//
//  NewChatView.swift
//  ZapLivre
//
//  Created by ZapLivre Team
//  Copyright © 2026 ZapLivre. All rights reserved.
//

import SwiftUI

struct NewChatView: View {
    @Environment(\.dismiss) var dismiss
    @EnvironmentObject var appState: AppState
    @State private var peerId = ""
    @State private var multiaddr: String? = nil
    @State private var showingQRScanner = false
    @State private var isStartingChat = false
    @State private var errorMessage: String?

    /// Parse QR data in format "peerId@multiaddr" or just "peerId"
    private func parseQRData(_ data: String) {
        if data.contains("@") {
            let parts = data.split(separator: "@", maxSplits: 1)
            if parts.count == 2 {
                peerId = String(parts[0])
                multiaddr = String(parts[1])
                if let addr = multiaddr {
                    UserDefaults.standard.set(addr, forKey: "zaplivre.multiaddr.\(peerId)")
                }
                print("📱 Parsed QR: peerId=\(peerId), multiaddr=\(multiaddr ?? "nil")")
                return
            }
        }
        // Fallback: just peer ID
        peerId = data
        multiaddr = nil
        print("📱 Parsed QR: peerId=\(peerId) (no address)")
    }

    var body: some View {
        NavigationView {
            VStack(spacing: 26) {
                // QR Scanner option
                Button(action: { showingQRScanner = true }) {
                    VStack(spacing: 12) {
                        Image(systemName: "qrcode.viewfinder")
                            .font(.system(size: 56))
                            .foregroundStyle(ZapColor.sparkGradient)

                        Text("Escanear QR Code")
                            .font(ZapFont.rowName)
                            .foregroundColor(ZapColor.ink)
                        Text("Aponte para o QR de um contato para conectar")
                            .font(ZapFont.caption)
                            .foregroundColor(ZapColor.slate)
                    }
                    .frame(maxWidth: .infinity)
                    .padding(.vertical, 34)
                    .background(ZapColor.surface)
                    .clipShape(RoundedRectangle(cornerRadius: ZapMetric.cardRadius, style: .continuous))
                    .overlay(
                        RoundedRectangle(cornerRadius: ZapMetric.cardRadius, style: .continuous)
                            .stroke(ZapColor.primary.opacity(0.25), style: StrokeStyle(lineWidth: 1.5, dash: [6, 4]))
                    )
                }
                .buttonStyle(.plain)

                // Or divider
                HStack(spacing: 8) {
                    Rectangle().frame(height: 1).foregroundColor(ZapColor.hairline)
                    Text("ou").font(ZapFont.caption).foregroundColor(ZapColor.slate)
                    Rectangle().frame(height: 1).foregroundColor(ZapColor.hairline)
                }

                // Manual peer ID input
                VStack(alignment: .leading, spacing: 12) {
                    Text("Inserir Peer ID manualmente")
                        .font(ZapFont.rowName)
                        .foregroundColor(ZapColor.ink)

                    TextField("12D3KooW...", text: $peerId)
                        .accessibilityIdentifier("new_chat_peer_input")
                        .font(.system(size: 15, design: .monospaced))
                        .padding(.horizontal, 14).padding(.vertical, 12)
                        .background(ZapColor.surface)
                        .clipShape(RoundedRectangle(cornerRadius: 12, style: .continuous))
                        .overlay(RoundedRectangle(cornerRadius: 12, style: .continuous)
                            .stroke(ZapColor.hairline, lineWidth: 1))
                        .autocapitalization(.none)
                        .autocorrectionDisabled()

                    Button(action: startChat) {
                        HStack(spacing: 8) {
                            if isStartingChat {
                                ProgressView()
                                    .progressViewStyle(CircularProgressViewStyle(tint: .white))
                                    .scaleEffect(0.8)
                            }
                            Text(isStartingChat ? "Conectando..." : "Iniciar conversa")
                        }
                    }
                    .buttonStyle(ZapPrimaryButtonStyle(enabled: !(peerId.isEmpty || isStartingChat)))
                    .accessibilityIdentifier("new_chat_confirm")
                    .disabled(peerId.isEmpty || isStartingChat)

                    // Error message
                    if let errorMessage = errorMessage {
                        Text(errorMessage)
                            .font(ZapFont.caption)
                            .foregroundColor(ZapColor.danger)
                            .multilineTextAlignment(.center)
                            .frame(maxWidth: .infinity)
                    }
                }

                Spacer()
            }
            .padding()
            .background(ZapColor.canvas.ignoresSafeArea())
            .navigationTitle("Nova Conversa")
            .navigationBarTitleDisplayMode(.inline)
            .toolbar {
                ToolbarItem(placement: .navigationBarLeading) {
                    Button("Cancelar") {
                        dismiss()
                    }
                }
            }
            .sheet(isPresented: $showingQRScanner) {
                QRScannerView { scannedData in
                    // Parse QR data: format is "peerId@multiaddr" or just "peerId"
                    parseQRData(scannedData)
                    showingQRScanner = false
                    // Automatically start chat after scanning
                    DispatchQueue.main.asyncAfter(deadline: .now() + 0.5) {
                        startChat()
                    }
                }
            }
        }
    }

    private func startChat() {
        guard !peerId.isEmpty else { return }

        // Validate peer ID format (should start with 12D3KooW for libp2p)
        guard peerId.starts(with: "12D3KooW") || peerId.starts(with: "Qm") else {
            errorMessage = "Peer ID inválido. Deve começar com 12D3KooW ou Qm"
            return
        }

        isStartingChat = true
        errorMessage = nil

        Task {
            do {
                // First, connect to the peer if we have an address
                if let addr = multiaddr {
                    print("🔗 Connecting to peer \(peerId) at \(addr)...")
                    UserDefaults.standard.set(addr, forKey: "zaplivre.multiaddr.\(peerId)")
                    try await ZapLivreCore.shared.connectToPeer(peerId: peerId, multiaddr: addr)
                    print("✅ Connected to peer!")

                    // Wait a bit for the connection to stabilize
                    try await Task.sleep(nanoseconds: 500_000_000) // 0.5 seconds
                }

                // Send a test message to establish conversation
                let testMessage = "👋 Olá! Conectado via QR Code"

                try await ZapLivreCore.shared.sendMessage(
                    to: peerId,
                    content: testMessage
                )

                print("✅ Chat initiated with peer: \(peerId)")

                // Navigate to conversations list (it will show the new chat)
                await MainActor.run {
                    isStartingChat = false
                    dismiss()
                }
            } catch {
                print("❌ Failed to start chat: \(error)")
                await MainActor.run {
                    isStartingChat = false
                    errorMessage = "Falha ao iniciar conversa: \(error.localizedDescription)"
                }
            }
        }
    }
}

#Preview {
    NewChatView()
}
