//
//  NewChatView.swift
//  ZapLivre
//
//  Created by ZapLivre Team
//  Copyright © 2026 ZapLivre. All rights reserved.
//

import SwiftUI

struct NewChatView: View {
    private struct ContactQRCode: Decodable {
        let v: Int
        let peerId: String
        let multiaddr: String
        let prekeyBundle: String?
    }

    @Environment(\.dismiss) var dismiss
    @EnvironmentObject var appState: AppState
    @State private var peerId = ""
    @State private var multiaddr: String? = nil
    @State private var showingQRScanner = false
    @State private var isStartingChat = false
    @State private var errorMessage: String?

    /// Formato atual: JSON versionado com material público de contato.
    /// Mantém compatibilidade com os QRs antigos `peerId@multiaddr`.
    @discardableResult
    private func parseQRData(_ data: String) -> Bool {
        if data.first == "{" {
            do {
                let qr = try JSONDecoder().decode(ContactQRCode.self, from: Data(data.utf8))
                guard qr.v == 1,
                      qr.peerId.starts(with: "12D3KooW") || qr.peerId.starts(with: "Qm"),
                      qr.multiaddr.starts(with: "/ip4/") || qr.multiaddr.starts(with: "/ip6/")
                else {
                    throw QRCodeError.invalidPayload
                }

                // O core normaliza a estrutura; a assinatura da signed prekey
                // é verificada pela implementação Signal ao criar a sessão.
                // New invitation QR codes contain only peer/address. Public
                // prekeys are exchanged automatically after authenticated P2P
                // connection; accept the legacy embedded bundle for backward
                // compatibility.
                if let bundle = qr.prekeyBundle {
                    try ZapLivreCore.shared.storePeerPrekeyBundle(
                        peerId: qr.peerId,
                        bundleJson: bundle
                    )
                }
                peerId = qr.peerId
                multiaddr = qr.multiaddr
                UserDefaults.standard.set(qr.multiaddr, forKey: "zaplivre.multiaddr.\(qr.peerId)")
                print("📱 Imported secure QR for peerId=\(peerId), multiaddr=\(qr.multiaddr)")
                return true
            } catch {
                errorMessage = "QR de contato inválido ou com prekeys incompatíveis"
                print("❌ Invalid secure contact QR: \(error)")
                return false
            }
        }

        if data.contains("@") {
            let parts = data.split(separator: "@", maxSplits: 1)
            if parts.count == 2 {
                peerId = String(parts[0])
                multiaddr = String(parts[1])
                if let addr = multiaddr {
                    UserDefaults.standard.set(addr, forKey: "zaplivre.multiaddr.\(peerId)")
                }
                print("📱 Parsed QR: peerId=\(peerId), multiaddr=\(multiaddr ?? "nil")")
                return true
            }
        }
        // Fallback: just peer ID
        peerId = data
        multiaddr = nil
        print("📱 Parsed QR: peerId=\(peerId) (no address)")
        return true
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
                    Text("Inserir @usuário ou Peer ID")
                        .font(ZapFont.rowName)
                        .foregroundColor(ZapColor.ink)

                    TextField("@usuario ou 12D3KooW...", text: $peerId)
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
                    let parsed = parseQRData(scannedData)
                    showingQRScanner = false
                    // Automatically start chat after scanning
                    if parsed {
                        DispatchQueue.main.asyncAfter(deadline: .now() + 0.5) {
                            startChat()
                        }
                    }
                }
            }
        }
    }

    /// An Ed25519 libp2p peer ID: "12D3KooW" + base58, 52 characters in total.
    /// A prefix check alone let malformed IDs through: the conversation opened
    /// and every message failed later with no explanation. Legacy "Qm..." (RSA)
    /// IDs are refused too: they carry no Ed25519 key, which the protocol needs
    /// to verify the contact's prekey bundle.
    static func isValidPeerId(_ value: String) -> Bool {
        let base58 = CharacterSet(charactersIn: "123456789ABCDEFGHJKLMNPQRSTUVWXYZabcdefghijkmnopqrstuvwxyz")
        return value.count == 52
            && value.hasPrefix("12D3KooW")
            && value.unicodeScalars.allSatisfy(base58.contains)
    }

    /// Usernames as the identity server accepts them (`^[a-z0-9_]{3,20}$`).
    static func normalizedUsername(_ input: String) -> String? {
        let name = input.trimmingCharacters(in: .whitespacesAndNewlines)
            .trimmingCharacters(in: CharacterSet(charactersIn: "@"))
            .lowercased()
        let allowed = CharacterSet(charactersIn: "abcdefghijklmnopqrstuvwxyz0123456789_")
        guard (3...20).contains(name.count), name.unicodeScalars.allSatisfy(allowed.contains)
        else { return nil }
        return name
    }

    private func startChat() {
        let input = peerId.trimmingCharacters(in: .whitespacesAndNewlines)
        guard !input.isEmpty else { return }

        // Anything that does not look like a peer ID is a username: the iOS app
        // could only start chats by QR or pasted peer ID, so iOS and Android
        // users could not find each other the same way.
        if !input.hasPrefix("12D3KooW") && !input.hasPrefix("Qm") {
            guard let username = Self.normalizedUsername(input) else {
                errorMessage = "Usuário inválido. Use de 3 a 20 letras minúsculas, números ou _."
                return
            }
            isStartingChat = true
            errorMessage = nil
            Task {
                do {
                    let found = try await ZapLivreCore.shared.lookupUsername(username)
                    await MainActor.run {
                        isStartingChat = false
                        appState.openConversation(peerId: found)
                        dismiss()
                    }
                } catch {
                    await MainActor.run {
                        isStartingChat = false
                        errorMessage = "Usuário @\(username) não encontrado"
                    }
                }
            }
            return
        }

        guard Self.isValidPeerId(peerId) else {
            errorMessage = "Peer ID inválido. Confira o código ou escaneie o QR do contato."
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

                // Scanning a contact must not depend on the offline message store.
                // Open a transient conversation immediately; the first real
                // message or call will establish/dial the P2P connection.
                await MainActor.run {
                    isStartingChat = false
                    appState.openConversation(peerId: peerId)
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

private enum QRCodeError: Error {
    case invalidPayload
}

#Preview {
    NewChatView()
}
