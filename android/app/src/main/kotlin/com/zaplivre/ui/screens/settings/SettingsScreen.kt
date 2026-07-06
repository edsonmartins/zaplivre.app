package com.zaplivre.ui.screens.settings

import androidx.compose.foundation.background
import androidx.compose.foundation.clickable
import androidx.compose.foundation.layout.*
import androidx.compose.foundation.lazy.LazyColumn
import androidx.compose.foundation.shape.RoundedCornerShape
import androidx.compose.material.icons.Icons
import androidx.compose.material.icons.filled.ArrowBack
import androidx.compose.material.icons.filled.Backup
import androidx.compose.material.icons.filled.DeleteSweep
import androidx.compose.material.icons.filled.Info
import androidx.compose.material.icons.filled.Key
import androidx.compose.material.icons.filled.Logout
import androidx.compose.material.icons.filled.Notifications
import androidx.compose.material.icons.filled.Storage
import androidx.compose.material.icons.filled.Verified
import androidx.compose.material.icons.filled.Vibration
import androidx.compose.material.icons.filled.Visibility
import androidx.compose.material.icons.filled.VolumeUp
import androidx.compose.material3.*
import androidx.compose.runtime.*
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.draw.clip
import androidx.compose.ui.graphics.Color
import androidx.compose.ui.graphics.vector.ImageVector
import androidx.compose.ui.platform.LocalContext
import androidx.compose.ui.platform.testTag
import androidx.compose.ui.text.font.FontFamily
import androidx.compose.ui.unit.dp
import com.zaplivre.core.ZapLivreClientWrapper
import com.zaplivre.ui.components.ZapAvatar
import com.zaplivre.ui.theme.ZapColor
import com.zaplivre.ui.theme.ZapMetric
import com.zaplivre.ui.theme.ZapType
import kotlinx.coroutines.launch

/**
 * SettingsScreen - configurações do app. Concentra o estado (toggles, diálogos,
 * uso de disco, chamadas ao client P2P) e delega toda a apresentação ao
 * composable stateless [SettingsContent].
 */
@Composable
fun SettingsScreen(
    onNavigateBack: () -> Unit,
    modifier: Modifier = Modifier
) {
    val context = LocalContext.current
    val scope = rememberCoroutineScope()

    val localPeerId by ZapLivreClientWrapper.localPeerId.collectAsState()

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
            val dataDir = java.io.File(context.filesDir, "zaplivre_data")
            val total = dirSizeBytes(dataDir) + dirSizeBytes(context.cacheDir)
            storageUsedMb = "%.1f MB".format(total / (1024.0 * 1024.0))
        }
    }

    LaunchedEffect(Unit) { refreshStorageUsage() }

    SettingsContent(
        modifier = modifier,
        peerId = localPeerId ?: "",
        name = "Você",
        storageUsed = storageUsedMb,
        appVersion = com.zaplivre.BuildConfig.VERSION_NAME,
        notificationsEnabled = notificationsEnabled,
        soundEnabled = soundEnabled,
        vibrationEnabled = vibrationEnabled,
        readReceiptsEnabled = readReceiptsEnabled,
        lastSeenEnabled = lastSeenEnabled,
        onNotificationsChange = { notificationsEnabled = it },
        onSoundChange = { soundEnabled = it },
        onVibrationChange = { vibrationEnabled = it },
        onReadReceiptsChange = { readReceiptsEnabled = it },
        onLastSeenChange = { lastSeenEnabled = it },
        onNavigateBack = onNavigateBack,
        onExportBackup = {
            scope.launch {
                val data = ZapLivreClientWrapper.exportIdentity(context)
                if (data == null) {
                    exportError = "Backup não encontrado"
                    showExportErrorDialog = true
                } else {
                    exportData = data
                    showExportDialog = true
                }
            }
        },
        onExportPrekeys = {
            scope.launch {
                val data = ZapLivreClientWrapper.exportPrekeyBundleJson()
                if (data == null) {
                    exportError = "Prekeys não disponíveis"
                    showExportErrorDialog = true
                } else {
                    prekeyData = data
                    showPrekeyDialog = true
                }
            }
        },
        onImportPrekeys = { showPrekeyImportDialog = true },
        onClearImageCache = {
            scope.launch(kotlinx.coroutines.Dispatchers.IO) {
                context.cacheDir.listFiles()?.forEach { it.deleteRecursively() }
                refreshStorageUsage()
            }
        },
        onClearVideoCache = {
            scope.launch(kotlinx.coroutines.Dispatchers.IO) {
                // Cache temporário de mídia (arquivos .part de downloads)
                java.io.File(context.filesDir, "zaplivre_data/media/tmp")
                    .deleteRecursively()
                context.externalCacheDir?.listFiles()?.forEach { it.deleteRecursively() }
                refreshStorageUsage()
            }
        },
        onLogout = { showLogoutDialog = true },
    )

    // Logout confirmation dialog
    if (showLogoutDialog) {
        AlertDialog(
            onDismissRequest = { showLogoutDialog = false },
            containerColor = ZapColor.surface,
            title = { Text("Sair", style = ZapType.title, color = ZapColor.ink) },
            text = {
                Text(
                    "Isso apaga sua identidade e todos os dados locais deste " +
                        "dispositivo. Sem um backup exportado, você perderá o " +
                        "acesso a este peer ID permanentemente. Continuar?",
                    style = ZapType.body,
                    color = ZapColor.slate,
                )
            },
            confirmButton = {
                Button(
                    onClick = {
                        showLogoutDialog = false
                        // Logout destrutivo: parar o service, apagar identidade
                        // segura + dados locais e encerrar o processo
                        com.zaplivre.service.ZapLivreService.stop(context)
                        com.zaplivre.core.AndroidIdentityStore.deleteIdentity(context)
                        java.io.File(context.filesDir, "zaplivre_data").deleteRecursively()
                        (context as? android.app.Activity)?.finishAffinity()
                        Runtime.getRuntime().exit(0)
                    },
                    colors = ButtonDefaults.buttonColors(
                        containerColor = ZapColor.danger
                    )
                ) {
                    Text("Apagar e sair", color = Color.White)
                }
            },
            dismissButton = {
                TextButton(onClick = { showLogoutDialog = false }) {
                    Text("Cancelar", color = ZapColor.slate)
                }
            }
        )
    }

    if (showExportDialog) {
        AlertDialog(
            onDismissRequest = { showExportDialog = false },
            containerColor = ZapColor.surface,
            title = { Text("Backup da identidade", style = ZapType.title, color = ZapColor.ink) },
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
                    Text("Fechar", color = ZapColor.primary)
                }
            }
        )
    }

    if (showExportErrorDialog) {
        AlertDialog(
            onDismissRequest = { showExportErrorDialog = false },
            containerColor = ZapColor.surface,
            title = { Text("Erro", style = ZapType.title, color = ZapColor.ink) },
            text = { Text(exportError ?: "", style = ZapType.body, color = ZapColor.slate) },
            confirmButton = {
                TextButton(onClick = { showExportErrorDialog = false }) {
                    Text("OK", color = ZapColor.primary)
                }
            }
        )
    }

    if (showPrekeyDialog) {
        AlertDialog(
            onDismissRequest = { showPrekeyDialog = false },
            containerColor = ZapColor.surface,
            title = { Text("Prekeys (JSON)", style = ZapType.title, color = ZapColor.ink) },
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
                    Text("Fechar", color = ZapColor.primary)
                }
            }
        )
    }

    if (showPrekeyImportDialog) {
        AlertDialog(
            onDismissRequest = { showPrekeyImportDialog = false },
            containerColor = ZapColor.surface,
            title = { Text("Importar prekeys", style = ZapType.title, color = ZapColor.ink) },
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
                            val ok = ZapLivreClientWrapper.storePeerPrekeyBundle(
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
                    Text("Salvar", color = ZapColor.primary)
                }
            },
            dismissButton = {
                TextButton(onClick = { showPrekeyImportDialog = false }) {
                    Text("Cancelar", color = ZapColor.slate)
                }
            }
        )
    }
}

/**
 * Apresentação stateless da tela de configurações. Recebe todos os valores e
 * callbacks por parâmetro para ser reutilizada pelo design preview.
 */
@OptIn(ExperimentalMaterial3Api::class)
@Composable
fun SettingsContent(
    peerId: String,
    name: String,
    storageUsed: String,
    appVersion: String,
    notificationsEnabled: Boolean,
    soundEnabled: Boolean,
    vibrationEnabled: Boolean,
    readReceiptsEnabled: Boolean,
    lastSeenEnabled: Boolean,
    onNotificationsChange: (Boolean) -> Unit,
    onSoundChange: (Boolean) -> Unit,
    onVibrationChange: (Boolean) -> Unit,
    onReadReceiptsChange: (Boolean) -> Unit,
    onLastSeenChange: (Boolean) -> Unit,
    onExportBackup: () -> Unit,
    onExportPrekeys: () -> Unit,
    onImportPrekeys: () -> Unit,
    onClearImageCache: () -> Unit,
    onClearVideoCache: () -> Unit,
    onLogout: () -> Unit,
    onNavigateBack: () -> Unit,
    modifier: Modifier = Modifier,
) {
    Scaffold(
        containerColor = ZapColor.canvas,
        topBar = {
            Column {
                TopAppBar(
                    title = { Text("Configurações", style = ZapType.title, color = ZapColor.ink) },
                    navigationIcon = {
                        IconButton(onClick = onNavigateBack) {
                            Icon(Icons.Default.ArrowBack, contentDescription = "Voltar", tint = ZapColor.slate)
                        }
                    },
                    colors = TopAppBarDefaults.topAppBarColors(
                        containerColor = ZapColor.canvas,
                        titleContentColor = ZapColor.ink,
                    )
                )
                Divider(color = ZapColor.hairline)
            }
        }
    ) { paddingValues ->
        LazyColumn(
            modifier = modifier
                .fillMaxSize()
                .padding(paddingValues),
            contentPadding = PaddingValues(bottom = 24.dp),
        ) {
            item { ProfileHeader(peerId = peerId, name = name) }

            item { SettingsSectionHeader("Notificações") }
            item {
                SettingsCard {
                    SettingsSwitchRow(
                        icon = Icons.Filled.Notifications,
                        title = "Ativar notificações",
                        description = "Receber notificações de novas mensagens",
                        checked = notificationsEnabled,
                        onCheckedChange = onNotificationsChange,
                        switchTestTag = "settings_toggle_notifications",
                    )
                    SettingsRowDivider()
                    SettingsSwitchRow(
                        icon = Icons.Filled.VolumeUp,
                        title = "Som",
                        description = "Tocar som ao receber mensagens",
                        checked = soundEnabled,
                        onCheckedChange = onSoundChange,
                        enabled = notificationsEnabled,
                        switchTestTag = "settings_toggle_sound",
                    )
                    SettingsRowDivider()
                    SettingsSwitchRow(
                        icon = Icons.Filled.Vibration,
                        title = "Vibração",
                        description = "Vibrar ao receber mensagens",
                        checked = vibrationEnabled,
                        onCheckedChange = onVibrationChange,
                        enabled = notificationsEnabled,
                        switchTestTag = "settings_toggle_vibration",
                    )
                }
            }

            item { SettingsSectionHeader("Privacidade") }
            item {
                SettingsCard {
                    SettingsSwitchRow(
                        icon = Icons.Filled.Visibility,
                        title = "Confirmações de leitura",
                        description = "Enviar confirmações quando ler mensagens",
                        checked = readReceiptsEnabled,
                        onCheckedChange = onReadReceiptsChange,
                        switchTestTag = "settings_toggle_read_receipts",
                    )
                    SettingsRowDivider()
                    SettingsSwitchRow(
                        icon = Icons.Filled.Visibility,
                        title = "Última visualização",
                        description = "Mostrar quando você esteve online",
                        checked = lastSeenEnabled,
                        onCheckedChange = onLastSeenChange,
                        switchTestTag = "settings_toggle_last_seen",
                    )
                }
            }

            item { SettingsSectionHeader("Identidade") }
            item {
                SettingsCard {
                    SettingsClickableRow(
                        icon = Icons.Filled.Backup,
                        title = "Exportar backup da identidade",
                        description = "Copie o backup Base64 para restaurar em outro aparelho",
                        onClick = onExportBackup,
                        modifier = Modifier.testTag("settings_export_backup"),
                    )
                    SettingsRowDivider()
                    SettingsClickableRow(
                        icon = Icons.Filled.Key,
                        title = "Exportar prekeys",
                        description = "Compartilhar chaves para E2E",
                        onClick = onExportPrekeys,
                    )
                    SettingsRowDivider()
                    SettingsClickableRow(
                        icon = Icons.Filled.Key,
                        title = "Importar prekeys do contato",
                        description = "Salvar as chaves do contato para E2E",
                        onClick = onImportPrekeys,
                    )
                }
            }

            item { SettingsSectionHeader("Armazenamento") }
            item {
                SettingsCard {
                    SettingsInfoRow(
                        icon = Icons.Filled.Storage,
                        title = "Armazenamento usado",
                        description = storageUsed,
                    )
                    SettingsRowDivider()
                    SettingsClickableRow(
                        icon = Icons.Filled.DeleteSweep,
                        title = "Limpar cache de imagens",
                        description = "Liberar espaço removendo imagens em cache",
                        onClick = onClearImageCache,
                    )
                    SettingsRowDivider()
                    SettingsClickableRow(
                        icon = Icons.Filled.DeleteSweep,
                        title = "Limpar cache de vídeos",
                        description = "Liberar espaço removendo vídeos em cache",
                        onClick = onClearVideoCache,
                    )
                }
            }

            item { SettingsSectionHeader("Sobre") }
            item {
                SettingsCard {
                    SettingsInfoRow(
                        icon = Icons.Filled.Info,
                        title = "Versão",
                        description = appVersion,
                    )
                }
            }

            item { Spacer(Modifier.height(8.dp)) }
            item {
                SettingsCard {
                    SettingsClickableRow(
                        icon = Icons.Filled.Logout,
                        title = "Sair",
                        description = "Desconectar desta conta",
                        onClick = onLogout,
                        destructive = true,
                        modifier = Modifier.testTag("settings_logout"),
                    )
                }
            }
        }
    }
}

@Composable
private fun ProfileHeader(peerId: String, name: String) {
    Row(
        modifier = Modifier
            .fillMaxWidth()
            .padding(horizontal = ZapMetric.gutter, vertical = 20.dp),
        verticalAlignment = Alignment.CenterVertically,
    ) {
        ZapAvatar(seed = peerId.ifEmpty { name }, name = name, size = 72.dp)
        Spacer(Modifier.width(ZapMetric.gutter))
        Column(modifier = Modifier.weight(1f)) {
            Text(text = name, style = ZapType.title, color = ZapColor.ink)
            Spacer(Modifier.height(4.dp))
            Text(
                text = if (peerId.isEmpty()) "—" else peerId.take(24) + "…",
                style = ZapType.caption,
                color = ZapColor.slate,
                fontFamily = FontFamily.Monospace,
            )
        }
    }
}

@Composable
fun SettingsSectionHeader(title: String) {
    Text(
        text = title.uppercase(),
        style = ZapType.caption,
        color = ZapColor.slate,
        modifier = Modifier.padding(
            start = ZapMetric.gutter,
            end = ZapMetric.gutter,
            top = 16.dp,
            bottom = 8.dp,
        )
    )
}

@Composable
private fun SettingsCard(content: @Composable ColumnScope.() -> Unit) {
    Column(
        modifier = Modifier
            .fillMaxWidth()
            .padding(horizontal = ZapMetric.gutter)
            .clip(RoundedCornerShape(ZapMetric.cardRadius))
            .background(ZapColor.surface),
        content = content,
    )
}

@Composable
private fun SettingsRowDivider() {
    Divider(
        color = ZapColor.hairline,
        modifier = Modifier.padding(start = ZapMetric.gutter + 40.dp),
    )
}

@Composable
private fun RowLeadingIcon(icon: ImageVector, tint: Color) {
    Icon(
        imageVector = icon,
        contentDescription = null,
        tint = tint,
        modifier = Modifier
            .padding(end = ZapMetric.rowGap)
            .size(24.dp),
    )
}

@Composable
fun SettingsInfoRow(
    icon: ImageVector,
    title: String,
    description: String,
    modifier: Modifier = Modifier,
) {
    Row(
        modifier = modifier
            .fillMaxWidth()
            .padding(horizontal = ZapMetric.gutter, vertical = 14.dp),
        verticalAlignment = Alignment.CenterVertically,
    ) {
        RowLeadingIcon(icon, ZapColor.slate)
        Column(modifier = Modifier.weight(1f)) {
            Text(text = title, style = ZapType.rowName, color = ZapColor.ink)
            Text(text = description, style = ZapType.caption, color = ZapColor.slate)
        }
    }
}

@Composable
fun SettingsClickableRow(
    icon: ImageVector,
    title: String,
    description: String,
    onClick: () -> Unit,
    destructive: Boolean = false,
    modifier: Modifier = Modifier,
) {
    val accent = if (destructive) ZapColor.danger else ZapColor.primary
    Row(
        modifier = modifier
            .fillMaxWidth()
            .clickable(onClick = onClick)
            .padding(horizontal = ZapMetric.gutter, vertical = 14.dp),
        verticalAlignment = Alignment.CenterVertically,
    ) {
        RowLeadingIcon(icon, accent)
        Column(modifier = Modifier.weight(1f)) {
            Text(
                text = title,
                style = ZapType.rowName,
                color = if (destructive) ZapColor.danger else ZapColor.ink,
            )
            Text(text = description, style = ZapType.caption, color = ZapColor.slate)
        }
    }
}

@Composable
fun SettingsSwitchRow(
    icon: ImageVector,
    title: String,
    description: String,
    checked: Boolean,
    onCheckedChange: (Boolean) -> Unit,
    enabled: Boolean = true,
    modifier: Modifier = Modifier,
    switchTestTag: String? = null,
) {
    val alpha = if (enabled) 1f else 0.5f
    Row(
        modifier = modifier
            .fillMaxWidth()
            .clickable(enabled = enabled) { onCheckedChange(!checked) }
            .padding(horizontal = ZapMetric.gutter, vertical = 10.dp),
        horizontalArrangement = Arrangement.SpaceBetween,
        verticalAlignment = Alignment.CenterVertically
    ) {
        RowLeadingIcon(icon, ZapColor.slate.copy(alpha = alpha))
        Column(modifier = Modifier.weight(1f)) {
            Text(
                text = title,
                style = ZapType.rowName,
                color = ZapColor.ink.copy(alpha = alpha),
            )
            Text(
                text = description,
                style = ZapType.caption,
                color = ZapColor.slate.copy(alpha = alpha),
            )
        }
        Switch(
            checked = checked,
            onCheckedChange = onCheckedChange,
            enabled = enabled,
            colors = SwitchDefaults.colors(checkedTrackColor = ZapColor.primary),
            modifier = if (switchTestTag != null) Modifier.testTag(switchTestTag) else Modifier
        )
    }
}
