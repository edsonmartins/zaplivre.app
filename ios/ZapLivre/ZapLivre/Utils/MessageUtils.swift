//
//  MessageUtils.swift
//  ZapLivre
//
//  Created by ZapLivre Team
//  Copyright © 2026 ZapLivre. All rights reserved.
//

import Foundation

/// MessageUtils - Utilities for message formatting and status
struct MessageUtils {

    /// Format timestamp to relative time (e.g., "5 min", "Ontem 14:30", "12/03/2025")
    static func formatTimestamp(_ timestampSeconds: Int64) -> String {
        let messageDate = Date(timeIntervalSince1970: TimeInterval(timestampSeconds))
        let now = Date()

        let calendar = Calendar.current
        let components = calendar.dateComponents([.second, .minute, .hour, .day], from: messageDate, to: now)

        if let seconds = components.second, seconds < 60 {
            return "agora"
        }

        if let minutes = components.minute, minutes < 60 {
            return "\(minutes)min"
        }

        if let hours = components.hour, hours < 24 {
            return "\(hours)h"
        }

        if let days = components.day, days < 2 && isYesterday(messageDate) {
            return "Ontem \(formatTime(messageDate))"
        }

        if let days = components.day, days < 7 {
            return "\(getDayName(messageDate)) \(formatTime(messageDate))"
        }

        if isSameYear(messageDate, now) {
            let formatter = DateFormatter()
            formatter.dateFormat = "dd/MM"
            return formatter.string(from: messageDate)
        }

        let formatter = DateFormatter()
        formatter.dateFormat = "dd/MM/yyyy"
        return formatter.string(from: messageDate)
    }

    /// Format full timestamp with date and time
    static func formatFullTimestamp(_ timestampSeconds: Int64) -> String {
        let messageDate = Date(timeIntervalSince1970: TimeInterval(timestampSeconds))
        let now = Date()

        if isToday(messageDate) {
            return "Hoje \(formatTime(messageDate))"
        }

        if isYesterday(messageDate) {
            return "Ontem \(formatTime(messageDate))"
        }

        if isSameYear(messageDate, now) {
            let formatter = DateFormatter()
            formatter.dateFormat = "dd/MM HH:mm"
            return formatter.string(from: messageDate)
        }

        let formatter = DateFormatter()
        formatter.dateFormat = "dd/MM/yyyy HH:mm"
        return formatter.string(from: messageDate)
    }

    /// Get status icon
    static func getStatusIcon(_ status: MessageStatus) -> String {
        switch status {
        case .pending:
            return "⏱️"  // Clock
        case .sent:
            return "✓"   // Single check
        case .delivered:
            return "✓✓"  // Double check
        case .read:
            return "✓✓"  // Double check (will be colored blue)
        case .failed:
            return "❌"  // Error
        }
    }

    /// Get status description
    static func getStatusDescription(_ status: MessageStatus) -> String {
        switch status {
        case .pending:
            return "Enviando..."
        case .sent:
            return "Enviado"
        case .delivered:
            return "Entregue"
        case .read:
            return "Lido"
        case .failed:
            return "Falha no envio"
        }
    }

    /// Get status color
    static func getStatusColor(_ status: MessageStatus) -> String {
        switch status {
        case .read:
            return "blue"
        case .failed:
            return "red"
        default:
            return "gray"
        }
    }

    // MARK: - Helper functions

    private static func formatTime(_ date: Date) -> String {
        let formatter = DateFormatter()
        formatter.dateFormat = "HH:mm"
        return formatter.string(from: date)
    }

    private static func getDayName(_ date: Date) -> String {
        let formatter = DateFormatter()
        formatter.locale = Locale(identifier: "pt_BR")
        formatter.dateFormat = "EEEE"
        let dayName = formatter.string(from: date)
        return dayName.prefix(1).uppercased() + dayName.dropFirst()
    }

    private static func isToday(_ date: Date) -> Bool {
        Calendar.current.isDateInToday(date)
    }

    private static func isYesterday(_ date: Date) -> Bool {
        Calendar.current.isDateInYesterday(date)
    }

    private static func isSameYear(_ date1: Date, _ date2: Date) -> Bool {
        let calendar = Calendar.current
        return calendar.component(.year, from: date1) == calendar.component(.year, from: date2)
    }
}

// MARK: - FfiMessage Extensions

extension FfiMessage {
    /// Get formatted time for the message
    var formattedTime: String {
        MessageUtils.formatTimestamp(self.createdAt)
    }

    /// Get full formatted timestamp
    var fullFormattedTime: String {
        MessageUtils.formatFullTimestamp(self.createdAt)
    }

    /// Get status icon
    var statusIcon: String {
        MessageUtils.getStatusIcon(self.status)
    }

    /// Get status description
    var statusDescription: String {
        MessageUtils.getStatusDescription(self.status)
    }
}
