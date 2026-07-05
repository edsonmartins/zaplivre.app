//
//  GroupChatView.swift
//  MePassa
//
//  Created by MePassa Team
//  Copyright © 2026 MePassa. All rights reserved.
//

import SwiftUI

struct GroupChatView: View {
    let group: ChatGroup

    @State private var messages: [GroupMessage] = []
    @State private var messageText = ""
    @State private var isSending = false
    @State private var showingGroupInfo = false
    @State private var isLoading = true

    var body: some View {
        VStack(spacing: 0) {
            if isLoading {
                Spacer()
                ProgressView()
                Text("Carregando mensagens...")
                    .font(.subheadline)
                    .foregroundColor(.secondary)
                    .padding(.top, 8)
                Spacer()
            } else if messages.isEmpty {
                Spacer()
                VStack(spacing: 16) {
                    Image(systemName: "message")
                        .font(.system(size: 60))
                        .foregroundColor(.secondary)

                    Text("Nenhuma mensagem ainda")
                        .font(.headline)
                        .foregroundColor(.secondary)

                    Text("Envie a primeira mensagem!")
                        .font(.subheadline)
                        .foregroundColor(.secondary)
                }
                Spacer()
            } else {
                ScrollViewReader { proxy in
                    ScrollView {
                        LazyVStack(spacing: 12) {
                            ForEach(messages) { message in
                                GroupMessageBubble(message: message)
                                    .id(message.id)
                            }
                        }
                        .padding()
                    }
                    .onChange(of: messages.count) { _ in
                        if let lastMessage = messages.last {
                            withAnimation {
                                proxy.scrollTo(lastMessage.id, anchor: .bottom)
                            }
                        }
                    }
                }
            }

            // Message input bar
            HStack(spacing: 12) {
                if #available(iOS 16.0, *) {
                    TextField("Mensagem", text: $messageText, axis: .vertical)
                        .accessibilityIdentifier("groupchat_input")
                        .textFieldStyle(.roundedBorder)
                        .lineLimit(1...4)
                        .disabled(isSending)
                } else {
                    TextField("Mensagem", text: $messageText)
                        .accessibilityIdentifier("groupchat_input")
                        .textFieldStyle(.roundedBorder)
                        .lineLimit(4)
                        .disabled(isSending)
                }

                Button(action: sendMessage) {
                    if isSending {
                        ProgressView()
                            .frame(width: 24, height: 24)
                    } else {
                        Image(systemName: "arrow.up.circle.fill")
                            .font(.system(size: 32))
                            .foregroundColor(messageText.trimmingCharacters(in: .whitespacesAndNewlines).isEmpty ? .gray : .blue)
                    }
                }
                .accessibilityIdentifier("groupchat_send")
                .disabled(messageText.trimmingCharacters(in: .whitespacesAndNewlines).isEmpty || isSending)
            }
            .padding()
            .background(Color(uiColor: .systemBackground))
        }
        .navigationTitle(group.name)
        .navigationBarTitleDisplayMode(.inline)
        .navigationBarItems(trailing:
            Button(action: { showingGroupInfo = true }) {
                Image(systemName: "info.circle")
            }
            .accessibilityIdentifier("groupchat_info")
        )
        .sheet(isPresented: $showingGroupInfo) {
            GroupInfoView(group: group)
        }
        .task {
            await loadMessages()
        }
    }

    private func loadMessages() async {
        isLoading = true

        do {
            let fetchedMessages = try await MePassaCore.shared.getGroupMessages(
                groupId: group.id
            )

            let localPeerId = MePassaCore.shared.localPeerId ?? ""
            messages = fetchedMessages.map {
                GroupMessage(
                    id: $0.id,
                    groupId: group.id,
                    senderPeerId: $0.senderPeerId,
                    senderName: $0.senderPeerId.prefix(8).description,
                    content: $0.content ?? "",
                    timestamp: $0.createdAt,
                    isOwnMessage: $0.senderPeerId == localPeerId
                )
            }
        } catch {
            print("❌ Error loading messages: \(error)")
        }

        isLoading = false
    }

    private func sendMessage() {
        let content = messageText.trimmingCharacters(in: .whitespacesAndNewlines)
        guard !content.isEmpty else { return }

        messageText = ""
        isSending = true

        Task {
            do {
                _ = try await MePassaCore.shared.sendGroupMessage(
                    groupId: group.id,
                    content: content
                )

                // Reload messages
                await loadMessages()
            } catch {
                print("❌ Error sending message: \(error)")
            }

            isSending = false
        }
    }
}

struct GroupMessageBubble: View {
    let message: GroupMessage

    var body: some View {
        HStack {
            if message.isOwnMessage {
                Spacer(minLength: 60)
            }

            VStack(alignment: message.isOwnMessage ? .trailing : .leading, spacing: 4) {
                // Sender name (only for other people's messages)
                if !message.isOwnMessage {
                    Text(message.senderName)
                        .font(.caption)
                        .fontWeight(.semibold)
                        .foregroundColor(.blue)
                }

                // Message bubble
                Text(message.content)
                    .padding(12)
                    .background(message.isOwnMessage ? Color.blue : Color(uiColor: .systemGray5))
                    .foregroundColor(message.isOwnMessage ? .white : .primary)
                    .cornerRadius(16)

                // Timestamp
                Text(formatTimestamp(message.timestamp))
                    .font(.caption2)
                    .foregroundColor(.secondary)
            }

            if !message.isOwnMessage {
                Spacer(minLength: 60)
            }
        }
    }

    private func formatTimestamp(_ date: Date) -> String {
        let formatter = DateFormatter()
        formatter.dateFormat = "HH:mm"
        return formatter.string(from: date)
    }
}

// MARK: - Models

struct GroupMessage: Identifiable {
    let id: String
    let groupId: String
    let senderPeerId: String
    let senderName: String
    let content: String
    let timestamp: Date
    let isOwnMessage: Bool
}

#Preview {
    NavigationView {
        GroupChatView(group: ChatGroup(
            id: "1",
            name: "Amigos da Faculdade",
            description: "Grupo de estudos",
            memberCount: 5,
            isAdmin: true,
            createdAt: Date()
        ))
    }
}
