//
//  MessageBubble.swift
//  ZapLivre / ZapLivre
//
//  Bolha de mensagem com cauda, no idioma visual dos mensageiros de mercado:
//  própria à direita (azul de marca), recebida à esquerda (superfície), com
//  cauda só na última mensagem de uma sequência do mesmo autor. Timestamp e
//  status embutidos no rodapé da bolha.
//

import SwiftUI

/// Retângulo arredondado com uma cauda opcional no canto inferior do lado do
/// autor. Desenhado como um único path contínuo (sem emenda) para a cauda se
/// fundir ao corpo. O espaço da cauda é sempre reservado, então bolhas com e
/// sem cauda alinham a mesma borda.
struct BubbleShape: Shape {
    var isOutgoing: Bool
    var hasTail: Bool
    var radius: CGFloat = ZapMetric.bubbleRadius

    func path(in rect: CGRect) -> Path {
        let r = min(radius, rect.height / 2)
        let tail: CGFloat = 7            // sempre reservado, para alinhar as bordas
        var p = Path()

        if isOutgoing {
            let maxX = rect.maxX - tail  // corpo termina aqui; a cauda ocupa o resto
            // topo, esquerda→direita
            p.move(to: CGPoint(x: rect.minX + r, y: rect.minY))
            p.addLine(to: CGPoint(x: maxX - r, y: rect.minY))
            p.addQuadCurve(to: CGPoint(x: maxX, y: rect.minY + r),
                           control: CGPoint(x: maxX, y: rect.minY))
            p.addLine(to: CGPoint(x: maxX, y: rect.maxY - r))
            if hasTail {
                // vira para fora e volta, formando uma cauda curta e suave
                p.addQuadCurve(to: CGPoint(x: rect.maxX, y: rect.maxY),
                               control: CGPoint(x: maxX, y: rect.maxY - r * 0.35))
                p.addQuadCurve(to: CGPoint(x: maxX - r * 0.85, y: rect.maxY),
                               control: CGPoint(x: maxX - r * 0.15, y: rect.maxY))
            } else {
                p.addQuadCurve(to: CGPoint(x: maxX - r, y: rect.maxY),
                               control: CGPoint(x: maxX, y: rect.maxY))
            }
            p.addLine(to: CGPoint(x: rect.minX + r, y: rect.maxY))
            p.addQuadCurve(to: CGPoint(x: rect.minX, y: rect.maxY - r),
                           control: CGPoint(x: rect.minX, y: rect.maxY))
            p.addLine(to: CGPoint(x: rect.minX, y: rect.minY + r))
            p.addQuadCurve(to: CGPoint(x: rect.minX + r, y: rect.minY),
                           control: CGPoint(x: rect.minX, y: rect.minY))
        } else {
            let minX = rect.minX + tail
            p.move(to: CGPoint(x: minX + r, y: rect.minY))
            p.addLine(to: CGPoint(x: rect.maxX - r, y: rect.minY))
            p.addQuadCurve(to: CGPoint(x: rect.maxX, y: rect.minY + r),
                           control: CGPoint(x: rect.maxX, y: rect.minY))
            p.addLine(to: CGPoint(x: rect.maxX, y: rect.maxY - r))
            p.addQuadCurve(to: CGPoint(x: rect.maxX - r, y: rect.maxY),
                           control: CGPoint(x: rect.maxX, y: rect.maxY))
            p.addLine(to: CGPoint(x: minX + r * 0.85, y: rect.maxY))
            if hasTail {
                p.addQuadCurve(to: CGPoint(x: rect.minX, y: rect.maxY),
                               control: CGPoint(x: minX + r * 0.15, y: rect.maxY))
                p.addQuadCurve(to: CGPoint(x: minX, y: rect.maxY - r),
                               control: CGPoint(x: minX, y: rect.maxY - r * 0.35))
            } else {
                p.addQuadCurve(to: CGPoint(x: minX, y: rect.maxY - r),
                               control: CGPoint(x: minX, y: rect.maxY))
            }
            p.addLine(to: CGPoint(x: minX, y: rect.minY + r))
            p.addQuadCurve(to: CGPoint(x: minX + r, y: rect.minY),
                           control: CGPoint(x: minX, y: rect.minY))
        }
        p.closeSubpath()
        return p
    }
}

struct ZapTextBubble: View {
    let text: String
    let isOutgoing: Bool
    let timestamp: String
    var hasTail: Bool = true
    /// Status só para mensagens próprias (ex.: check de entrega/leitura).
    var statusIcon: String? = nil
    var statusRead: Bool = false

    var body: some View {
        HStack(alignment: .bottom, spacing: 0) {
            if isOutgoing { Spacer(minLength: 48) }

            content
                .padding(.horizontal, 12)
                .padding(.vertical, 8)
                .background(
                    (isOutgoing ? ZapColor.bubbleOut : ZapColor.bubbleIn)
                        .clipShape(BubbleShape(isOutgoing: isOutgoing, hasTail: hasTail))
                )
                .overlay(
                    // Hairline sutil só nas recebidas em light, para descolar do fundo.
                    isOutgoing
                        ? nil
                        : BubbleShape(isOutgoing: false, hasTail: hasTail)
                            .stroke(ZapColor.hairline, lineWidth: 0.5)
                )
                .shadow(color: Color.black.opacity(0.06), radius: 1, x: 0, y: 1)

            if !isOutgoing { Spacer(minLength: 48) }
        }
    }

    private var inkColor: Color { isOutgoing ? ZapColor.bubbleOutInk : ZapColor.bubbleInInk }

    private var content: some View {
        // Texto + meta (hora/status) no mesmo bloco; a meta "flutua" no fim.
        ZStack(alignment: .bottomTrailing) {
            Text(text)
                .font(ZapFont.body)
                .foregroundColor(inkColor)
                // reserva espaço para a meta na última linha
                .padding(.trailing, isOutgoing ? 62 : 44)

            HStack(spacing: 3) {
                Text(timestamp)
                    .font(.system(size: 11))
                    .foregroundColor(isOutgoing ? Color.white.opacity(0.8) : ZapColor.slate)
                if isOutgoing, let statusIcon {
                    Image(systemName: statusIcon)
                        .font(.system(size: 11, weight: .semibold))
                        .foregroundColor(statusRead ? ZapColor.spark : Color.white.opacity(0.8))
                }
            }
        }
    }
}
