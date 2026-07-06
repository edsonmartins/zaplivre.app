//
//  MessageStatusIndicator.swift
//  MePassa
//
//  Created by MePassa Team
//  Copyright © 2026 MePassa. All rights reserved.
//

import SwiftUI

/// MessageStatusIndicator - Shows message status and timestamp
struct MessageStatusIndicator: View {
    let message: FfiMessageWrapper?
    let isOwnMessage: Bool

    var body: some View {
        HStack(spacing: 4) {
            // Timestamp
            if let message = message {
                Text(formatTime(message.createdAt))
                    .font(.caption2)
                    .foregroundColor(.secondary.opacity(0.8))

                // Status indicator (only for own messages)
                if isOwnMessage {
                    Text(statusIcon(message.status))
                        .font(.caption2)
                        .foregroundColor(statusColor(message.status))
                }
            }
        }
    }

    private func formatTime(_ date: Date) -> String {
        let formatter = DateFormatter()
        let calendar = Calendar.current
        if calendar.isDateInToday(date) {
            formatter.dateFormat = "HH:mm"
        } else if calendar.isDateInYesterday(date) {
            return "Ontem"
        } else {
            formatter.dateFormat = "dd/MM/yy"
        }
        return formatter.string(from: date)
    }

    private func statusIcon(_ status: MessageStatus) -> String {
        switch status {
        case .pending: return "○"
        case .sent: return "✓"
        case .delivered: return "✓✓"
        case .read: return "✓✓"
        case .failed: return "!"
        }
    }

    private func statusColor(_ status: MessageStatus) -> Color {
        switch status {
        case .read:
            return ZapColor.primary
        case .failed:
            return .red
        default:
            return .secondary.opacity(0.8)
        }
    }
}

/// Full message status with description
struct MessageStatusFull: View {
    let message: FfiMessage

    var body: some View {
        HStack(spacing: 4) {
            Text(message.statusDescription)
                .font(.caption)
                .foregroundColor(statusColor)

            Text(message.statusIcon)
                .font(.caption)
                .foregroundColor(statusColor)
        }
    }

    private var statusColor: Color {
        switch message.status {
        case .read:
            return ZapColor.primary
        case .failed:
            return .red
        default:
            return .secondary
        }
    }
}

#Preview {
    VStack(spacing: 16) {
        MessageStatusIndicator(
            message: FfiMessageWrapper(
                id: "1",
                conversationId: "conv1",
                senderPeerId: "peer1",
                recipientPeerId: "peer2",
                content: "Hello",
                messageType: "text",
                createdAt: Date(),
                status: .sent
            ),
            isOwnMessage: true
        )

        MessageStatusIndicator(
            message: FfiMessageWrapper(
                id: "2",
                conversationId: "conv1",
                senderPeerId: "peer1",
                recipientPeerId: "peer2",
                content: "Hello",
                messageType: "text",
                createdAt: Date(),
                status: .read
            ),
            isOwnMessage: true
        )
    }
    .padding()
}
