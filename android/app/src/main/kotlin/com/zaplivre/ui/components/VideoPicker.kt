package com.zaplivre.ui.components

import android.content.Context
import android.graphics.Bitmap
import android.media.MediaMetadataRetriever
import android.net.Uri
import androidx.activity.compose.rememberLauncherForActivityResult
import androidx.activity.result.contract.ActivityResultContracts
import androidx.compose.foundation.layout.*
import androidx.compose.material.icons.Icons
import androidx.compose.material.icons.filled.VideoLibrary
import androidx.compose.material3.*
import androidx.compose.runtime.Composable
import androidx.compose.ui.Modifier
import java.io.ByteArrayOutputStream

/**
 * VideoPicker component - Allows user to select videos from device
 *
 * Uses ActivityResultContracts.PickVisualMedia for video selection.
 * Supports extracting video metadata (duration, dimensions, thumbnail).
 */
@Composable
fun VideoPickerButton(
    onVideoPicked: (VideoInfo) -> Unit,
    enabled: Boolean = true,
    context: Context,
    modifier: Modifier = Modifier
) {
    // Video picker launcher
    val videoPickerLauncher = rememberLauncherForActivityResult(
        contract = ActivityResultContracts.PickVisualMedia()
    ) { uri: Uri? ->
        uri?.let {
            // Extract video metadata
            val videoInfo = extractVideoInfo(context, it)
            if (videoInfo != null) {
                onVideoPicked(videoInfo)
            }
        }
    }

    // Video picker button
    IconButton(
        onClick = {
            videoPickerLauncher.launch(
                androidx.activity.result.PickVisualMediaRequest(
                    androidx.activity.result.contract.ActivityResultContracts.PickVisualMedia.VideoOnly
                )
            )
        },
        enabled = enabled,
        modifier = modifier
    ) {
        Icon(
            imageVector = Icons.Default.VideoLibrary,
            contentDescription = "Select video",
            tint = if (enabled) {
                MaterialTheme.colorScheme.primary
            } else {
                MaterialTheme.colorScheme.onSurface.copy(alpha = 0.38f)
            }
        )
    }
}

/**
 * Data class for video information
 */
data class VideoInfo(
    val uri: Uri,
    val fileName: String,
    val fileSize: Long,
    val durationSeconds: Int,
    val width: Int,
    val height: Int,
    val thumbnailData: ByteArray?
)

/**
 * Extract video metadata from URI
 */
private fun extractVideoInfo(context: Context, uri: Uri): VideoInfo? {
    try {
        val retriever = MediaMetadataRetriever()
        retriever.setDataSource(context, uri)

        // Extract metadata
        val duration = retriever.extractMetadata(MediaMetadataRetriever.METADATA_KEY_DURATION)?.toLongOrNull() ?: 0L
        val width = retriever.extractMetadata(MediaMetadataRetriever.METADATA_KEY_VIDEO_WIDTH)?.toIntOrNull() ?: 0
        val height = retriever.extractMetadata(MediaMetadataRetriever.METADATA_KEY_VIDEO_HEIGHT)?.toIntOrNull() ?: 0

        // Get thumbnail (first frame)
        val thumbnail = retriever.getFrameAtTime(0)
        val thumbnailData = thumbnail?.let { bitmap ->
            val outputStream = ByteArrayOutputStream()
            bitmap.compress(Bitmap.CompressFormat.JPEG, 80, outputStream)
            bitmap.recycle()
            outputStream.toByteArray()
        }

        retriever.release()

        // Get file info
        val cursor = context.contentResolver.query(uri, null, null, null, null)
        val fileName = cursor?.use {
            if (it.moveToFirst()) {
                val nameIndex = it.getColumnIndex(android.provider.OpenableColumns.DISPLAY_NAME)
                if (nameIndex >= 0) it.getString(nameIndex) else "video.mp4"
            } else "video.mp4"
        } ?: "video.mp4"

        val fileSize = cursor?.use {
            if (it.moveToFirst()) {
                val sizeIndex = it.getColumnIndex(android.provider.OpenableColumns.SIZE)
                if (sizeIndex >= 0) it.getLong(sizeIndex) else 0L
            } else 0L
        } ?: 0L

        return VideoInfo(
            uri = uri,
            fileName = fileName,
            fileSize = fileSize,
            durationSeconds = (duration / 1000).toInt(),
            width = width,
            height = height,
            thumbnailData = thumbnailData
        )
    } catch (e: Exception) {
        println("Error extracting video info: ${e.message}")
        return null
    }
}
