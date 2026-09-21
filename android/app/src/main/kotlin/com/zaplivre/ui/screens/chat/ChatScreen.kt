package com.zaplivre.ui.screens.chat

import android.net.Uri
import androidx.compose.animation.*
import androidx.compose.animation.core.*
import androidx.compose.foundation.ExperimentalFoundationApi
import androidx.compose.foundation.Image
import androidx.compose.foundation.background
import androidx.compose.foundation.clickable
import androidx.compose.foundation.combinedClickable
import androidx.compose.foundation.layout.*
import androidx.compose.foundation.lazy.LazyColumn
import androidx.compose.foundation.lazy.items
import androidx.compose.foundation.lazy.rememberLazyListState
import androidx.compose.foundation.shape.CircleShape
import androidx.compose.foundation.shape.RoundedCornerShape
import androidx.compose.material.icons.Icons
import androidx.compose.material.icons.filled.ArrowBack
import androidx.compose.material.icons.filled.Send
import androidx.compose.material.icons.filled.Phone
import androidx.compose.material.icons.filled.Photo
import androidx.compose.material.icons.filled.Mic
import androidx.compose.material.icons.filled.Close
import androidx.compose.material.icons.filled.Search
import androidx.compose.material.icons.filled.Security
import androidx.compose.material3.*
import androidx.compose.runtime.*
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.draw.clip
import androidx.compose.ui.graphics.Color
import androidx.compose.ui.layout.ContentScale
import androidx.compose.ui.platform.LocalContext
import androidx.compose.ui.platform.testTag
import androidx.compose.ui.res.stringResource
import androidx.compose.ui.text.style.TextOverflow
import androidx.compose.ui.unit.dp
import androidx.compose.ui.window.Dialog
import androidx.compose.ui.window.DialogProperties
import androidx.lifecycle.viewmodel.compose.viewModel
import coil.compose.AsyncImage
import com.zaplivre.R
import com.zaplivre.core.ZapLivreClientApi
import com.zaplivre.core.ZapLivreClientWrapper
import com.zaplivre.core.VoiceRecorderViewModel
import com.zaplivre.ui.components.ImagePickerButton
import com.zaplivre.ui.components.MessageStatusIndicator
import com.zaplivre.ui.components.PeerQrCode
import com.zaplivre.ui.components.SelectedImagesPreview
import com.zaplivre.ui.components.VoiceMessageBubble
import com.zaplivre.ui.components.VoiceRecordButton
import com.zaplivre.ui.components.VoiceRecordingInlineBar
import com.zaplivre.ui.components.ZapAvatar
import com.zaplivre.ui.components.ZapBubbleContainer
import com.zaplivre.ui.theme.ZapColor
import com.zaplivre.ui.theme.ZapMetric
import com.zaplivre.ui.theme.ZapType
import com.zaplivre.utils.rememberHapticFeedback
import kotlinx.coroutines.launch
import uniffi.zaplivre.FfiMediaType
import uniffi.zaplivre.FfiMessage
import java.io.File
import java.text.SimpleDateFormat
import java.util.*

/**
 * ChatScreen - Tela de conversa individual
 *
 * Exibe mensagens trocadas com um peer específico.
 * Permite enviar novas mensagens de texto.
 */
@OptIn(ExperimentalMaterial3Api::class, ExperimentalUnsignedTypes::class)
@Composable
fun ChatScreen(
    peerId: String,
    onNavigateBack: () -> Unit,
    onStartCall: () -> Unit,
    onOpenMediaGallery: () -> Unit = {},
    onOpenSearch: () -> Unit = {},
    chatViewModel: ChatViewModel = viewModel(key = "chat_$peerId") { ChatViewModel(peerId) }
) {
    val scope = rememberCoroutineScope()
    val listState = rememberLazyListState()
    val context = androidx.compose.ui.platform.LocalContext.current
    val haptic = rememberHapticFeedback()

    // Mensagens de texto (load/send/markRead/eventos) vivem no ViewModel;
    // os fluxos de mídia/reações continuam aqui e usam chatViewModel.refresh()
    val messages by chatViewModel.messages.collectAsState()
    var messageInput by remember { mutableStateOf("") }
    val isSendingText by chatViewModel.isSending.collectAsState()
    val peerName by chatViewModel.peerName.collectAsState()
    var isSendingMedia by remember { mutableStateOf(false) }
    val isSending = isSendingText || isSendingMedia
    val localPeerId by chatViewModel.localPeerId.collectAsState()

    // Image selection state
    var selectedImages by remember { mutableStateOf<List<Uri>>(emptyList()) }

    // Voice recorder
    val voiceRecorderViewModel = remember { VoiceRecorderViewModel(context) }
    val isVoiceRecording by voiceRecorderViewModel.isRecording.collectAsState()
    val voiceWaveform by voiceRecorderViewModel.waveform.collectAsState()
    val voiceDuration by voiceRecorderViewModel.recordingDuration.collectAsState()

    // Message actions state
    var selectedMessage by remember { mutableStateOf<FfiMessage?>(null) }
    var showDeleteDialog by remember { mutableStateOf(false) }
    var showForwardDialog by remember { mutableStateOf(false) }
    var showSecurityDialog by remember { mutableStateOf(false) }
    var securityFingerprint by remember { mutableStateOf<String?>(null) }
    var securityError by remember { mutableStateOf<String?>(null) }
    var securityVerified by remember { mutableStateOf(false) }
    var transparencyVerified by remember { mutableStateOf(false) }

    LaunchedEffect(peerId, showSecurityDialog) {
        if (showSecurityDialog) {
            val preferences = context.getSharedPreferences("security_numbers", android.content.Context.MODE_PRIVATE)
            val saved = preferences.getString("fingerprint_$peerId", null)
            securityVerified = false
            securityFingerprint = null
            securityError = null
            transparencyVerified = false
            try {
                securityFingerprint = ZapLivreClientWrapper.contactIdentityFingerprint(peerId)
                securityVerified = !securityFingerprint.isNullOrBlank() && securityFingerprint == saved
                transparencyVerified = ZapLivreClientWrapper.contactTransparencyProof(peerId).isNotBlank()
            } catch (error: Exception) {
                securityError = "Não foi possível carregar a identidade deste contato."
            }
        }
    }

    // Reactions state
    var messageReactions by remember { mutableStateOf<Map<String, List<com.zaplivre.ui.components.ReactionCount>>>(emptyMap()) }
    var showReactionPicker by remember { mutableStateOf(false) }
    var reactionPickerMessageId by remember { mutableStateOf<String?>(null) }

    // Scroll para última mensagem quando a lista muda (load inicial,
    // envio ou mensagem recebida)
    LaunchedEffect(messages.size) {
        if (messages.isNotEmpty()) {
            listState.animateScrollToItem(messages.lastIndex)
        }
    }

    LaunchedEffect(listState) {
        snapshotFlow { listState.firstVisibleItemIndex }.collect { index ->
            if (index == 0) chatViewModel.loadOlderMessages()
        }
    }

    // Feedback do envio de texto: haptics + restaura o input em caso de
    // falha (não perde a mensagem digitada)
    LaunchedEffect(Unit) {
        chatViewModel.sendResults.collect { result ->
            when (result) {
                is ChatViewModel.SendResult.Success -> haptic.light()
                is ChatViewModel.SendResult.Failure -> {
                    haptic.reject()
                    if (messageInput.isBlank()) {
                        messageInput = result.content
                    }
                }
            }
        }
    }

    // Load reactions for all messages
    LaunchedEffect(messages) {
        scope.launch {
            val reactionsMap = mutableMapOf<String, List<com.zaplivre.ui.components.ReactionCount>>()
            messages.forEach { message ->
                try {
                    val reactions = ZapLivreClientWrapper.getMessageReactions(message.messageId)

                    // Aggregate reactions by emoji
                    val reactionCounts = reactions
                        .groupBy { it.emoji }
                        .map { (emoji, reactionList) ->
                            com.zaplivre.ui.components.ReactionCount(
                                emoji = emoji,
                                count = reactionList.size,
                                hasReacted = reactionList.any { it.peerId == localPeerId }
                            )
                        }
                        .sortedByDescending { it.count }

                    reactionsMap[message.messageId] = reactionCounts
                } catch (e: Exception) {
                    android.util.Log.e("ChatScreen", "Error loading reactions for ${message.messageId}", e)
                }
            }
            messageReactions = reactionsMap
        }
    }

    // Helper functions
    fun handleReactionClick(messageId: String, emoji: String) {
        scope.launch {
            try {
                val currentReactions = messageReactions[messageId] ?: emptyList()
                val hasReacted = currentReactions.find { it.emoji == emoji }?.hasReacted ?: false

                if (hasReacted) {
                    // Remove reaction
                    ZapLivreClientWrapper.removeReaction(messageId, emoji)
                } else {
                    // Add reaction
                    ZapLivreClientWrapper.addReaction(messageId, emoji)
                    haptic.medium()  // Haptic feedback on reaction
                }

                // Reload reactions for this message
                val reactions = ZapLivreClientWrapper.getMessageReactions(messageId)
                val reactionCounts = reactions
                    .groupBy { it.emoji }
                    .map { (emoji, reactionList) ->
                        com.zaplivre.ui.components.ReactionCount(
                            emoji = emoji,
                            count = reactionList.size,
                            hasReacted = reactionList.any { it.peerId == localPeerId }
                        )
                    }
                    .sortedByDescending { it.count }

                messageReactions = messageReactions + (messageId to reactionCounts)
            } catch (e: Exception) {
                android.util.Log.e("ChatScreen", "Error toggling reaction", e)
            }
        }
    }

    fun showReactionPickerForMessage(messageId: String) {
        reactionPickerMessageId = messageId
        showReactionPicker = true
    }

    Scaffold(
        containerColor = ZapColor.chatCanvas,
        topBar = {
            TopAppBar(
                title = {
                    Row(verticalAlignment = Alignment.CenterVertically) {
                        ZapAvatar(seed = peerId, name = peerName ?: peerId, size = ZapMetric.avatarSmall, online = true)
                        Spacer(modifier = Modifier.width(10.dp))
                        Column {
                            Text(
                                text = peerName ?: peerId.take(14) + "…",
                                style = ZapType.rowName,
                                color = ZapColor.ink,
                                maxLines = 1,
                                overflow = TextOverflow.Ellipsis
                            )
                            Text(
                                text = stringResource(R.string.chat_status_connected),
                                style = ZapType.caption,
                                color = ZapColor.online
                            )
                        }
                    }
                },
                navigationIcon = {
                    IconButton(
                        onClick = onNavigateBack,
                        modifier = Modifier.testTag("chat_back")
                    ) {
                        Icon(
                            Icons.Filled.ArrowBack,
                            contentDescription = "Voltar",
                            tint = ZapColor.ink
                        )
                    }
                },
                actions = {
                    IconButton(
                        onClick = onOpenSearch,
                        modifier = Modifier.testTag("chat_search")
                    ) {
                        Icon(Icons.Default.Search, "Buscar mensagens", tint = ZapColor.slate)
                    }
                    IconButton(
                        onClick = onOpenMediaGallery,
                        modifier = Modifier.testTag("chat_media_gallery")
                    ) {
                        Icon(Icons.Default.Photo, "Galeria de mídia", tint = ZapColor.slate)
                    }
                    IconButton(onClick = onStartCall) {
                        Icon(Icons.Default.Phone, "Iniciar chamada", tint = ZapColor.primary)
                    }
                    IconButton(
                        onClick = { showSecurityDialog = true },
                        modifier = Modifier.testTag("chat_security")
                    ) {
                        Icon(Icons.Default.Security, "Verificar segurança", tint = ZapColor.slate)
                    }
                },
                colors = TopAppBarDefaults.topAppBarColors(
                    containerColor = ZapColor.canvas,
                    titleContentColor = ZapColor.ink
                )
            )
        },
        bottomBar = {
            Column {
                // Selected images preview
                if (selectedImages.isNotEmpty()) {
                    SelectedImagesPreview(
                        selectedImages = selectedImages.map { uri ->
                            com.zaplivre.core.MediaItem(
                                uri = uri,
                                type = com.zaplivre.core.MediaType.IMAGE,
                                fileName = null,
                                fileSize = null
                            )
                        },
                        onRemoveImage = { uri ->
                            selectedImages = selectedImages.filterNot { it == uri }
                        },
                        onSendImages = {
                            scope.launch {
                                try {
                                    // Send each selected image via FFI
                                    selectedImages.forEach { uri ->
                                        val inputStream = context.contentResolver.openInputStream(uri)
                                        if (inputStream != null) {
                                            val imageBytes = inputStream.use { it.readBytes() }
                                            val fileName = uri.lastPathSegment ?: "image_${System.currentTimeMillis()}.jpg"

                                            // Call FFI to send image with compression
                                            ZapLivreClientWrapper.sendImageMessage(
                                                toPeerId = peerId,
                                                imageData = imageBytes,
                                                fileName = fileName,
                                                quality = 85u
                                            )
                                        }
                                    }

                                    // Clear selection after sending
                                    selectedImages = emptyList()

                                    // Reload messages to show sent images
                                    chatViewModel.refresh()
                                } catch (e: Exception) {
                                    // TODO: Show error to user
                                    android.util.Log.e("ChatScreen", "Error sending images", e)
                                }
                            }
                        }
                    )
                }

                // Message input bar
                MessageInputBar(
                    messageInput = messageInput,
                    onMessageInputChange = { messageInput = it },
                    onSendClick = {
                        if (messageInput.isNotBlank() && !isSending) {
                            val content = messageInput.trim()
                            messageInput = ""
                            chatViewModel.sendTextMessage(content)
                        }
                    },
                    onSelectImages = { uris ->
                        selectedImages = selectedImages + uris
                    },
                    onVoiceMessageRecorded = { audioFile ->
                        scope.launch {
                            try {
                                // Read audio file bytes
                                val audioBytes = audioFile.readBytes()
                                val durationSeconds = (audioFile.length() / 16000).toInt() // Rough estimate

                                // Call FFI to send voice message
                                ZapLivreClientWrapper.sendVoiceMessage(
                                    toPeerId = peerId,
                                    audioData = audioBytes,
                                    fileName = audioFile.name,
                                    durationSeconds = durationSeconds
                                )

                                // Reload messages to show sent voice message
                                chatViewModel.refresh()
                            } catch (e: Exception) {
                                // TODO: Show error to user
                                android.util.Log.e("ChatScreen", "Error sending voice message", e)
                            }
                        }
                    },
                    onFilePicked = { uri ->
                        scope.launch {
                            try {
                                // Read file data
                                val inputStream = context.contentResolver.openInputStream(uri)
                                if (inputStream != null) {
                                    val fileBytes = inputStream.use { it.readBytes() }

                                    // Get file info
                                    val cursor = context.contentResolver.query(uri, null, null, null, null)
                                    val fileName = cursor?.use {
                                        if (it.moveToFirst()) {
                                            val nameIndex = it.getColumnIndex(android.provider.OpenableColumns.DISPLAY_NAME)
                                            if (nameIndex >= 0) it.getString(nameIndex) else "file"
                                        } else "file"
                                    } ?: "file"

                                    val mimeType = context.contentResolver.getType(uri) ?: "application/octet-stream"

                                    // Send via FFI
                                    ZapLivreClientWrapper.sendDocumentMessage(
                                        toPeerId = peerId,
                                        fileData = fileBytes,
                                        fileName = fileName,
                                        mimeType = mimeType
                                    )

                                    // Reload messages
                                    chatViewModel.refresh()
                                }
                            } catch (e: Exception) {
                                android.util.Log.e("ChatScreen", "Error sending file", e)
                            }
                        }
                    },
                    onVideoPicked = { uri ->
                        scope.launch {
                            isSendingMedia = true
                            try {
                                val videoBytes = context.contentResolver
                                    .openInputStream(uri)?.use { it.readBytes() }
                                if (videoBytes == null) {
                                    android.util.Log.e("ChatScreen", "Could not read video: $uri")
                                } else if (videoBytes.size > 100 * 1024 * 1024) {
                                    android.util.Log.e("ChatScreen", "Video too large (>100MB)")
                                } else {
                                    val fileName = uri.lastPathSegment
                                        ?.substringAfterLast('/') ?: "video.mp4"

                                    // Duração via MediaMetadataRetriever
                                    val duration = try {
                                        android.media.MediaMetadataRetriever().use { mmr ->
                                            mmr.setDataSource(context, uri)
                                            (mmr.extractMetadata(
                                                android.media.MediaMetadataRetriever.METADATA_KEY_DURATION
                                            )?.toLongOrNull() ?: 0L) / 1000L
                                        }
                                    } catch (e: Exception) {
                                        0L
                                    }

                                    ZapLivreClientWrapper.sendVideoMessage(
                                        toPeerId = peerId,
                                        videoData = videoBytes,
                                        fileName = fileName,
                                        durationSeconds = duration.toInt()
                                    )

                                    chatViewModel.refresh()
                                }
                            } catch (e: Exception) {
                                android.util.Log.e("ChatScreen", "Error sending video", e)
                            } finally {
                                isSendingMedia = false
                            }
                        }
                    },
                    voiceRecorderViewModel = voiceRecorderViewModel,
                    isSending = isSending
                )
            }
        }
    ) { paddingValues ->
        if (messages.isEmpty()) {
            Box(
                modifier = Modifier
                    .fillMaxSize()
                    .padding(paddingValues),
                contentAlignment = Alignment.Center
            ) {
                Text(
                    text = "Nenhuma mensagem ainda.\nEnvie a primeira!",
                    style = MaterialTheme.typography.bodyLarge,
                    color = MaterialTheme.colorScheme.onSurfaceVariant
                )
            }
        } else {
            LazyColumn(
                modifier = Modifier
                    .fillMaxSize()
                    .padding(paddingValues)
                    .testTag("chat_messages_list"),
                state = listState,
                contentPadding = PaddingValues(16.dp),
                verticalArrangement = Arrangement.spacedBy(8.dp)
            ) {
                items(
                    items = messages,
                    key = { it.messageId }
                ) { message ->
                    MessageBubble(
                            message = message,
                            isOwnMessage = message.senderPeerId == localPeerId,
                            reactions = messageReactions[message.messageId] ?: emptyList(),
                            onLongPress = {
                                selectedMessage = message
                            },
                            onDelete = {
                                selectedMessage = message
                                showDeleteDialog = true
                            },
                            onForward = {
                                selectedMessage = message
                                showForwardDialog = true
                            },
                            onReactionClick = { emoji ->
                                handleReactionClick(message.messageId, emoji)
                            },
                            onAddReactionClick = {
                                showReactionPickerForMessage(message.messageId)
                        }
                        )
                }
            }
        }
    }

    // Delete confirmation dialog
    if (showDeleteDialog && selectedMessage != null) {
        AlertDialog(
            onDismissRequest = { showDeleteDialog = false },
            title = { Text("Excluir mensagem") },
            text = { Text("Tem certeza que deseja excluir esta mensagem?") },
            confirmButton = {
                TextButton(
                    onClick = {
                        scope.launch {
                            try {
                                ZapLivreClientWrapper.deleteMessage(selectedMessage!!.messageId)
                                // Reload messages
                                chatViewModel.refresh()
                            } catch (e: Exception) {
                                android.util.Log.e("ChatScreen", "Error deleting message", e)
                            }
                        }
                        showDeleteDialog = false
                        selectedMessage = null
                    }
                ) {
                    Text("Excluir", color = MaterialTheme.colorScheme.error)
                }
            },
            dismissButton = {
                TextButton(onClick = { showDeleteDialog = false }) {
                    Text("Cancelar")
                }
            }
        )
    }

    // Forward dialog: seletor de conversas (UX-01)
    if (showForwardDialog && selectedMessage != null) {
        var forwardTargets by remember { mutableStateOf<List<uniffi.zaplivre.FfiConversation>>(emptyList()) }
        LaunchedEffect(Unit) {
            forwardTargets = ZapLivreClientWrapper.listConversations()
                .filter { it.peerId != null && it.peerId != peerId }
        }

        AlertDialog(
            onDismissRequest = { showForwardDialog = false },
            title = { Text("Encaminhar para...") },
            text = {
                if (forwardTargets.isEmpty()) {
                    Text("Nenhuma outra conversa disponível.")
                } else {
                    androidx.compose.foundation.lazy.LazyColumn {
                        items(forwardTargets.size) { index ->
                            val target = forwardTargets[index]
                            ListItem(
                                headlineContent = {
                                    Text(target.displayName ?: target.peerId?.take(16) ?: "?")
                                },
                                modifier = Modifier.clickable {
                                    val messageId = selectedMessage!!.messageId
                                    val toPeer = target.peerId!!
                                    showForwardDialog = false
                                    selectedMessage = null
                                    scope.launch {
                                        try {
                                            ZapLivreClientWrapper.forwardMessage(messageId, toPeer)
                                        } catch (e: Exception) {
                                            android.util.Log.e("ChatScreen", "Forward failed", e)
                                        }
                                    }
                                }
                            )
                        }
                    }
                }
            },
            confirmButton = {},
            dismissButton = {
                TextButton(onClick = {
                    showForwardDialog = false
                    selectedMessage = null
                }) {
                    Text("Cancelar")
                }
            }
        )
    }

    // Reaction picker bottom sheet
    if (showReactionPicker && reactionPickerMessageId != null) {
        com.zaplivre.ui.components.ReactionPicker(
            onReactionSelected = { emoji ->
                handleReactionClick(reactionPickerMessageId!!, emoji)
            },
            onDismiss = {
                showReactionPicker = false
                reactionPickerMessageId = null
            }
        )
    }

    if (showSecurityDialog) {
        val currentFingerprint = securityFingerprint
        AlertDialog(
            onDismissRequest = { showSecurityDialog = false },
            icon = { Icon(Icons.Default.Security, contentDescription = null, tint = ZapColor.primary) },
            title = { Text("Número de segurança") },
            text = {
                Column(horizontalAlignment = Alignment.CenterHorizontally) {
                    Text("Compare este código com o exibido no dispositivo do contato.", style = ZapType.body, color = ZapColor.slate)
                    Spacer(Modifier.height(16.dp))
                    when {
                        securityError != null -> Text(securityError!!, color = ZapColor.danger)
                        currentFingerprint == null -> CircularProgressIndicator()
                        else -> {
                            Text(currentFingerprint, style = ZapType.rowName.copy(fontFamily = androidx.compose.ui.text.font.FontFamily.Monospace), color = ZapColor.ink, modifier = Modifier.fillMaxWidth().background(ZapColor.canvas, RoundedCornerShape(12.dp)).padding(14.dp))
                            Spacer(Modifier.height(14.dp))
                            PeerQrCode(
                                payload = "zaplivre-safety:v1:$peerId:$currentFingerprint",
                                size = 190.dp
                            )
                            Text("QR Code para comparação de segurança", style = ZapType.caption, color = ZapColor.slate)
                            Spacer(Modifier.height(8.dp))
                            Text(
                                if (transparencyVerified) "Identidade incluída no log de transparência" else "Log de transparência indisponível",
                                style = ZapType.caption,
                                color = if (transparencyVerified) ZapColor.online else ZapColor.slate
                            )
                            Spacer(Modifier.height(10.dp))
                            Text(if (securityVerified) "Identidade verificada neste dispositivo" else "Ainda não verificado", color = if (securityVerified) ZapColor.online else ZapColor.slate, style = ZapType.caption)
                        }
                    }
                }
            },
            confirmButton = {
                if (currentFingerprint != null && !securityVerified) {
                    TextButton(onClick = {
                        context.getSharedPreferences("security_numbers", android.content.Context.MODE_PRIVATE).edit().putString("fingerprint_$peerId", currentFingerprint).apply()
                        securityVerified = true
                    }) { Text("Marcar como verificado") }
                } else TextButton(onClick = { showSecurityDialog = false }) { Text("Fechar") }
            },
            dismissButton = {
                if (currentFingerprint != null) TextButton(onClick = {
                    val clipboard = context.getSystemService(android.content.Context.CLIPBOARD_SERVICE) as android.content.ClipboardManager
                    clipboard.setPrimaryClip(android.content.ClipData.newPlainText("Número de segurança", currentFingerprint))
                }) { Text("Copiar") }
            }
        )
    }
}

/**
 * Barra de input de mensagem
 */
@Composable
fun MessageInputBar(
    messageInput: String,
    onMessageInputChange: (String) -> Unit,
    onSendClick: () -> Unit,
    onSelectImages: (List<Uri>) -> Unit,
    onVoiceMessageRecorded: (java.io.File) -> Unit,
    onFilePicked: (Uri) -> Unit,
    onVideoPicked: (Uri) -> Unit,
    voiceRecorderViewModel: VoiceRecorderViewModel,
    isSending: Boolean
) {
    val isVoiceRecording by voiceRecorderViewModel.isRecording.collectAsState()
    Surface(color = ZapColor.surface, modifier = Modifier.fillMaxWidth()) {
        Column {
            Divider(color = ZapColor.hairline)
            Row(
                modifier = Modifier
                    .padding(horizontal = 6.dp, vertical = 8.dp)
                    .fillMaxWidth(),
                verticalAlignment = Alignment.CenterVertically,
                horizontalArrangement = Arrangement.spacedBy(4.dp)
            ) {
                if (isVoiceRecording) {
                    // Barra de gravação inline (estilo WhatsApp): ocupa o lugar
                    // do campo de texto + pickers, sem deslocar o layout.
                    VoiceRecordingInlineBar(
                        viewModel = voiceRecorderViewModel,
                        onVoiceMessageRecorded = onVoiceMessageRecorded
                    )
                } else {
                // Image picker button
                ImagePickerButton(
                    onImagesPicked = onSelectImages,
                    maxSelection = 10,
                    enabled = !isSending
                )

                // File picker button
                com.zaplivre.ui.components.FilePickerButton(
                    onFilePicked = onFilePicked,
                    enabled = !isSending
                )

                // Video picker button
                com.zaplivre.ui.components.VideoPickerButton(
                    onVideoPicked = { info ->
                        onVideoPicked(info.uri)
                    },
                    enabled = !isSending,
                    context = androidx.compose.ui.platform.LocalContext.current
                )

                OutlinedTextField(
                    value = messageInput,
                    onValueChange = onMessageInputChange,
                    modifier = Modifier
                        .weight(1f)
                        .testTag("chat_input"),
                    placeholder = {
                        Text(stringResource(R.string.chat_input_hint), color = ZapColor.slate)
                    },
                    maxLines = 4,
                    enabled = !isSending,
                    shape = RoundedCornerShape(24.dp),
                    colors = OutlinedTextFieldDefaults.colors(
                        focusedContainerColor = ZapColor.chatCanvas,
                        unfocusedContainerColor = ZapColor.chatCanvas,
                        focusedBorderColor = Color.Transparent,
                        unfocusedBorderColor = Color.Transparent,
                        disabledBorderColor = Color.Transparent,
                        cursorColor = ZapColor.primary,
                        focusedTextColor = ZapColor.ink,
                        unfocusedTextColor = ZapColor.ink,
                    ),
                )

                // Send button (gradient) ou botão de gravar voz
                if (messageInput.isNotBlank()) {
                    Box(
                        modifier = Modifier
                            .padding(start = 2.dp)
                            .size(46.dp)
                            .clip(CircleShape)
                            .background(ZapColor.sparkBrush)
                            .clickable(enabled = !isSending, onClick = onSendClick)
                            .testTag("chat_send"),
                        contentAlignment = Alignment.Center,
                    ) {
                        if (isSending) {
                            CircularProgressIndicator(
                                modifier = Modifier.size(22.dp),
                                strokeWidth = 2.dp,
                                color = Color.White,
                            )
                        } else {
                            Icon(
                                Icons.Filled.Send,
                                contentDescription = stringResource(R.string.chat_send),
                                tint = Color.White,
                                modifier = Modifier.size(22.dp),
                            )
                        }
                    }
                } else {
                    VoiceRecordButton(
                        viewModel = voiceRecorderViewModel,
                        onVoiceMessageRecorded = onVoiceMessageRecorded
                    )
                }
                }
            }
        }
    }
}

/**
 * Bolha de mensagem individual
 */
@OptIn(ExperimentalFoundationApi::class)
@Composable
fun MessageBubble(
    message: FfiMessage,
    isOwnMessage: Boolean,
    reactions: List<com.zaplivre.ui.components.ReactionCount> = emptyList(),
    onLongPress: () -> Unit = {},
    onDelete: () -> Unit = {},
    onForward: () -> Unit = {},
    onReactionClick: (String) -> Unit = {},
    onAddReactionClick: () -> Unit = {}
) {
    var showMenu by remember { mutableStateOf(false) }

    // Mídia associada à mensagem (imagem/vídeo/áudio/documento)
    var mediaItems by remember(message.messageId) { mutableStateOf<List<uniffi.zaplivre.FfiMedia>>(emptyList()) }
    LaunchedEffect(message.messageId) {
        if (message.messageType in MEDIA_MESSAGE_TYPES) {
            mediaItems = ZapLivreClientWrapper.getMessageMedia(message.messageId)
        }
    }

    Column(modifier = Modifier.fillMaxWidth()) {
        Box {
            ZapBubbleContainer(
                outgoing = isOwnMessage,
                onLongPress = {
                    onLongPress()
                    showMenu = true
                },
            ) { fg ->
                Column {
                    when (message.messageType) {
                        "image" -> ImageMessageContent(mediaItems.firstOrNull { it.mediaType == FfiMediaType.IMAGE }, isOwnMessage)
                        "video" -> VideoMessageContent(mediaItems.firstOrNull { it.mediaType == FfiMediaType.VIDEO }, isOwnMessage)
                        "voice", "voice_message" -> VoiceMessageContent(mediaItems.firstOrNull { it.mediaType == FfiMediaType.VOICE_MESSAGE }, isOwnMessage)
                        "document" -> DocumentMessageContent(mediaItems.firstOrNull { it.mediaType == FfiMediaType.DOCUMENT }, isOwnMessage)
                        else -> message.contentPlaintext?.let { content ->
                            Text(text = content, style = ZapType.body, color = fg)
                        }
                    }
                    Spacer(modifier = Modifier.height(2.dp))
                    MessageStatusIndicator(message = message, isOwnMessage = isOwnMessage)
                }
            }

            // Context menu
            DropdownMenu(
                expanded = showMenu,
                onDismissRequest = { showMenu = false }
            ) {
                DropdownMenuItem(
                    text = { Text("Encaminhar") },
                    onClick = {
                        showMenu = false
                        onForward()
                    }
                )
                DropdownMenuItem(
                    text = { Text("Excluir", color = ZapColor.danger) },
                    onClick = {
                        showMenu = false
                        onDelete()
                    }
                )
            }
        }

        // Reaction bar
        if (reactions.isNotEmpty()) {
            Row(
                modifier = Modifier.fillMaxWidth().padding(horizontal = 10.dp),
                horizontalArrangement = if (isOwnMessage) Arrangement.End else Arrangement.Start
            ) {
                com.zaplivre.ui.components.ReactionBar(
                    reactions = reactions,
                    onReactionClick = onReactionClick,
                    onAddReactionClick = onAddReactionClick,
                    modifier = Modifier.widthIn(max = 280.dp)
                )
            }
        }
    }
}

/** Tipos de mensagem que carregam um registro de mídia associado */
private val MEDIA_MESSAGE_TYPES = setOf("image", "video", "voice", "voice_message", "document")

/**
 * Conteúdo de imagem: thumbnail renderizado a partir do arquivo local.
 */
@Composable
private fun ImageMessageContent(media: uniffi.zaplivre.FfiMedia?, isOwnMessage: Boolean) {
    var showViewer by remember { mutableStateOf(false) }
    val path = media?.localPath ?: media?.thumbnailPath
    val file = path?.let { File(it) }?.takeIf { it.exists() }

    if (file != null) {
        AsyncImage(
            model = file,
            contentDescription = media?.fileName ?: "Imagem",
            contentScale = ContentScale.Crop,
            modifier = Modifier
                .widthIn(max = 240.dp)
                .heightIn(max = 320.dp)
                .clip(RoundedCornerShape(12.dp))
                .clickable { showViewer = true }
        )
        if (showViewer && media != null) {
            MediaViewerDialog(media = media, onDismiss = { showViewer = false })
        }
    } else {
        // Mídia ainda não baixada localmente — mostra placeholder
        MediaPlaceholder("Imagem", isOwnMessage)
    }
}

/**
 * Conteúdo de vídeo: thumbnail + ícone de play.
 */
@Composable
private fun VideoMessageContent(media: uniffi.zaplivre.FfiMedia?, isOwnMessage: Boolean) {
    var showViewer by remember { mutableStateOf(false) }
    val path = media?.localPath ?: media?.thumbnailPath
    val file = path?.let { File(it) }?.takeIf { it.exists() }

    Box(
        modifier = Modifier
            .widthIn(max = 240.dp)
            .height(180.dp)
            .clip(RoundedCornerShape(12.dp))
            .background(Color.Black)
            .clickable { showViewer = true },
        contentAlignment = Alignment.Center
    ) {
        if (file != null) {
            AsyncImage(
                model = file,
                contentDescription = media?.fileName ?: "Vídeo",
                contentScale = ContentScale.Crop,
                modifier = Modifier.fillMaxSize()
            )
        }
        // Play overlay
        Box(
            modifier = Modifier
                .size(48.dp)
                .clip(CircleShape)
                .background(Color.Black.copy(alpha = 0.55f)),
            contentAlignment = Alignment.Center
        ) {
            Icon(
                Icons.Default.Phone, // placeholder visual
                contentDescription = "Play",
                tint = Color.White,
                modifier = Modifier.size(24.dp)
            )
        }
        if (showViewer && media != null) {
            MediaViewerDialog(media = media, onDismiss = { showViewer = false })
        }
    }
}

/**
 * Conteúdo de voz: player com duração (VoiceMessageBubble).
 */
@Composable
private fun VoiceMessageContent(media: uniffi.zaplivre.FfiMedia?, isOwnMessage: Boolean) {
    val path = media?.localPath
    val file = path?.let { File(it) }?.takeIf { it.exists() }
    if (file != null) {
        VoiceMessageBubble(
            audioFilePath = file.absolutePath,
            durationSeconds = media?.durationSeconds,
            isOwnMessage = isOwnMessage,
            timestamp = ""
        )
    } else {
        MediaPlaceholder("Voz", isOwnMessage)
    }
}

/**
 * Conteúdo de documento: nome do arquivo.
 */
@Composable
private fun DocumentMessageContent(media: uniffi.zaplivre.FfiMedia?, isOwnMessage: Boolean) {
    val primaryColor = if (isOwnMessage) ZapColor.bubbleOutInk else ZapColor.bubbleInInk
    val secondaryColor = if (isOwnMessage) ZapColor.bubbleOutInk.copy(alpha = 0.8f) else ZapColor.slate
    val sizeBytes = media?.fileSize
    Column {
        Text(
            text = media?.fileName ?: "Documento",
            style = ZapType.body,
            color = primaryColor
        )
        if (sizeBytes != null) {
            Text(
                text = formatFileSize(sizeBytes),
                style = ZapType.caption,
                color = secondaryColor
            )
        }
    }
}

/**
 * Placeholder para mídia ainda não baixada.
 */
@Composable
private fun MediaPlaceholder(label: String, isOwnMessage: Boolean) {
    Text(
        text = "[$label]",
        style = ZapType.body,
        color = if (isOwnMessage) ZapColor.bubbleOutInk else ZapColor.bubbleInInk
    )
}

private fun formatFileSize(bytes: Long): String {
    return when {
        bytes >= 1024 * 1024 -> String.format(Locale.getDefault(), "%.1f MB", bytes / 1024f / 1024f)
        bytes >= 1024 -> String.format(Locale.getDefault(), "%.1f KB", bytes / 1024f)
        else -> "$bytes B"
    }
}

/**
 * Abre o visualizador fullscreen de mídia (Dialog local) para a mídia clicada.
 */
@Composable
private fun MediaViewerDialog(
    media: uniffi.zaplivre.FfiMedia,
    onDismiss: () -> Unit
) {
    Dialog(
        onDismissRequest = onDismiss,
        properties = DialogProperties(usePlatformDefaultWidth = false)
    ) {
        Box(modifier = Modifier.fillMaxSize()) {
            com.zaplivre.ui.screens.media.MediaViewerScreen(
                mediaItems = listOf(media),
                initialIndex = 0,
                onNavigateBack = onDismiss
            )
        }
    }
}

/**
 * Formata timestamp da mensagem (HH:mm)
 */
private fun formatMessageTime(timestamp: Long): String {
    val date = Date(timestamp * 1000)
    return SimpleDateFormat("HH:mm", Locale.getDefault()).format(date)
}
