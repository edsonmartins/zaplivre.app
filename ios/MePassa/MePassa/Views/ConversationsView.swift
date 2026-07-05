//
//  ConversationsView.swift
//  MePassa
//
//  Created by MePassa Team
//  Copyright © 2026 MePassa. All rights reserved.
//

import SwiftUI

struct ConversationsView: View {
    @EnvironmentObject var appState: AppState
    @State private var showingNewChat = false
    @State private var showingSettings = false
    @State private var showingGroups = false
    @State private var pushConversation: Conversation?

    var body: some View {
        NavigationView {
            Group {
                if appState.conversations.isEmpty {
                    // Empty state
                    VStack(spacing: 20) {
                        Image(systemName: "bubble.left.and.bubble.right")
                            .font(.system(size: 60))
                            .foregroundColor(.secondary)

                        Text("Nenhuma conversa ainda")
                            .font(.headline)
                            .foregroundColor(.secondary)

                        Text("Toque em + para iniciar uma nova conversa")
                            .font(.subheadline)
                            .foregroundColor(.secondary)
                            .multilineTextAlignment(.center)
                            .padding(.horizontal, 40)
                    }
                } else {
                    // Conversations list
                    List {
                        ForEach(appState.conversations) { conversation in
                            NavigationLink(destination: ChatView(conversation: conversation)) {
                                ConversationRow(conversation: conversation)
                            }
                        }
                    }
                    .listStyle(.plain)
                    .accessibilityIdentifier("conversations_list")
                }
            }
            .background(
                NavigationLink(
                    destination: Group {
                        if let convo = pushConversation {
                            ChatView(conversation: convo)
                        }
                    },
                    isActive: Binding(
                        get: { appState.pendingConversationPeerId != nil },
                        set: { active in
                            if !active {
                                appState.pendingConversationPeerId = nil
                                pushConversation = nil
                            }
                        }
                    )
                ) { EmptyView() }
                .hidden()
            )
            .navigationTitle("Conversas")
            .toolbar {
                ToolbarItem(placement: .navigationBarLeading) {
                    Button(action: { showingSettings = true }) {
                        Image(systemName: "gear")
                    }
                    .accessibilityIdentifier("conversations_settings")
                }

                ToolbarItemGroup(placement: .navigationBarTrailing) {
                    Button(action: { showingGroups = true }) {
                        Image(systemName: "person.3")
                    }
                    .accessibilityIdentifier("conversations_groups")

                    Button(action: { showingNewChat = true }) {
                        Image(systemName: "plus")
                    }
                    .accessibilityIdentifier("conversations_new_chat")
                }
            }
            .sheet(isPresented: $showingNewChat) {
                NewChatView()
            }
            .sheet(isPresented: $showingSettings) {
                SettingsView()
            }
            .sheet(isPresented: $showingGroups) {
                GroupListView()
            }
            .onChange(of: appState.pendingConversationPeerId) { newValue in
                guard let peerId = newValue else { return }
                if let existing = appState.conversations.first(where: { $0.peerId == peerId }) {
                    pushConversation = existing
                } else {
                    let displayName = String(peerId.prefix(12)) + "..."
                    pushConversation = Conversation(
                        id: "1:1:\(peerId)",
                        peerId: peerId,
                        displayName: displayName,
                        lastMessage: nil,
                        unreadCount: 0
                    )
                }
            }
        }
    }
}

struct ConversationRow: View {
    let conversation: Conversation

    var body: some View {
        HStack(alignment: .top, spacing: 12) {
            // Avatar
            Circle()
                .fill(Color.blue)
                .frame(width: 50, height: 50)
                .overlay(
                    Text(conversation.displayName.prefix(1).uppercased())
                        .font(.title3)
                        .fontWeight(.semibold)
                        .foregroundColor(.white)
                )

            // Content
            VStack(alignment: .leading, spacing: 4) {
                HStack {
                    Text(conversation.displayName)
                        .font(.headline)

                    Spacer()

                    // Unread badge
                    if conversation.unreadCount > 0 {
                        Text("\(conversation.unreadCount)")
                            .font(.caption2)
                            .fontWeight(.bold)
                            .foregroundColor(.white)
                            .padding(.horizontal, 8)
                            .padding(.vertical, 4)
                            .background(Color.blue)
                            .clipShape(Capsule())
                    }
                }

                if let lastMessage = conversation.lastMessage {
                    Text(lastMessage)
                        .font(.subheadline)
                        .foregroundColor(.secondary)
                        .lineLimit(2)
                }
            }
        }
        .padding(.vertical, 4)
    }
}

#Preview {
    ConversationsView()
        .environmentObject(AppState())
}
