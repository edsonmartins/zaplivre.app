package com.mepassa.ui.screens.onboarding

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
import com.mepassa.R
import com.mepassa.core.MePassaClientWrapper
import kotlinx.coroutines.launch

/**
 * OnboardingScreen - Primeira tela do app
 *
 * Exibida apenas na primeira execução.
 * Responsável por:
 * - Inicializar MePassaClient (gerar keypair)
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

    // Observar estado de inicialização
    val isInitialized by MePassaClientWrapper.isInitialized.collectAsState()
    val clientPeerId by MePassaClientWrapper.localPeerId.collectAsState()

    // Auto-complete quando inicializado
    LaunchedEffect(isInitialized) {
        if (isInitialized) {
            localPeerId = clientPeerId
            // Iniciar o foreground service (na primeira execução ele parou
            // aguardando o onboarding decidir criar/restaurar identidade)
            com.mepassa.service.MePassaService.start(context)
            // Pequeno delay para usuário ver o peer ID
            kotlinx.coroutines.delay(1500)
            onOnboardingComplete()
        }
    }

    Scaffold { paddingValues ->
        Box(
            modifier = Modifier
                .fillMaxSize()
                .padding(paddingValues)
                .padding(24.dp),
            contentAlignment = Alignment.Center
        ) {
            Column(
                horizontalAlignment = Alignment.CenterHorizontally,
                verticalArrangement = Arrangement.spacedBy(24.dp)
            ) {
                // Logo placeholder
                Surface(
                    modifier = Modifier.size(120.dp),
                    shape = MaterialTheme.shapes.extraLarge,
                    color = MaterialTheme.colorScheme.primaryContainer
                ) {
                    Box(contentAlignment = Alignment.Center) {
                        Text(
                            text = "ZL",
                            style = MaterialTheme.typography.displayLarge,
                            color = MaterialTheme.colorScheme.onPrimaryContainer
                        )
                    }
                }

                Spacer(modifier = Modifier.height(16.dp))

                // Title
                Text(
                    text = stringResource(R.string.onboarding_title),
                    style = MaterialTheme.typography.headlineMedium,
                    textAlign = TextAlign.Center
                )

                // Subtitle
                Text(
                    text = stringResource(R.string.onboarding_subtitle),
                    style = MaterialTheme.typography.bodyLarge,
                    textAlign = TextAlign.Center,
                    color = MaterialTheme.colorScheme.onSurfaceVariant
                )

                Spacer(modifier = Modifier.height(16.dp))

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

                // Botão começar
                Button(
                    onClick = {
                        isInitializing = true
                        scope.launch {
                            val success = MePassaClientWrapper.initialize(context)
                            if (!success) {
                                // TODO: Mostrar erro
                                isInitializing = false
                            }
                        }
                    },
                    modifier = Modifier
                        .fillMaxWidth()
                        .height(56.dp)
                        .testTag("onboarding_create"),
                    enabled = !isInitializing && !isInitialized
                ) {
                    Text(
                        text = stringResource(R.string.onboarding_button),
                        style = MaterialTheme.typography.labelLarge
                    )
                }

                OutlinedButton(
                    onClick = { showImportDialog = true },
                    modifier = Modifier
                        .fillMaxWidth()
                        .height(56.dp)
                        .testTag("onboarding_restore"),
                    enabled = !isInitializing && !isInitialized
                ) {
                    Text(
                        text = stringResource(R.string.onboarding_import_button),
                        style = MaterialTheme.typography.labelLarge
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
                            val ok = MePassaClientWrapper.importIdentity(context, importText)
                            if (!ok) {
                                importError = context.getString(R.string.onboarding_import_failed)
                                isInitializing = false
                                return@launch
                            }
                            val success = MePassaClientWrapper.initialize(context)
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
}
