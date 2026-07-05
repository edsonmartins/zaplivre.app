package com.mepassa.ui.screens.media

import android.graphics.BitmapFactory
import androidx.compose.foundation.Image
import androidx.compose.foundation.background
import androidx.compose.foundation.clickable
import androidx.compose.foundation.layout.*
import androidx.compose.foundation.lazy.grid.GridCells
import androidx.compose.foundation.lazy.grid.LazyVerticalGrid
import androidx.compose.foundation.lazy.grid.items
import androidx.compose.material.icons.Icons
import androidx.compose.material.icons.filled.ArrowBack
import androidx.compose.material.icons.filled.PlayCircle
import androidx.compose.material3.*
import androidx.compose.runtime.*
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.graphics.Color
import androidx.compose.ui.graphics.asImageBitmap
import androidx.compose.ui.layout.ContentScale
import androidx.compose.ui.platform.testTag
import androidx.compose.ui.unit.dp
import com.mepassa.core.MePassaClientWrapper
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.launch
import kotlinx.coroutines.withContext
import uniffi.mepassa.FfiMedia
import uniffi.mepassa.FfiMediaType

/**
 * MediaGalleryScreen - Displays all media (images/videos) from a conversation
 */
@OptIn(ExperimentalMaterial3Api::class)
@Composable
fun MediaGalleryScreen(
    conversationId: String,
    peerName: String,
    onBack: () -> Unit,
    onMediaClick: (FfiMedia, List<FfiMedia>) -> Unit,
    modifier: Modifier = Modifier
) {
    val scope = rememberCoroutineScope()
    var mediaItems by remember { mutableStateOf<List<FfiMedia>>(emptyList()) }
    var isLoading by remember { mutableStateOf(true) }
    var selectedTab by remember { mutableStateOf(MediaTab.ALL) }

    // Load media on mount
    LaunchedEffect(conversationId, selectedTab) {
        isLoading = true
        scope.launch {
            try {
                val mediaType = when (selectedTab) {
                    MediaTab.ALL -> null
                    MediaTab.IMAGES -> FfiMediaType.IMAGE
                    MediaTab.VIDEOS -> FfiMediaType.VIDEO
                }

                val media = withContext(Dispatchers.IO) {
                    MePassaClientWrapper.getConversationMedia(
                        conversationId = conversationId,
                        mediaType = mediaType,
                        limit = 500u
                    )
                }

                mediaItems = media
            } catch (e: Exception) {
                println("❌ Error loading media: ${e.message}")
            } finally {
                isLoading = false
            }
        }
    }

    Scaffold(
        topBar = {
            TopAppBar(
                title = { Text("Mídia - $peerName") },
                navigationIcon = {
                    IconButton(onClick = onBack) {
                        Icon(Icons.Default.ArrowBack, contentDescription = "Back")
                    }
                },
                colors = TopAppBarDefaults.topAppBarColors(
                    containerColor = MaterialTheme.colorScheme.primary,
                    titleContentColor = Color.White,
                    navigationIconContentColor = Color.White
                )
            )
        }
    ) { paddingValues ->
        Column(
            modifier = modifier
                .fillMaxSize()
                .padding(paddingValues)
        ) {
            // Tab row
            TabRow(
                selectedTabIndex = selectedTab.ordinal,
                containerColor = MaterialTheme.colorScheme.surface
            ) {
                MediaTab.values().forEach { tab ->
                    Tab(
                        selected = selectedTab == tab,
                        onClick = { selectedTab = tab },
                        text = { Text(tab.title) }
                    )
                }
            }

            // Content
            if (isLoading) {
                Box(
                    modifier = Modifier.fillMaxSize(),
                    contentAlignment = Alignment.Center
                ) {
                    CircularProgressIndicator()
                }
            } else if (mediaItems.isEmpty()) {
                // Empty state
                Box(
                    modifier = Modifier
                        .fillMaxSize()
                        .testTag("mediagallery_empty"),
                    contentAlignment = Alignment.Center
                ) {
                    Column(
                        horizontalAlignment = Alignment.CenterHorizontally,
                        verticalArrangement = Arrangement.spacedBy(8.dp)
                    ) {
                        Text(
                            text = "Nenhuma mídia",
                            style = MaterialTheme.typography.titleMedium,
                            color = MaterialTheme.colorScheme.onSurfaceVariant
                        )
                        Text(
                            text = "As fotos e vídeos compartilhados aparecerão aqui",
                            style = MaterialTheme.typography.bodyMedium,
                            color = MaterialTheme.colorScheme.onSurfaceVariant.copy(alpha = 0.7f)
                        )
                    }
                }
            } else {
                // Media grid
                LazyVerticalGrid(
                    columns = GridCells.Fixed(3),
                    contentPadding = PaddingValues(2.dp),
                    horizontalArrangement = Arrangement.spacedBy(2.dp),
                    verticalArrangement = Arrangement.spacedBy(2.dp),
                    modifier = Modifier
                        .fillMaxSize()
                        .testTag("mediagallery_grid")
                ) {
                    items(mediaItems) { media ->
                        MediaGridItem(
                            media = media,
                            onClick = { onMediaClick(media, mediaItems) }
                        )
                    }
                }
            }
        }
    }
}

/**
 * MediaGridItem - Single item in the media grid
 */
@Composable
fun MediaGridItem(
    media: FfiMedia,
    onClick: () -> Unit,
    modifier: Modifier = Modifier
) {
    val scope = rememberCoroutineScope()
    var thumbnail by remember { mutableStateOf<android.graphics.Bitmap?>(null) }

    // Load thumbnail
    LaunchedEffect(media.id) {
        scope.launch {
            try {
                val thumbnailData = withContext(Dispatchers.IO) {
                    // Try to load from thumbnail path first
                    media.thumbnailPath?.let { path ->
                        val file = java.io.File(path)
                        if (file.exists()) {
                            return@withContext file.readBytes()
                        }
                    }

                    // Otherwise download from media hash
                    MePassaClientWrapper.downloadMedia(media.mediaHash)
                }

                thumbnailData?.let { data ->
                    thumbnail = BitmapFactory.decodeByteArray(
                        data,
                        0,
                        data.size
                    )
                }
            } catch (e: Exception) {
                println("❌ Error loading thumbnail: ${e.message}")
            }
        }
    }

    Box(
        modifier = modifier
            .aspectRatio(1f)
            .background(MaterialTheme.colorScheme.surfaceVariant)
            .clickable(onClick = onClick)
    ) {
        thumbnail?.let { bitmap ->
            Image(
                bitmap = bitmap.asImageBitmap(),
                contentDescription = media.fileName,
                contentScale = ContentScale.Crop,
                modifier = Modifier.fillMaxSize()
            )
        }

        // Video play icon overlay
        if (media.mediaType == FfiMediaType.VIDEO) {
            Icon(
                imageVector = Icons.Default.PlayCircle,
                contentDescription = "Video",
                tint = Color.White,
                modifier = Modifier
                    .align(Alignment.Center)
                    .size(48.dp)
            )

            // Duration badge
            media.durationSeconds?.let { duration ->
                Surface(
                    color = Color.Black.copy(alpha = 0.7f),
                    shape = MaterialTheme.shapes.small,
                    modifier = Modifier
                        .align(Alignment.BottomEnd)
                        .padding(4.dp)
                ) {
                    Text(
                        text = formatDuration(duration),
                        color = Color.White,
                        style = MaterialTheme.typography.labelSmall,
                        modifier = Modifier.padding(horizontal = 6.dp, vertical = 2.dp)
                    )
                }
            }
        }
    }
}

/**
 * Media tab options
 */
enum class MediaTab(val title: String) {
    ALL("Todas"),
    IMAGES("Fotos"),
    VIDEOS("Vídeos")
}

/**
 * Format duration seconds to MM:SS
 */
private fun formatDuration(seconds: Int): String {
    val mins = seconds / 60
    val secs = seconds % 60
    return String.format("%d:%02d", mins, secs)
}
