//
//  GroupChatView.swift
//  ZapLivre
//
//  Created by ZapLivre Team
//  Copyright © 2026 ZapLivre. All rights reserved.
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
                VStack(spacing: 14) {
                    ZStack {
                        Circle().fill(ZapColor.primary.opacity(0.12)).frame(width: 88, height: 88)
                        Image(systemName: "message.fill")
                            .font(.system(size: 34))
                            .foregroundStyle(ZapColor.sparkGradient)
                    }
                    Text("Nenhuma mensagem ainda")
                        .font(ZapFont.title)
                        .foregroundColor(ZapColor.ink)
                    Text("Envie a primeira mensagem do grupo!")
                        .font(ZapFont.preview)
                        .foregroundColor(ZapColor.slate)
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
            HStack(spacing: 10) {
                Group {
                    if #available(iOS 16.0, *) {
                        TextField("Mensagem", text: $messageText, axis: .vertical)
                            .lineLimit(1...4)
                    } else {
                        TextField("Mensagem", text: $messageText)
                            .lineLimit(4)
                    }
                }
                .accessibilityIdentifier("groupchat_input")
                .font(ZapFont.body)
                .padding(.horizontal, 14).padding(.vertical, 9)
                .background(ZapColor.surface)
                .clipShape(RoundedRectangle(cornerRadius: 22, style: .continuous))
                .overlay(RoundedRectangle(cornerRadius: 22, style: .continuous)
                    .stroke(ZapColor.hairline, lineWidth: 1))
                .disabled(isSending)

                Button(action: sendMessage) {
                    if isSending {
                        ProgressView().tint(.white).frame(width: 38, height: 38)
                            .background(ZapColor.slate).clipShape(Circle())
                    } else {
                        let empty = messageText.trimmingCharacters(in: .whitespacesAndNewlines).isEmpty
                        Image(systemName: "arrow.up")
                            .font(.system(size: 18, weight: .bold))
                            .foregroundColor(.white)
                            .frame(width: 38, height: 38)
                            .background(empty ? AnyShapeStyle(ZapColor.slate.opacity(0.5))
                                              : AnyShapeStyle(ZapColor.sparkGradient))
                            .clipShape(Circle())
                    }
                }
                .accessibilityIdentifier("groupchat_send")
                .disabled(messageText.trimmingCharacters(in: .whitespacesAndNewlines).isEmpty || isSending)
            }
            .padding(.horizontal).padding(.vertical, 8)
            .background(ZapColor.canvas
                .overlay(ZapColor.hairline.frame(height: 0.5), alignment: .top))
        }
        .background(ZapColor.chatCanvas.ignoresSafeArea())
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
            let fetchedMessages = try await ZapLivreCore.shared.getGroupMessages(
                groupId: group.id
            )

            let localPeerId = ZapLivreCore.shared.localPeerId ?? ""
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
                _ = try await ZapLivreCore.shared.sendGroupMessage(
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

            VStack(alignment: message.isOwnMessage ? .trailing : .leading, spacing: 2) {
                // Sender name (only for other people's messages)
                if !message.isOwnMessage {
                    Text(message.senderName)
                        .font(.system(size: 12, weight: .semibold, design: .rounded))
                        .foregroundColor(ZapColor.accent(for: message.senderPeerId))
                        .padding(.leading, 12)
                }

                // Message bubble
                Text(message.content)
                    .font(ZapFont.body)
                    .padding(.horizontal, 12).padding(.vertical, 8)
                    .background(
                        (message.isOwnMessage ? ZapColor.bubbleOut : ZapColor.bubbleIn)
                            .clipShape(BubbleShape(isOutgoing: message.isOwnMessage, hasTail: true))
                    )
                    .foregroundColor(message.isOwnMessage ? ZapColor.bubbleOutInk : ZapColor.bubbleInInk)
                    .overlay(
                        message.isOwnMessage
                            ? nil
                            : BubbleShape(isOutgoing: false, hasTail: true)
                                .stroke(ZapColor.hairline, lineWidth: 0.5)
                    )
                    .shadow(color: .black.opacity(0.05), radius: 1, y: 1)

                // Timestamp
                Text(formatTimestamp(message.timestamp))
                    .font(ZapFont.caption)
                    .foregroundColor(ZapColor.slate)
                    .padding(.horizontal, 4)
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
