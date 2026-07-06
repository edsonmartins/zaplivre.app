package com.zaplivre.ui.screens.call

import android.util.Log
import androidx.camera.view.PreviewView
import androidx.compose.foundation.background
import androidx.compose.foundation.layout.*
import androidx.compose.foundation.shape.CircleShape
import androidx.compose.foundation.shape.RoundedCornerShape
import androidx.compose.material.icons.Icons
import androidx.compose.material.icons.filled.*
import androidx.compose.material3.*
import androidx.compose.runtime.*
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.draw.clip
import androidx.compose.ui.graphics.Color
import androidx.compose.ui.platform.LocalContext
import androidx.compose.ui.platform.LocalLifecycleOwner
import androidx.compose.ui.text.font.FontWeight
import androidx.compose.ui.unit.dp
import androidx.compose.ui.unit.sp
import androidx.compose.ui.viewinterop.AndroidView
import com.google.accompanist.permissions.ExperimentalPermissionsApi
import com.google.accompanist.permissions.isGranted
import com.google.accompanist.permissions.rememberPermissionState
import com.zaplivre.core.ZapLivreClientWrapper
import com.zaplivre.ui.components.RemoteVideoView
import com.zaplivre.voip.CameraManager
import com.zaplivre.voip.CallAudioManager
import com.zaplivre.voip.VideoEncoder
import kotlinx.coroutines.launch
import uniffi.zaplivre.FfiVideoCodec

/**
 * VideoCallScreen - UI for video call with local preview and remote video
 */
@OptIn(ExperimentalPermissionsApi::class)
@Composable
fun VideoCallScreen(
    callId: String,
    peerName: String,
    onHangup: () -> Unit,
    modifier: Modifier = Modifier
) {
    val context = LocalContext.current
    val lifecycleOwner = LocalLifecycleOwner.current
    val scope = rememberCoroutineScope()

    val audioManager = remember { CallAudioManager(context) }

    DisposableEffect(Unit) {
        audioManager.startCall()
        onDispose {
            audioManager.stopCall()
        }
    }

    // Camera permission
    val cameraPermissionState = rememberPermissionState(android.Manifest.permission.CAMERA)

    // Request permission on first composition
    LaunchedEffect(Unit) {
        if (!cameraPermissionState.status.isGranted) {
            cameraPermissionState.launchPermissionRequest()
        }
    }

    // State
    var videoEnabled by remember { mutableStateOf(true) }
    var isMuted by remember { mutableStateOf(false) }
    var callDuration by remember { mutableStateOf(0) }

    // Camera manager
    val cameraManager = remember { CameraManager(context) }
    val videoEncoder = remember {
        VideoEncoder(width = 640, height = 480) { encoded, _ ->
            scope.launch {
                try {
                    ZapLivreClientWrapper.sendVideoFrame(
                        callId = callId,
                        frameData = encoded,
                        width = 640u,
                        height = 480u
                    )
                } catch (e: Exception) {
                    // Frame drop is acceptable
                }
            }
        }
    }

    // Preview views
    var localPreviewView by remember { mutableStateOf<PreviewView?>(null) }
    
    DisposableEffect(cameraPermissionState.status.isGranted, videoEnabled, localPreviewView) {
        // Start camera when screen appears and permission is granted
        if (cameraPermissionState.status.isGranted && videoEnabled && localPreviewView != null) {
            videoEncoder.start()
            cameraManager.startCamera(
                lifecycleOwner = lifecycleOwner,
                previewView = localPreviewView!!,
                onFrameCallback = { data, width, height ->
                    val pts = System.nanoTime() / 1000
                    videoEncoder.encodeFrame(data, pts)
                }
            )

            // Enable video track on WebRTC
            scope.launch {
                try {
                    ZapLivreClientWrapper.enableVideo(callId, FfiVideoCodec.H264)
                } catch (e: Exception) {
                    Log.e("VideoCallScreen", "Failed to enable video", e)
                }
            }
        }

        onDispose {
            cameraManager.stopCamera()
            cameraManager.release()
            videoEncoder.stop()
        }
    }

    Box(
        modifier = modifier.fillMaxSize()
    ) {
        // Remote video (full screen)
        RemoteVideoView(
            callId = callId,
            modifier = Modifier.fillMaxSize()
        )

        // Local video preview (PiP - top right corner)
        if (videoEnabled) {
            Box(
                modifier = Modifier
                    .align(Alignment.TopEnd)
                    .padding(16.dp)
                    .size(120.dp, 160.dp)
                    .clip(RoundedCornerShape(12.dp))
                    .background(Color.Black)
            ) {
                AndroidView(
                    factory = { ctx ->
                        PreviewView(ctx).also { preview ->
                            localPreviewView = preview
                            if (cameraPermissionState.status.isGranted && videoEnabled) {
                                videoEncoder.start()
                                cameraManager.startCamera(
                                    lifecycleOwner = lifecycleOwner,
                                    previewView = preview,
                                    onFrameCallback = { data, width, height ->
                                        val pts = System.nanoTime() / 1000
                                        videoEncoder.encodeFrame(data, pts)
                                    }
                                )
                            }
                        }
                    },
                    modifier = Modifier.fillMaxSize()
                )
            }
        }

        // Controls overlay (bottom)
        Column(
            modifier = Modifier
                .align(Alignment.BottomCenter)
                .fillMaxWidth()
                .background(Color.Black.copy(alpha = 0.5f))
                .padding(24.dp),
            horizontalAlignment = Alignment.CenterHorizontally,
            verticalArrangement = Arrangement.spacedBy(16.dp)
        ) {
            // Call info
            Text(
                text = peerName,
                color = Color.White,
                fontSize = 20.sp,
                fontWeight = FontWeight.Medium
            )

            // Call duration
            Text(
                text = formatDuration(callDuration),
                color = Color.White.copy(alpha = 0.8f),
                fontSize = 16.sp
            )

            // Control buttons row
            Row(
                horizontalArrangement = Arrangement.spacedBy(20.dp),
                verticalAlignment = Alignment.CenterVertically
            ) {
                // Video toggle
                IconButton(
                    onClick = {
                        if (!cameraPermissionState.status.isGranted) {
                            // Request permission if not granted
                            cameraPermissionState.launchPermissionRequest()
                            return@IconButton
                        }

                        videoEnabled = !videoEnabled
                        if (videoEnabled && localPreviewView != null) {
                            videoEncoder.start()
                            cameraManager.startCamera(
                                lifecycleOwner = lifecycleOwner,
                                previewView = localPreviewView!!,
                                onFrameCallback = { data, width, height ->
                                    val pts = System.nanoTime() / 1000
                                    videoEncoder.encodeFrame(data, pts)
                                }
                            )
                            // Enable video track on WebRTC
                            scope.launch {
                                try {
                                    ZapLivreClientWrapper.enableVideo(callId, FfiVideoCodec.H264)
                                } catch (e: Exception) {
                                    Log.e("VideoCallScreen", "Failed to enable video", e)
                                }
                            }
                        } else {
                            cameraManager.stopCamera()
                            videoEncoder.stop()
                            // Disable video track on WebRTC
                            scope.launch {
                                try {
                                    ZapLivreClientWrapper.disableVideo(callId)
                                } catch (e: Exception) {
                                    Log.e("VideoCallScreen", "Failed to disable video", e)
                                }
                            }
                        }
                    },
                    modifier = Modifier
                        .size(56.dp)
                        .background(
                            color = if (videoEnabled) MaterialTheme.colorScheme.primary
                            else Color.Red,
                            shape = CircleShape
                        )
                ) {
                    Icon(
                        imageVector = if (videoEnabled) Icons.Default.Videocam
                        else Icons.Default.VideocamOff,
                        contentDescription = "Toggle video",
                        tint = Color.White
                    )
                }

                // Mute toggle
                IconButton(
                    onClick = {
                        scope.launch {
                            try {
                                ZapLivreClientWrapper.toggleMute(callId)
                                isMuted = audioManager.toggleMute()
                            } catch (e: Exception) {
                                Log.e("VideoCallScreen", "Failed to toggle mute", e)
                            }
                        }
                    },
                    modifier = Modifier
                        .size(56.dp)
                        .background(
                            color = if (isMuted) Color.Red
                            else MaterialTheme.colorScheme.primary,
                            shape = CircleShape
                        )
                ) {
                    Icon(
                        imageVector = if (isMuted) Icons.Default.MicOff
                        else Icons.Default.Mic,
                        contentDescription = "Toggle mute",
                        tint = Color.White
                    )
                }

                // Switch camera
                IconButton(
                    onClick = {
                        if (!cameraPermissionState.status.isGranted) {
                            // Request permission if not granted
                            cameraPermissionState.launchPermissionRequest()
                            return@IconButton
                        }

                        if (localPreviewView != null) {
                            cameraManager.switchCamera(
                                lifecycleOwner = lifecycleOwner,
                                previewView = localPreviewView!!,
                                onFrameCallback = { data, width, height ->
                                    val pts = System.nanoTime() / 1000
                                    videoEncoder.encodeFrame(data, pts)
                                }
                            )
                            // Notify FFI about camera switch
                            scope.launch {
                                try {
                                    ZapLivreClientWrapper.switchCamera(callId)
                                } catch (e: Exception) {
                                    Log.e("VideoCallScreen", "Failed to switch camera", e)
                                }
                            }
                        }
                    },
                    modifier = Modifier
                        .size(56.dp)
                        .background(
                            color = MaterialTheme.colorScheme.primary,
                            shape = CircleShape
                        )
                ) {
                    Icon(
                        imageVector = Icons.Default.Cameraswitch,
                        contentDescription = "Switch camera",
                        tint = Color.White
                    )
                }

                // Hangup
                IconButton(
                    onClick = {
                        scope.launch {
                            try {
                                ZapLivreClientWrapper.hangupCall(callId)
                            } catch (e: Exception) {
                                Log.e("VideoCallScreen", "Failed to hangup", e)
                            } finally {
                                onHangup()
                            }
                        }
                    },
                    modifier = Modifier
                        .size(72.dp)
                        .background(color = Color(0xFFE53935), shape = CircleShape)
                ) {
                    Icon(
                        imageVector = Icons.Default.CallEnd,
                        contentDescription = "End call",
                        tint = Color.White,
                        modifier = Modifier.size(32.dp)
                    )
                }
            }
        }
    }

    // Call duration timer
    LaunchedEffect(Unit) {
        while (true) {
            kotlinx.coroutines.delay(1000)
            callDuration++
        }
    }
}

/**
 * Format call duration (seconds → MM:SS or HH:MM:SS)
 */
private fun formatDuration(seconds: Int): String {
    val hours = seconds / 3600
    val minutes = (seconds % 3600) / 60
    val secs = seconds % 60

    return if (hours > 0) {
        String.format("%d:%02d:%02d", hours, minutes, secs)
    } else {
        String.format("%02d:%02d", minutes, secs)
    }
}
