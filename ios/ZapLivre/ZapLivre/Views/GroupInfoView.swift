//
//  GroupInfoView.swift
//  ZapLivre
//
//  Created by ZapLivre Team
//  Copyright © 2026 ZapLivre. All rights reserved.
//

import SwiftUI

struct GroupInfoView: View {
    @Environment(\.dismiss) var dismiss
    let group: ChatGroup

    @State private var showingLeaveConfirmation = false
    @State private var showingAddMember = false
    @State private var showingEditGroup = false
    @State private var isLoading = false
    @State private var errorMessage: String?

    var body: some View {
        NavigationView {
            List {
                // Group header
                Section {
                    VStack(spacing: 16) {
                        // Group icon
                        Circle()
                            .fill(ZapColor.primary.opacity(0.2))
                            .frame(width: 80, height: 80)
                            .overlay(
                                Image(systemName: "person.3.fill")
                                    .font(.system(size: 40))
                                    .foregroundColor(ZapColor.primary)
                            )

                        // Group name
                        Text(group.name)
                            .font(.title2)
                            .fontWeight(.bold)

                        // Admin badge
                        if group.isAdmin {
                            HStack {
                                Image(systemName: "star.fill")
                                    .font(.caption2)
                                Text("Administrador")
                                    .font(.caption)
                                    .fontWeight(.semibold)
                            }
                            .padding(.horizontal, 12)
                            .padding(.vertical, 4)
                            .background(ZapColor.primary.opacity(0.2))
                            .foregroundColor(ZapColor.primary)
                            .cornerRadius(12)
                        }
                    }
                    .frame(maxWidth: .infinity)
                    .padding(.vertical, 8)
                }
                .listRowBackground(Color.clear)

                // Description
                if let description = group.description {
                    Section("Descrição") {
                        Text(description)
                            .font(.body)
                    }
                }

                // Members
                Section {
                    HStack {
                        Image(systemName: "person.3")
                        Text("\(group.memberCount) \(group.memberCount == 1 ? "Membro" : "Membros")")
                        Spacer()
                    }
                } header: {
                    Text("Membros")
                }

                // Actions section (admin only)
                if group.isAdmin {
                    Section("Ações de Administrador") {
                        Button(action: { showingAddMember = true }) {
                            Label("Adicionar membro", systemImage: "person.badge.plus")
                        }
                        .accessibilityIdentifier("groupinfo_add_member")

                        Button(action: { showingEditGroup = true }) {
                            Label("Editar informações", systemImage: "pencil")
                        }
                    }
                }

                // Leave group
                Section {
                    Button(role: .destructive, action: { showingLeaveConfirmation = true }) {
                        Label("Sair do grupo", systemImage: "rectangle.portrait.and.arrow.right")
                            .foregroundStyle(ZapColor.danger)
                    }
                    .accessibilityIdentifier("groupinfo_leave")
                }

                // Group info
                Section("Informações") {
                    InfoRow(label: "ID do Grupo", value: String(group.id.prefix(16)) + "...")

                    InfoRow(label: "Criado em", value: formatDate(group.createdAt))
                }

                if let error = errorMessage {
                    Section {
                        Text(error)
                            .foregroundColor(.red)
                            .font(.caption)
                    }
                }
            }
            .navigationTitle("Informações do Grupo")
            .navigationBarTitleDisplayMode(.inline)
            .toolbar {
                ToolbarItem(placement: .navigationBarTrailing) {
                    Button("Fechar") {
                        dismiss()
                    }
                }
            }
            .alert("Sair do Grupo", isPresented: $showingLeaveConfirmation) {
                Button("Cancelar", role: .cancel) { }
                Button("Sair", role: .destructive) {
                    Task {
                        await leaveGroup()
                    }
                }
            } message: {
                Text("Tem certeza que deseja sair de \"\(group.name)\"? Você precisará ser adicionado novamente para voltar.")
            }
            .sheet(isPresented: $showingAddMember) {
                AddMemberView(groupId: group.id)
            }
            .sheet(isPresented: $showingEditGroup) {
                EditGroupView(group: group)
            }
        }
    }

    private func leaveGroup() async {
        isLoading = true
        errorMessage = nil

        do {
            try await ZapLivreCore.shared.leaveGroup(groupId: group.id)
            print("👋 Left group: \(group.name)")

            dismiss()
        } catch {
            errorMessage = "Erro ao sair do grupo: \(error.localizedDescription)"
        }

        isLoading = false
    }

    private func formatDate(_ date: Date) -> String {
        let formatter = DateFormatter()
        formatter.dateStyle = .medium
        formatter.timeStyle = .short
        return formatter.string(from: date)
    }
}

struct InfoRow: View {
    let label: String
    let value: String

    var body: some View {
        HStack {
            Text(label)
                .foregroundColor(.secondary)
            Spacer()
            Text(value)
                .foregroundColor(.primary)
        }
        .font(.subheadline)
    }
}

struct AddMemberView: View {
    @Environment(\.dismiss) var dismiss
    let groupId: String

    @State private var peerIdInput = ""
    @State private var isAdding = false
    @State private var errorMessage: String?

    var body: some View {
        NavigationView {
            Form {
                Section {
                    TextField("Peer ID", text: $peerIdInput)
                        .autocapitalization(.none)
                        .autocorrectionDisabled()
                        .textContentType(.none)
                }

                if let error = errorMessage {
                    Section {
                        Text(error)
                            .foregroundColor(.red)
                            .font(.caption)
                    }
                }
            }
            .navigationTitle("Adicionar Membro")
            .navigationBarTitleDisplayMode(.inline)
            .toolbar {
                ToolbarItem(placement: .navigationBarLeading) {
                    Button("Cancelar") {
                        dismiss()
                    }
                    .disabled(isAdding)
                }

                ToolbarItem(placement: .navigationBarTrailing) {
                    Button("Adicionar") {
                        Task {
                            await addMember()
                        }
                    }
                    .disabled(peerIdInput.trimmingCharacters(in: .whitespacesAndNewlines).isEmpty || isAdding)
                }
            }
        }
    }

    private func addMember() async {
        isAdding = true
        errorMessage = nil

        do {
            try await ZapLivreCore.shared.addGroupMember(
                groupId: groupId,
                peerId: peerIdInput.trimmingCharacters(in: .whitespacesAndNewlines)
            )
            let peerId = peerIdInput.trimmingCharacters(in: .whitespacesAndNewlines)
            try await ZapLivreCore.shared.sendGroupSenderKey(groupId: groupId, toPeerId: peerId)

            dismiss()
        } catch {
            errorMessage = "Erro ao adicionar membro: \(error.localizedDescription)"
        }

        isAdding = false
    }
}

struct EditGroupView: View {
    @Environment(\.dismiss) var dismiss
    let group: ChatGroup

    @State private var groupName: String
    @State private var groupDescription: String
    @State private var isSaving = false
    @State private var errorMessage: String?

    init(group: ChatGroup) {
        self.group = group
        _groupName = State(initialValue: group.name)
        _groupDescription = State(initialValue: group.description ?? "")
    }

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
            .navigationTitle("Editar Grupo")
            .navigationBarTitleDisplayMode(.inline)
            .navigationBarItems(
                leading: Button("Cancelar") {
                    dismiss()
                }
                .disabled(isSaving),
                trailing: Button("Salvar") {
                    Task {
                        await saveChanges()
                    }
                }
                .disabled(groupName.trimmingCharacters(in: .whitespacesAndNewlines).isEmpty || isSaving)
            )
        }
    }

    private func saveChanges() async {
        isSaving = true
        errorMessage = nil

        do {
            let trimmedDescription = groupDescription.trimmingCharacters(in: .whitespacesAndNewlines)
            try await ZapLivreCore.shared.updateGroup(
                groupId: group.id,
                name: groupName.trimmingCharacters(in: .whitespacesAndNewlines),
                description: trimmedDescription.isEmpty ? nil : trimmedDescription
            )
            dismiss()
        } catch {
            errorMessage = "Erro ao salvar: \(error.localizedDescription)"
        }

        isSaving = false
    }
}

#Preview {
    GroupInfoView(group: ChatGroup(
        id: "1",
        name: "Amigos da Faculdade",
        description: "Grupo de estudos",
        memberCount: 5,
        isAdmin: true,
        createdAt: Date()
    ))
}
