//
//  ConversationsView.swift
//  ZapLivre
//
//  Created by ZapLivre Team
//  Copyright © 2026 ZapLivre. All rights reserved.
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
                    VStack(spacing: 16) {
                        ZStack {
                            Circle()
                                .fill(ZapColor.primary.opacity(0.12))
                                .frame(width: 96, height: 96)
                            Image(systemName: "bubble.left.and.bubble.right.fill")
                                .font(.system(size: 40))
                                .foregroundStyle(ZapColor.sparkGradient)
                        }

                        Text("Nenhuma conversa ainda")
                            .font(ZapFont.title)
                            .foregroundColor(ZapColor.ink)

                        Text("Toque em + para começar a conversar com privacidade total.")
                            .font(ZapFont.preview)
                            .foregroundColor(ZapColor.slate)
                            .multilineTextAlignment(.center)
                            .padding(.horizontal, 48)
                    }
                    .frame(maxWidth: .infinity, maxHeight: .infinity)
                    .background(ZapColor.canvas)
                } else {
                    // Conversations list
                    List {
                        ForEach(appState.conversations) { conversation in
                            ZStack {
                                NavigationLink(destination: ChatView(conversation: conversation)) {
                                    EmptyView()
                                }
                                .opacity(0)
                                ConversationRow(conversation: conversation)
                            }
                            .listRowInsets(EdgeInsets(top: 2, leading: ZapMetric.gutter,
                                                      bottom: 2, trailing: ZapMetric.gutter))
                            .listRowSeparatorTint(ZapColor.hairline)
                            .listRowBackground(ZapColor.canvas)
                        }
                    }
                    .listStyle(.plain)
                    .background(ZapColor.canvas)
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

    private var hasUnread: Bool { conversation.unreadCount > 0 }

    var body: some View {
        HStack(spacing: ZapMetric.rowGap) {
            AvatarView(seed: conversation.peerId ?? conversation.id,
                       name: conversation.displayName)

            VStack(alignment: .leading, spacing: 3) {
                Text(conversation.displayName)
                    .font(ZapFont.rowName)
                    .foregroundColor(ZapColor.ink)
                    .lineLimit(1)

                Text(conversation.lastMessage ?? "Toque para conversar")
                    .font(ZapFont.preview)
                    .foregroundColor(hasUnread ? ZapColor.ink : ZapColor.slate)
                    .fontWeight(hasUnread ? .medium : .regular)
                    .lineLimit(1)
            }

            Spacer(minLength: 8)

            if hasUnread {
                Text("\(conversation.unreadCount)")
                    .font(ZapFont.badge)
                    .foregroundColor(.white)
                    .padding(.horizontal, 7)
                    .frame(minWidth: 22, minHeight: 22)
                    .background(ZapColor.primary)
                    .clipShape(Capsule())
            }
        }
        .padding(.vertical, 6)
    }
}

#Preview {
    ConversationsView()
        .environmentObject(AppState())
}
