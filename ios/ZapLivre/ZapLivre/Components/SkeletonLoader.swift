//
//  SkeletonLoader.swift
//  ZapLivre
//
//  Created by ZapLivre Team
//  Copyright © 2026 ZapLivre. All rights reserved.
//

import SwiftUI

/// Shimmer effect modifier for skeleton loaders
struct ShimmerModifier: ViewModifier {
    @State private var phase: CGFloat = 0

    func body(content: Content) -> some View {
        content
            .overlay(
                LinearGradient(
                    colors: [
                        Color.gray.opacity(0.3),
                        Color.white.opacity(0.5),
                        Color.gray.opacity(0.3)
                    ],
                    startPoint: .leading,
                    endPoint: .trailing
                )
                .offset(x: phase)
                .mask(content)
            )
            .onAppear {
                withAnimation(
                    Animation.linear(duration: 1.2)
                        .repeatForever(autoreverses: false)
                ) {
                    phase = UIScreen.main.bounds.width * 2
                }
            }
    }
}

extension View {
    /// Apply shimmer effect
    func shimmer() -> some View {
        self.modifier(ShimmerModifier())
    }
}

/// Skeleton loader for messages
struct MessageSkeleton: View {
    var body: some View {
        HStack(alignment: .top, spacing: 12) {
            // Avatar placeholder
            Circle()
                .fill(Color.gray.opacity(0.3))
                .frame(width: 40, height: 40)
                .shimmer()

            VStack(alignment: .leading, spacing: 8) {
                // Name placeholder
                RoundedRectangle(cornerRadius: 4)
                    .fill(Color.gray.opacity(0.3))
                    .frame(width: 120, height: 16)
                    .shimmer()

                // Message content placeholder
                RoundedRectangle(cornerRadius: 4)
                    .fill(Color.gray.opacity(0.3))
                    .frame(height: 14)
                    .frame(maxWidth: .infinity)
                    .shimmer()

                RoundedRectangle(cornerRadius: 4)
                    .fill(Color.gray.opacity(0.3))
                    .frame(width: 180, height: 14)
                    .shimmer()
            }
        }
        .padding(.horizontal, 16)
        .padding(.vertical, 8)
    }
}

/// Skeleton loader for conversation list
struct ConversationSkeleton: View {
    var body: some View {
        HStack(spacing: 16) {
            // Avatar placeholder
            Circle()
                .fill(Color.gray.opacity(0.3))
                .frame(width: 56, height: 56)
                .shimmer()

            VStack(alignment: .leading, spacing: 8) {
                // Name placeholder
                RoundedRectangle(cornerRadius: 4)
                    .fill(Color.gray.opacity(0.3))
                    .frame(width: 140, height: 18)
                    .shimmer()

                // Last message placeholder
                RoundedRectangle(cornerRadius: 4)
                    .fill(Color.gray.opacity(0.3))
                    .frame(width: 200, height: 14)
                    .shimmer()
            }

            Spacer()

            // Time placeholder
            RoundedRectangle(cornerRadius: 4)
                .fill(Color.gray.opacity(0.3))
                .frame(width: 48, height: 14)
                .shimmer()
        }
        .padding(.horizontal, 16)
        .padding(.vertical, 12)
    }
}

/// Generic skeleton box
struct SkeletonBox: View {
    let width: CGFloat
    let height: CGFloat

    var body: some View {
        RoundedRectangle(cornerRadius: 4)
            .fill(Color.gray.opacity(0.3))
            .frame(width: width, height: height)
            .shimmer()
    }
}

#Preview("Message Skeleton") {
    VStack(spacing: 12) {
        MessageSkeleton()
        MessageSkeleton()
        MessageSkeleton()
    }
}

#Preview("Conversation Skeleton") {
    VStack(spacing: 0) {
        ConversationSkeleton()
        Divider()
        ConversationSkeleton()
        Divider()
        ConversationSkeleton()
    }
}
