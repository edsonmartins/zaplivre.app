package com.mepassa.ui.screens.group

import androidx.compose.foundation.clickable
import androidx.compose.foundation.layout.*
import androidx.compose.foundation.lazy.LazyColumn
import androidx.compose.foundation.lazy.items
import androidx.compose.material.icons.Icons
import androidx.compose.material.icons.filled.*
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
import java.text.SimpleDateFormat
import java.util.*

/**
 * GroupInfoScreen - Tela de informações do grupo
 *
 * Exibe detalhes do grupo, lista de membros e controles de administração.
 */
@OptIn(ExperimentalMaterial3Api::class)
@Composable
fun GroupInfoScreen(
    groupId: String,
    onNavigateBack: () -> Unit
) {
    val scope = rememberCoroutineScope()

    var group by remember { mutableStateOf<FfiGroup?>(null) }
    var members by remember { mutableStateOf<List<String>>(emptyList()) }
    var isLoading by remember { mutableStateOf(true) }
    var errorMessage by remember { mutableStateOf<String?>(null) }
    var showLeaveDialog by remember { mutableStateOf(false) }
    var showAddMemberDialog by remember { mutableStateOf(false) }
    var showEditDialog by remember { mutableStateOf(false) }
    val localPeerId by MePassaClientWrapper.localPeerId.collectAsState()

    // Carregar informações do grupo
    LaunchedEffect(groupId) {
        scope.launch {
            try {
                val groups = MePassaClientWrapper.getGroups()
                group = groups.find { it.id == groupId }
                members = MePassaClientWrapper.getGroupMembers(groupId)
                isLoading = false

                if (group == null) {
                    errorMessage = "Grupo não encontrado"
                }
            } catch (e: Exception) {
                errorMessage = "Erro ao carregar grupo: ${e.message}"
                isLoading = false
            }
        }
    }

    Scaffold(
        topBar = {
            TopAppBar(
                title = {
                    Text("Informações do Grupo")
                },
                navigationIcon = {
                    IconButton(onClick = onNavigateBack) {
                        Icon(
                            Icons.Filled.ArrowBack,
                            contentDescription = "Voltar"
                        )
                    }
                },
                colors = TopAppBarDefaults.topAppBarColors(
                    containerColor = MaterialTheme.colorScheme.primaryContainer,
                    titleContentColor = MaterialTheme.colorScheme.onPrimaryContainer
                )
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
                        Button(onClick = onNavigateBack) {
                            Text("Voltar")
                        }
                    }
                }
            }
            group != null -> {
                LazyColumn(
                    modifier = Modifier
                        .fillMaxSize()
                        .padding(paddingValues),
                    contentPadding = PaddingValues(16.dp),
                    verticalArrangement = Arrangement.spacedBy(16.dp)
                ) {
                    // Cabeçalho do grupo
                    item {
                        GroupHeader(group = group!!)
                    }

                    // Descrição
                    if (group!!.description != null) {
                        item {
                            Card {
                                Column(
                                    modifier = Modifier.padding(16.dp)
                                ) {
                                    Text(
                                        text = "Descrição",
                                        style = MaterialTheme.typography.titleSmall,
                                        color = MaterialTheme.colorScheme.primary
                                    )
                                    Spacer(modifier = Modifier.height(8.dp))
                                    Text(
                                        text = group!!.description!!,
                                        style = MaterialTheme.typography.bodyMedium
                                    )
                                }
                            }
                        }
                    }

                    // Seção de membros
                    item {
                        Text(
                            text = "${group!!.memberCount} ${if (group!!.memberCount == 1u) "Membro" else "Membros"}",
                            style = MaterialTheme.typography.titleMedium,
                            color = MaterialTheme.colorScheme.primary
                        )
                    }

                    // Lista de membros (peer IDs)
                    items(members.size) { index ->
                        val member = members[index]
                        val isMe = member == localPeerId
                        Card {
                            ListItem(
                                headlineContent = {
                                    Text(
                                        text = member.take(24) + if (member.length > 24) "..." else "",
                                        style = MaterialTheme.typography.bodyMedium
                                    )
                                },
                                supportingContent = {
                                    val roles = buildList {
                                        if (isMe) add("Você")
                                        if (member == group!!.creatorPeerId) add("Admin")
                                    }
                                    if (roles.isNotEmpty()) {
                                        Text(roles.joinToString(" · "))
                                    }
                                },
                                leadingContent = {
                                    Icon(
                                        if (isMe) Icons.Default.Person else Icons.Default.Group,
                                        contentDescription = null
                                    )
                                }
                            )
                        }
                    }

                    // Ações do grupo
                    item {
                        Text(
                            text = "Ações",
                            style = MaterialTheme.typography.titleMedium,
                            color = MaterialTheme.colorScheme.primary
                        )
                    }

                    // Botão de adicionar membro (apenas admin)
                    if (group!!.isAdmin) {
                        item {
                            Card(
                                modifier = Modifier
                                    .clickable { showAddMemberDialog = true }
                                    .testTag("groupinfo_add_member")
                            ) {
                                ListItem(
                                    headlineContent = {
                                        Text("Adicionar membro")
                                    },
                                    leadingContent = {
                                        Icon(
                                            Icons.Default.PersonAdd,
                                            contentDescription = null,
                                            tint = MaterialTheme.colorScheme.primary
                                        )
                                    },
                                    trailingContent = {
                                        Icon(
                                            Icons.Default.ChevronRight,
                                            contentDescription = null
                                        )
                                    }
                                )
                            }
                        }

                        // Botão de editar grupo (apenas admin)
                        item {
                            Card(
                                modifier = Modifier.clickable { showEditDialog = true }
                            ) {
                                ListItem(
                                    headlineContent = {
                                        Text("Editar informações")
                                    },
                                    leadingContent = {
                                        Icon(
                                            Icons.Default.Edit,
                                            contentDescription = null,
                                            tint = MaterialTheme.colorScheme.primary
                                        )
                                    },
                                    trailingContent = {
                                        Icon(
                                            Icons.Default.ChevronRight,
                                            contentDescription = null
                                        )
                                    }
                                )
                            }
                        }
                    }

                    // Botão de sair do grupo
                    item {
                        Card(
                            modifier = Modifier
                                .clickable { showLeaveDialog = true }
                                .testTag("groupinfo_leave")
                        ) {
                            ListItem(
                                headlineContent = {
                                    Text(
                                        "Sair do grupo",
                                        color = MaterialTheme.colorScheme.error
                                    )
                                },
                                leadingContent = {
                                    Icon(
                                        Icons.Default.ExitToApp,
                                        contentDescription = null,
                                        tint = MaterialTheme.colorScheme.error
                                    )
                                },
                                trailingContent = {
                                    Icon(
                                        Icons.Default.ChevronRight,
                                        contentDescription = null
                                    )
                                }
                            )
                        }
                    }

                    // Informações do grupo
                    item {
                        Card {
                            Column(
                                modifier = Modifier.padding(16.dp),
                                verticalArrangement = Arrangement.spacedBy(8.dp)
                            ) {
                                Text(
                                    text = "Informações",
                                    style = MaterialTheme.typography.titleSmall,
                                    color = MaterialTheme.colorScheme.primary
                                )

                                InfoRow(
                                    label = "ID do Grupo",
                                    value = group!!.id.take(16) + "..."
                                )

                                InfoRow(
                                    label = "Criador",
                                    value = group!!.creatorPeerId.take(16) + "..."
                                )

                                InfoRow(
                                    label = "Criado em",
                                    value = formatDate(group!!.createdAt)
                                )

                                if (group!!.isAdmin) {
                                    InfoRow(
                                        label = "Seu papel",
                                        value = "Administrador"
                                    )
                                }
                            }
                        }
                    }
                }
            }
        }
    }

    // Dialog de confirmação para sair
    if (showLeaveDialog) {
        AlertDialog(
            onDismissRequest = { showLeaveDialog = false },
            title = {
                Text("Sair do Grupo")
            },
            text = {
                Text("Tem certeza que deseja sair de \"${group?.name}\"? Você precisará ser adicionado novamente para voltar.")
            },
            confirmButton = {
                TextButton(
                    onClick = {
                        scope.launch {
                            try {
                                MePassaClientWrapper.leaveGroup(groupId)
                                showLeaveDialog = false
                                onNavigateBack()
                            } catch (e: Exception) {
                                errorMessage = "Erro ao sair do grupo: ${e.message}"
                                showLeaveDialog = false
                            }
                        }
                    }
                ) {
                    Text("Sair", color = MaterialTheme.colorScheme.error)
                }
            },
            dismissButton = {
                TextButton(onClick = { showLeaveDialog = false }) {
                    Text("Cancelar")
                }
            }
        )
    }

    // Dialog de adicionar membro
    if (showAddMemberDialog) {
        AddMemberDialog(
            groupId = groupId,
            onDismiss = { showAddMemberDialog = false },
            onSuccess = {
                showAddMemberDialog = false
                // Recarregar informações do grupo
                scope.launch {
                    val groups = MePassaClientWrapper.getGroups()
                    group = groups.find { it.id == groupId }
                }
            }
        )
    }

    // Dialog de editar grupo
    if (showEditDialog) {
        EditGroupDialog(
            group = group!!,
            onDismiss = { showEditDialog = false },
            onSuccess = {
                showEditDialog = false
                // Recarregar informações do grupo
                scope.launch {
                    val groups = MePassaClientWrapper.getGroups()
                    group = groups.find { it.id == groupId }
                }
            }
        )
    }
}

/**
 * Cabeçalho com ícone e nome do grupo
 */
@Composable
fun GroupHeader(group: FfiGroup) {
    Column(
        modifier = Modifier.fillMaxWidth(),
        horizontalAlignment = Alignment.CenterHorizontally
    ) {
        // Ícone do grupo
        Surface(
            modifier = Modifier.size(80.dp),
            shape = MaterialTheme.shapes.large,
            color = MaterialTheme.colorScheme.primaryContainer
        ) {
            Box(
                contentAlignment = Alignment.Center
            ) {
                Icon(
                    imageVector = Icons.Default.Group,
                    contentDescription = null,
                    modifier = Modifier.size(40.dp),
                    tint = MaterialTheme.colorScheme.onPrimaryContainer
                )
            }
        }

        Spacer(modifier = Modifier.height(16.dp))

        // Nome do grupo
        Text(
            text = group.name,
            style = MaterialTheme.typography.headlineMedium
        )

        // Badge de admin
        if (group.isAdmin) {
            Spacer(modifier = Modifier.height(8.dp))
            Badge {
                Text("Administrador")
            }
        }
    }
}

/**
 * Linha de informação (label: valor)
 */
@Composable
fun InfoRow(label: String, value: String) {
    Row(
        modifier = Modifier.fillMaxWidth(),
        horizontalArrangement = Arrangement.SpaceBetween
    ) {
        Text(
            text = label,
            style = MaterialTheme.typography.bodyMedium,
            color = MaterialTheme.colorScheme.onSurfaceVariant
        )
        Text(
            text = value,
            style = MaterialTheme.typography.bodyMedium,
            fontFamily = androidx.compose.ui.text.font.FontFamily.Monospace
        )
    }
}

/**
 * Dialog para adicionar membro
 */
@Composable
fun AddMemberDialog(
    groupId: String,
    onDismiss: () -> Unit,
    onSuccess: () -> Unit
) {
    val scope = rememberCoroutineScope()
    var peerIdInput by remember { mutableStateOf("") }
    var isAdding by remember { mutableStateOf(false) }
    var errorMessage by remember { mutableStateOf<String?>(null) }

    AlertDialog(
        onDismissRequest = onDismiss,
        title = {
            Text("Adicionar Membro")
        },
        text = {
            Column(
                verticalArrangement = Arrangement.spacedBy(8.dp)
            ) {
                OutlinedTextField(
                    value = peerIdInput,
                    onValueChange = { peerIdInput = it },
                    label = { Text("Peer ID") },
                    placeholder = { Text("12D3KooW...") },
                    singleLine = true,
                    modifier = Modifier.fillMaxWidth(),
                    enabled = !isAdding
                )

                if (errorMessage != null) {
                    Text(
                        text = errorMessage!!,
                        color = MaterialTheme.colorScheme.error,
                        style = MaterialTheme.typography.bodySmall
                    )
                }
            }
        },
        confirmButton = {
            TextButton(
                onClick = {
                    scope.launch {
                        isAdding = true
                        errorMessage = null
                        try {
                            // O core envia invite + sender keys automaticamente
                            // (protocolo in-band de grupo)
                            MePassaClientWrapper.addGroupMember(groupId, peerIdInput.trim())
                            onSuccess()
                        } catch (e: Exception) {
                            errorMessage = "Erro ao adicionar: ${e.message}"
                        } finally {
                            isAdding = false
                        }
                    }
                },
                enabled = peerIdInput.trim().isNotEmpty() && !isAdding
            ) {
                if (isAdding) {
                    CircularProgressIndicator(
                        modifier = Modifier.size(16.dp),
                        strokeWidth = 2.dp
                    )
                } else {
                    Text("Adicionar")
                }
            }
        },
        dismissButton = {
            TextButton(
                onClick = onDismiss,
                enabled = !isAdding
            ) {
                Text("Cancelar")
            }
        }
    )
}

/**
 * Dialog para editar grupo
 */
@Composable
fun EditGroupDialog(
    group: FfiGroup,
    onDismiss: () -> Unit,
    onSuccess: () -> Unit
) {
    val scope = rememberCoroutineScope()
    var nameInput by remember { mutableStateOf(group.name) }
    var descriptionInput by remember { mutableStateOf(group.description ?: "") }
    var isSaving by remember { mutableStateOf(false) }
    var errorMessage by remember { mutableStateOf<String?>(null) }

    AlertDialog(
        onDismissRequest = onDismiss,
        title = {
            Text("Editar Grupo")
        },
        text = {
            Column(
                verticalArrangement = Arrangement.spacedBy(12.dp)
            ) {
                OutlinedTextField(
                    value = nameInput,
                    onValueChange = { nameInput = it },
                    label = { Text("Nome do grupo") },
                    singleLine = true,
                    modifier = Modifier.fillMaxWidth(),
                    enabled = !isSaving
                )

                OutlinedTextField(
                    value = descriptionInput,
                    onValueChange = { descriptionInput = it },
                    label = { Text("Descrição") },
                    maxLines = 3,
                    modifier = Modifier.fillMaxWidth(),
                    enabled = !isSaving
                )

                if (errorMessage != null) {
                    Text(
                        text = errorMessage!!,
                        color = MaterialTheme.colorScheme.error,
                        style = MaterialTheme.typography.bodySmall
                    )
                }
            }
        },
        confirmButton = {
            TextButton(
                onClick = {
                    scope.launch {
                        isSaving = true
                        errorMessage = null
                        try {
                            val ok = MePassaClientWrapper.updateGroup(
                                group.id,
                                nameInput.trim(),
                                descriptionInput.trim().ifEmpty { null }
                            )
                            if (ok) {
                                onSuccess()
                            } else {
                                errorMessage = "Erro ao salvar alterações"
                            }
                        } catch (e: Exception) {
                            errorMessage = "Erro ao salvar: ${e.message}"
                        } finally {
                            isSaving = false
                        }
                    }
                },
                enabled = nameInput.trim().isNotEmpty() && !isSaving
            ) {
                if (isSaving) {
                    CircularProgressIndicator(
                        modifier = Modifier.size(16.dp),
                        strokeWidth = 2.dp
                    )
                } else {
                    Text("Salvar")
                }
            }
        },
        dismissButton = {
            TextButton(
                onClick = onDismiss,
                enabled = !isSaving
            ) {
                Text("Cancelar")
            }
        }
    )
}

/**
 * Formata timestamp para data legível
 */
private fun formatDate(timestamp: Long): String {
    val date = Date(timestamp * 1000)
    return SimpleDateFormat("dd/MM/yyyy HH:mm", Locale.getDefault()).format(date)
}
