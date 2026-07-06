package com.zaplivre.ui.screens.group

import androidx.compose.foundation.background
import androidx.compose.foundation.clickable
import androidx.compose.foundation.layout.*
import androidx.compose.foundation.lazy.LazyColumn
import androidx.compose.foundation.lazy.items
import androidx.compose.foundation.shape.RoundedCornerShape
import androidx.compose.material.icons.Icons
import androidx.compose.material.icons.filled.Add
import androidx.compose.material.icons.filled.Group
import androidx.compose.material3.*
import androidx.compose.runtime.*
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.draw.clip
import androidx.compose.ui.graphics.Color
import androidx.compose.ui.platform.testTag
import androidx.compose.ui.text.style.TextOverflow
import androidx.compose.ui.unit.dp
import com.zaplivre.core.ZapLivreClientWrapper
import com.zaplivre.ui.components.ZapAvatar
import com.zaplivre.ui.theme.ZapColor
import com.zaplivre.ui.theme.ZapMetric
import com.zaplivre.ui.theme.ZapType
import kotlinx.coroutines.launch
import uniffi.zaplivre.FfiGroup

/** Modelo de UI desacoplado do FFI — usado pela tela e pelo design preview. */
data class GroupUi(
    val id: String,
    val name: String,
    val subtitle: String,
    val members: Int,
    val isAdmin: Boolean = false,
)

/**
 * GroupListScreen - Lista de grupos.
 *
 * Carrega os grupos via [ZapLivreClientWrapper], mapeia o modelo FFI para
 * [GroupUi] e delega a apresentação a [GroupListContent] (stateless).
 */
@Composable
fun GroupListScreen(
    onGroupClick: (String) -> Unit,
    onBack: () -> Unit
) {
    val scope = rememberCoroutineScope()
    var groups by remember { mutableStateOf<List<FfiGroup>>(emptyList()) }
    var isLoading by remember { mutableStateOf(true) }
    var showCreateGroupDialog by remember { mutableStateOf(false) }
    var errorMessage by remember { mutableStateOf<String?>(null) }

    // Carregar grupos
    LaunchedEffect(Unit) {
        scope.launch {
            try {
                groups = ZapLivreClientWrapper.getGroups()
                isLoading = false
            } catch (e: Exception) {
                errorMessage = "Erro ao carregar grupos: ${e.message}"
                isLoading = false
            }
        }
    }

    // Recarregar grupos periodicamente
    LaunchedEffect(Unit) {
        while (true) {
            kotlinx.coroutines.delay(10000) // A cada 10 segundos
            scope.launch {
                try {
                    groups = ZapLivreClientWrapper.getGroups()
                } catch (e: Exception) {
                    // Silently fail on background refresh
                }
            }
        }
    }

    GroupListContent(
        groups = groups.map { it.toUi() },
        isLoading = isLoading,
        error = errorMessage,
        onGroupClick = onGroupClick,
        onCreateGroup = { showCreateGroupDialog = true },
        onBack = onBack,
        onRetry = {
            errorMessage = null
            isLoading = true
            scope.launch {
                try {
                    groups = ZapLivreClientWrapper.getGroups()
                    isLoading = false
                } catch (e: Exception) {
                    errorMessage = "Erro ao carregar grupos: ${e.message}"
                    isLoading = false
                }
            }
        },
    )

    // Dialog para criar grupo
    if (showCreateGroupDialog) {
        CreateGroupDialog(
            onDismiss = { showCreateGroupDialog = false },
            onConfirm = { name, description ->
                scope.launch {
                    try {
                        val group = ZapLivreClientWrapper.createGroup(name, description)
                        showCreateGroupDialog = false
                        // Recarregar lista de grupos
                        groups = ZapLivreClientWrapper.getGroups()
                        // Navegar para o grupo criado
                        onGroupClick(group.id)
                    } catch (e: Exception) {
                        errorMessage = "Erro ao criar grupo: ${e.message}"
                        showCreateGroupDialog = false
                    }
                }
            }
        )
    }
}

/**
 * Apresentação stateless da lista de grupos — recebe [GroupUi] + callbacks.
 * Usada pela tela real e pelo design preview.
 */
@OptIn(ExperimentalMaterial3Api::class)
@Composable
fun GroupListContent(
    groups: List<GroupUi>,
    isLoading: Boolean = false,
    error: String? = null,
    onGroupClick: (String) -> Unit = {},
    onCreateGroup: () -> Unit = {},
    onBack: () -> Unit = {},
    onRetry: () -> Unit = {},
) {
    Scaffold(
        containerColor = ZapColor.canvas,
        topBar = {
            Column {
                TopAppBar(
                    title = { Text("Grupos", style = ZapType.title, color = ZapColor.ink) },
                    navigationIcon = {
                        IconButton(onClick = onBack) {
                            Icon(Icons.Default.Group, "Voltar", tint = ZapColor.slate)
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
                onClick = onCreateGroup,
                modifier = Modifier.testTag("grouplist_fab"),
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
                    Icon(Icons.Default.Add, "Criar grupo", tint = Color.White)
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
                error != null -> Column(
                    modifier = Modifier.fillMaxSize().padding(32.dp),
                    horizontalAlignment = Alignment.CenterHorizontally,
                    verticalArrangement = Arrangement.Center,
                ) {
                    Text(error, style = ZapType.body, color = ZapColor.danger)
                    Spacer(Modifier.height(16.dp))
                    Button(onClick = onRetry) { Text("Tentar novamente") }
                }
                groups.isEmpty() -> Box(
                    Modifier.fillMaxSize().padding(32.dp),
                    contentAlignment = Alignment.Center,
                ) {
                    Text(
                        text = "Nenhum grupo ainda. Crie ou entre em um grupo para começar.",
                        style = ZapType.body,
                        color = ZapColor.slate,
                    )
                }
                else -> LazyColumn(modifier = Modifier.fillMaxSize().testTag("grouplist_list")) {
                    items(groups, key = { it.id }) { group ->
                        GroupRow(group) { onGroupClick(group.id) }
                    }
                }
            }
        }
    }
}

@Composable
fun GroupRow(group: GroupUi, onClick: () -> Unit) {
    Row(
        modifier = Modifier
            .fillMaxWidth()
            .clickable(onClick = onClick)
            .padding(horizontal = ZapMetric.gutter, vertical = ZapMetric.rowGap),
        verticalAlignment = Alignment.CenterVertically,
    ) {
        ZapAvatar(seed = group.id, name = group.name)
        Spacer(Modifier.width(ZapMetric.rowGap))
        Column(modifier = Modifier.weight(1f)) {
            Row(verticalAlignment = Alignment.CenterVertically) {
                Text(
                    text = group.name,
                    style = ZapType.rowName,
                    color = ZapColor.ink,
                    maxLines = 1,
                    overflow = TextOverflow.Ellipsis,
                    modifier = Modifier.weight(1f, fill = false),
                )
                if (group.isAdmin) {
                    Spacer(Modifier.width(8.dp))
                    Text("Admin", style = ZapType.badge, color = ZapColor.primary)
                }
            }
            Spacer(Modifier.height(3.dp))
            Text(
                text = group.subtitle,
                style = ZapType.preview,
                color = ZapColor.slate,
                maxLines = 1,
                overflow = TextOverflow.Ellipsis,
            )
        }
    }
}

/**
 * Dialog para criar novo grupo
 */
@Composable
fun CreateGroupDialog(
    onDismiss: () -> Unit,
    onConfirm: (String, String?) -> Unit
) {
    var nameInput by remember { mutableStateOf("") }
    var descriptionInput by remember { mutableStateOf("") }

    AlertDialog(
        onDismissRequest = onDismiss,
        title = {
            Text("Criar Grupo")
        },
        text = {
            Column(
                verticalArrangement = Arrangement.spacedBy(12.dp)
            ) {
                OutlinedTextField(
                    value = nameInput,
                    onValueChange = { nameInput = it },
                    label = { Text("Nome do grupo") },
                    placeholder = { Text("Ex: Amigos da faculdade") },
                    singleLine = true,
                    modifier = Modifier
                        .fillMaxWidth()
                        .testTag("grouplist_create_name_input")
                )
                OutlinedTextField(
                    value = descriptionInput,
                    onValueChange = { descriptionInput = it },
                    label = { Text("Descrição (opcional)") },
                    placeholder = { Text("Ex: Grupo para discussões") },
                    maxLines = 3,
                    modifier = Modifier.fillMaxWidth()
                )
            }
        },
        confirmButton = {
            TextButton(
                onClick = {
                    val desc = if (descriptionInput.trim().isEmpty()) null else descriptionInput.trim()
                    onConfirm(nameInput.trim(), desc)
                },
                enabled = nameInput.trim().isNotEmpty(),
                modifier = Modifier.testTag("grouplist_create_confirm")
            ) {
                Text("Criar")
            }
        },
        dismissButton = {
            TextButton(onClick = onDismiss) {
                Text("Cancelar")
            }
        }
    )
}

private fun FfiGroup.toUi(): GroupUi {
    val memberLabel = "$memberCount ${if (memberCount == 1u) "membro" else "membros"}"
    val subtitle = description?.takeIf { it.isNotBlank() }
        ?.let { "$it · $memberLabel" }
        ?: memberLabel
    return GroupUi(
        id = id,
        name = name,
        subtitle = subtitle,
        members = memberCount.toInt(),
        isAdmin = isAdmin,
    )
}
