//
//  ReactionBar.swift
//  ZapLivre
//
//  Created by ZapLivre Team
//  Copyright © 2026 ZapLivre. All rights reserved.
//

import SwiftUI

/// Reaction count model
struct ReactionCount: Identifiable {
    let id = UUID()
    let emoji: String
    let count: Int
    let hasReacted: Bool
}

/// ReactionBar - Displays emoji reactions under a message
///
/// Shows aggregated reactions with counts and highlights user's reactions.
struct ReactionBar: View {
    let reactions: [ReactionCount]
    let onReactionTap: (String) -> Void
    let onAddReactionTap: () -> Void

    var body: some View {
        if !reactions.isEmpty {
            ScrollView(.horizontal, showsIndicators: false) {
                HStack(spacing: 6) {
                    ForEach(reactions) { reaction in
                        ReactionChip(
                            emoji: reaction.emoji,
                            count: reaction.count,
                            isSelected: reaction.hasReacted,
                            onTap: { onReactionTap(reaction.emoji) }
                        )
                    }

                    // Add reaction button
                    Button(action: onAddReactionTap) {
                        Image(systemName: "plus")
                            .font(.system(size: 12, weight: .medium))
                            .foregroundColor(.secondary)
                            .frame(height: 24)
                            .padding(.horizontal, 8)
                            .background(Color.secondary.opacity(0.1))
                            .cornerRadius(12)
                    }
                }
                .padding(.horizontal, 8)
                .padding(.vertical, 4)
            }
        }
    }
}

/// ReactionChip - Individual reaction bubble
struct ReactionChip: View {
    let emoji: String
    let count: Int
    let isSelected: Bool
    let onTap: () -> Void

    var body: some View {
        Button(action: onTap) {
            HStack(spacing: 4) {
                Text(emoji)
                    .font(.system(size: 14))

                Text("\(count)")
                    .font(.system(size: 12, weight: .medium))
                    .foregroundColor(isSelected ? .white : .secondary)
            }
            .padding(.horizontal, 8)
            .frame(height: 24)
            .background(isSelected ? Color.blue : Color.secondary.opacity(0.1))
            .cornerRadius(12)
        }
    }
}

#Preview {
    VStack(spacing: 12) {
        ReactionBar(
            reactions: [
                ReactionCount(emoji: "👍", count: 3, hasReacted: true),
                ReactionCount(emoji: "❤️", count: 2, hasReacted: false),
                ReactionCount(emoji: "😂", count: 1, hasReacted: false)
            ],
            onReactionTap: { emoji in print("Tapped: \(emoji)") },
            onAddReactionTap: { print("Add reaction") }
        )
        .padding()
        .background(Color.gray.opacity(0.1))

        ReactionBar(
            reactions: [],
            onReactionTap: { _ in },
            onAddReactionTap: {}
        )
    }
}
