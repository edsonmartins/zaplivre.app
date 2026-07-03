package com.mepassa.ui.screens.media

import android.content.ContentValues
import android.content.Intent
import android.provider.MediaStore
import android.widget.Toast
import androidx.compose.foundation.background
import androidx.compose.foundation.gestures.detectTapGestures
import androidx.compose.foundation.gestures.rememberTransformableState
import androidx.compose.foundation.gestures.transformable
import androidx.compose.foundation.layout.*
import androidx.compose.foundation.pager.HorizontalPager
import androidx.compose.foundation.pager.rememberPagerState
import androidx.compose.material.icons.Icons
import androidx.compose.material.icons.filled.ArrowBack
import androidx.compose.material.icons.filled.Download
import androidx.compose.material.icons.filled.PlayArrow
import androidx.compose.material.icons.filled.Share
import androidx.compose.material3.*
import androidx.compose.runtime.*
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.graphics.Color
import androidx.compose.ui.graphics.graphicsLayer
import androidx.compose.ui.input.pointer.pointerInput
import androidx.compose.ui.unit.dp
import androidx.core.content.FileProvider
import coil.compose.AsyncImage
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.launch
import kotlinx.coroutines.withContext
import uniffi.mepassa.FfiMedia
import uniffi.mepassa.FfiMediaType
import java.io.File

/**
 * MediaViewerScreen - visualizador fullscreen (UX-09)
 *
 * Imagens: zoom por pinça + pan + double-tap para resetar, swipe entre itens.
 * Vídeos/documentos: abrir com app externo (sem player embutido nesta fase).
 * Compartilhar via FileProvider; salvar imagens na galeria via MediaStore.
 */
@OptIn(ExperimentalMaterial3Api::class)
@Composable
fun MediaViewerScreen(
    mediaItems: List<FfiMedia>,
    initialIndex: Int = 0,
    onNavigateBack: () -> Unit
) {
    val context = androidx.compose.ui.platform.LocalContext.current
    val scope = rememberCoroutineScope()
    val pagerState = rememberPagerState(
        initialPage = initialIndex.coerceIn(0, (mediaItems.size - 1).coerceAtLeast(0)),
        pageCount = { mediaItems.size }
    )

    val current = mediaItems.getOrNull(pagerState.currentPage)

    fun fileFor(media: FfiMedia?): File? =
        media?.localPath?.let { path -> File(path).takeIf { it.exists() } }

    fun shareCurrent() {
        val media = current ?: return
        val file = fileFor(media) ?: run {
            Toast.makeText(context, "Arquivo não disponível localmente", Toast.LENGTH_SHORT).show()
            return
        }
        try {
            val uri = FileProvider.getUriForFile(
                context,
                "${context.packageName}.fileprovider",
                file
            )
            val intent = Intent(Intent.ACTION_SEND).apply {
                type = media.mimeType ?: "*/*"
                putExtra(Intent.EXTRA_STREAM, uri)
                addFlags(Intent.FLAG_GRANT_READ_URI_PERMISSION)
            }
            context.startActivity(Intent.createChooser(intent, "Compartilhar"))
        } catch (e: Exception) {
            Toast.makeText(context, "Falha ao compartilhar: ${e.message}", Toast.LENGTH_SHORT).show()
        }
    }

    fun saveCurrentToGallery() {
        val media = current ?: return
        val file = fileFor(media) ?: run {
            Toast.makeText(context, "Arquivo não disponível localmente", Toast.LENGTH_SHORT).show()
            return
        }
        scope.launch(Dispatchers.IO) {
            try {
                val isVideo = media.mediaType == FfiMediaType.VIDEO
                val collection = if (isVideo) {
                    MediaStore.Video.Media.EXTERNAL_CONTENT_URI
                } else {
                    MediaStore.Images.Media.EXTERNAL_CONTENT_URI
                }
                val values = ContentValues().apply {
                    put(MediaStore.MediaColumns.DISPLAY_NAME, media.fileName ?: file.name)
                    put(MediaStore.MediaColumns.MIME_TYPE, media.mimeType ?: "application/octet-stream")
                    put(MediaStore.MediaColumns.RELATIVE_PATH, "Pictures/ZapLivre")
                }
                val uri = context.contentResolver.insert(collection, values)
                    ?: throw IllegalStateException("MediaStore recusou o insert")
                context.contentResolver.openOutputStream(uri)?.use { out ->
                    file.inputStream().use { it.copyTo(out) }
                }
                withContext(Dispatchers.Main) {
                    Toast.makeText(context, "Salvo na galeria", Toast.LENGTH_SHORT).show()
                }
            } catch (e: Exception) {
                withContext(Dispatchers.Main) {
                    Toast.makeText(context, "Falha ao salvar: ${e.message}", Toast.LENGTH_SHORT).show()
                }
            }
        }
    }

    fun openExternally() {
        val media = current ?: return
        val file = fileFor(media) ?: return
        try {
            val uri = FileProvider.getUriForFile(context, "${context.packageName}.fileprovider", file)
            val intent = Intent(Intent.ACTION_VIEW).apply {
                setDataAndType(uri, media.mimeType ?: "*/*")
                addFlags(Intent.FLAG_GRANT_READ_URI_PERMISSION)
            }
            context.startActivity(intent)
        } catch (e: Exception) {
            Toast.makeText(context, "Nenhum app para abrir este arquivo", Toast.LENGTH_SHORT).show()
        }
    }

    Scaffold(
        containerColor = Color.Black,
        topBar = {
            TopAppBar(
                title = {
                    Text(
                        current?.fileName
                            ?: "${pagerState.currentPage + 1} / ${mediaItems.size}"
                    )
                },
                navigationIcon = {
                    IconButton(onClick = onNavigateBack) {
                        Icon(Icons.Filled.ArrowBack, contentDescription = "Voltar")
                    }
                },
                actions = {
                    IconButton(onClick = { shareCurrent() }) {
                        Icon(Icons.Filled.Share, contentDescription = "Compartilhar")
                    }
                    IconButton(onClick = { saveCurrentToGallery() }) {
                        Icon(Icons.Filled.Download, contentDescription = "Salvar na galeria")
                    }
                },
                colors = TopAppBarDefaults.topAppBarColors(
                    containerColor = Color.Black.copy(alpha = 0.6f),
                    titleContentColor = Color.White,
                    navigationIconContentColor = Color.White,
                    actionIconContentColor = Color.White
                )
            )
        }
    ) { padding ->
        if (mediaItems.isEmpty()) {
            Box(
                modifier = Modifier.fillMaxSize().padding(padding),
                contentAlignment = Alignment.Center
            ) {
                Text("Nenhuma mídia", color = Color.White)
            }
            return@Scaffold
        }

        HorizontalPager(
            state = pagerState,
            modifier = Modifier
                .fillMaxSize()
                .background(Color.Black)
                .padding(padding)
        ) { page ->
            val media = mediaItems[page]
            when (media.mediaType) {
                FfiMediaType.IMAGE -> ZoomableImage(media)
                else -> {
                    // Vídeo/áudio/documento: sem player embutido nesta fase -
                    // thumbnail (se houver) + abrir com app externo
                    Box(
                        modifier = Modifier.fillMaxSize(),
                        contentAlignment = Alignment.Center
                    ) {
                        Column(
                            horizontalAlignment = Alignment.CenterHorizontally,
                            verticalArrangement = Arrangement.spacedBy(16.dp)
                        ) {
                            media.thumbnailPath?.let { thumb ->
                                AsyncImage(
                                    model = File(thumb),
                                    contentDescription = media.fileName,
                                    modifier = Modifier.fillMaxWidth(0.8f)
                                )
                            }
                            Button(onClick = { openExternally() }) {
                                Icon(Icons.Filled.PlayArrow, contentDescription = null)
                                Spacer(Modifier.width(8.dp))
                                Text("Abrir")
                            }
                            Text(
                                media.fileName ?: media.mediaHash.take(16),
                                color = Color.White,
                                style = MaterialTheme.typography.bodyMedium
                            )
                        }
                    }
                }
            }
        }
    }
}

/** Imagem com pinça para zoom, pan e double-tap para resetar */
@Composable
private fun ZoomableImage(media: FfiMedia) {
    var scale by remember { mutableStateOf(1f) }
    var offsetX by remember { mutableStateOf(0f) }
    var offsetY by remember { mutableStateOf(0f) }

    val transformState = rememberTransformableState { zoomChange, panChange, _ ->
        scale = (scale * zoomChange).coerceIn(1f, 6f)
        if (scale > 1f) {
            offsetX += panChange.x
            offsetY += panChange.y
        } else {
            offsetX = 0f
            offsetY = 0f
        }
    }

    Box(
        modifier = Modifier
            .fillMaxSize()
            .pointerInput(Unit) {
                detectTapGestures(
                    onDoubleTap = {
                        scale = 1f
                        offsetX = 0f
                        offsetY = 0f
                    }
                )
            }
            .transformable(transformState),
        contentAlignment = Alignment.Center
    ) {
        AsyncImage(
            model = media.localPath?.let { File(it) },
            contentDescription = media.fileName,
            modifier = Modifier
                .fillMaxSize()
                .graphicsLayer(
                    scaleX = scale,
                    scaleY = scale,
                    translationX = offsetX,
                    translationY = offsetY
                )
        )
    }
}
