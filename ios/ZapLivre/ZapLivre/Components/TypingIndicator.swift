//
//  TypingIndicator.swift
//  ZapLivre
//
//  Created by ZapLivre Team
//  Copyright © 2026 ZapLivre. All rights reserved.
//

import SwiftUI

/// TypingIndicator - Animated dots showing someone is typing
struct TypingIndicator: View {
    let peerName: String

    var body: some View {
        HStack(spacing: 8) {
            // Animated dots
            HStack(spacing: 4) {
                ForEach(0..<3) { index in
                    AnimatedDot(delay: Double(index) * 0.15)
                }
            }
            .padding(.horizontal, 12)
            .padding(.vertical, 8)
            .background(Color(UIColor.systemGray5))
            .cornerRadius(16)

            // "está digitando..." text
            Text("\(peerName) está digitando...")
                .font(.caption)
                .foregroundColor(.secondary)
        }
        .padding(.horizontal, 16)
        .padding(.vertical, 8)
    }
}

/// AnimatedDot - Single animated dot
struct AnimatedDot: View {
    let delay: Double

    @State private var isAnimating = false

    var body: some View {
        Circle()
            .fill(Color.gray)
            .frame(width: 8, height: 8)
            .opacity(isAnimating ? 1.0 : 0.3)
            .animation(
                Animation
                    .easeInOut(duration: 0.6)
                    .repeatForever(autoreverses: true)
                    .delay(delay),
                value: isAnimating
            )
            .onAppear {
                isAnimating = true
            }
    }
}

/// Compact typing indicator (just dots, no text)
struct TypingIndicatorCompact: View {
    var body: some View {
        HStack(spacing: 4) {
            ForEach(0..<3) { index in
                AnimatedDot(delay: Double(index) * 0.15)
            }
        }
        .padding(.horizontal, 12)
        .padding(.vertical, 8)
        .background(Color(UIColor.systemGray5))
        .cornerRadius(16)
    }
}

#Preview {
    VStack(spacing: 20) {
        TypingIndicator(peerName: "João")

        HStack {
            TypingIndicatorCompact()
            Spacer()
        }
        .padding(.horizontal)
    }
}
