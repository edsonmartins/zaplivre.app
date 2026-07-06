package com.zaplivre.ui.screens.conversations

import androidx.compose.foundation.background
import androidx.compose.foundation.clickable
import androidx.compose.foundation.layout.*
import androidx.compose.foundation.lazy.LazyColumn
import androidx.compose.foundation.lazy.items
import androidx.compose.foundation.shape.CircleShape
import androidx.compose.foundation.shape.RoundedCornerShape
import androidx.compose.material.icons.Icons
import androidx.compose.material.icons.filled.Chat
import androidx.compose.material.icons.filled.Group
import androidx.compose.material.icons.filled.Search
import androidx.compose.material.icons.filled.Settings
import androidx.compose.material3.*
import androidx.compose.runtime.*
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.draw.clip
import androidx.compose.ui.graphics.Color
import androidx.compose.ui.platform.testTag
import androidx.compose.ui.res.stringResource
import androidx.compose.ui.text.font.FontWeight
import androidx.compose.ui.text.style.TextOverflow
import androidx.compose.ui.unit.dp
import androidx.lifecycle.viewmodel.compose.viewModel
import com.zaplivre.R
import com.zaplivre.ui.components.ZapAvatar
import com.zaplivre.ui.theme.ZapColor
import com.zaplivre.ui.theme.ZapMetric
import com.zaplivre.ui.theme.ZapType
import uniffi.zaplivre.FfiConversation
import java.text.SimpleDateFormat
import java.util.*

/** Modelo de UI desacoplado do FFI — usado pela tela e pelo design preview. */
data class ConversationUi(
    val id: String,
    val name: String,
    val preview: String,
    val time: String,
    val unread: Int = 0,
    val online: Boolean = false,
)

/**
 * ConversationsScreen - Lista de conversas. Mapeia o estado do ViewModel para
 * [ConversationUi] e delega a apresentação a [ConversationsContent] (stateless).
 */
@Composable
fun ConversationsScreen(
    onConversationClick: (String) -> Unit,
    onGroupsClick: (() -> Unit)? = null,
    onSearchClick: (() -> Unit)? = null,
    onSettingsClick: (() -> Unit)? = null,
    viewModel: ConversationsViewModel = viewModel { ConversationsViewModel() }
) {
    val uiState by viewModel.uiState.collectAsState()
    var showNewConversationDialog by remember { mutableStateOf(false) }

    val (rows, isLoading, error) = when (val state = uiState) {
        is ConversationsUiState.Loading -> Triple(emptyList(), true, null)
        is ConversationsUiState.Error -> Triple(emptyList(), false, state.message)
        is ConversationsUiState.Success -> Triple(
            state.conversations.map { it.toUi() }, false, null
        )
    }
    // mapa id->peerId original para o clique (o peerId pode ser null)
    val peerIds = remember(uiState) {
        (uiState as? ConversationsUiState.Success)?.conversations
            ?.associate { (it.peerId ?: it.displayName ?: "") to it.peerId } ?: emptyMap()
    }

    ConversationsContent(
        rows = rows,
        isLoading = isLoading,
        error = error,
        onConversationClick = { id -> peerIds[id]?.let(onConversationClick) },
        onSearchClick = onSearchClick,
        onGroupsClick = onGroupsClick,
        onSettingsClick = onSettingsClick,
        onNewChat = { showNewConversationDialog = true },
    )

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

@OptIn(ExperimentalMaterial3Api::class)
@Composable
fun ConversationsContent(
    rows: List<ConversationUi>,
    isLoading: Boolean = false,
    error: String? = null,
    onConversationClick: (String) -> Unit = {},
    onSearchClick: (() -> Unit)? = null,
    onGroupsClick: (() -> Unit)? = null,
    onSettingsClick: (() -> Unit)? = null,
    onNewChat: () -> Unit = {},
) {
    Scaffold(
        containerColor = ZapColor.canvas,
        topBar = {
            Column {
                TopAppBar(
                    title = { Text("ZapLivre", style = ZapType.title, color = ZapColor.ink) },
                    actions = {
                        onSearchClick?.let {
                            IconButton(onClick = it, modifier = Modifier.testTag("conversations_search")) {
                                Icon(Icons.Default.Search, "Buscar", tint = ZapColor.slate)
                            }
                        }
                        onGroupsClick?.let {
                            IconButton(onClick = it, modifier = Modifier.testTag("conversations_groups")) {
                                Icon(Icons.Default.Group, "Grupos", tint = ZapColor.slate)
                            }
                        }
                        onSettingsClick?.let {
                            IconButton(onClick = it, modifier = Modifier.testTag("conversations_settings")) {
                                Icon(Icons.Default.Settings, "Configurações", tint = ZapColor.slate)
                            }
                        }
                    },
                    colors = TopAppBarDefaults.topAppBarColors(
                        containerColor = ZapColor.canvas,
                        titleContentColor = ZapColor.ink,
                    )
                )
                Divider(color = ZapColor.hairline)
            }
        },
        floatingActionButton = {
            FloatingActionButton(
                onClick = onNewChat,
                modifier = Modifier.testTag("conversations_fab"),
                containerColor = Color.Transparent,
                elevation = FloatingActionButtonDefaults.elevation(0.dp, 0.dp, 0.dp, 0.dp),
            ) {
                Box(
                    modifier = Modifier
                        .size(56.dp)
                        .clip(RoundedCornerShape(18.dp))
                        .background(ZapColor.sparkBrush),
                    contentAlignment = Alignment.Center,
                ) {
                    Icon(Icons.Filled.Chat, stringResource(R.string.conversations_new), tint = Color.White)
                }
            }
        }
    ) { paddingValues ->
        Box(modifier = Modifier.fillMaxSize().padding(paddingValues)) {
            when {
                isLoading -> CircularProgressIndicator(
                    color = ZapColor.primary,
                    modifier = Modifier.align(Alignment.Center)
                )
                error != null -> EmptyState(error, ZapColor.danger)
                rows.isEmpty() -> EmptyState(stringResource(R.string.conversations_empty), ZapColor.slate)
                else -> LazyColumn(modifier = Modifier.fillMaxSize().testTag("conversations_list")) {
                    items(rows, key = { it.id }) { row ->
                        ConversationRow(row) { onConversationClick(row.id) }
                    }
                }
            }
        }
    }
}

@Composable
private fun EmptyState(message: String, color: Color) {
    Box(Modifier.fillMaxSize().padding(32.dp), contentAlignment = Alignment.Center) {
        Text(message, style = ZapType.body, color = color)
    }
}

@Composable
fun ConversationRow(row: ConversationUi, onClick: () -> Unit) {
    Row(
        modifier = Modifier
            .fillMaxWidth()
            .clickable(onClick = onClick)
            .padding(horizontal = ZapMetric.gutter, vertical = ZapMetric.rowGap),
        verticalAlignment = Alignment.CenterVertically,
    ) {
        ZapAvatar(seed = row.id, name = row.name, online = row.online)
        Spacer(Modifier.width(ZapMetric.rowGap))
        Column(modifier = Modifier.weight(1f)) {
            Row(verticalAlignment = Alignment.CenterVertically) {
                Text(
                    text = row.name,
                    style = ZapType.rowName,
                    color = ZapColor.ink,
                    maxLines = 1,
                    overflow = TextOverflow.Ellipsis,
                    modifier = Modifier.weight(1f),
                )
                Spacer(Modifier.width(8.dp))
                Text(
                    text = row.time,
                    style = ZapType.caption,
                    color = if (row.unread > 0) ZapColor.primary else ZapColor.slate,
                )
            }
            Spacer(Modifier.height(3.dp))
            Row(verticalAlignment = Alignment.CenterVertically) {
                Text(
                    text = row.preview,
                    style = ZapType.preview,
                    color = ZapColor.slate,
                    maxLines = 1,
                    overflow = TextOverflow.Ellipsis,
                    modifier = Modifier.weight(1f),
                )
                if (row.unread > 0) {
                    Spacer(Modifier.width(8.dp))
                    Box(
                        modifier = Modifier
                            .clip(CircleShape)
                            .background(ZapColor.primary)
                            .defaultMinSize(minWidth = 20.dp, minHeight = 20.dp)
                            .padding(horizontal = 6.dp),
                        contentAlignment = Alignment.Center,
                    ) {
                        Text(
                            text = if (row.unread > 99) "99+" else row.unread.toString(),
                            style = ZapType.badge,
                            color = Color.White,
                        )
                    }
                }
            }
        }
    }
}

@Composable
fun NewConversationDialog(
    onDismiss: () -> Unit,
    onConfirm: (String) -> Unit
) {
    var peerIdInput by remember { mutableStateOf("") }
    AlertDialog(
        onDismissRequest = onDismiss,
        title = { Text(stringResource(R.string.conversations_new)) },
        text = {
            OutlinedTextField(
                value = peerIdInput,
                onValueChange = { peerIdInput = it },
                label = { Text("Peer ID") },
                placeholder = { Text("12D3KooW...") },
                singleLine = true,
                modifier = Modifier.fillMaxWidth().testTag("new_chat_peer_input")
            )
        },
        confirmButton = {
            TextButton(
                onClick = { onConfirm(peerIdInput.trim()) },
                enabled = peerIdInput.trim().isNotEmpty(),
                modifier = Modifier.testTag("new_chat_confirm")
            ) { Text(stringResource(R.string.ok)) }
        },
        dismissButton = {
            TextButton(onClick = onDismiss) { Text(stringResource(R.string.cancel)) }
        }
    )
}

private fun FfiConversation.toUi(): ConversationUi {
    val id = peerId ?: displayName ?: "?"
    return ConversationUi(
        id = id,
        name = displayName ?: peerId?.take(16)?.plus("…") ?: "Desconhecido",
        preview = peerId?.let { "${it.take(16)}…" } ?: "",
        time = lastMessageAt?.let { formatTimestamp(it) } ?: "",
        unread = 0,
        online = false,
    )
}

private fun formatTimestamp(timestamp: Long): String {
    val now = System.currentTimeMillis() / 1000
    val diff = now - timestamp
    return when {
        diff < 60 -> "Agora"
        diff < 3600 -> "${diff / 60}m"
        diff < 86400 -> "${diff / 3600}h"
        diff < 604800 -> "${diff / 86400}d"
        else -> SimpleDateFormat("dd/MM", Locale.getDefault()).format(Date(timestamp * 1000))
    }
}
