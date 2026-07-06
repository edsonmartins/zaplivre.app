package com.zaplivre.ui.screens.media

import android.net.Uri
import android.widget.VideoView
import androidx.compose.foundation.background
import androidx.compose.foundation.clickable
import androidx.compose.foundation.layout.*
import androidx.compose.material.icons.Icons
import androidx.compose.material.icons.filled.Pause
import androidx.compose.material.icons.filled.PlayArrow
import androidx.compose.material3.*
import androidx.compose.runtime.*
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.graphics.Color
import androidx.compose.ui.platform.LocalContext
import androidx.compose.ui.unit.dp
import androidx.compose.ui.viewinterop.AndroidView
import com.zaplivre.core.ZapLivreClientWrapper
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.launch
import kotlinx.coroutines.withContext
import uniffi.zaplivre.FfiMedia
import java.io.File

/**
 * VideoPlayerView - Video player with playback controls
 */
@Composable
fun VideoPlayerView(
    media: FfiMedia,
    onToggleUI: () -> Unit,
    modifier: Modifier = Modifier
) {
    val context = LocalContext.current
    val scope = rememberCoroutineScope()

    var isPlaying by remember { mutableStateOf(false) }
    var isLoading by remember { mutableStateOf(true) }
    var videoUri by remember { mutableStateOf<Uri?>(null) }
    var showControls by remember { mutableStateOf(true) }
    var videoView by remember { mutableStateOf<VideoView?>(null) }

    // Load video file
    LaunchedEffect(media.id) {
        scope.launch {
            try {
                val uri = withContext(Dispatchers.IO) {
                    // Try local path first
                    media.localPath?.let { path ->
                        val file = File(path)
                        if (file.exists()) {
                            return@withContext Uri.fromFile(file)
                        }
                    }

                    // Download video to cache
                    val videoData = ZapLivreClientWrapper.downloadMedia(media.mediaHash)
                    val cacheFile = File(context.cacheDir, "video_${media.id}.mp4")
                    cacheFile.writeBytes(videoData)

                    Uri.fromFile(cacheFile)
                }

                videoUri = uri
            } catch (e: Exception) {
                println("❌ Error loading video: ${e.message}")
            } finally {
                isLoading = false
            }
        }
    }

    Box(
        modifier = modifier
            .fillMaxSize()
            .background(Color.Black)
            .clickable {
                showControls = !showControls
                onToggleUI()
            }
    ) {
        if (isLoading) {
            CircularProgressIndicator(
                color = Color.White,
                modifier = Modifier.align(Alignment.Center)
            )
        } else {
            videoUri?.let { uri ->
                AndroidView(
                    factory = { ctx ->
                        VideoView(ctx).apply {
                            setVideoURI(uri)
                            setOnPreparedListener { player ->
                                isLoading = false
                                player.setOnVideoSizeChangedListener { _, _, _ ->
                                    // Adjust aspect ratio
                                }
                            }
                            setOnCompletionListener {
                                isPlaying = false
                            }
                            videoView = this
                        }
                    },
                    modifier = Modifier.fillMaxSize()
                )

                // Play/Pause control overlay
                if (showControls) {
                    Box(
                        modifier = Modifier
                            .fillMaxSize()
                            .background(Color.Black.copy(alpha = 0.3f)),
                        contentAlignment = Alignment.Center
                    ) {
                        FloatingActionButton(
                            onClick = {
                                videoView?.let { vv ->
                                    if (isPlaying) {
                                        vv.pause()
                                        isPlaying = false
                                    } else {
                                        vv.start()
                                        isPlaying = true
                                    }
                                }
                            },
                            containerColor = MaterialTheme.colorScheme.primary,
                            modifier = Modifier.size(72.dp)
                        ) {
                            Icon(
                                imageVector = if (isPlaying) Icons.Default.Pause else Icons.Default.PlayArrow,
                                contentDescription = if (isPlaying) "Pause" else "Play",
                                tint = Color.White,
                                modifier = Modifier.size(36.dp)
                            )
                        }
                    }

                    // Video info overlay (bottom)
                    media.durationSeconds?.let { duration ->
                        Surface(
                            color = Color.Black.copy(alpha = 0.7f),
                            modifier = Modifier
                                .align(Alignment.BottomStart)
                                .padding(16.dp)
                        ) {
                            Column(
                                modifier = Modifier.padding(12.dp),
                                verticalArrangement = Arrangement.spacedBy(4.dp)
                            ) {
                                media.fileName?.let { name ->
                                    Text(
                                        text = name,
                                        color = Color.White,
                                        style = MaterialTheme.typography.bodyMedium
                                    )
                                }

                                Row(
                                    horizontalArrangement = Arrangement.spacedBy(16.dp)
                                ) {
                                    Text(
                                        text = formatDuration(duration),
                                        color = Color.White.copy(alpha = 0.8f),
                                        style = MaterialTheme.typography.bodySmall
                                    )

                                    media.width?.let { w ->
                                        media.height?.let { h ->
                                            Text(
                                                text = "${w}x${h}",
                                                color = Color.White.copy(alpha = 0.8f),
                                                style = MaterialTheme.typography.bodySmall
                                            )
                                        }
                                    }

                                    media.fileSize?.let { size ->
                                        Text(
                                            text = formatFileSize(size),
                                            color = Color.White.copy(alpha = 0.8f),
                                            style = MaterialTheme.typography.bodySmall
                                        )
                                    }
                                }
                            }
                        }
                    }
                }
            } ?: run {
                Text(
                    text = "Erro ao carregar vídeo",
                    color = Color.White,
                    style = MaterialTheme.typography.bodyLarge,
                    modifier = Modifier.align(Alignment.Center)
                )
            }
        }
    }

    // Cleanup on dispose
    DisposableEffect(Unit) {
        onDispose {
            videoView?.stopPlayback()
        }
    }
}

/**
 * Format duration seconds to MM:SS
 */
private fun formatDuration(seconds: Int): String {
    val mins = seconds / 60
    val secs = seconds % 60
    return String.format("%d:%02d", mins, secs)
}

/**
 * Format file size to human readable format
 */
private fun formatFileSize(bytes: Long): String {
    return when {
        bytes < 1024 -> "$bytes B"
        bytes < 1024 * 1024 -> "${bytes / 1024} KB"
        bytes < 1024 * 1024 * 1024 -> "${bytes / (1024 * 1024)} MB"
        else -> "${bytes / (1024 * 1024 * 1024)} GB"
    }
}
