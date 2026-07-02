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
                    VStack(spacing: 20) {
                        Image(systemName: "person.3")
                            .font(.system(size: 60))
                            .foregroundColor(.secondary)

                        Text("Nenhum grupo ainda")
                            .font(.headline)
                            .foregroundColor(.secondary)

                        Text("Crie ou entre em um grupo para começar")
                            .font(.subheadline)
                            .foregroundColor(.secondary)
                            .multilineTextAlignment(.center)
                            .padding(.horizontal, 40)
                    }
                } else {
                    // Groups list
                    List {
                        ForEach(appState.groups) { group in
                            NavigationLink(destination: GroupChatView(group: group)) {
                                GroupRow(group: group)
                            }
                        }
                    }
                    .listStyle(.plain)
                }
            }
            .navigationTitle("Grupos")
            .toolbar {
                ToolbarItem(placement: .navigationBarTrailing) {
                    Button(action: { showingCreateGroup = true }) {
                        Image(systemName: "plus")
                    }
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
        HStack(alignment: .top, spacing: 12) {
            // Group icon
            Circle()
                .fill(Color.blue)
                .frame(width: 50, height: 50)
                .overlay(
                    Image(systemName: "person.3.fill")
                        .font(.title3)
                        .foregroundColor(.white)
                )

            // Content
            VStack(alignment: .leading, spacing: 4) {
                HStack {
                    Text(group.name)
                        .font(.headline)

                    Spacer()

                    // Admin badge
                    if group.isAdmin {
                        Text("Admin")
                            .font(.caption2)
                            .fontWeight(.semibold)
                            .padding(.horizontal, 8)
                            .padding(.vertical, 2)
                            .background(Color.blue.opacity(0.2))
                            .foregroundColor(.blue)
                            .cornerRadius(8)
                    }
                }

                // Description
                if let description = group.description {
                    Text(description)
                        .font(.subheadline)
                        .foregroundColor(.secondary)
                        .lineLimit(1)
                }

                // Member count
                Text("\(group.memberCount) \(group.memberCount == 1 ? "membro" : "membros")")
                    .font(.caption)
                    .foregroundColor(.secondary)
            }
        }
        .padding(.vertical, 4)
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
