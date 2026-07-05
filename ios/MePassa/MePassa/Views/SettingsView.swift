//
//  SettingsView.swift
//  MePassa
//
//  Created by MePassa Team
//  Copyright © 2026 MePassa. All rights reserved.
//

import SwiftUI

/// SettingsView - App settings screen
struct SettingsView: View {
    @EnvironmentObject var appState: AppState
    @State private var notificationsEnabled = true
    @State private var soundEnabled = true
    @State private var vibrationEnabled = true
    @State private var readReceiptsEnabled = true
    @State private var lastSeenEnabled = true
    @State private var showLogoutAlert = false
    @State private var showExportSheet = false
    @State private var exportData = ""
    @State private var showExportError = false
    @State private var exportErrorMessage = ""
    @State private var showPrekeyImportSheet = false
    @State private var prekeyPeerId = ""
    @State private var prekeyJson = ""
    @State private var storageUsed = "calculando..."

    private func directorySize(_ url: URL) -> Int64 {
        let fm = FileManager.default
        guard let enumerator = fm.enumerator(at: url, includingPropertiesForKeys: [.fileSizeKey]) else {
            return 0
        }
        var total: Int64 = 0
        for case let fileURL as URL in enumerator {
            total += Int64((try? fileURL.resourceValues(forKeys: [.fileSizeKey]).fileSize) ?? 0)
        }
        return total
    }

    private func refreshStorageUsage() {
        DispatchQueue.global(qos: .utility).async {
            let fm = FileManager.default
            var total: Int64 = 0
            if let docs = fm.urls(for: .documentDirectory, in: .userDomainMask).first {
                total += directorySize(docs)
            }
            if let caches = fm.urls(for: .cachesDirectory, in: .userDomainMask).first {
                total += directorySize(caches)
            }
            let formatted = String(format: "%.1f MB", Double(total) / (1024.0 * 1024.0))
            DispatchQueue.main.async { storageUsed = formatted }
        }
    }

    private func clearCaches() {
        DispatchQueue.global(qos: .utility).async {
            let fm = FileManager.default
            if let caches = fm.urls(for: .cachesDirectory, in: .userDomainMask).first,
               let contents = try? fm.contentsOfDirectory(at: caches, includingPropertiesForKeys: nil) {
                contents.forEach { try? fm.removeItem(at: $0) }
            }
            let tmp = fm.temporaryDirectory
            if let contents = try? fm.contentsOfDirectory(at: tmp, includingPropertiesForKeys: nil) {
                contents.forEach { try? fm.removeItem(at: $0) }
            }
            refreshStorageUsage()
        }
    }

    var body: some View {
        Form {
            // Notifications section
            Section("Notificações") {
                Toggle("Ativar notificações", isOn: $notificationsEnabled)
                    .accessibilityIdentifier("settings_toggle_notifications")

                Toggle("Som", isOn: $soundEnabled)
                    .accessibilityIdentifier("settings_toggle_sound")
                    .disabled(!notificationsEnabled)

                Toggle("Vibração", isOn: $vibrationEnabled)
                    .accessibilityIdentifier("settings_toggle_vibration")
                    .disabled(!notificationsEnabled)
            }

            // Privacy section
            Section("Privacidade") {
                Toggle("Confirmações de leitura", isOn: $readReceiptsEnabled)
                    .accessibilityIdentifier("settings_toggle_read_receipts")

                Toggle("Última visualização", isOn: $lastSeenEnabled)
                    .accessibilityIdentifier("settings_toggle_last_seen")
            }

            // Identity section
            Section("Identidade") {
                Button("Exportar backup da identidade") {
                    Task {
                        do {
                            exportData = try await MePassaCore.shared.exportIdentity()
                            showExportSheet = true
                        } catch {
                            exportErrorMessage = error.localizedDescription
                            showExportError = true
                        }
                    }
                }
                .accessibilityIdentifier("settings_export_backup")

                Button("Exportar prekeys") {
                    Task {
                        do {
                            exportData = try await MePassaCore.shared.exportPrekeyBundle()
                            showExportSheet = true
                        } catch {
                            exportErrorMessage = error.localizedDescription
                            showExportError = true
                        }
                    }
                }

                Button("Importar prekeys de contato") {
                    showPrekeyImportSheet = true
                }
            }

            // Storage section
            Section("Armazenamento") {
                HStack {
                    Text("Armazenamento usado")
                    Spacer()
                    Text(storageUsed)
                        .foregroundColor(.secondary)
                }

                Button("Limpar caches") {
                    clearCaches()
                }
            }

            // About section
            Section("Sobre") {
                HStack {
                    Text("Versão")
                    Spacer()
                    Text(Bundle.main.infoDictionary?["CFBundleShortVersionString"] as? String ?? "?")
                        .foregroundColor(.secondary)
                }

            }

            // Logout section
            Section {
                Button("Sair", role: .destructive) {
                    showLogoutAlert = true
                }
                .accessibilityIdentifier("settings_logout")
            }
        }
        .navigationTitle("Configurações")
        .navigationBarTitleDisplayMode(.inline)
        .onAppear { refreshStorageUsage() }
        .alert("Sair", isPresented: $showLogoutAlert) {
            Button("Cancelar", role: .cancel) { }
            Button("Apagar e sair", role: .destructive) {
                // Logout destrutivo: apaga a identidade do Keychain e o estado
                // local. Sem backup exportado o peer ID é perdido.
                do {
                    try KeychainStore.deleteIdentity()
                } catch {
                    print("⚠️ Failed to delete identity from keychain: \(error)")
                }
                UserDefaults.standard.removeObject(forKey: "local_peer_id")
                appState.logout()
            }
        } message: {
            Text("Isso apaga sua identidade deste dispositivo. Sem um backup exportado, você perderá o acesso a este peer ID permanentemente. Continuar?")
        }
        .alert("Erro", isPresented: $showExportError) {
            Button("OK", role: .cancel) { }
        } message: {
            Text(exportErrorMessage)
        }
        .sheet(isPresented: $showExportSheet) {
            NavigationView {
                VStack(spacing: 16) {
                    Text("Backup da identidade (Base64)")
                        .font(.headline)

                    TextEditor(text: $exportData)
                        .font(.system(.body, design: .monospaced))
                        .frame(minHeight: 220)
                        .overlay(
                            RoundedRectangle(cornerRadius: 8)
                                .stroke(Color.secondary.opacity(0.4))
                        )
                        .padding(.horizontal)

                    Button(action: {
                        UIPasteboard.general.string = exportData
                    }) {
                        Text("Copiar")
                            .fontWeight(.semibold)
                            .frame(maxWidth: .infinity)
                            .padding()
                            .background(Color.blue)
                            .foregroundColor(.white)
                            .cornerRadius(12)
                    }
                    .padding(.horizontal)

                    Spacer()
                }
                .padding(.top, 12)
                .navigationTitle("Exportar Identidade")
                .navigationBarTitleDisplayMode(.inline)
                .toolbar {
                    ToolbarItem(placement: .cancellationAction) {
                        Button("Fechar") { showExportSheet = false }
                    }
                }
            }
        }
        .sheet(isPresented: $showPrekeyImportSheet) {
            NavigationView {
                VStack(spacing: 16) {
                    Text("Salvar prekeys do contato")
                        .font(.headline)

                    TextField("Peer ID", text: $prekeyPeerId)
                        .textFieldStyle(.roundedBorder)
                        .padding(.horizontal)

                    TextEditor(text: $prekeyJson)
                        .font(.system(.body, design: .monospaced))
                        .frame(minHeight: 200)
                        .overlay(
                            RoundedRectangle(cornerRadius: 8)
                                .stroke(Color.secondary.opacity(0.4))
                        )
                        .padding(.horizontal)

                    Button(action: {
                        do {
                            try MePassaCore.shared.storePeerPrekeyBundle(
                                peerId: prekeyPeerId,
                                bundleJson: prekeyJson
                            )
                            prekeyPeerId = ""
                            prekeyJson = ""
                            showPrekeyImportSheet = false
                        } catch {
                            exportErrorMessage = error.localizedDescription
                            showExportError = true
                        }
                    }) {
                        Text("Salvar")
                            .fontWeight(.semibold)
                            .frame(maxWidth: .infinity)
                            .padding()
                            .background(Color.blue)
                            .foregroundColor(.white)
                            .cornerRadius(12)
                    }
                    .padding(.horizontal)
                    .disabled(prekeyPeerId.trimmingCharacters(in: .whitespacesAndNewlines).isEmpty || prekeyJson.trimmingCharacters(in: .whitespacesAndNewlines).isEmpty)

                    Spacer()
                }
                .padding(.top, 12)
                .navigationTitle("Importar Prekeys")
                .navigationBarTitleDisplayMode(.inline)
                .toolbar {
                    ToolbarItem(placement: .cancellationAction) {
                        Button("Fechar") { showPrekeyImportSheet = false }
                    }
                }
            }
        }
    }
}

#Preview {
    NavigationView {
        SettingsView()
    }
}
