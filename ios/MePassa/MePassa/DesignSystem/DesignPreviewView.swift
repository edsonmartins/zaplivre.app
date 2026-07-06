//
//  DesignPreviewView.swift
//  MePassa / ZapLivre
//
//  Vitrine do design system com dados de exemplo, para revisar a direção visual
//  sem depender de rede/2 devices. Acessível só via launch argument
//  `-designPreview` (ver ContentView) — não faz parte do app em produção.
//

import SwiftUI

struct DesignPreviewView: View {
    @State private var tab = 0
    @State private var draft = ""

    var body: some View {
        VStack(spacing: 0) {
            Picker("", selection: $tab) {
                Text("Conversas").tag(0)
                Text("Chat").tag(1)
            }
            .pickerStyle(.segmented)
            .padding()

            if tab == 0 { conversationsDemo } else { chatDemo }
        }
    }

    // MARK: Conversas

    private let demoRows: [(name: String, msg: String, time: String, unread: Int, online: Bool)] = [
        ("Ana Beatriz", "Bora marcar aquele café ☕️", "09:24", 2, true),
        ("Time ZapLivre", "Rodrigo: subi a nova build", "08:51", 5, false),
        ("Carlos Mendes", "Perfeito, obrigado!", "Ontem", 0, false),
        ("Família 💙", "Mãe: chega que horas?", "Ontem", 0, true),
        ("Júlia Ferraz", "kkkk manda o print", "Seg", 0, false),
    ]

    private var conversationsDemo: some View {
        List {
            ForEach(Array(demoRows.enumerated()), id: \.offset) { _, r in
                HStack(spacing: ZapMetric.rowGap) {
                    AvatarView(seed: r.name, name: r.name, isOnline: r.online)
                    VStack(alignment: .leading, spacing: 3) {
                        Text(r.name).font(ZapFont.rowName).foregroundColor(ZapColor.ink)
                        Text(r.msg).font(ZapFont.preview)
                            .foregroundColor(r.unread > 0 ? ZapColor.ink : ZapColor.slate)
                            .lineLimit(1)
                    }
                    Spacer(minLength: 8)
                    VStack(alignment: .trailing, spacing: 6) {
                        Text(r.time).font(ZapFont.caption)
                            .foregroundColor(r.unread > 0 ? ZapColor.primary : ZapColor.slate)
                        if r.unread > 0 {
                            Text("\(r.unread)").font(ZapFont.badge).foregroundColor(.white)
                                .padding(.horizontal, 7).frame(minWidth: 22, minHeight: 22)
                                .background(ZapColor.primary).clipShape(Capsule())
                        }
                    }
                }
                .padding(.vertical, 6)
                .listRowInsets(EdgeInsets(top: 2, leading: ZapMetric.gutter, bottom: 2, trailing: ZapMetric.gutter))
                .listRowBackground(ZapColor.canvas)
            }
        }
        .listStyle(.plain)
        .background(ZapColor.canvas)
    }

    // MARK: Chat

    private let demoMsgs: [(text: String, out: Bool, time: String, tail: Bool, read: Bool)] = [
        ("Oi! Tudo bem? 😊", false, "09:20", true, false),
        ("Tudo ótimo! E aí, viu a nova versão do ZapLivre?", true, "09:21", false, true),
        ("Ficou muito mais bonito, sério", true, "09:21", true, true),
        ("Vi sim! As bolhas agora têm cauda, ficou com cara de app de verdade 🔥", false, "09:22", true, false),
        ("E é tudo criptografado ponta a ponta, sem servidor central", true, "09:23", true, false),
    ]

    private var chatDemo: some View {
        VStack(spacing: 0) {
            ScrollView {
                VStack(spacing: 3) {
                    encryptionNote
                    ForEach(Array(demoMsgs.enumerated()), id: \.offset) { _, m in
                        HStack(alignment: .bottom, spacing: 0) {
                            if m.out { Spacer(minLength: 48) }
                            Text(m.text)
                                .font(ZapFont.body)
                                .padding(.horizontal, 12).padding(.vertical, 8)
                                .background(
                                    (m.out ? ZapColor.bubbleOut : ZapColor.bubbleIn)
                                        .clipShape(BubbleShape(isOutgoing: m.out, hasTail: m.tail))
                                )
                                .foregroundColor(m.out ? ZapColor.bubbleOutInk : ZapColor.bubbleInInk)
                                .overlay(
                                    m.out ? nil : BubbleShape(isOutgoing: false, hasTail: m.tail)
                                        .stroke(ZapColor.hairline, lineWidth: 0.5)
                                )
                                .shadow(color: .black.opacity(0.05), radius: 1, y: 1)
                            if !m.out { Spacer(minLength: 48) }
                        }
                        .padding(.top, m.tail ? 4 : 1)
                        .padding(.horizontal, 12)
                    }
                }
                .padding(.vertical, 12)
            }
            .background(ZapColor.chatCanvas)

            inputBar
        }
    }

    private var encryptionNote: some View {
        Text("🔒 As mensagens são protegidas com criptografia de ponta a ponta.")
            .font(ZapFont.caption).foregroundColor(ZapColor.slate)
            .multilineTextAlignment(.center)
            .padding(.vertical, 8).padding(.horizontal, 12)
            .background(RoundedRectangle(cornerRadius: 10).fill(ZapColor.primary.opacity(0.08)))
            .padding(.bottom, 6)
    }

    private var inputBar: some View {
        HStack(spacing: 12) {
            Image(systemName: "photo.on.rectangle").font(.system(size: 22)).foregroundColor(ZapColor.slate)
            TextField("Mensagem", text: $draft)
                .font(ZapFont.body).padding(.horizontal, 14).padding(.vertical, 9)
                .background(ZapColor.surface)
                .clipShape(RoundedRectangle(cornerRadius: 22, style: .continuous))
                .overlay(RoundedRectangle(cornerRadius: 22, style: .continuous).stroke(ZapColor.hairline, lineWidth: 1))
            Image(systemName: "arrow.up").font(.system(size: 18, weight: .bold)).foregroundColor(.white)
                .frame(width: 38, height: 38).background(ZapColor.sparkGradient).clipShape(Circle())
        }
        .padding(.horizontal).padding(.vertical, 8)
        .background(ZapColor.canvas.overlay(ZapColor.hairline.frame(height: 0.5), alignment: .top))
    }
}
