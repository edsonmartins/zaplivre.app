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
import androidx.compose.ui.semantics.semantics
import androidx.compose.ui.ExperimentalComposeUiApi
import androidx.compose.ui.semantics.testTagsAsResourceId
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
import com.zaplivre.core.ZapLivreClientWrapper
import kotlinx.coroutines.launch
import org.json.JSONObject
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
@OptIn(
    ExperimentalComposeUiApi::class,
    com.google.accompanist.permissions.ExperimentalPermissionsApi::class,
)
fun NewConversationDialog(
    onDismiss: () -> Unit,
    onConfirm: (String) -> Unit
) {
    var usernameInput by remember { mutableStateOf("") }
    var error by remember { mutableStateOf<String?>(null) }
    var loading by remember { mutableStateOf(false) }
    val scope = rememberCoroutineScope()
    var scanning by remember { mutableStateOf(false) }
    val cameraPermission = com.google.accompanist.permissions.rememberPermissionState(
        android.Manifest.permission.CAMERA
    ) { granted -> if (granted) scanning = true }

    if (scanning) {
        com.zaplivre.ui.components.QrScannerDialog(
            onDismiss = { scanning = false },
            onResult = { raw ->
                scanning = false
                val qr = com.zaplivre.core.ContactQrCode.parse(raw)
                if (qr == null) {
                    error = "QR de contato inválido"
                    return@QrScannerDialog
                }
                loading = true
                scope.launch {
                    // O bundle embutido (QRs antigos do iOS) é verificado pelo core
                    // antes de abrir sessão; o normal é trocar prekeys após conectar.
                    qr.prekeyBundle?.let {
                        ZapLivreClientWrapper.storePeerPrekeyBundle(qr.peerId, it)
                    }
                    qr.multiaddr?.let { ZapLivreClientWrapper.connectToPeer(qr.peerId, it) }
                    loading = false
                    onConfirm(qr.peerId)
                }
            },
        )
    }

    AlertDialog(
        onDismissRequest = onDismiss,
        title = { Text(stringResource(R.string.conversations_new)) },
        text = {
            Column(modifier = Modifier.semantics { testTagsAsResourceId = true }) {
            OutlinedTextField(
                value = usernameInput,
                onValueChange = { usernameInput = it.removePrefix("@").lowercase(); error = null },
                label = { Text("Username") },
                placeholder = { Text("exemplo") },
                singleLine = true,
                modifier = Modifier.fillMaxWidth().testTag("new_chat_peer_input")
            )
            error?.let { Text(it, color = MaterialTheme.colorScheme.error) }
            TextButton(
                onClick = {
                    error = null
                    if (cameraPermission.status == com.google.accompanist.permissions.PermissionStatus.Granted) {
                        scanning = true
                    } else {
                        cameraPermission.launchPermissionRequest()
                    }
                },
                enabled = !loading,
                modifier = Modifier.testTag("new_chat_scan_qr")
            ) { Text("Escanear QR do contato") }
            }
        },
        confirmButton = {
            Box(modifier = Modifier.semantics { testTagsAsResourceId = true }) {
                TextButton(
                    onClick = {
                        loading = true
                        scope.launch {
                            try {
                                val result = ZapLivreClientWrapper.lookupUsername(usernameInput.trim())
                                val json = JSONObject(result)
                                val peerId = json.getString("peer_id")
                                ZapLivreClientWrapper.storePeerPrekeyBundle(
                                    peerId,
                                    json.getJSONObject("prekey_bundle").toString()
                                )
                                onConfirm(peerId)
                            } catch (e: Exception) {
                                error = e.message ?: "Usuário não encontrado"
                            } finally { loading = false }
                        }
                    },
                    enabled = usernameInput.trim().isNotEmpty() && !loading,
                    modifier = Modifier.testTag("new_chat_confirm")
                ) { Text(if (loading) "Buscando…" else "Adicionar") }
            }
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
        unread = unreadCount,
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
