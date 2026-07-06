package com.zaplivre.ui.screens.chat

import android.net.Uri
import androidx.compose.animation.*
import androidx.compose.animation.core.*
import androidx.compose.foundation.ExperimentalFoundationApi
import androidx.compose.foundation.background
import androidx.compose.foundation.clickable
import androidx.compose.foundation.combinedClickable
import androidx.compose.foundation.layout.*
import androidx.compose.foundation.lazy.LazyColumn
import androidx.compose.foundation.lazy.items
import androidx.compose.foundation.lazy.rememberLazyListState
import androidx.compose.foundation.shape.RoundedCornerShape
import androidx.compose.material.icons.Icons
import androidx.compose.material.icons.filled.ArrowBack
import androidx.compose.material.icons.filled.Send
import androidx.compose.material.icons.filled.Phone
import androidx.compose.material.icons.filled.Photo
import androidx.compose.material.icons.filled.Search
import androidx.compose.material3.*
import androidx.compose.runtime.*
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.graphics.Color
import androidx.compose.ui.platform.testTag
import androidx.compose.ui.res.stringResource
import androidx.compose.ui.text.style.TextOverflow
import androidx.compose.ui.unit.dp
import androidx.lifecycle.viewmodel.compose.viewModel
import com.zaplivre.R
import com.zaplivre.core.ZapLivreClientWrapper
import com.zaplivre.core.VoiceRecorderViewModel
import com.zaplivre.ui.components.ImagePickerButton
import com.zaplivre.ui.components.MessageStatusIndicator
import com.zaplivre.ui.components.SelectedImagesPreview
import com.zaplivre.ui.components.VoiceRecordButton
import com.zaplivre.utils.rememberHapticFeedback
import kotlinx.coroutines.launch
import uniffi.zaplivre.FfiMessage
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
    var isSendingMedia by remember { mutableStateOf(false) }
    val isSending = isSendingText || isSendingMedia
    val localPeerId by chatViewModel.localPeerId.collectAsState()

    // Image selection state
    var selectedImages by remember { mutableStateOf<List<Uri>>(emptyList()) }

    // Voice recorder
    val voiceRecorderViewModel = remember { VoiceRecorderViewModel(context) }

    // Message actions state
    var selectedMessage by remember { mutableStateOf<FfiMessage?>(null) }
    var showDeleteDialog by remember { mutableStateOf(false) }
    var showForwardDialog by remember { mutableStateOf(false) }

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
        topBar = {
            TopAppBar(
                title = {
                    Column {
                        Text(
                            text = peerId.take(16) + "...",
                            style = MaterialTheme.typography.titleMedium,
                            maxLines = 1,
                            overflow = TextOverflow.Ellipsis
                        )
                        Text(
                            text = stringResource(R.string.chat_status_connected),
                            style = MaterialTheme.typography.bodySmall,
                            color = MaterialTheme.colorScheme.onSurfaceVariant
                        )
                    }
                },
                navigationIcon = {
                    IconButton(
                        onClick = onNavigateBack,
                        modifier = Modifier.testTag("chat_back")
                    ) {
                        Icon(
                            Icons.Filled.ArrowBack,
                            contentDescription = "Voltar"
                        )
                    }
                },
                actions = {
                    // Botão de busca
                    IconButton(
                        onClick = onOpenSearch,
                        modifier = Modifier.testTag("chat_search")
                    ) {
                        Icon(
                            imageVector = Icons.Default.Search,
                            contentDescription = "Buscar mensagens",
                            tint = MaterialTheme.colorScheme.onPrimaryContainer
                        )
                    }

                    // Botão de galeria de mídia
                    IconButton(
                        onClick = onOpenMediaGallery,
                        modifier = Modifier.testTag("chat_media_gallery")
                    ) {
                        Icon(
                            imageVector = Icons.Default.Photo,
                            contentDescription = "Galeria de mídia",
                            tint = MaterialTheme.colorScheme.onPrimaryContainer
                        )
                    }

                    // Botão de chamada de voz
                    IconButton(onClick = onStartCall) {
                        Icon(
                            imageVector = Icons.Default.Phone,
                            contentDescription = "Iniciar chamada",
                            tint = MaterialTheme.colorScheme.onPrimaryContainer
                        )
                    }
                },
                colors = TopAppBarDefaults.topAppBarColors(
                    containerColor = MaterialTheme.colorScheme.primaryContainer,
                    titleContentColor = MaterialTheme.colorScheme.onPrimaryContainer
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
                                                imageData = imageBytes.toUByteArray().toList(),
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
                                    audioData = audioBytes.toUByteArray().toList(),
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
                                        fileData = fileBytes.toUByteArray().toList(),
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
                                        videoData = videoBytes.toUByteArray().toList(),
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
                    AnimatedVisibility(
                        visible = true,
                        enter = slideInVertically(
                            initialOffsetY = { it / 4 },
                            animationSpec = tween(300, easing = FastOutSlowInEasing)
                        ) + fadeIn(
                            animationSpec = tween(300)
                        ),
                        modifier = Modifier.animateItemPlacement()
                    ) {
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
    @Suppress("UNUSED_PARAMETER") onVideoPicked: (Uri) -> Unit,
    voiceRecorderViewModel: VoiceRecorderViewModel,
    isSending: Boolean
) {
    Surface(
        tonalElevation = 3.dp,
        modifier = Modifier.fillMaxWidth()
    ) {
        Row(
            modifier = Modifier
                .padding(8.dp)
                .fillMaxWidth(),
            verticalAlignment = Alignment.CenterVertically,
            horizontalArrangement = Arrangement.spacedBy(8.dp)
        ) {
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

            // Video picker button (placeholder - will be implemented later)
            // TODO: Implement VideoPicker component
            // com.zaplivre.ui.components.VideoPickerButton(
            //     onVideoPicked = onVideoPicked,
            //     enabled = !isSending
            // )

            OutlinedTextField(
                value = messageInput,
                onValueChange = onMessageInputChange,
                modifier = Modifier
                    .weight(1f)
                    .testTag("chat_input"),
                placeholder = {
                    Text(stringResource(R.string.chat_input_hint))
                },
                maxLines = 4,
                enabled = !isSending,
                shape = RoundedCornerShape(24.dp)
            )

            // Send button or Voice record button
            if (messageInput.isNotBlank()) {
                IconButton(
                    onClick = onSendClick,
                    enabled = !isSending,
                    modifier = Modifier.testTag("chat_send")
                ) {
                    if (isSending) {
                        CircularProgressIndicator(
                            modifier = Modifier.size(24.dp),
                            strokeWidth = 2.dp
                        )
                    } else {
                        Icon(
                            Icons.Filled.Send,
                            contentDescription = stringResource(R.string.chat_send),
                            tint = MaterialTheme.colorScheme.primary
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

    Row(
        modifier = Modifier.fillMaxWidth(),
        horizontalArrangement = if (isOwnMessage) Arrangement.End else Arrangement.Start
    ) {
        Column(
            horizontalAlignment = if (isOwnMessage) Alignment.End else Alignment.Start
        ) {
            Surface(
                shape = RoundedCornerShape(
                    topStart = 16.dp,
                    topEnd = 16.dp,
                    bottomStart = if (isOwnMessage) 16.dp else 4.dp,
                    bottomEnd = if (isOwnMessage) 4.dp else 16.dp
                ),
                color = if (isOwnMessage) {
                    MaterialTheme.colorScheme.primaryContainer
                } else {
                    MaterialTheme.colorScheme.surfaceVariant
                },
                modifier = Modifier
                    .widthIn(max = 280.dp)
                    .combinedClickable(
                        onClick = {},
                        onLongClick = {
                            onLongPress()
                            showMenu = true
                        }
                    )
            ) {
                Column(
                    modifier = Modifier.padding(12.dp)
                ) {
                    message.contentPlaintext?.let { content ->
                        Text(
                            text = content,
                            style = MaterialTheme.typography.bodyMedium
                        )
                    }

                    Spacer(modifier = Modifier.height(4.dp))

                    MessageStatusIndicator(
                        message = message,
                        isOwnMessage = isOwnMessage
                    )
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
                    text = { Text("Excluir", color = MaterialTheme.colorScheme.error) },
                    onClick = {
                        showMenu = false
                        onDelete()
                    }
                )
            }
        }

        // Reaction bar
        if (reactions.isNotEmpty()) {
            com.zaplivre.ui.components.ReactionBar(
                reactions = reactions,
                onReactionClick = onReactionClick,
                onAddReactionClick = onAddReactionClick,
                modifier = Modifier.widthIn(max = 280.dp)
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
