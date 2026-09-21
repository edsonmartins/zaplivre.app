package com.zaplivre.ui.screens.onboarding

import androidx.compose.foundation.layout.*
import androidx.compose.material3.*
import androidx.compose.runtime.*
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.platform.LocalContext
import androidx.compose.ui.platform.testTag
import androidx.compose.ui.ExperimentalComposeUiApi
import androidx.compose.ui.semantics.semantics
import androidx.compose.ui.semantics.testTagsAsResourceId
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
@OptIn(ExperimentalComposeUiApi::class)
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
    val onboardingComplete by ZapLivreClientWrapper.onboardingComplete.collectAsState()

    // Auto-complete quando inicializado
    LaunchedEffect(isInitialized) {
        if (isInitialized) {
            localPeerId = clientPeerId
            isInitializing = false
            showUsernameDialog = !onboardingComplete
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

                // Logo (glyph real + gradiente de marca) + wordmark
                ZapLogo(size = 116.dp)

                Text(
                    text = stringResource(R.string.onboarding_title),
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

                // Status / criação da conta
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
                            } else if (isInitialized) {
                                Text(
                                    text = stringResource(R.string.onboarding_created),
                                    style = MaterialTheme.typography.bodyMedium,
                                    textAlign = TextAlign.Center
                                )
                            }
                        }
                    }
                }

                Spacer(modifier = Modifier.weight(1f))

                // Ação principal: criar conta (gradiente de marca — ação principal)
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
                Column(
                    verticalArrangement = Arrangement.spacedBy(8.dp),
                    modifier = Modifier.semantics { testTagsAsResourceId = true }
                ) {
                    Text("Use 3 a 20 caracteres: letras minúsculas, números e underscore.")
                    Text(
                        "Opcional: é por ele que outras pessoas te encontram. " +
                            "Sem username, os contatos te adicionam pelo QR code."
                    )
                    OutlinedTextField(
                        value = username,
                        onValueChange = { username = it.lowercase(); usernameError = null },
                        singleLine = true,
                        modifier = Modifier.fillMaxWidth().testTag("onboarding_username_input"),
                        placeholder = { Text("seu_username") }
                    )
                    usernameError?.let { Text(it, color = MaterialTheme.colorScheme.error) }
                }
            },
            confirmButton = {
                Box(modifier = Modifier.semantics { testTagsAsResourceId = true }) {
                    TextButton(
                        enabled = !isInitializing,
                        modifier = Modifier.testTag("onboarding_register"),
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
                                    ZapLivreClientWrapper.markUsernameRegistered(context, value)
                                    com.zaplivre.service.ZapLivreService.start(context)
                                    showUsernameDialog = false
                                    onOnboardingComplete()
                                } catch (error: Exception) {
                                    usernameError = "Não foi possível registrar agora " +
                                        "(${error.message ?: "servidor indisponível"}). " +
                                        "Você pode pular e registrar depois."
                                } finally {
                                    isInitializing = false
                                }
                            }
                        }
                    ) { Text("Registrar") }
                }
            },
            dismissButton = {
                Box(modifier = Modifier.semantics { testTagsAsResourceId = true }) {
                    TextButton(
                        enabled = !isInitializing,
                        modifier = Modifier.testTag("onboarding_skip_username"),
                        onClick = {
                            ZapLivreClientWrapper.skipUsername(context)
                            com.zaplivre.service.ZapLivreService.start(context)
                            showUsernameDialog = false
                            onOnboardingComplete()
                        }
                    ) { Text("Pular por agora") }
                }
            },
        )
    }
}
