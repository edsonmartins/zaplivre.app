package com.zaplivre.ui.components

import android.net.Uri
import androidx.compose.foundation.background
import androidx.compose.foundation.layout.*
import androidx.compose.foundation.lazy.LazyRow
import androidx.compose.foundation.lazy.items
import androidx.compose.foundation.shape.RoundedCornerShape
import androidx.compose.material.icons.Icons
import androidx.compose.material.icons.filled.Close
import androidx.compose.material.icons.filled.Image
import androidx.compose.material.icons.filled.Send
import androidx.compose.material3.*
import androidx.compose.runtime.Composable
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.draw.clip
import androidx.compose.ui.graphics.Color
import androidx.compose.ui.layout.ContentScale
import androidx.compose.ui.unit.dp
import coil.compose.AsyncImage
import com.zaplivre.core.MediaItem

/**
 * Preview of selected images before sending
 *
 * Shows a horizontal scrollable list of thumbnails with remove buttons
 * and a send button to upload all images
 *
 * @param selectedImages List of selected media items
 * @param onRemoveImage Callback when an image should be removed
 * @param onSendImages Callback when send button is clicked
 * @param modifier Modifier for styling
 */
@Composable
fun SelectedImagesPreview(
    selectedImages: List<MediaItem>,
    onRemoveImage: (Uri) -> Unit,
    onSendImages: () -> Unit,
    modifier: Modifier = Modifier
) {
    if (selectedImages.isEmpty()) return

    Surface(
        modifier = modifier.fillMaxWidth(),
        color = MaterialTheme.colorScheme.surfaceVariant,
        tonalElevation = 2.dp
    ) {
        Column(
            modifier = Modifier.padding(8.dp)
        ) {
            // Header with count and send button
            Row(
                modifier = Modifier.fillMaxWidth(),
                horizontalArrangement = Arrangement.SpaceBetween,
                verticalAlignment = Alignment.CenterVertically
            ) {
                Text(
                    text = "${selectedImages.size} image${if (selectedImages.size > 1) "s" else ""} selected",
                    style = MaterialTheme.typography.labelMedium,
                    color = MaterialTheme.colorScheme.onSurfaceVariant
                )

                Button(
                    onClick = onSendImages,
                    contentPadding = PaddingValues(horizontal = 16.dp, vertical = 8.dp)
                ) {
                    Icon(
                        imageVector = Icons.Default.Send,
                        contentDescription = null,
                        modifier = Modifier.size(18.dp)
                    )
                    Spacer(modifier = Modifier.width(8.dp))
                    Text("Send")
                }
            }

            Spacer(modifier = Modifier.height(8.dp))

            // Horizontal scrollable image list
            LazyRow(
                horizontalArrangement = Arrangement.spacedBy(8.dp)
            ) {
                items(selectedImages) { mediaItem ->
                    SelectedImageThumbnail(
                        uri = mediaItem.uri,
                        fileName = mediaItem.fileName,
                        onRemove = { onRemoveImage(mediaItem.uri) }
                    )
                }
            }
        }
    }
}

/**
 * Single thumbnail in the selected images preview
 */
@Composable
private fun SelectedImageThumbnail(
    uri: Uri,
    fileName: String?,
    onRemove: () -> Unit,
    modifier: Modifier = Modifier
) {
    Box(
        modifier = modifier
            .size(80.dp)
            .clip(RoundedCornerShape(8.dp))
    ) {
        // Image thumbnail
        AsyncImage(
            model = uri,
            contentDescription = fileName ?: "Selected image",
            contentScale = ContentScale.Crop,
            modifier = Modifier.fillMaxSize()
        )

        // Remove button overlay
        IconButton(
            onClick = onRemove,
            modifier = Modifier
                .align(Alignment.TopEnd)
                .size(24.dp)
                .background(
                    color = Color.Black.copy(alpha = 0.6f),
                    shape = RoundedCornerShape(12.dp)
                )
        ) {
            Icon(
                imageVector = Icons.Default.Close,
                contentDescription = "Remove image",
                tint = Color.White,
                modifier = Modifier.size(16.dp)
            )
        }
    }
}

/**
 * Compact version for chat input area
 */
@Composable
fun CompactSelectedImagesIndicator(
    selectedCount: Int,
    onClear: () -> Unit,
    onView: () -> Unit,
    modifier: Modifier = Modifier
) {
    if (selectedCount == 0) return

    Surface(
        onClick = onView,
        modifier = modifier,
        shape = RoundedCornerShape(16.dp),
        color = MaterialTheme.colorScheme.primaryContainer
    ) {
        Row(
            modifier = Modifier.padding(horizontal = 12.dp, vertical = 8.dp),
            horizontalArrangement = Arrangement.spacedBy(8.dp),
            verticalAlignment = Alignment.CenterVertically
        ) {
            Icon(
                imageVector = Icons.Filled.Image,
                contentDescription = null,
                modifier = Modifier.size(18.dp),
                tint = MaterialTheme.colorScheme.onPrimaryContainer
            )

            Text(
                text = "$selectedCount",
                style = MaterialTheme.typography.labelLarge,
                color = MaterialTheme.colorScheme.onPrimaryContainer
            )

            IconButton(
                onClick = onClear,
                modifier = Modifier.size(20.dp)
            ) {
                Icon(
                    imageVector = Icons.Default.Close,
                    contentDescription = "Clear selection",
                    modifier = Modifier.size(16.dp),
                    tint = MaterialTheme.colorScheme.onPrimaryContainer
                )
            }
        }
    }
}
