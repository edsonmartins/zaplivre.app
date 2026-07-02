package com.mepassa.ui.screens.settings

import androidx.compose.foundation.clickable
import androidx.compose.foundation.layout.*
import androidx.compose.foundation.lazy.LazyColumn
import androidx.compose.material.icons.Icons
import androidx.compose.material.icons.filled.ArrowBack
import androidx.compose.material3.*
import androidx.compose.runtime.*
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.unit.dp
import androidx.compose.ui.platform.LocalContext
import com.mepassa.core.MePassaClientWrapper
import kotlinx.coroutines.launch

/**
 * SettingsScreen - App settings
 */
@OptIn(ExperimentalMaterial3Api::class)
@Composable
fun SettingsScreen(
    onNavigateBack: () -> Unit,
    modifier: Modifier = Modifier
) {
    val context = LocalContext.current
    val scope = rememberCoroutineScope()

    var notificationsEnabled by remember { mutableStateOf(true) }
    var soundEnabled by remember { mutableStateOf(true) }
    var vibrationEnabled by remember { mutableStateOf(true) }
    var readReceiptsEnabled by remember { mutableStateOf(true) }
    var lastSeenEnabled by remember { mutableStateOf(true) }
    var showLogoutDialog by remember { mutableStateOf(false) }
    var showExportDialog by remember { mutableStateOf(false) }
    var exportData by remember { mutableStateOf("") }
    var exportError by remember { mutableStateOf<String?>(null) }
    var showExportErrorDialog by remember { mutableStateOf(false) }
    var showPrekeyDialog by remember { mutableStateOf(false) }
    var prekeyData by remember { mutableStateOf("") }
    var showPrekeyImportDialog by remember { mutableStateOf(false) }
    var prekeyImportPeerId by remember { mutableStateOf("") }
    var prekeyImportJson by remember { mutableStateOf("") }
    var storageUsedMb by remember { mutableStateOf("calculando...") }

    fun dirSizeBytes(dir: java.io.File): Long =
        dir.walkTopDown().filter { it.isFile }.sumOf { it.length() }

    fun refreshStorageUsage() {
        scope.launch(kotlinx.coroutines.Dispatchers.IO) {
            val dataDir = java.io.File(context.filesDir, "mepassa_data")
            val total = dirSizeBytes(dataDir) + dirSizeBytes(context.cacheDir)
            storageUsedMb = "%.1f MB".format(total / (1024.0 * 1024.0))
        }
    }

    LaunchedEffect(Unit) { refreshStorageUsage() }

    Scaffold(
        topBar = {
            TopAppBar(
                title = { Text("Configurações") },
                navigationIcon = {
                    IconButton(onClick = onNavigateBack) {
                        Icon(Icons.Default.ArrowBack, contentDescription = "Voltar")
                    }
                },
                colors = TopAppBarDefaults.topAppBarColors(
                    containerColor = MaterialTheme.colorScheme.primaryContainer,
                    titleContentColor = MaterialTheme.colorScheme.onPrimaryContainer
                )
            )
        }
    ) { paddingValues ->
        LazyColumn(
            modifier = modifier
                .fillMaxSize()
                .padding(paddingValues),
            contentPadding = PaddingValues(vertical = 8.dp)
        ) {
            // Notifications section
            item {
                SettingsSectionHeader("Notificações")
            }

            item {
                SettingsSwitchItem(
                    title = "Ativar notificações",
                    description = "Receber notificações de novas mensagens",
                    checked = notificationsEnabled,
                    onCheckedChange = { notificationsEnabled = it }
                )
            }

            item {
                SettingsSwitchItem(
                    title = "Som",
                    description = "Tocar som ao receber mensagens",
                    checked = soundEnabled,
                    onCheckedChange = { soundEnabled = it },
                    enabled = notificationsEnabled
                )
            }

            item {
                SettingsSwitchItem(
                    title = "Vibração",
                    description = "Vibrar ao receber mensagens",
                    checked = vibrationEnabled,
                    onCheckedChange = { vibrationEnabled = it },
                    enabled = notificationsEnabled
                )
            }

            item {
                Divider(modifier = Modifier.padding(vertical = 8.dp))
            }

            // Privacy section
            item {
                SettingsSectionHeader("Privacidade")
            }

            item {
                SettingsSwitchItem(
                    title = "Confirmações de leitura",
                    description = "Enviar confirmações quando ler mensagens",
                    checked = readReceiptsEnabled,
                    onCheckedChange = { readReceiptsEnabled = it }
                )
            }

            item {
                SettingsSwitchItem(
                    title = "Última visualização",
                    description = "Mostrar quando você esteve online",
                    checked = lastSeenEnabled,
                    onCheckedChange = { lastSeenEnabled = it }
                )
            }

            item {
                Divider(modifier = Modifier.padding(vertical = 8.dp))
            }

            // Identity section
            item {
                SettingsSectionHeader("Identidade")
            }

            item {
                SettingsClickableItem(
                    title = "Exportar backup da identidade",
                    description = "Copie o backup Base64 para restaurar em outro aparelho",
                    onClick = {
                        scope.launch {
                            val data = MePassaClientWrapper.exportIdentity(context)
                            if (data == null) {
                                exportError = "Backup não encontrado"
                                showExportErrorDialog = true
                            } else {
                                exportData = data
                                showExportDialog = true
                            }
                        }
                    }
                )
            }

            item {
                SettingsClickableItem(
                    title = "Exportar prekeys",
                    description = "Compartilhar chaves para E2E",
                    onClick = {
                        scope.launch {
                            val data = MePassaClientWrapper.exportPrekeyBundleJson()
                            if (data == null) {
                                exportError = "Prekeys não disponíveis"
                                showExportErrorDialog = true
                            } else {
                                prekeyData = data
                                showPrekeyDialog = true
                            }
                        }
                    }
                )
            }

            item {
                SettingsClickableItem(
                    title = "Importar prekeys do contato",
                    description = "Salvar as chaves do contato para E2E",
                    onClick = { showPrekeyImportDialog = true }
                )
            }

            item {
                Divider(modifier = Modifier.padding(vertical = 8.dp))
            }

            // Storage section
            item {
                SettingsSectionHeader("Armazenamento")
            }

            item {
                SettingsItem(
                    title = "Armazenamento usado",
                    description = storageUsedMb
                )
            }

            item {
                SettingsClickableItem(
                    title = "Limpar cache de imagens",
                    description = "Liberar espaço removendo imagens em cache",
                    onClick = {
                        scope.launch(kotlinx.coroutines.Dispatchers.IO) {
                            context.cacheDir.listFiles()?.forEach { it.deleteRecursively() }
                            refreshStorageUsage()
                        }
                    }
                )
            }

            item {
                SettingsClickableItem(
                    title = "Limpar cache de vídeos",
                    description = "Liberar espaço removendo vídeos em cache",
                    onClick = {
                        scope.launch(kotlinx.coroutines.Dispatchers.IO) {
                            // Cache temporário de mídia (arquivos .part de downloads)
                            java.io.File(context.filesDir, "mepassa_data/media/tmp")
                                .deleteRecursively()
                            context.externalCacheDir?.listFiles()?.forEach { it.deleteRecursively() }
                            refreshStorageUsage()
                        }
                    }
                )
            }

            item {
                Divider(modifier = Modifier.padding(vertical = 8.dp))
            }

            // About section
            item {
                SettingsSectionHeader("Sobre")
            }

            item {
                SettingsItem(
                    title = "Versão",
                    description = com.mepassa.BuildConfig.VERSION_NAME
                )
            }




            item {
                Divider(modifier = Modifier.padding(vertical = 8.dp))
            }

            // Logout
            item {
                SettingsClickableItem(
                    title = "Sair",
                    description = "Desconectar desta conta",
                    onClick = { showLogoutDialog = true },
                    destructive = true
                )
            }

            item {
                Spacer(modifier = Modifier.height(16.dp))
            }
        }
    }

    // Logout confirmation dialog
    if (showLogoutDialog) {
        AlertDialog(
            onDismissRequest = { showLogoutDialog = false },
            title = { Text("Sair") },
            text = {
                Text(
                    "Isso apaga sua identidade e todos os dados locais deste " +
                        "dispositivo. Sem um backup exportado, você perderá o " +
                        "acesso a este peer ID permanentemente. Continuar?"
                )
            },
            confirmButton = {
                Button(
                    onClick = {
                        showLogoutDialog = false
                        // Logout destrutivo: parar o service, apagar identidade
                        // segura + dados locais e encerrar o processo
                        com.mepassa.service.MePassaService.stop(context)
                        com.mepassa.core.AndroidIdentityStore.deleteIdentity(context)
                        java.io.File(context.filesDir, "mepassa_data").deleteRecursively()
                        (context as? android.app.Activity)?.finishAffinity()
                        Runtime.getRuntime().exit(0)
                    },
                    colors = ButtonDefaults.buttonColors(
                        containerColor = MaterialTheme.colorScheme.error
                    )
                ) {
                    Text("Apagar e sair")
                }
            },
            dismissButton = {
                TextButton(onClick = { showLogoutDialog = false }) {
                    Text("Cancelar")
                }
            }
        )
    }

    if (showExportDialog) {
        AlertDialog(
            onDismissRequest = { showExportDialog = false },
            title = { Text("Backup da identidade") },
            text = {
                Column(verticalArrangement = Arrangement.spacedBy(8.dp)) {
                    OutlinedTextField(
                        value = exportData,
                        onValueChange = {},
                        modifier = Modifier.fillMaxWidth(),
                        readOnly = true,
                        minLines = 4
                    )
                }
            },
            confirmButton = {
                TextButton(onClick = { showExportDialog = false }) {
                    Text("Fechar")
                }
            }
        )
    }

    if (showExportErrorDialog) {
        AlertDialog(
            onDismissRequest = { showExportErrorDialog = false },
            title = { Text("Erro") },
            text = { Text(exportError ?: "") },
            confirmButton = {
                TextButton(onClick = { showExportErrorDialog = false }) {
                    Text("OK")
                }
            }
        )
    }

    if (showPrekeyDialog) {
        AlertDialog(
            onDismissRequest = { showPrekeyDialog = false },
            title = { Text("Prekeys (JSON)") },
            text = {
                Column(verticalArrangement = Arrangement.spacedBy(8.dp)) {
                    OutlinedTextField(
                        value = prekeyData,
                        onValueChange = {},
                        modifier = Modifier.fillMaxWidth(),
                        readOnly = true,
                        minLines = 4
                    )
                }
            },
            confirmButton = {
                TextButton(onClick = { showPrekeyDialog = false }) {
                    Text("Fechar")
                }
            }
        )
    }

    if (showPrekeyImportDialog) {
        AlertDialog(
            onDismissRequest = { showPrekeyImportDialog = false },
            title = { Text("Importar prekeys") },
            text = {
                Column(verticalArrangement = Arrangement.spacedBy(8.dp)) {
                    OutlinedTextField(
                        value = prekeyImportPeerId,
                        onValueChange = { prekeyImportPeerId = it },
                        modifier = Modifier.fillMaxWidth(),
                        label = { Text("Peer ID") }
                    )
                    OutlinedTextField(
                        value = prekeyImportJson,
                        onValueChange = { prekeyImportJson = it },
                        modifier = Modifier.fillMaxWidth(),
                        minLines = 4,
                        label = { Text("Prekey JSON") }
                    )
                }
            },
            confirmButton = {
                TextButton(
                    enabled = prekeyImportPeerId.trim().isNotEmpty() && prekeyImportJson.trim().isNotEmpty(),
                    onClick = {
                        scope.launch {
                            val ok = MePassaClientWrapper.storePeerPrekeyBundle(
                                prekeyImportPeerId.trim(),
                                prekeyImportJson.trim()
                            )
                            if (!ok) {
                                exportError = "Falha ao salvar prekeys"
                                showExportErrorDialog = true
                            } else {
                                prekeyImportPeerId = ""
                                prekeyImportJson = ""
                                showPrekeyImportDialog = false
                            }
                        }
                    }
                ) {
                    Text("Salvar")
                }
            },
            dismissButton = {
                TextButton(onClick = { showPrekeyImportDialog = false }) {
                    Text("Cancelar")
                }
            }
        )
    }
}

@Composable
fun SettingsSectionHeader(title: String) {
    Text(
        text = title,
        style = MaterialTheme.typography.titleSmall,
        color = MaterialTheme.colorScheme.primary,
        modifier = Modifier.padding(horizontal = 16.dp, vertical = 8.dp)
    )
}

@Composable
fun SettingsItem(
    title: String,
    description: String,
    modifier: Modifier = Modifier
) {
    Column(
        modifier = modifier
            .fillMaxWidth()
            .padding(horizontal = 16.dp, vertical = 12.dp)
    ) {
        Text(
            text = title,
            style = MaterialTheme.typography.bodyLarge
        )

        Text(
            text = description,
            style = MaterialTheme.typography.bodySmall,
            color = MaterialTheme.colorScheme.onSurfaceVariant
        )
    }
}

@Composable
fun SettingsClickableItem(
    title: String,
    description: String,
    onClick: () -> Unit,
    destructive: Boolean = false,
    modifier: Modifier = Modifier
) {
    Column(
        modifier = modifier
            .fillMaxWidth()
            .clickable(onClick = onClick)
            .padding(horizontal = 16.dp, vertical = 12.dp)
    ) {
        Text(
            text = title,
            style = MaterialTheme.typography.bodyLarge,
            color = if (destructive) MaterialTheme.colorScheme.error else MaterialTheme.colorScheme.onSurface
        )

        Text(
            text = description,
            style = MaterialTheme.typography.bodySmall,
            color = MaterialTheme.colorScheme.onSurfaceVariant
        )
    }
}

@Composable
fun SettingsSwitchItem(
    title: String,
    description: String,
    checked: Boolean,
    onCheckedChange: (Boolean) -> Unit,
    enabled: Boolean = true,
    modifier: Modifier = Modifier
) {
    Row(
        modifier = modifier
            .fillMaxWidth()
            .clickable(enabled = enabled) { onCheckedChange(!checked) }
            .padding(horizontal = 16.dp, vertical = 8.dp),
        horizontalArrangement = Arrangement.SpaceBetween,
        verticalAlignment = Alignment.CenterVertically
    ) {
        Column(modifier = Modifier.weight(1f)) {
            Text(
                text = title,
                style = MaterialTheme.typography.bodyLarge,
                color = if (enabled) MaterialTheme.colorScheme.onSurface else MaterialTheme.colorScheme.onSurface.copy(alpha = 0.5f)
            )

            Text(
                text = description,
                style = MaterialTheme.typography.bodySmall,
                color = MaterialTheme.colorScheme.onSurfaceVariant.copy(alpha = if (enabled) 1f else 0.5f)
            )
        }

        Switch(
            checked = checked,
            onCheckedChange = onCheckedChange,
            enabled = enabled
        )
    }
}
