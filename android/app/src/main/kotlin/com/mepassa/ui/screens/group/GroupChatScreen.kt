package com.mepassa.ui.screens.group

import androidx.compose.foundation.layout.*
import androidx.compose.foundation.lazy.LazyColumn
import androidx.compose.foundation.lazy.items
import androidx.compose.foundation.lazy.rememberLazyListState
import androidx.compose.foundation.shape.RoundedCornerShape
import androidx.compose.material.icons.Icons
import androidx.compose.material.icons.filled.ArrowBack
import androidx.compose.material.icons.filled.Send
import androidx.compose.material.icons.filled.Info
import androidx.compose.material3.*
import androidx.compose.runtime.*
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.platform.testTag
import androidx.compose.ui.text.style.TextOverflow
import androidx.compose.ui.unit.dp
import com.mepassa.core.MePassaClientWrapper
import kotlinx.coroutines.launch
import uniffi.mepassa.FfiGroup
import uniffi.mepassa.FfiMessage
import java.text.SimpleDateFormat
import java.util.*

/**
 * GroupChatScreen - Tela de conversa em grupo
 *
 * Exibe mensagens do grupo e permite enviar novas mensagens.
 */
@OptIn(ExperimentalMaterial3Api::class)
@Composable
fun GroupChatScreen(
    groupId: String,
    onNavigateBack: () -> Unit,
    onGroupInfo: (String) -> Unit
) {
    val scope = rememberCoroutineScope()
    val listState = rememberLazyListState()

    var group by remember { mutableStateOf<FfiGroup?>(null) }
    var messages by remember { mutableStateOf<List<FfiMessage>>(emptyList()) }
    var messageInput by remember { mutableStateOf("") }
    var isSending by remember { mutableStateOf(false) }
    var isLoading by remember { mutableStateOf(true) }
    var errorMessage by remember { mutableStateOf<String?>(null) }
    val localPeerId by MePassaClientWrapper.localPeerId.collectAsState()

    // Carregar grupo e mensagens
    LaunchedEffect(groupId) {
        scope.launch {
            try {
                // Carregar informações do grupo
                val groups = MePassaClientWrapper.getGroups()
                group = groups.find { it.id == groupId }

                // Carregar mensagens do grupo
                messages = MePassaClientWrapper.getGroupMessages(groupId)
                isLoading = false

                // Scroll para última mensagem
                if (messages.isNotEmpty()) {
                    listState.animateScrollToItem(messages.lastIndex)
                }
            } catch (e: Exception) {
                errorMessage = "Erro ao carregar grupo: ${e.message}"
                isLoading = false
            }
        }
    }

    // Recarregar mensagens periodicamente
    LaunchedEffect(groupId) {
        while (true) {
            kotlinx.coroutines.delay(3000) // A cada 3 segundos
            scope.launch {
                try {
                    val newMessages = MePassaClientWrapper.getGroupMessages(groupId)
                    if (newMessages.size != messages.size) {
                        messages = newMessages
                        if (messages.isNotEmpty()) {
                            listState.animateScrollToItem(messages.lastIndex)
                        }
                    }
                } catch (e: Exception) {
                    // Silently fail on background refresh
                }
            }
        }
    }

    Scaffold(
        topBar = {
            TopAppBar(
                title = {
                    Column {
                        Text(
                            text = group?.name ?: "Carregando...",
                            style = MaterialTheme.typography.titleMedium,
                            maxLines = 1,
                            overflow = TextOverflow.Ellipsis
                        )
                        Text(
                            text = if (group != null) {
                                "${group!!.memberCount} ${if (group!!.memberCount == 1u) "membro" else "membros"}"
                            } else {
                                ""
                            },
                            style = MaterialTheme.typography.bodySmall,
                            color = MaterialTheme.colorScheme.onSurfaceVariant
                        )
                    }
                },
                navigationIcon = {
                    IconButton(onClick = onNavigateBack) {
                        Icon(
                            Icons.Filled.ArrowBack,
                            contentDescription = "Voltar"
                        )
                    }
                },
                actions = {
                    // Botão de informações do grupo
                    IconButton(
                        onClick = { onGroupInfo(groupId) },
                        modifier = Modifier.testTag("groupchat_info")
                    ) {
                        Icon(
                            imageVector = Icons.Default.Info,
                            contentDescription = "Informações do grupo",
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
            GroupMessageInputBar(
                messageInput = messageInput,
                onMessageInputChange = { messageInput = it },
                onSendClick = {
                    if (messageInput.isNotBlank() && !isSending) {
                        val content = messageInput.trim()
                        messageInput = ""
                        isSending = true

                        scope.launch {
                            try {
                                MePassaClientWrapper.sendGroupMessage(groupId, content)
                                messages = MePassaClientWrapper.getGroupMessages(groupId)
                                if (messages.isNotEmpty()) {
                                    listState.animateScrollToItem(messages.lastIndex)
                                }
                            } catch (e: Exception) {
                                errorMessage = "Erro ao enviar mensagem: ${e.message}"
                            } finally {
                                isSending = false
                            }
                        }
                    }
                },
                isSending = isSending
            )
        }
    ) { paddingValues ->
        when {
            isLoading -> {
                Box(
                    modifier = Modifier
                        .fillMaxSize()
                        .padding(paddingValues),
                    contentAlignment = Alignment.Center
                ) {
                    CircularProgressIndicator()
                }
            }
            errorMessage != null -> {
                Box(
                    modifier = Modifier
                        .fillMaxSize()
                        .padding(paddingValues),
                    contentAlignment = Alignment.Center
                ) {
                    Column(
                        horizontalAlignment = Alignment.CenterHorizontally,
                        verticalArrangement = Arrangement.spacedBy(16.dp)
                    ) {
                        Text(
                            text = errorMessage ?: "",
                            style = MaterialTheme.typography.bodyLarge,
                            color = MaterialTheme.colorScheme.error
                        )
                        Button(onClick = { errorMessage = null }) {
                            Text("OK")
                        }
                    }
                }
            }
            messages.isEmpty() -> {
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
            }
            else -> {
                LazyColumn(
                    modifier = Modifier
                        .fillMaxSize()
                        .padding(paddingValues),
                    state = listState,
                    contentPadding = PaddingValues(16.dp),
                    verticalArrangement = Arrangement.spacedBy(8.dp)
                ) {
                    items(messages) { message ->
                        GroupMessageBubble(
                            message = message,
                            isOwnMessage = message.senderPeerId == localPeerId,
                            showSenderName = message.senderPeerId != localPeerId
                        )
                    }
                }
            }
        }
    }
}

/**
 * Barra de input de mensagem para grupos
 */
@Composable
fun GroupMessageInputBar(
    messageInput: String,
    onMessageInputChange: (String) -> Unit,
    onSendClick: () -> Unit,
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
            OutlinedTextField(
                value = messageInput,
                onValueChange = onMessageInputChange,
                modifier = Modifier
                    .weight(1f)
                    .testTag("groupchat_input"),
                placeholder = {
                    Text("Mensagem")
                },
                maxLines = 4,
                enabled = !isSending,
                shape = RoundedCornerShape(24.dp)
            )

            IconButton(
                onClick = onSendClick,
                enabled = messageInput.isNotBlank() && !isSending,
                modifier = Modifier.testTag("groupchat_send")
            ) {
                if (isSending) {
                    CircularProgressIndicator(
                        modifier = Modifier.size(24.dp),
                        strokeWidth = 2.dp
                    )
                } else {
                    Icon(
                        Icons.Filled.Send,
                        contentDescription = "Enviar",
                        tint = if (messageInput.isNotBlank()) {
                            MaterialTheme.colorScheme.primary
                        } else {
                            MaterialTheme.colorScheme.onSurfaceVariant
                        }
                    )
                }
            }
        }
    }
}

/**
 * Bolha de mensagem para grupos (com nome do remetente)
 */
@Composable
fun GroupMessageBubble(
    message: FfiMessage,
    isOwnMessage: Boolean,
    showSenderName: Boolean
) {
    Row(
        modifier = Modifier.fillMaxWidth(),
        horizontalArrangement = if (isOwnMessage) Arrangement.End else Arrangement.Start
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
            modifier = Modifier.widthIn(max = 280.dp)
        ) {
            Column(
                modifier = Modifier.padding(12.dp)
            ) {
                // Nome do remetente (apenas para mensagens de outros)
                if (showSenderName) {
                    Text(
                        text = message.senderPeerId.take(8),
                        style = MaterialTheme.typography.labelMedium,
                        color = MaterialTheme.colorScheme.primary
                    )
                    Spacer(modifier = Modifier.height(4.dp))
                }

                // Conteúdo da mensagem
                message.contentPlaintext?.let { content ->
                    Text(
                        text = content,
                        style = MaterialTheme.typography.bodyMedium
                    )
                }

                Spacer(modifier = Modifier.height(4.dp))

                // Timestamp
                Text(
                    text = formatMessageTime(message.createdAt),
                    style = MaterialTheme.typography.labelSmall,
                    color = MaterialTheme.colorScheme.onSurfaceVariant
                )
            }
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
