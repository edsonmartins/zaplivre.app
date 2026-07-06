package com.zaplivre.ui.components

import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.Spacer
import androidx.compose.foundation.layout.width
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.Text
import androidx.compose.runtime.Composable
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.graphics.Color
import androidx.compose.ui.unit.dp
import com.zaplivre.utils.MessageUtils
import com.zaplivre.utils.getFormattedTime
import uniffi.zaplivre.FfiMessage
import uniffi.zaplivre.MessageStatus

/**
 * MessageStatusIndicator - Shows message status and timestamp
 */
@Composable
fun MessageStatusIndicator(
    message: FfiMessage,
    isOwnMessage: Boolean,
    modifier: Modifier = Modifier
) {
    Row(
        modifier = modifier,
        verticalAlignment = Alignment.CenterVertically
    ) {
        // Timestamp
        Text(
            text = message.getFormattedTime(),
            style = MaterialTheme.typography.labelSmall,
            color = MaterialTheme.colorScheme.onSurfaceVariant.copy(alpha = 0.7f)
        )

        // Status indicator (only for own messages)
        if (isOwnMessage) {
            Spacer(modifier = Modifier.width(4.dp))

            val statusColor = when (message.status) {
                MessageStatus.READ -> Color(0xFF0288D1)  // Blue for read
                MessageStatus.FAILED -> MaterialTheme.colorScheme.error
                else -> MaterialTheme.colorScheme.onSurfaceVariant.copy(alpha = 0.7f)
            }

            Text(
                text = MessageUtils.getStatusIcon(message.status),
                style = MaterialTheme.typography.labelSmall,
                color = statusColor
            )
        }
    }
}

/**
 * Full message status with description
 */
@Composable
fun MessageStatusFull(
    message: FfiMessage,
    modifier: Modifier = Modifier
) {
    Row(
        modifier = modifier,
        verticalAlignment = Alignment.CenterVertically
    ) {
        val statusColor = when (message.status) {
            MessageStatus.READ -> Color(0xFF0288D1)
            MessageStatus.FAILED -> MaterialTheme.colorScheme.error
            else -> MaterialTheme.colorScheme.onSurfaceVariant
        }

        Text(
            text = MessageUtils.getStatusDescription(message.status),
            style = MaterialTheme.typography.labelSmall,
            color = statusColor
        )

        Spacer(modifier = Modifier.width(4.dp))

        Text(
            text = MessageUtils.getStatusIcon(message.status),
            style = MaterialTheme.typography.labelSmall,
            color = statusColor
        )
    }
}
