package com.zaplivre.utils

import uniffi.zaplivre.MessageStatus
import java.text.SimpleDateFormat
import java.util.*
import kotlin.math.abs

/**
 * MessageUtils - Utilities for message formatting and status
 */
object MessageUtils {

    /**
     * Format timestamp to relative time (e.g., "5 min", "Ontem 14:30", "12/03/2025")
     */
    fun formatTimestamp(timestampSeconds: Long): String {
        val messageTime = Date(timestampSeconds * 1000)
        val now = Date()

        val diffMillis = now.time - messageTime.time
        val diffSeconds = diffMillis / 1000
        val diffMinutes = diffSeconds / 60
        val diffHours = diffMinutes / 60
        val diffDays = diffHours / 24

        return when {
            // Less than 1 minute
            diffSeconds < 60 -> "agora"

            // Less than 1 hour - show minutes
            diffMinutes < 60 -> "${diffMinutes}min"

            // Less than 24 hours - show hours
            diffHours < 24 -> "${diffHours}h"

            // Yesterday
            diffDays < 2 && isSameDay(messageTime, getYesterday()) -> {
                "Ontem ${formatTime(messageTime)}"
            }

            // This week - show day name
            diffDays < 7 -> {
                "${getDayName(messageTime)} ${formatTime(messageTime)}"
            }

            // This year - show date without year
            isSameYear(messageTime, now) -> {
                SimpleDateFormat("dd/MM", Locale.getDefault()).format(messageTime)
            }

            // Older - show full date
            else -> {
                SimpleDateFormat("dd/MM/yyyy", Locale.getDefault()).format(messageTime)
            }
        }
    }

    /**
     * Format full timestamp with date and time
     */
    fun formatFullTimestamp(timestampSeconds: Long): String {
        val messageTime = Date(timestampSeconds * 1000)
        val now = Date()

        return when {
            isSameDay(messageTime, now) -> {
                "Hoje ${formatTime(messageTime)}"
            }
            isSameDay(messageTime, getYesterday()) -> {
                "Ontem ${formatTime(messageTime)}"
            }
            isSameYear(messageTime, now) -> {
                SimpleDateFormat("dd/MM HH:mm", Locale.getDefault()).format(messageTime)
            }
            else -> {
                SimpleDateFormat("dd/MM/yyyy HH:mm", Locale.getDefault()).format(messageTime)
            }
        }
    }

    /**
     * Get status icon description
     */
    fun getStatusIcon(status: MessageStatus, isRead: Boolean = false): String {
        return when (status) {
            MessageStatus.PENDING -> "⏱️"  // Clock
            MessageStatus.SENT -> "✓"      // Single check
            MessageStatus.DELIVERED -> "✓✓" // Double check
            MessageStatus.READ -> "✓✓"     // Double check (will be colored blue)
            MessageStatus.FAILED -> "❌"    // Error
        }
    }

    /**
     * Get status description
     */
    fun getStatusDescription(status: MessageStatus): String {
        return when (status) {
            MessageStatus.PENDING -> "Enviando..."
            MessageStatus.SENT -> "Enviado"
            MessageStatus.DELIVERED -> "Entregue"
            MessageStatus.READ -> "Lido"
            MessageStatus.FAILED -> "Falha no envio"
        }
    }

    // Helper functions

    private fun formatTime(date: Date): String {
        return SimpleDateFormat("HH:mm", Locale.getDefault()).format(date)
    }

    private fun getDayName(date: Date): String {
        val dayFormat = SimpleDateFormat("EEEE", Locale("pt", "BR"))
        return dayFormat.format(date).replaceFirstChar { it.uppercase() }
    }

    private fun isSameDay(date1: Date, date2: Date): Boolean {
        val cal1 = Calendar.getInstance().apply { time = date1 }
        val cal2 = Calendar.getInstance().apply { time = date2 }

        return cal1.get(Calendar.YEAR) == cal2.get(Calendar.YEAR) &&
               cal1.get(Calendar.DAY_OF_YEAR) == cal2.get(Calendar.DAY_OF_YEAR)
    }

    private fun isSameYear(date1: Date, date2: Date): Boolean {
        val cal1 = Calendar.getInstance().apply { time = date1 }
        val cal2 = Calendar.getInstance().apply { time = date2 }

        return cal1.get(Calendar.YEAR) == cal2.get(Calendar.YEAR)
    }

    private fun getYesterday(): Date {
        val calendar = Calendar.getInstance()
        calendar.add(Calendar.DAY_OF_YEAR, -1)
        return calendar.time
    }
}

/**
 * Extension function for FfiMessage to get formatted timestamp
 */
fun uniffi.zaplivre.FfiMessage.getFormattedTime(): String {
    return MessageUtils.formatTimestamp(this.createdAt)
}

/**
 * Extension function for FfiMessage to get full formatted timestamp
 */
fun uniffi.zaplivre.FfiMessage.getFullFormattedTime(): String {
    return MessageUtils.formatFullTimestamp(this.createdAt)
}
