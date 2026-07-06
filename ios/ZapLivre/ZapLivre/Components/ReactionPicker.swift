//
//  ReactionPicker.swift
//  ZapLivre
//
//  Created by ZapLivre Team
//  Copyright © 2026 ZapLivre. All rights reserved.
//

import SwiftUI

/// Common emoji reactions (WhatsApp-style)
let COMMON_REACTIONS = [
    "👍", "❤️", "😂", "😮", "😢", "🙏",
    "🔥", "🎉", "👏", "✅", "❌", "🤔",
    "😊", "😍", "🤩", "😎", "🥳", "😇"
]

/// ReactionPicker - Sheet for selecting emoji reactions
///
/// Displays a grid of common emoji reactions for quick selection.
struct ReactionPicker: View {
    @Environment(\.dismiss) var dismiss
    let onReactionSelected: (String) -> Void

    let columns = Array(repeating: GridItem(.flexible(), spacing: 8), count: 6)

    var body: some View {
        if #available(iOS 16.0, *) {
            NavigationView {
                VStack(spacing: 16) {
                    // Emoji grid
                    LazyVGrid(columns: columns, spacing: 8) {
                        ForEach(COMMON_REACTIONS, id: \.self) { emoji in
                            EmojiButton(emoji: emoji) {
                                onReactionSelected(emoji)
                                dismiss()
            }
                        }
                    }
                    .padding(.horizontal)

                    Spacer()
                }
                .padding(.top, 16)
                .navigationTitle("Reagir à mensagem")
                .navigationBarTitleDisplayMode(.inline)
                .toolbar {
                    ToolbarItem(placement: .navigationBarTrailing) {
                        Button("Fechar") {
                            dismiss()
                        }
                    }
                }
            }
            .presentationDetents([.medium])
        } else {
            NavigationView {
                VStack(spacing: 16) {
                    // Emoji grid
                    LazyVGrid(columns: columns, spacing: 8) {
                        ForEach(COMMON_REACTIONS, id: \.self) { emoji in
                            EmojiButton(emoji: emoji) {
                                onReactionSelected(emoji)
                                dismiss()
            }
                        }
                    }
                    .padding(.horizontal)

                    Spacer()
                }
                .padding(.top, 16)
                .navigationTitle("Reagir à mensagem")
                .navigationBarTitleDisplayMode(.inline)
                .toolbar {
                    ToolbarItem(placement: .navigationBarTrailing) {
                        Button("Fechar") {
                            dismiss()
                        }
                    }
                }
            }
        }
    }
}

/// EmojiButton - Individual emoji button in picker
struct EmojiButton: View {
    let emoji: String
    let onTap: () -> Void

    var body: some View {
        Button(action: onTap) {
            Text(emoji)
                .font(.system(size: 32))
                .frame(width: 48, height: 48)
                .background(Color.secondary.opacity(0.1))
                .cornerRadius(8)
        }
    }
}

#Preview {
    ReactionPicker { emoji in
        print("Selected: \(emoji)")
    }
}
