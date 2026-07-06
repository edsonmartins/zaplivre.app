package com.zaplivre.ui.screens.profile

import android.content.ClipData
import android.content.ClipboardManager
import android.content.Context
import androidx.compose.foundation.Image
import androidx.compose.foundation.background
import androidx.compose.foundation.layout.*
import androidx.compose.foundation.shape.CircleShape
import androidx.compose.material.icons.Icons
import androidx.compose.material.icons.filled.ArrowBack
import androidx.compose.material.icons.filled.ContentCopy
import androidx.compose.material.icons.filled.Edit
import androidx.compose.material.icons.filled.Person
import androidx.compose.material3.*
import androidx.compose.runtime.*
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.draw.clip
import androidx.compose.ui.graphics.Color
import androidx.compose.ui.platform.LocalContext
import androidx.compose.ui.platform.testTag
import androidx.compose.ui.text.font.FontFamily
import androidx.compose.ui.text.font.FontWeight
import androidx.compose.ui.unit.dp
import com.zaplivre.core.ZapLivreClientWrapper
import kotlinx.coroutines.launch

/**
 * ProfileScreen - User profile view
 */
@OptIn(ExperimentalMaterial3Api::class)
@Composable
fun ProfileScreen(
    onNavigateBack: () -> Unit,
    onNavigateToSettings: () -> Unit,
    modifier: Modifier = Modifier
) {
    val scope = rememberCoroutineScope()
    val context = LocalContext.current

    val profilePrefs = remember {
        context.getSharedPreferences("zaplivre_profile", android.content.Context.MODE_PRIVATE)
    }
    var userName by remember {
        mutableStateOf(profilePrefs.getString("display_name", null) ?: "Usuário ZapLivre")
    }
    var isEditingName by remember { mutableStateOf(false) }
    val localPeerId by ZapLivreClientWrapper.localPeerId.collectAsState()
    var showExportDialog by remember { mutableStateOf(false) }
    var exportData by remember { mutableStateOf("") }
    var exportError by remember { mutableStateOf<String?>(null) }
    var showExportErrorDialog by remember { mutableStateOf(false) }
    var showPrekeyDialog by remember { mutableStateOf(false) }
    var prekeyData by remember { mutableStateOf("") }
    var showPrekeyImportDialog by remember { mutableStateOf(false) }
    var prekeyImportPeerId by remember { mutableStateOf("") }
    var prekeyImportJson by remember { mutableStateOf("") }

    Scaffold(
        topBar = {
            TopAppBar(
                title = { Text("Perfil") },
                navigationIcon = {
                    IconButton(onClick = onNavigateBack) {
                        Icon(Icons.Default.ArrowBack, contentDescription = "Voltar")
                    }
                },
                actions = {
                    TextButton(onClick = onNavigateToSettings) {
                        Text("Configurações")
                    }
                },
                colors = TopAppBarDefaults.topAppBarColors(
                    containerColor = MaterialTheme.colorScheme.primaryContainer,
                    titleContentColor = MaterialTheme.colorScheme.onPrimaryContainer
                )
            )
        }
    ) { paddingValues ->
        Column(
            modifier = modifier
                .fillMaxSize()
                .padding(paddingValues)
                .padding(24.dp),
            horizontalAlignment = Alignment.CenterHorizontally,
            verticalArrangement = Arrangement.spacedBy(24.dp)
        ) {
            Spacer(modifier = Modifier.height(16.dp))

            // Avatar
            Box(
                modifier = Modifier.size(120.dp),
                contentAlignment = Alignment.Center
            ) {
                Box(
                    modifier = Modifier
                        .size(120.dp)
                        .clip(CircleShape)
                        .background(MaterialTheme.colorScheme.primaryContainer),
                    contentAlignment = Alignment.Center
                ) {
                    Icon(
                        imageVector = Icons.Default.Person,
                        contentDescription = "Avatar",
                        modifier = Modifier.size(64.dp),
                        tint = MaterialTheme.colorScheme.onPrimaryContainer
                    )
                }

                // Edit button
                FloatingActionButton(
                    onClick = { /* TODO: Open avatar picker */ },
                    modifier = Modifier
                        .size(40.dp)
                        .align(Alignment.BottomEnd),
                    containerColor = MaterialTheme.colorScheme.primary
                ) {
                    Icon(
                        Icons.Default.Edit,
                        contentDescription = "Editar avatar",
                        modifier = Modifier.size(20.dp)
                    )
                }
            }

            // Name
            if (isEditingName) {
                OutlinedTextField(
                    value = userName,
                    onValueChange = { userName = it },
                    label = { Text("Nome") },
                    singleLine = true,
                    modifier = Modifier
                        .fillMaxWidth()
                        .testTag("profile_name_input")
                )

                Row(
                    horizontalArrangement = Arrangement.spacedBy(8.dp)
                ) {
                    OutlinedButton(
                        onClick = { isEditingName = false }
                    ) {
                        Text("Cancelar")
                    }

                    Button(
                        onClick = {
                            // UX-06: nome local do dispositivo (sem protocolo
                            // de perfil sincronizado ainda)
                            profilePrefs.edit().putString("display_name", userName.trim()).apply()
                            isEditingName = false
                        },
                        modifier = Modifier.testTag("profile_save_name")
                    ) {
                        Text("Salvar")
                    }
                }
            } else {
                Text(
                    text = userName,
                    style = MaterialTheme.typography.headlineMedium,
                    fontWeight = FontWeight.Bold
                )

                TextButton(
                    onClick = { isEditingName = true },
                    modifier = Modifier.testTag("profile_edit_name")
                ) {
                    Text("Editar nome")
                }
            }

            Divider()

            // Peer ID section
            Column(
                modifier = Modifier.fillMaxWidth(),
                verticalArrangement = Arrangement.spacedBy(8.dp)
            ) {
                Text(
                    text = "Seu ID ZapLivre",
                    style = MaterialTheme.typography.titleMedium,
                    fontWeight = FontWeight.SemiBold
                )

                Surface(
                    modifier = Modifier.fillMaxWidth(),
                    shape = MaterialTheme.shapes.medium,
                    color = MaterialTheme.colorScheme.surfaceVariant
                ) {
                    Row(
                        modifier = Modifier
                            .fillMaxWidth()
                            .padding(16.dp),
                        horizontalArrangement = Arrangement.SpaceBetween,
                        verticalAlignment = Alignment.CenterVertically
                    ) {
                        Text(
                            text = (localPeerId?.take(32) ?: "") + "...",
                            style = MaterialTheme.typography.bodyMedium,
                            fontFamily = FontFamily.Monospace,
                            modifier = Modifier
                                .weight(1f)
                                .testTag("profile_peer_id")
                        )

                        IconButton(
                            onClick = {
                                val clipboard = context.getSystemService(Context.CLIPBOARD_SERVICE) as ClipboardManager
                                val clip = ClipData.newPlainText("Peer ID", localPeerId)
                                clipboard.setPrimaryClip(clip)

                                // TODO: Show toast
                            },
                            modifier = Modifier.testTag("profile_copy_peer_id")
                        ) {
                            Icon(
                                Icons.Default.ContentCopy,
                                contentDescription = "Copiar"
                            )
                        }
                    }
                }

                Text(
                    text = "Compartilhe este ID para que outros possam te adicionar",
                    style = MaterialTheme.typography.bodySmall,
                    color = MaterialTheme.colorScheme.onSurfaceVariant
                )
            }

            Button(
                onClick = {
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
                modifier = Modifier
                    .fillMaxWidth()
                    .height(56.dp)
            ) {
                Text("Exportar identidade")
            }

            OutlinedButton(
                onClick = {
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
                modifier = Modifier
                    .fillMaxWidth()
                    .height(56.dp)
            ) {
                Text("Exportar prekeys")
            }

            OutlinedButton(
                onClick = { showPrekeyImportDialog = true },
                modifier = Modifier
                    .fillMaxWidth()
                    .height(56.dp)
            ) {
                Text("Importar prekeys")
            }

            // QR Code placeholder
            Surface(
                modifier = Modifier
                    .size(200.dp)
                    .padding(16.dp)
                    .testTag("qr_image"),
                shape = MaterialTheme.shapes.medium,
                color = Color.White
            ) {
                Box(
                    contentAlignment = Alignment.Center
                ) {
                    Text(
                        text = "QR CODE\n${localPeerId?.take(8) ?: ""}...",
                        style = MaterialTheme.typography.bodySmall,
                        color = Color.Gray
                    )
                }
            }

            Text(
                text = "Escaneie para conectar",
                style = MaterialTheme.typography.bodySmall,
                color = MaterialTheme.colorScheme.onSurfaceVariant
            )
        }
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
