package com.mepassa.ui.screens.conversations

import androidx.compose.foundation.clickable
import androidx.compose.foundation.layout.*
import androidx.compose.foundation.lazy.LazyColumn
import androidx.compose.foundation.lazy.items
import androidx.compose.material.icons.Icons
import androidx.compose.material.icons.filled.Add
import androidx.compose.material.icons.filled.Group
import androidx.compose.material3.*
import androidx.compose.runtime.*
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.res.stringResource
import androidx.compose.ui.text.style.TextOverflow
import androidx.compose.ui.unit.dp
import com.mepassa.R
import com.mepassa.core.MePassaClientWrapper
import kotlinx.coroutines.launch
import uniffi.mepassa.FfiConversation
import java.text.SimpleDateFormat
import java.util.*

/**
 * ConversationsScreen - Lista de conversas
 *
 * Exibe todas as conversas do usuário ordenadas por data.
 * Permite navegar para uma conversa específica.
 */
@OptIn(ExperimentalMaterial3Api::class)
@Composable
fun ConversationsScreen(
    onConversationClick: (String) -> Unit,
    onGroupsClick: (() -> Unit)? = null
) {
    val scope = rememberCoroutineScope()
    var conversations by remember { mutableStateOf<List<FfiConversation>>(emptyList()) }
    var isLoading by remember { mutableStateOf(true) }
    var showNewConversationDialog by remember { mutableStateOf(false) }

    // Carregar conversas (sender keys de grupo agora são distribuídas
    // pelo core via protocolo in-band - sem varredura manual)
    LaunchedEffect(Unit) {
        scope.launch {
            conversations = MePassaClientWrapper.listConversations()
            isLoading = false
        }
    }

    // EVT-01: recarregar a lista quando o core avisa de mensagem nova
    LaunchedEffect(Unit) {
        MePassaClientWrapper.messageEvents.collect { event ->
            if (event !is MePassaClientWrapper.MessageUiEvent.Typing) {
                conversations = MePassaClientWrapper.listConversations()
            }
        }
    }

    // Safety net: refresh lento caso algum evento se perca
    LaunchedEffect(Unit) {
        while (true) {
            kotlinx.coroutines.delay(30000)
            conversations = MePassaClientWrapper.listConversations()
        }
    }

    Scaffold(
        topBar = {
            TopAppBar(
                title = {
                    Text(stringResource(R.string.conversations_title))
                },
                actions = {
                    // Botão de grupos
                    if (onGroupsClick != null) {
                        IconButton(onClick = onGroupsClick) {
                            Icon(
                                imageVector = Icons.Default.Group,
                                contentDescription = "Grupos",
                                tint = MaterialTheme.colorScheme.onPrimaryContainer
                            )
                        }
                    }
                },
                colors = TopAppBarDefaults.topAppBarColors(
                    containerColor = MaterialTheme.colorScheme.primaryContainer,
                    titleContentColor = MaterialTheme.colorScheme.onPrimaryContainer
                )
            )
        },
        floatingActionButton = {
            FloatingActionButton(
                onClick = { showNewConversationDialog = true }
            ) {
                Icon(Icons.Default.Add, contentDescription = stringResource(R.string.conversations_new))
            }
        }
    ) { paddingValues ->
        Box(
            modifier = Modifier
                .fillMaxSize()
                .padding(paddingValues)
        ) {
            when {
                isLoading -> {
                    CircularProgressIndicator(
                        modifier = Modifier.align(Alignment.Center)
                    )
                }
                conversations.isEmpty() -> {
                    Column(
                        modifier = Modifier
                            .fillMaxSize()
                            .padding(32.dp),
                        horizontalAlignment = Alignment.CenterHorizontally,
                        verticalArrangement = Arrangement.Center
                    ) {
                        Text(
                            text = stringResource(R.string.conversations_empty),
                            style = MaterialTheme.typography.bodyLarge,
                            color = MaterialTheme.colorScheme.onSurfaceVariant
                        )
                    }
                }
                else -> {
                    LazyColumn(
                        modifier = Modifier.fillMaxSize()
                    ) {
                        items(conversations) { conversation ->
                            ConversationItem(
                                conversation = conversation,
                                onClick = {
                                    conversation.peerId?.let { onConversationClick(it) }
                                }
                            )
                            Divider()
                        }
                    }
                }
            }
        }
    }

    // Dialog para nova conversa
    if (showNewConversationDialog) {
        NewConversationDialog(
            onDismiss = { showNewConversationDialog = false },
            onConfirm = { peerId ->
                showNewConversationDialog = false
                onConversationClick(peerId)
            }
        )
    }
}

/**
 * Item de conversa individual
 */
@Composable
fun ConversationItem(
    conversation: FfiConversation,
    onClick: () -> Unit
) {
    ListItem(
        headlineContent = {
            Text(
                text = conversation.displayName ?: conversation.peerId ?: "Unknown",
                style = MaterialTheme.typography.titleMedium,
                maxLines = 1,
                overflow = TextOverflow.Ellipsis
            )
        },
        supportingContent = conversation.peerId?.let {
            {
                Text(
                    text = it.take(16) + "...",
                    style = MaterialTheme.typography.bodySmall,
                    color = MaterialTheme.colorScheme.onSurfaceVariant,
                    fontFamily = androidx.compose.ui.text.font.FontFamily.Monospace
                )
            }
        },
        trailingContent = conversation.lastMessageAt?.let { timestamp ->
            {
                Text(
                    text = formatTimestamp(timestamp),
                    style = MaterialTheme.typography.labelSmall,
                    color = MaterialTheme.colorScheme.onSurfaceVariant
                )
            }
        },
        modifier = Modifier.clickable(onClick = onClick)
    )
}

/**
 * Dialog para iniciar nova conversa
 */
@Composable
fun NewConversationDialog(
    onDismiss: () -> Unit,
    onConfirm: (String) -> Unit
) {
    var peerIdInput by remember { mutableStateOf("") }

    AlertDialog(
        onDismissRequest = onDismiss,
        title = {
            Text(stringResource(R.string.conversations_new))
        },
        text = {
            OutlinedTextField(
                value = peerIdInput,
                onValueChange = { peerIdInput = it },
                label = { Text("Peer ID") },
                placeholder = { Text("12D3KooW...") },
                singleLine = true,
                modifier = Modifier.fillMaxWidth()
            )
        },
        confirmButton = {
            TextButton(
                onClick = { onConfirm(peerIdInput.trim()) },
                enabled = peerIdInput.trim().isNotEmpty()
            ) {
                Text(stringResource(R.string.ok))
            }
        },
        dismissButton = {
            TextButton(onClick = onDismiss) {
                Text(stringResource(R.string.cancel))
            }
        }
    )
}

/**
 * Formata timestamp para exibição (relativo ao tempo atual)
 */
private fun formatTimestamp(timestamp: Long): String {
    val now = System.currentTimeMillis() / 1000
    val diff = now - timestamp

    return when {
        diff < 60 -> "Agora"
        diff < 3600 -> "${diff / 60}m"
        diff < 86400 -> "${diff / 3600}h"
        diff < 604800 -> "${diff / 86400}d"
        else -> {
            val date = Date(timestamp * 1000)
            SimpleDateFormat("dd/MM", Locale.getDefault()).format(date)
        }
    }
}
