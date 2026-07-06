package com.zaplivre.ui.components

import androidx.compose.foundation.clickable
import androidx.compose.foundation.layout.*
import androidx.compose.foundation.lazy.grid.GridCells
import androidx.compose.foundation.lazy.grid.LazyVerticalGrid
import androidx.compose.foundation.lazy.grid.items
import androidx.compose.foundation.shape.RoundedCornerShape
import androidx.compose.material.icons.Icons
import androidx.compose.material.icons.filled.ArrowBack
import androidx.compose.material3.*
import androidx.compose.runtime.Composable
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.draw.clip
import androidx.compose.ui.layout.ContentScale
import androidx.compose.ui.unit.dp
import coil.compose.AsyncImage

/**
 * Image item for gallery
 */
data class GalleryImage(
    val id: Long,
    val url: String,
    val thumbnailUrl: String?,
    val width: Int?,
    val height: Int?,
    val fileName: String?
)

/**
 * Grid gallery view for images in a conversation
 *
 * @param images List of images to display
 * @param onImageClick Callback when an image is clicked
 * @param modifier Modifier for styling
 */
@Composable
fun ImageGallery(
    images: List<GalleryImage>,
    onImageClick: (GalleryImage) -> Unit,
    modifier: Modifier = Modifier
) {
    if (images.isEmpty()) {
        EmptyGalleryPlaceholder(modifier = modifier)
        return
    }

    LazyVerticalGrid(
        columns = GridCells.Fixed(3),
        contentPadding = PaddingValues(4.dp),
        horizontalArrangement = Arrangement.spacedBy(4.dp),
        verticalArrangement = Arrangement.spacedBy(4.dp),
        modifier = modifier
    ) {
        items(images) { image ->
            ImageThumbnail(
                image = image,
                onClick = { onImageClick(image) }
            )
        }
    }
}

/**
 * Single image thumbnail in the gallery
 */
@Composable
private fun ImageThumbnail(
    image: GalleryImage,
    onClick: () -> Unit,
    modifier: Modifier = Modifier
) {
    Box(
        modifier = modifier
            .aspectRatio(1f)
            .clip(RoundedCornerShape(4.dp))
            .clickable(onClick = onClick)
    ) {
        AsyncImage(
            model = image.thumbnailUrl ?: image.url,
            contentDescription = image.fileName ?: "Image",
            contentScale = ContentScale.Crop,
            modifier = Modifier.fillMaxSize()
        )
    }
}

/**
 * Placeholder when there are no images
 */
@Composable
private fun EmptyGalleryPlaceholder(
    modifier: Modifier = Modifier
) {
    Box(
        modifier = modifier
            .fillMaxSize()
            .padding(32.dp),
        contentAlignment = Alignment.Center
    ) {
        Column(
            horizontalAlignment = Alignment.CenterHorizontally,
            verticalArrangement = Arrangement.spacedBy(8.dp)
        ) {
            Text(
                text = "No images yet",
                style = MaterialTheme.typography.bodyLarge,
                color = MaterialTheme.colorScheme.onSurfaceVariant
            )
            Text(
                text = "Share photos to see them here",
                style = MaterialTheme.typography.bodyMedium,
                color = MaterialTheme.colorScheme.onSurfaceVariant.copy(alpha = 0.7f)
            )
        }
    }
}

/**
 * Gallery screen with top bar
 */
@OptIn(ExperimentalMaterial3Api::class)
@Composable
fun ImageGalleryScreen(
    conversationName: String,
    images: List<GalleryImage>,
    onImageClick: (GalleryImage) -> Unit,
    onNavigateBack: () -> Unit,
    modifier: Modifier = Modifier
) {
    Scaffold(
        topBar = {
            TopAppBar(
                title = {
                    Column {
                        Text(text = conversationName)
                        Text(
                            text = "${images.size} photos",
                            style = MaterialTheme.typography.bodySmall,
                            color = MaterialTheme.colorScheme.onSurfaceVariant
                        )
                    }
                },
                navigationIcon = {
                    IconButton(onClick = onNavigateBack) {
                        Icon(
                            imageVector = Icons.Filled.ArrowBack,
                            contentDescription = "Back"
                        )
                    }
                }
            )
        }
    ) { padding ->
        ImageGallery(
            images = images,
            onImageClick = onImageClick,
            modifier = modifier
                .fillMaxSize()
                .padding(padding)
        )
    }
}
