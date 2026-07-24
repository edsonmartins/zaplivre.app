//
//  LoginView.swift
//  ZapLivre
//
//  Created by ZapLivre Team
//  Copyright © 2026 ZapLivre. All rights reserved.
//

import SwiftUI

struct LoginView: View {
    @EnvironmentObject var appState: AppState
    @State private var peerId: String = ""
    @State private var isGeneratingId = false
    @State private var showError = false
    @State private var errorMessage = ""
    @State private var showImportSheet = false
    @State private var importText = ""
    @State private var showUsernameSheet = false
    @State private var username = ""
    @State private var usernameError: String?

    var body: some View {
        NavigationView {
            VStack(spacing: 30) {
                // Logo and title
                VStack(spacing: 18) {
                    Image("ZapLogo")
                        .resizable()
                        .scaledToFit()
                        .frame(width: 104, height: 104)
                        .clipShape(RoundedRectangle(cornerRadius: 26, style: .continuous))
                        .shadow(color: ZapColor.primary.opacity(0.35), radius: 22, x: 0, y: 12)

                    VStack(spacing: 8) {
                        Text("ZapLivre")
                            .font(.system(size: 40, weight: .heavy, design: .rounded))
                            .foregroundColor(ZapColor.ink)

                        Text("Privacidade total. Sem servidores centrais.")
                            .font(ZapFont.preview)
                            .foregroundColor(ZapColor.slate)
                            .multilineTextAlignment(.center)
                            .padding(.horizontal)
                    }
                }
                .padding(.top, 72)

                Spacer()

                // Login options
                VStack(spacing: 16) {
                    // Generate new identity
                    Button(action: generateNewIdentity) {
                        Label("Criar nova identidade", systemImage: "person.badge.plus")
                    }
                    .buttonStyle(ZapPrimaryButtonStyle(enabled: !isGeneratingId))
                    .accessibilityIdentifier("onboarding_create")
                    .disabled(isGeneratingId)

                    // Or divider
                    HStack(spacing: 8) {
                        Rectangle().frame(height: 1).foregroundColor(ZapColor.hairline)
                        Text("ou").font(ZapFont.caption).foregroundColor(ZapColor.slate)
                        Rectangle().frame(height: 1).foregroundColor(ZapColor.hairline)
                    }

                    // Import existing identity
                    Button(action: { showImportSheet = true }) {
                        Label("Importar identidade existente", systemImage: "qrcode.viewfinder")
                    }
                    .buttonStyle(ZapSecondaryButtonStyle())
                    .accessibilityIdentifier("onboarding_restore")
                }
                .padding(.horizontal, 30)

                Spacer()

                // Info text
                HStack(spacing: 7) {
                    Image(systemName: "lock.fill").font(.system(size: 11))
                    Text("Sua identidade é criptograficamente segura e não está vinculada a telefone ou email.")
                }
                .font(ZapFont.caption)
                .foregroundColor(ZapColor.slate)
                .multilineTextAlignment(.center)
                .padding(.horizontal, 40)
                .padding(.bottom, 40)
            }
            .frame(maxWidth: .infinity, maxHeight: .infinity)
            .background(ZapColor.canvas.ignoresSafeArea())
            .navigationTitle("")
            .navigationBarHidden(true)
            .alert("Erro", isPresented: $showError) {
                Button("OK", role: .cancel) { }
            } message: {
                Text(errorMessage)
            }
            .sheet(isPresented: $showImportSheet) {
                NavigationView {
                    VStack(spacing: 16) {
                        Text("Cole o backup da identidade (Base64)")
                            .font(.headline)
                            .multilineTextAlignment(.center)
                            .padding(.top, 12)

                        TextEditor(text: $importText)
                            .accessibilityIdentifier("onboarding_peer_id")
                            .font(.system(.body, design: .monospaced))
                            .frame(minHeight: 200)
                            .overlay(
                                RoundedRectangle(cornerRadius: 8)
                                    .stroke(Color.secondary.opacity(0.4))
                            )
                            .padding(.horizontal)

                        Button(action: importIdentity) {
                            Text("Importar identidade")
                        }
                        .buttonStyle(ZapPrimaryButtonStyle(
                            enabled: !importText.trimmingCharacters(in: .whitespacesAndNewlines).isEmpty))
                        .disabled(importText.trimmingCharacters(in: .whitespacesAndNewlines).isEmpty)
                        .padding(.horizontal)

                        Spacer()
                    }
                    .navigationTitle("Importar Identidade")
                    .navigationBarTitleDisplayMode(.inline)
                    .toolbar {
                        ToolbarItem(placement: .cancellationAction) {
                            Button("Fechar") { showImportSheet = false }
                        }
                    }
                }
            }
            .sheet(isPresented: $showUsernameSheet) {
                NavigationView {
                    VStack(spacing: 16) {
                        Text("Escolha seu username")
                            .font(.headline)
                        Text("Use 3 a 20 caracteres: letras minúsculas, números e underscore.")
                            .font(.subheadline)
                            .multilineTextAlignment(.center)
                        TextField("seu_username", text: $username)
                            .textInputAutocapitalization(.never)
                            .autocorrectionDisabled()
                            .textFieldStyle(.roundedBorder)
                        if let usernameError {
                            Text(usernameError)
                                .foregroundColor(.red)
                                .font(.caption)
                        }
                        Button("Registrar") { registerUsername() }
                            .buttonStyle(ZapPrimaryButtonStyle(enabled: !username.isEmpty))
                            .disabled(username.isEmpty)
                        Button("Continuar sem username") {
                            showUsernameSheet = false
                            NotificationCenter.default.post(name: .zapLivreCoreStarted, object: nil)
                        }
                        .buttonStyle(ZapSecondaryButtonStyle())
                        Spacer()
                    }
                    .padding(24)
                    .navigationTitle("Identidade")
                    .navigationBarTitleDisplayMode(.inline)
                }
                .interactiveDismissDisabled()
            }
        }
    }

    private func generateNewIdentity() {
        isGeneratingId = true

        Task {
            do {
                if !ZapLivreCore.shared.isInitialized {
                    try await ZapLivreCore.shared.initialize()
                    try await ZapLivreCore.shared.startListening()
                }

                if let realPeerId = ZapLivreCore.shared.localPeerId, !realPeerId.isEmpty {
                    await MainActor.run {
                        appState.login(peerId: realPeerId)
                        showUsernameSheet = true
                        isGeneratingId = false
                    }
                } else {
                    await MainActor.run {
                        isGeneratingId = false
                        showError = true
                        errorMessage = "Não foi possível obter o Peer ID"
                    }
                }
            } catch {
                await MainActor.run {
                    isGeneratingId = false
                    showError = true
                    errorMessage = "Falha ao inicializar identidade: \(error.localizedDescription)"
                }
            }
        }
    }

    private func importIdentity() {
        isGeneratingId = true

        Task {
            do {
                try await ZapLivreCore.shared.importIdentity(backup: importText)
                showImportSheet = false
                importText = ""

                if !ZapLivreCore.shared.isInitialized {
                    try await ZapLivreCore.shared.initialize()
                    try await ZapLivreCore.shared.startListening()
                }

                if let id = ZapLivreCore.shared.localPeerId {
                    await MainActor.run {
                        appState.login(peerId: id)
                        showUsernameSheet = true
                        isGeneratingId = false
                    }
                }
            } catch {
                await MainActor.run {
                    isGeneratingId = false
                    showError = true
                    errorMessage = error.localizedDescription
                }
            }
        }
    }

    private func registerUsername() {
        let value = username.trimmingCharacters(in: .whitespacesAndNewlines).lowercased()
        guard value.range(of: "^[a-z0-9_]{3,20}$", options: .regularExpression) != nil else {
            usernameError = "Username inválido"
            return
        }
        isGeneratingId = true
        Task {
            do {
                _ = try await ZapLivreCore.shared.registerUsername(value)
                await MainActor.run {
                    isGeneratingId = false
                    showUsernameSheet = false
                    NotificationCenter.default.post(name: .zapLivreCoreStarted, object: nil)
                }
            } catch {
                await MainActor.run {
                    isGeneratingId = false
                    usernameError = error.localizedDescription
                }
            }
        }
    }
}

#Preview {
    LoginView()
        .environmentObject(AppState())
}
