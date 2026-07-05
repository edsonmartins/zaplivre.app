package com.mepassa.ui.screens.group

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
import androidx.compose.ui.platform.testTag
import androidx.compose.ui.text.style.TextOverflow
import androidx.compose.ui.unit.dp
import com.mepassa.core.MePassaClientWrapper
import kotlinx.coroutines.launch
import uniffi.mepassa.FfiGroup
import java.text.SimpleDateFormat
import java.util.*

/**
 * GroupListScreen - Lista de grupos
 *
 * Exibe todos os grupos do usuário.
 * Permite criar novo grupo e navegar para chat de grupo.
 */
@OptIn(ExperimentalMaterial3Api::class)
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
                groups = MePassaClientWrapper.getGroups()
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
                    groups = MePassaClientWrapper.getGroups()
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
                    Text("Grupos")
                },
                navigationIcon = {
                    IconButton(onClick = onBack) {
                        Icon(Icons.Default.Group, contentDescription = "Voltar")
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
                onClick = { showCreateGroupDialog = true },
                modifier = Modifier.testTag("grouplist_fab")
            ) {
                Icon(Icons.Default.Add, contentDescription = "Criar grupo")
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
                errorMessage != null -> {
                    Column(
                        modifier = Modifier
                            .fillMaxSize()
                            .padding(32.dp),
                        horizontalAlignment = Alignment.CenterHorizontally,
                        verticalArrangement = Arrangement.Center
                    ) {
                        Text(
                            text = errorMessage ?: "",
                            style = MaterialTheme.typography.bodyLarge,
                            color = MaterialTheme.colorScheme.error
                        )
                        Spacer(modifier = Modifier.height(16.dp))
                        Button(onClick = {
                            errorMessage = null
                            isLoading = true
                            scope.launch {
                                try {
                                    groups = MePassaClientWrapper.getGroups()
                                    isLoading = false
                                } catch (e: Exception) {
                                    errorMessage = "Erro ao carregar grupos: ${e.message}"
                                    isLoading = false
                                }
                            }
                        }) {
                            Text("Tentar novamente")
                        }
                    }
                }
                groups.isEmpty() -> {
                    Column(
                        modifier = Modifier
                            .fillMaxSize()
                            .padding(32.dp),
                        horizontalAlignment = Alignment.CenterHorizontally,
                        verticalArrangement = Arrangement.Center
                    ) {
                        Icon(
                            imageVector = Icons.Default.Group,
                            contentDescription = null,
                            modifier = Modifier.size(64.dp),
                            tint = MaterialTheme.colorScheme.onSurfaceVariant.copy(alpha = 0.5f)
                        )
                        Spacer(modifier = Modifier.height(16.dp))
                        Text(
                            text = "Nenhum grupo ainda",
                            style = MaterialTheme.typography.titleMedium,
                            color = MaterialTheme.colorScheme.onSurfaceVariant
                        )
                        Spacer(modifier = Modifier.height(8.dp))
                        Text(
                            text = "Crie ou entre em um grupo para começar",
                            style = MaterialTheme.typography.bodyMedium,
                            color = MaterialTheme.colorScheme.onSurfaceVariant.copy(alpha = 0.7f)
                        )
                    }
                }
                else -> {
                    LazyColumn(
                        modifier = Modifier.fillMaxSize()
                    ) {
                        items(groups) { group ->
                            GroupItem(
                                group = group,
                                onClick = {
                                    onGroupClick(group.id)
                                }
                            )
                            Divider()
                        }
                    }
                }
            }
        }
    }

    // Dialog para criar grupo
    if (showCreateGroupDialog) {
        CreateGroupDialog(
            onDismiss = { showCreateGroupDialog = false },
            onConfirm = { name, description ->
                scope.launch {
                    try {
                        val group = MePassaClientWrapper.createGroup(name, description)
                        showCreateGroupDialog = false
                        // Recarregar lista de grupos
                        groups = MePassaClientWrapper.getGroups()
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
 * Item de grupo individual
 */
@Composable
fun GroupItem(
    group: FfiGroup,
    onClick: () -> Unit
) {
    ListItem(
        headlineContent = {
            Text(
                text = group.name,
                style = MaterialTheme.typography.titleMedium,
                maxLines = 1,
                overflow = TextOverflow.Ellipsis
            )
        },
        supportingContent = {
            Column {
                if (group.description != null) {
                    Text(
                        text = group.description!!,
                        style = MaterialTheme.typography.bodySmall,
                        color = MaterialTheme.colorScheme.onSurfaceVariant,
                        maxLines = 1,
                        overflow = TextOverflow.Ellipsis
                    )
                }
                Text(
                    text = "${group.memberCount} ${if (group.memberCount == 1u) "membro" else "membros"}",
                    style = MaterialTheme.typography.labelSmall,
                    color = MaterialTheme.colorScheme.onSurfaceVariant.copy(alpha = 0.7f)
                )
            }
        },
        leadingContent = {
            Icon(
                imageVector = Icons.Default.Group,
                contentDescription = null,
                modifier = Modifier.size(40.dp),
                tint = MaterialTheme.colorScheme.primary
            )
        },
        trailingContent = {
            if (group.isAdmin) {
                Badge {
                    Text("Admin", style = MaterialTheme.typography.labelSmall)
                }
            }
        },
        modifier = Modifier.clickable(onClick = onClick)
    )
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
