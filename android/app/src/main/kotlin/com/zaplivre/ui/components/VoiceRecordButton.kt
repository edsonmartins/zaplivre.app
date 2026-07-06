package com.zaplivre.ui.components

import androidx.compose.animation.*
import androidx.compose.animation.core.*
import androidx.compose.foundation.background
import androidx.compose.foundation.gestures.detectTapGestures
import androidx.compose.foundation.layout.*
import androidx.compose.foundation.shape.CircleShape
import androidx.compose.material.icons.Icons
import androidx.compose.material.icons.filled.Close
import androidx.compose.material.icons.filled.Mic
import androidx.compose.material3.*
import androidx.compose.runtime.*
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.draw.scale
import androidx.compose.ui.graphics.Color
import androidx.compose.ui.input.pointer.pointerInput
import androidx.compose.ui.unit.dp
import com.zaplivre.core.VoiceRecorderViewModel

/**
 * Voice record button with hold-to-record functionality
 */
@Composable
fun VoiceRecordButton(
    viewModel: VoiceRecorderViewModel,
    onVoiceMessageRecorded: (java.io.File) -> Unit,
    modifier: Modifier = Modifier
) {
    val isRecording by viewModel.isRecording.collectAsState()
    val recordingDuration by viewModel.recordingDuration.collectAsState()

    // Animation for pulsing effect when recording
    val infiniteTransition = rememberInfiniteTransition(label = "pulse")
    val pulseScale by infiniteTransition.animateFloat(
        initialValue = 1f,
        targetValue = 1.2f,
        animationSpec = infiniteRepeatable(
            animation = tween(600, easing = FastOutSlowInEasing),
            repeatMode = RepeatMode.Reverse
        ),
        label = "pulseScale"
    )

    Box(modifier = modifier) {
        if (isRecording) {
            // Recording UI
            Row(
                verticalAlignment = Alignment.CenterVertically,
                horizontalArrangement = Arrangement.spacedBy(12.dp)
            ) {
                // Cancel button
                IconButton(
                    onClick = { viewModel.cancelRecording() },
                    modifier = Modifier.size(48.dp)
                ) {
                    Icon(
                        imageVector = Icons.Default.Close,
                        contentDescription = "Cancel recording",
                        tint = MaterialTheme.colorScheme.error
                    )
                }

                // Duration display
                Surface(
                    shape = MaterialTheme.shapes.medium,
                    color = MaterialTheme.colorScheme.errorContainer,
                    modifier = Modifier.weight(1f)
                ) {
                    Row(
                        modifier = Modifier.padding(horizontal = 16.dp, vertical = 12.dp),
                        verticalAlignment = Alignment.CenterVertically,
                        horizontalArrangement = Arrangement.spacedBy(8.dp)
                    ) {
                        // Pulsing red dot
                        Box(
                            modifier = Modifier
                                .size(12.dp)
                                .scale(pulseScale)
                                .background(Color.Red, CircleShape)
                        )

                        Text(
                            text = viewModel.formatDuration(recordingDuration),
                            style = MaterialTheme.typography.bodyLarge,
                            color = MaterialTheme.colorScheme.onErrorContainer
                        )

                        Spacer(modifier = Modifier.weight(1f))

                        Text(
                            text = "Recording...",
                            style = MaterialTheme.typography.bodyMedium,
                            color = MaterialTheme.colorScheme.onErrorContainer.copy(alpha = 0.7f)
                        )
                    }
                }

                // Stop/Send button
                IconButton(
                    onClick = {
                        val file = viewModel.stopRecording()
                        if (file != null) {
                            onVoiceMessageRecorded(file)
                        }
                    },
                    modifier = Modifier
                        .size(48.dp)
                        .background(MaterialTheme.colorScheme.primary, CircleShape)
                ) {
                    Icon(
                        imageVector = Icons.Default.Mic,
                        contentDescription = "Send voice message",
                        tint = MaterialTheme.colorScheme.onPrimary
                    )
                }
            }
        } else {
            // Mic button (hold to record)
            IconButton(
                onClick = { /* No-op, use long press */ },
                modifier = Modifier
                    .size(48.dp)
                    .pointerInput(Unit) {
                        detectTapGestures(
                            onPress = {
                                // Start recording on press
                                viewModel.startRecording()

                                // Wait for release
                                tryAwaitRelease()

                                // Stop recording on release
                                val file = viewModel.stopRecording()
                                if (file != null && recordingDuration > 500) {
                                    // Only send if recorded for more than 0.5s
                                    onVoiceMessageRecorded(file)
                                } else {
                                    viewModel.cancelRecording()
                                }
                            }
                        )
                    }
            ) {
                Icon(
                    imageVector = Icons.Default.Mic,
                    contentDescription = "Hold to record voice message",
                    tint = MaterialTheme.colorScheme.primary
                )
            }
        }
    }
}

/**
 * Compact voice record button for chat input
 */
@Composable
fun CompactVoiceRecordButton(
    onStartRecording: () -> Unit,
    modifier: Modifier = Modifier
) {
    IconButton(
        onClick = onStartRecording,
        modifier = modifier
    ) {
        Icon(
            imageVector = Icons.Default.Mic,
            contentDescription = "Record voice message",
            tint = MaterialTheme.colorScheme.primary
        )
    }
}

/**
 * Voice recording indicator overlay
 */
@Composable
fun VoiceRecordingOverlay(
    isRecording: Boolean,
    duration: Long,
    onCancel: () -> Unit,
    onSend: () -> Unit,
    modifier: Modifier = Modifier
) {
    AnimatedVisibility(
        visible = isRecording,
        enter = slideInVertically(initialOffsetY = { it }) + fadeIn(),
        exit = slideOutVertically(targetOffsetY = { it }) + fadeOut(),
        modifier = modifier
    ) {
        Surface(
            modifier = Modifier.fillMaxWidth(),
            color = MaterialTheme.colorScheme.surfaceVariant,
            tonalElevation = 8.dp
        ) {
            Row(
                modifier = Modifier
                    .padding(16.dp)
                    .fillMaxWidth(),
                verticalAlignment = Alignment.CenterVertically,
                horizontalArrangement = Arrangement.SpaceBetween
            ) {
                // Cancel button
                TextButton(onClick = onCancel) {
                    Text("Cancel", color = MaterialTheme.colorScheme.error)
                }

                // Duration
                Row(
                    verticalAlignment = Alignment.CenterVertically,
                    horizontalArrangement = Arrangement.spacedBy(8.dp)
                ) {
                    Box(
                        modifier = Modifier
                            .size(8.dp)
                            .background(Color.Red, CircleShape)
                    )

                    Text(
                        text = formatDuration(duration),
                        style = MaterialTheme.typography.titleMedium,
                        color = MaterialTheme.colorScheme.onSurfaceVariant
                    )
                }

                // Send button
                TextButton(onClick = onSend) {
                    Text("Send")
                }
            }
        }
    }
}

private fun formatDuration(durationMs: Long): String {
    val totalSeconds = durationMs / 1000
    val minutes = totalSeconds / 60
    val seconds = totalSeconds % 60
    return String.format("%02d:%02d", minutes, seconds)
}
