package com.mepassa.ui.screens.conversations

import androidx.compose.foundation.clickable
import androidx.compose.foundation.layout.*
import androidx.compose.foundation.lazy.LazyColumn
import androidx.compose.foundation.lazy.items
import androidx.compose.material.icons.Icons
import androidx.compose.material.icons.filled.Add
import androidx.compose.material.icons.filled.Group
import androidx.compose.material.icons.filled.Search
import androidx.compose.material.icons.filled.Settings
import androidx.compose.material3.*
import androidx.compose.runtime.*
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.platform.testTag
import androidx.compose.ui.res.stringResource
import androidx.compose.ui.text.style.TextOverflow
import androidx.compose.ui.unit.dp
import androidx.lifecycle.viewmodel.compose.viewModel
import com.mepassa.R
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
    onGroupsClick: (() -> Unit)? = null,
    onSearchClick: (() -> Unit)? = null,
    onSettingsClick: (() -> Unit)? = null,
    viewModel: ConversationsViewModel = viewModel { ConversationsViewModel() }
) {
    // Carregamento, eventos do core (EVT-01) e safety net vivem no ViewModel
    val uiState by viewModel.uiState.collectAsState()
    var showNewConversationDialog by remember { mutableStateOf(false) }

    Scaffold(
        topBar = {
            TopAppBar(
                title = {
                    Text(stringResource(R.string.conversations_title))
                },
                actions = {
                    // Busca global de mensagens
                    if (onSearchClick != null) {
                        IconButton(
                            onClick = onSearchClick,
                            modifier = Modifier.testTag("conversations_search")
                        ) {
                            Icon(
                                imageVector = Icons.Default.Search,
                                contentDescription = "Buscar",
                                tint = MaterialTheme.colorScheme.onPrimaryContainer
                            )
                        }
                    }
                    // Botão de grupos
                    if (onGroupsClick != null) {
                        IconButton(
                            onClick = onGroupsClick,
                            modifier = Modifier.testTag("conversations_groups")
                        ) {
                            Icon(
                                imageVector = Icons.Default.Group,
                                contentDescription = "Grupos",
                                tint = MaterialTheme.colorScheme.onPrimaryContainer
                            )
                        }
                    }
                    // Configurações (backup de identidade, prekeys E2E, etc.)
                    if (onSettingsClick != null) {
                        IconButton(
                            onClick = onSettingsClick,
                            modifier = Modifier.testTag("conversations_settings")
                        ) {
                            Icon(
                                imageVector = Icons.Default.Settings,
                                contentDescription = "Configurações",
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
                onClick = { showNewConversationDialog = true },
                modifier = Modifier.testTag("conversations_fab")
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
            when (val state = uiState) {
                is ConversationsUiState.Loading -> {
                    CircularProgressIndicator(
                        modifier = Modifier.align(Alignment.Center)
                    )
                }
                is ConversationsUiState.Error -> {
                    Column(
                        modifier = Modifier
                            .fillMaxSize()
                            .padding(32.dp),
                        horizontalAlignment = Alignment.CenterHorizontally,
                        verticalArrangement = Arrangement.Center
                    ) {
                        Text(
                            text = state.message,
                            style = MaterialTheme.typography.bodyLarge,
                            color = MaterialTheme.colorScheme.error
                        )
                    }
                }
                is ConversationsUiState.Success -> {
                    if (state.conversations.isEmpty()) {
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
                    } else {
                        LazyColumn(
                            modifier = Modifier
                                .fillMaxSize()
                                .testTag("conversations_list")
                        ) {
                            items(state.conversations) { conversation ->
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
                modifier = Modifier
                    .fillMaxWidth()
                    .testTag("new_chat_peer_input")
            )
        },
        confirmButton = {
            TextButton(
                onClick = { onConfirm(peerIdInput.trim()) },
                enabled = peerIdInput.trim().isNotEmpty(),
                modifier = Modifier.testTag("new_chat_confirm")
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
