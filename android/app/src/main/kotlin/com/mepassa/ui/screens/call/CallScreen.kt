package com.mepassa.ui.screens.call

import androidx.compose.foundation.background
import androidx.compose.foundation.layout.*
import androidx.compose.foundation.shape.CircleShape
import androidx.compose.material.icons.Icons
import androidx.compose.material.icons.filled.*
import androidx.compose.material3.*
import androidx.compose.runtime.*
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.graphics.Color
import androidx.compose.ui.platform.LocalContext
import androidx.compose.ui.text.font.FontWeight
import androidx.compose.ui.unit.dp
import androidx.compose.ui.unit.sp
import com.mepassa.core.MePassaClientWrapper
import com.mepassa.voip.CallAudioManager
import kotlinx.coroutines.delay
import kotlinx.coroutines.launch
import kotlin.time.Duration.Companion.seconds

/**
 * CallScreen - Tela durante uma chamada de voz ativa
 *
 * Exibe:
 * - Informações do peer remoto
 * - Timer de duração da chamada
 * - Botões de controle: mute, speakerphone, hangup
 */
@Composable
fun CallScreen(
    callId: String,
    remotePeerId: String,
    onOpenVideo: () -> Unit,
    onCallEnded: () -> Unit
) {
    val scope = rememberCoroutineScope()
    val context = LocalContext.current

    // Audio manager para controle de áudio durante chamada
    val audioManager = remember { CallAudioManager(context) }

    var isMuted by remember { mutableStateOf(false) }
    var isSpeakerOn by remember { mutableStateOf(false) }
    var callDuration by remember { mutableStateOf(0) } // em segundos
    var isCallActive by remember { mutableStateOf(true) }

    // Iniciar gerenciamento de áudio
    DisposableEffect(Unit) {
        audioManager.startCall()
        onDispose {
            audioManager.stopCall()
        }
    }

    // Timer para duração da chamada
    LaunchedEffect(Unit) {
        while (isCallActive) {
            delay(1.seconds)
            callDuration++
        }
    }

    // Layout principal
    Box(
        modifier = Modifier
            .fillMaxSize()
            .background(MaterialTheme.colorScheme.surface)
    ) {
        Column(
            modifier = Modifier
                .fillMaxSize()
                .padding(32.dp),
            horizontalAlignment = Alignment.CenterHorizontally,
            verticalArrangement = Arrangement.SpaceBetween
        ) {
            // Header: Info do peer e status
            Column(
                horizontalAlignment = Alignment.CenterHorizontally,
                modifier = Modifier.padding(top = 64.dp)
            ) {
                // Avatar (placeholder)
                Surface(
                    modifier = Modifier.size(120.dp),
                    shape = CircleShape,
                    color = MaterialTheme.colorScheme.primaryContainer
                ) {
                    Box(contentAlignment = Alignment.Center) {
                        Icon(
                            imageVector = Icons.Default.Person,
                            contentDescription = "Peer Avatar",
                            modifier = Modifier.size(64.dp),
                            tint = MaterialTheme.colorScheme.onPrimaryContainer
                        )
                    }
                }

                Spacer(modifier = Modifier.height(24.dp))

                // Nome do peer (primeiros 16 caracteres)
                Text(
                    text = remotePeerId.take(16) + "...",
                    style = MaterialTheme.typography.headlineMedium,
                    fontWeight = FontWeight.Bold
                )

                Spacer(modifier = Modifier.height(8.dp))

                // Status
                Text(
                    text = "Chamada em andamento",
                    style = MaterialTheme.typography.bodyLarge,
                    color = MaterialTheme.colorScheme.onSurfaceVariant
                )

                Spacer(modifier = Modifier.height(16.dp))

                // Timer
                Text(
                    text = formatDuration(callDuration),
                    style = MaterialTheme.typography.headlineSmall,
                    color = MaterialTheme.colorScheme.primary,
                    fontWeight = FontWeight.Medium
                )
            }

            // Botões de controle
            Row(
                modifier = Modifier
                    .fillMaxWidth()
                    .padding(bottom = 48.dp),
                horizontalArrangement = Arrangement.SpaceEvenly,
                verticalAlignment = Alignment.CenterVertically
            ) {
                // Botão Video
                IconButton(
                    onClick = onOpenVideo,
                    modifier = Modifier
                        .size(72.dp)
                        .background(
                            MaterialTheme.colorScheme.primary,
                            CircleShape
                        )
                ) {
                    Icon(
                        imageVector = Icons.Default.Videocam,
                        contentDescription = "Video",
                        modifier = Modifier.size(32.dp),
                        tint = MaterialTheme.colorScheme.onPrimary
                    )
                }

                // Botão Mute
                IconButton(
                    onClick = {
                        scope.launch {
                            // Toggle mute no backend
                            MePassaClientWrapper.toggleMute(callId)
                            // Toggle mute local (AudioManager)
                            isMuted = audioManager.toggleMute()
                        }
                    },
                    modifier = Modifier
                        .size(72.dp)
                        .background(
                            if (isMuted) MaterialTheme.colorScheme.error
                            else MaterialTheme.colorScheme.surfaceVariant,
                            CircleShape
                        )
                ) {
                    Icon(
                        imageVector = if (isMuted) Icons.Default.MicOff else Icons.Default.Mic,
                        contentDescription = if (isMuted) "Unmute" else "Mute",
                        modifier = Modifier.size(32.dp),
                        tint = if (isMuted) MaterialTheme.colorScheme.onError
                        else MaterialTheme.colorScheme.onSurfaceVariant
                    )
                }

                // Botão Hangup (maior e vermelho)
                IconButton(
                    onClick = {
                        scope.launch {
                            if (MePassaClientWrapper.hangupCall(callId)) {
                                isCallActive = false
                                onCallEnded()
                            }
                        }
                    },
                    modifier = Modifier
                        .size(88.dp)
                        .background(MaterialTheme.colorScheme.error, CircleShape)
                ) {
                    Icon(
                        imageVector = Icons.Default.CallEnd,
                        contentDescription = "Hangup",
                        modifier = Modifier.size(40.dp),
                        tint = MaterialTheme.colorScheme.onError
                    )
                }

                // Botão Speakerphone
                IconButton(
                    onClick = {
                        scope.launch {
                            // Toggle speaker no backend (futuro)
                            // MePassaClientWrapper.toggleSpeakerphone(callId)
                            // Toggle speaker local (AudioManager)
                            isSpeakerOn = audioManager.toggleSpeakerphone()
                        }
                    },
                    modifier = Modifier
                        .size(72.dp)
                        .background(
                            if (isSpeakerOn) MaterialTheme.colorScheme.primary
                            else MaterialTheme.colorScheme.surfaceVariant,
                            CircleShape
                        )
                ) {
                    Icon(
                        imageVector = if (isSpeakerOn) Icons.Default.VolumeUp else Icons.Default.VolumeDown,
                        contentDescription = if (isSpeakerOn) "Speaker Off" else "Speaker On",
                        modifier = Modifier.size(32.dp),
                        tint = if (isSpeakerOn) MaterialTheme.colorScheme.onPrimary
                        else MaterialTheme.colorScheme.onSurfaceVariant
                    )
                }
            }
        }
    }
}

/**
 * Formata duração em segundos para MM:SS
 */
private fun formatDuration(seconds: Int): String {
    val minutes = seconds / 60
    val secs = seconds % 60
    return String.format("%02d:%02d", minutes, secs)
}
