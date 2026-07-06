//
//  ZapTheme.swift
//  ZapLivre / ZapLivre
//
//  Design system central. A identidade: familiar como um mensageiro moderno,
//  mas com o azul ZapLivre no lugar do verde e um gradiente "spark" (raio) usado
//  com muita restrição — só onde o app quer chamar a ação. Tudo o mais é quieto.
//

import SwiftUI
import UIKit

// MARK: - Hex helpers

extension UIColor {
    convenience init(hex: UInt) {
        self.init(
            red: CGFloat((hex >> 16) & 0xFF) / 255.0,
            green: CGFloat((hex >> 8) & 0xFF) / 255.0,
            blue: CGFloat(hex & 0xFF) / 255.0,
            alpha: 1.0
        )
    }
}

extension Color {
    /// Cor adaptativa: um valor para light, outro para dark. Dá dark mode
    /// automático sem depender de Asset Catalog.
    init(light: UInt, dark: UInt) {
        self.init(UIColor { trait in
            trait.userInterfaceStyle == .dark ? UIColor(hex: dark) : UIColor(hex: light)
        })
    }
}

// MARK: - Paleta ZapLivre

enum ZapColor {
    /// Azul de marca — bolha própria, botões, badges, links.
    static let primary = Color(light: 0x2F6BFF, dark: 0x3D78FF)
    /// Ciano do "raio" — só no gradiente signature e micro-detalhes.
    static let spark = Color(light: 0x37E0FF, dark: 0x37E0FF)

    /// Texto principal.
    static let ink = Color(light: 0x0D1B2A, dark: 0xE9EDF1)
    /// Texto secundário, ícones neutros, timestamps.
    static let slate = Color(light: 0x667085, dark: 0x8A97A3)

    /// Fundo das telas de lista (Conversas, Grupos, Settings).
    static let canvas = Color(light: 0xFFFFFF, dark: 0x0B141A)
    /// Fundo do chat (atrás das bolhas).
    static let chatCanvas = Color(light: 0xEDF1F7, dark: 0x0B141A)
    /// Cartões / superfícies elevadas / bolha recebida.
    static let surface = Color(light: 0xFFFFFF, dark: 0x1F2C33)
    /// Divisórias / hairlines.
    static let hairline = Color(light: 0xE6E9EF, dark: 0x223038)

    /// Bolha própria (enviada).
    static let bubbleOut = Color(light: 0x2F6BFF, dark: 0x1B49B8)
    static let bubbleOutInk = Color.white
    /// Bolha recebida.
    static let bubbleIn = Color(light: 0xFFFFFF, dark: 0x1F2C33)
    static let bubbleInInk = Color(light: 0x0D1B2A, dark: 0xE9EDF1)

    /// Presença online (verde é convenção universal, não exclusiva de terceiros).
    static let online = Color(light: 0x22C55E, dark: 0x2ED573)
    /// Destrutivo / erro.
    static let danger = Color(light: 0xE5484D, dark: 0xFF6369)

    /// Paleta de avatares sem foto — cor derivada do id, para dar vida à lista.
    static let avatarPalette: [Color] = [
        Color(hex6: 0x2F6BFF), Color(hex6: 0x7C5CFF), Color(hex6: 0x00A6A6),
        Color(hex6: 0xE8618C), Color(hex6: 0xF2884B), Color(hex6: 0x1FA971),
        Color(hex6: 0x4B7BEC), Color(hex6: 0xB8449B),
    ]

    /// Cor estável derivada de um id — mesma lógica dos avatares. Usada para
    /// avatar sem foto e nome de autor em grupos.
    static func accent(for seed: String) -> Color {
        var hash = 5381
        for byte in seed.utf8 { hash = ((hash << 5) &+ hash) &+ Int(byte) }
        return avatarPalette[abs(hash) % avatarPalette.count]
    }

    /// Gradiente signature (o "raio"): azul → ciano. Usar com restrição.
    static let sparkGradient = LinearGradient(
        colors: [Color(hex6: 0x2F6BFF), Color(hex6: 0x37E0FF)],
        startPoint: .topLeading,
        endPoint: .bottomTrailing
    )
}

private extension Color {
    init(hex6: UInt) {
        self.init(
            red: Double((hex6 >> 16) & 0xFF) / 255.0,
            green: Double((hex6 >> 8) & 0xFF) / 255.0,
            blue: Double(hex6 & 0xFF) / 255.0
        )
    }
}

// MARK: - Tipografia

/// Escala tipográfica. Títulos e branding em SF Rounded (amigável, "livre");
/// corpo e utilitários na SF padrão. Substitui os `.font(.system(size:))` soltos.
enum ZapFont {
    static let brand = Font.system(size: 26, weight: .heavy, design: .rounded)
    static let title = Font.system(size: 20, weight: .bold, design: .rounded)
    static let rowName = Font.system(size: 17, weight: .semibold, design: .rounded)
    static let body = Font.system(size: 16, weight: .regular)
    static let preview = Font.system(size: 15, weight: .regular)
    static let caption = Font.system(size: 12, weight: .regular)
    static let badge = Font.system(size: 12, weight: .bold, design: .rounded)
}

// MARK: - Métricas

enum ZapMetric {
    static let bubbleRadius: CGFloat = 18
    static let cardRadius: CGFloat = 16
    static let buttonRadius: CGFloat = 14
    static let avatar: CGFloat = 52
    static let avatarSmall: CGFloat = 38

    static let gutter: CGFloat = 16
    static let rowGap: CGFloat = 12
    static let tight: CGFloat = 8
}
