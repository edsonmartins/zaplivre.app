package com.zaplivre.ui.screens.onboarding

import androidx.compose.foundation.layout.*
import androidx.compose.material3.*
import androidx.compose.runtime.*
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.platform.LocalContext
import androidx.compose.ui.platform.testTag
import androidx.compose.ui.res.stringResource
import androidx.compose.ui.text.style.TextAlign
import androidx.compose.ui.unit.dp
import com.zaplivre.R
import com.zaplivre.core.ZapLivreClientWrapper
import com.zaplivre.ui.components.ZapGradientButton
import com.zaplivre.ui.components.ZapLogo
import com.zaplivre.ui.theme.ZapColor
import com.zaplivre.ui.theme.ZapType
import kotlinx.coroutines.launch

/**
 * OnboardingScreen - Primeira tela do app
 *
 * Exibida apenas na primeira execução.
 * Responsável por:
 * - Inicializar ZapLivreClient (gerar keypair)
 * - Mostrar mensagem de boas-vindas
 * - Redirecionar para Conversations após setup
 */
@Composable
fun OnboardingScreen(
    onOnboardingComplete: () -> Unit
) {
    val context = LocalContext.current
    val scope = rememberCoroutineScope()

    var isInitializing by remember { mutableStateOf(false) }
    var localPeerId by remember { mutableStateOf<String?>(null) }
    var showImportDialog by remember { mutableStateOf(false) }
    var importText by remember { mutableStateOf("") }
    var importError by remember { mutableStateOf<String?>(null) }
    var showUsernameDialog by remember { mutableStateOf(false) }
    var username by remember { mutableStateOf("") }
    var usernameError by remember { mutableStateOf<String?>(null) }

    // Observar estado de inicialização
    val isInitialized by ZapLivreClientWrapper.isInitialized.collectAsState()
    val clientPeerId by ZapLivreClientWrapper.localPeerId.collectAsState()

    // Auto-complete quando inicializado
    LaunchedEffect(isInitialized) {
        if (isInitialized) {
            localPeerId = clientPeerId
            isInitializing = false
            showUsernameDialog = true
            // Iniciar o foreground service (na primeira execução ele parou
            // aguardando o onboarding decidir criar/restaurar identidade)
            com.zaplivre.service.ZapLivreService.start(context)
            // Pequeno delay para usuário ver o peer ID
            kotlinx.coroutines.delay(500)
        }
    }

    Scaffold(containerColor = ZapColor.canvas) { paddingValues ->
        Box(
            modifier = Modifier
                .fillMaxSize()
                .padding(paddingValues)
                .padding(28.dp),
            contentAlignment = Alignment.Center
        ) {
            Column(
                horizontalAlignment = Alignment.CenterHorizontally,
                verticalArrangement = Arrangement.spacedBy(20.dp)
            ) {
                Spacer(modifier = Modifier.weight(0.6f))

                // Logo (raio + gradiente spark) + wordmark
                ZapLogo(size = 104.dp)

                Text(
                    text = "ZapLivre",
                    style = ZapType.brand,
                    color = ZapColor.ink,
                )

                // Subtitle
                Text(
                    text = stringResource(R.string.onboarding_subtitle),
                    style = ZapType.preview,
                    textAlign = TextAlign.Center,
                    color = ZapColor.slate
                )

                Spacer(modifier = Modifier.height(4.dp))

                // Status / Peer ID
                if (isInitializing || isInitialized) {
                    Card(
                        modifier = Modifier.fillMaxWidth(),
                        colors = CardDefaults.cardColors(
                            containerColor = MaterialTheme.colorScheme.surfaceVariant
                        )
                    ) {
                        Column(
                            modifier = Modifier.padding(16.dp),
                            horizontalAlignment = Alignment.CenterHorizontally
                        ) {
                            if (isInitializing && !isInitialized) {
                                CircularProgressIndicator(
                                    modifier = Modifier.size(32.dp)
                                )
                                Spacer(modifier = Modifier.height(8.dp))
                                Text(
                                    text = stringResource(R.string.onboarding_generating),
                                    style = MaterialTheme.typography.bodyMedium
                                )
                            }

                            if (localPeerId != null) {
                                Text(
                                    text = "Seu Peer ID:",
                                    style = MaterialTheme.typography.labelSmall,
                                    color = MaterialTheme.colorScheme.onSurfaceVariant
                                )
                                Spacer(modifier = Modifier.height(4.dp))
                                Text(
                                    text = localPeerId!!.take(16) + "...",
                                    style = MaterialTheme.typography.bodySmall,
                                    fontFamily = androidx.compose.ui.text.font.FontFamily.Monospace
                                )
                            }
                        }
                    }
                }

                Spacer(modifier = Modifier.weight(1f))

                // Botão começar (gradiente spark — ação principal)
                ZapGradientButton(
                    text = stringResource(R.string.onboarding_button),
                    onClick = {
                        isInitializing = true
                        scope.launch {
                            val success = ZapLivreClientWrapper.initialize(context)
                            if (!success) {
                                isInitializing = false
                            }
                        }
                    },
                    enabled = !isInitializing && !isInitialized,
                    modifier = Modifier.testTag("onboarding_create"),
                )

                OutlinedButton(
                    onClick = { showImportDialog = true },
                    modifier = Modifier
                        .fillMaxWidth()
                        .height(54.dp)
                        .testTag("onboarding_restore"),
                    shape = MaterialTheme.shapes.small,
                    enabled = !isInitializing && !isInitialized
                ) {
                    Text(
                        text = stringResource(R.string.onboarding_import_button),
                        style = ZapType.rowName,
                        color = ZapColor.primary,
                    )
                }
            }
        }
    }

    if (showImportDialog) {
        AlertDialog(
            onDismissRequest = { showImportDialog = false },
            title = { Text(text = stringResource(R.string.onboarding_import_title)) },
            text = {
                Column(verticalArrangement = Arrangement.spacedBy(8.dp)) {
                    Text(text = stringResource(R.string.onboarding_import_hint))
                    OutlinedTextField(
                        value = importText,
                        onValueChange = { importText = it },
                        modifier = Modifier.fillMaxWidth(),
                        minLines = 4
                    )
                    if (importError != null) {
                        Text(
                            text = importError ?: "",
                            color = MaterialTheme.colorScheme.error
                        )
                    }
                }
            },
            confirmButton = {
                TextButton(
                    enabled = importText.trim().isNotEmpty(),
                    onClick = {
                        isInitializing = true
                        importError = null
                        scope.launch {
                            val ok = ZapLivreClientWrapper.importIdentity(context, importText)
                            if (!ok) {
                                importError = context.getString(R.string.onboarding_import_failed)
                                isInitializing = false
                                return@launch
                            }
                            val success = ZapLivreClientWrapper.initialize(context)
                            if (!success) {
                                importError = context.getString(R.string.onboarding_import_failed)
                                isInitializing = false
                            } else {
                                showImportDialog = false
                            }
                        }
                    }
                ) {
                    Text(text = stringResource(R.string.onboarding_import_confirm))
                }
            },
            dismissButton = {
                TextButton(onClick = { showImportDialog = false }) {
                    Text(text = stringResource(R.string.onboarding_import_cancel))
                }
            }
        )
    }

    if (showUsernameDialog) {
        AlertDialog(
            onDismissRequest = { },
            title = { Text("Escolha seu username") },
            text = {
                Column(verticalArrangement = Arrangement.spacedBy(8.dp)) {
                    Text("Use 3 a 20 caracteres: letras minúsculas, números e underscore.")
                    OutlinedTextField(
                        value = username,
                        onValueChange = { username = it.lowercase(); usernameError = null },
                        singleLine = true,
                        modifier = Modifier.fillMaxWidth(),
                        placeholder = { Text("seu_username") }
                    )
                    usernameError?.let { Text(it, color = MaterialTheme.colorScheme.error) }
                }
            },
            confirmButton = {
                TextButton(
                    enabled = !isInitializing,
                    onClick = {
                        val value = username.trim()
                        if (!Regex("^[a-z0-9_]{3,20}$").matches(value)) {
                            usernameError = "Username inválido"
                            return@TextButton
                        }
                        isInitializing = true
                        scope.launch {
                            try {
                                ZapLivreClientWrapper.registerUsername(value)
                                showUsernameDialog = false
                                onOnboardingComplete()
                            } catch (error: Exception) {
                                usernameError = error.message ?: "Não foi possível registrar o username"
                            } finally {
                                isInitializing = false
                            }
                        }
                    }
                ) { Text("Registrar") }
            },
            dismissButton = {
                TextButton(onClick = {
                    showUsernameDialog = false
                    onOnboardingComplete()
                }) { Text("Continuar sem username") }
            }
        )
    }
}
