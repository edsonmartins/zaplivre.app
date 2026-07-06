//
//  AvatarView.swift
//  ZapLivre / ZapLivre
//
//  Avatar reutilizável: inicial sobre uma cor derivada do id (para a lista
//  ganhar variedade como nos mensageiros de mercado), com indicador de presença
//  opcional. Substitui os `Circle().fill(Color.blue)...` recriados em cada tela.
//

import SwiftUI

struct AvatarView: View {
    let seed: String
    let name: String
    var size: CGFloat = ZapMetric.avatar
    var isOnline: Bool = false

    private var initial: String {
        let trimmed = name.trimmingCharacters(in: .whitespaces)
        return String(trimmed.isEmpty ? "?" : trimmed.prefix(1)).uppercased()
    }

    private var color: Color { ZapColor.accent(for: seed) }

    var body: some View {
        Circle()
            .fill(color)
            .frame(width: size, height: size)
            .overlay(
                Text(initial)
                    .font(.system(size: size * 0.42, weight: .semibold, design: .rounded))
                    .foregroundColor(.white)
            )
            .overlay(alignment: .bottomTrailing) {
                if isOnline {
                    Circle()
                        .fill(ZapColor.online)
                        .frame(width: size * 0.28, height: size * 0.28)
                        .overlay(Circle().stroke(ZapColor.canvas, lineWidth: size * 0.05))
                }
            }
    }
}
