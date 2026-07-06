//
//  ZapButtonStyle.swift
//  ZapLivre / ZapLivre
//
//  Estilos de botão do design system. O primário carrega o gradiente signature
//  (o "raio") — é a ação que o app quer que você tome. O secundário é quieto.
//

import SwiftUI

/// Botão primário: gradiente spark, largura total, cantos suaves.
struct ZapPrimaryButtonStyle: ButtonStyle {
    var enabled: Bool = true

    func makeBody(configuration: Configuration) -> some View {
        configuration.label
            .font(.system(size: 17, weight: .semibold, design: .rounded))
            .foregroundColor(.white)
            .frame(maxWidth: .infinity)
            .padding(.vertical, 15)
            .background(
                Group {
                    if enabled {
                        ZapColor.sparkGradient
                    } else {
                        Color.gray.opacity(0.4)
                    }
                }
            )
            .clipShape(RoundedRectangle(cornerRadius: ZapMetric.buttonRadius, style: .continuous))
            .opacity(configuration.isPressed ? 0.85 : 1.0)
            .scaleEffect(configuration.isPressed ? 0.98 : 1.0)
            .animation(.easeOut(duration: 0.12), value: configuration.isPressed)
    }
}

/// Botão secundário: contorno discreto na cor de marca.
struct ZapSecondaryButtonStyle: ButtonStyle {
    func makeBody(configuration: Configuration) -> some View {
        configuration.label
            .font(.system(size: 17, weight: .semibold, design: .rounded))
            .foregroundColor(ZapColor.primary)
            .frame(maxWidth: .infinity)
            .padding(.vertical, 15)
            .background(ZapColor.primary.opacity(0.10))
            .clipShape(RoundedRectangle(cornerRadius: ZapMetric.buttonRadius, style: .continuous))
            .opacity(configuration.isPressed ? 0.7 : 1.0)
            .animation(.easeOut(duration: 0.12), value: configuration.isPressed)
    }
}
