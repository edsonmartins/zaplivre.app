//
//  GroupListView.swift
//  MePassa
//
//  Created by MePassa Team
//  Copyright © 2026 MePassa. All rights reserved.
//

import SwiftUI

struct GroupListView: View {
    @EnvironmentObject var appState: AppState
    @State private var showingCreateGroup = false
    @State private var isLoading = false
    @State private var errorMessage: String?

    var body: some View {
        NavigationView {
            Group {
                if isLoading {
                    // Loading state
                    VStack(spacing: 20) {
                        ProgressView()
                        Text("Carregando grupos...")
                            .font(.subheadline)
                            .foregroundColor(.secondary)
                    }
                } else if let error = errorMessage {
                    // Error state
                    VStack(spacing: 20) {
                        Image(systemName: "exclamationmark.triangle")
                            .font(.system(size: 60))
                            .foregroundColor(.red)

                        Text("Erro ao carregar grupos")
                            .font(.headline)

                        Text(error)
                            .font(.subheadline)
                            .foregroundColor(.secondary)
                            .multilineTextAlignment(.center)
                            .padding(.horizontal, 40)

                        Button("Tentar novamente") {
                            Task {
                                await loadGroups()
                            }
                        }
                        .buttonStyle(.borderedProminent)
                    }
                } else if appState.groups.isEmpty {
                    // Empty state
                    VStack(spacing: 16) {
                        ZStack {
                            Circle().fill(ZapColor.primary.opacity(0.12)).frame(width: 96, height: 96)
                            Image(systemName: "person.3.fill")
                                .font(.system(size: 38))
                                .foregroundStyle(ZapColor.sparkGradient)
                        }
                        Text("Nenhum grupo ainda")
                            .font(ZapFont.title)
                            .foregroundColor(ZapColor.ink)
                        Text("Crie um grupo para conversar com várias pessoas de uma vez.")
                            .font(ZapFont.preview)
                            .foregroundColor(ZapColor.slate)
                            .multilineTextAlignment(.center)
                            .padding(.horizontal, 48)
                    }
                    .frame(maxWidth: .infinity, maxHeight: .infinity)
                    .background(ZapColor.canvas)
                } else {
                    // Groups list
                    List {
                        ForEach(appState.groups) { group in
                            ZStack {
                                NavigationLink(destination: GroupChatView(group: group)) {
                                    EmptyView()
                                }
                                .opacity(0)
                                GroupRow(group: group)
                            }
                            .listRowInsets(EdgeInsets(top: 2, leading: ZapMetric.gutter,
                                                      bottom: 2, trailing: ZapMetric.gutter))
                            .listRowBackground(ZapColor.canvas)
                        }
                    }
                    .listStyle(.plain)
                    .background(ZapColor.canvas)
                }
            }
            .navigationTitle("Grupos")
            .toolbar {
                ToolbarItem(placement: .navigationBarTrailing) {
                    Button(action: { showingCreateGroup = true }) {
                        Image(systemName: "plus")
                    }
                    .accessibilityIdentifier("grouplist_fab")
                }
            }
            .sheet(isPresented: $showingCreateGroup) {
                CreateGroupView()
            }
            .task {
                await loadGroups()
            }
        }
    }

    private func loadGroups() async {
        isLoading = true
        errorMessage = nil

        do {
            let groups = try await MePassaCore.shared.getGroups()
            appState.groups = groups.map {
                ChatGroup(
                    id: $0.id,
                    name: $0.name,
                    description: $0.description,
                    memberCount: $0.memberCount,
                    isAdmin: $0.isAdmin,
                    createdAt: $0.createdAt
                )
            }
        } catch {
            errorMessage = error.localizedDescription
        }

        isLoading = false
    }
}

struct GroupRow: View {
    let group: ChatGroup

    var body: some View {
        HStack(spacing: ZapMetric.rowGap) {
            // Group icon — gradiente spark diferencia grupo de contato
            ZStack {
                Circle().fill(ZapColor.sparkGradient)
                Image(systemName: "person.3.fill")
                    .font(.system(size: 20, weight: .semibold))
                    .foregroundColor(.white)
            }
            .frame(width: ZapMetric.avatar, height: ZapMetric.avatar)

            VStack(alignment: .leading, spacing: 3) {
                HStack(spacing: 8) {
                    Text(group.name)
                        .font(ZapFont.rowName)
                        .foregroundColor(ZapColor.ink)
                        .lineLimit(1)

                    if group.isAdmin {
                        Text("Admin")
                            .font(.system(size: 11, weight: .bold, design: .rounded))
                            .padding(.horizontal, 7)
                            .padding(.vertical, 2)
                            .background(ZapColor.primary.opacity(0.12))
                            .foregroundColor(ZapColor.primary)
                            .clipShape(Capsule())
                    }
                    Spacer(minLength: 0)
                }

                Text(group.description?.isEmpty == false
                     ? group.description!
                     : "\(group.memberCount) \(group.memberCount == 1 ? "membro" : "membros")")
                    .font(ZapFont.preview)
                    .foregroundColor(ZapColor.slate)
                    .lineLimit(1)
            }
        }
        .padding(.vertical, 6)
    }
}

struct CreateGroupView: View {
    @Environment(\.dismiss) var dismiss
    @EnvironmentObject var appState: AppState

    @State private var groupName = ""
    @State private var groupDescription = ""
    @State private var isCreating = false
    @State private var errorMessage: String?

    var body: some View {
        NavigationView {
            Form {
                Section {
                    TextField("Nome do grupo", text: $groupName)
                        .accessibilityIdentifier("grouplist_create_name_input")
                        .autocapitalization(.words)

                    if #available(iOS 16.0, *) {
                        TextField("Descrição (opcional)", text: $groupDescription, axis: .vertical)
                            .lineLimit(3...6)
                    } else {
                        TextField("Descrição (opcional)", text: $groupDescription)
                            .lineLimit(6)
                    }
                }

                if let error = errorMessage {
                    Section {
                        Text(error)
                            .foregroundColor(.red)
                            .font(.caption)
                    }
                }
            }
            .navigationTitle("Criar Grupo")
            .navigationBarTitleDisplayMode(.inline)
            .navigationBarItems(
                leading: Button("Cancelar") {
                    dismiss()
                }
                .disabled(isCreating),
                trailing: Button("Criar") {
                    Task {
                        await createGroup()
                    }
                }
                .accessibilityIdentifier("grouplist_create_confirm")
                .disabled(groupName.trimmingCharacters(in: .whitespacesAndNewlines).isEmpty || isCreating)
            )
        }
    }

    private func createGroup() async {
        isCreating = true
        errorMessage = nil

        do {
            let trimmedName = groupName.trimmingCharacters(in: .whitespacesAndNewlines)
            let trimmedDescription = groupDescription.trimmingCharacters(in: .whitespacesAndNewlines)
            let group = try await MePassaCore.shared.createGroup(
                name: trimmedName,
                description: trimmedDescription.isEmpty ? nil : trimmedDescription
            )

            appState.groups.append(ChatGroup(
                id: group.id,
                name: group.name,
                description: group.description,
                memberCount: group.memberCount,
                isAdmin: group.isAdmin,
                createdAt: group.createdAt
            ))

            dismiss()
        } catch {
            errorMessage = "Erro ao criar grupo: \(error.localizedDescription)"
        }

        isCreating = false
    }
}

#Preview {
    GroupListView()
        .environmentObject(AppState())
}
