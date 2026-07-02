package com.mepassa

import android.Manifest
import android.content.Intent
import android.content.pm.PackageManager
import android.os.Build
import android.os.Bundle
import android.util.Log
import androidx.activity.ComponentActivity
import androidx.activity.compose.setContent
import androidx.activity.result.contract.ActivityResultContracts
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.Surface
import androidx.compose.runtime.*
import androidx.compose.ui.Modifier
import androidx.core.content.ContextCompat
import androidx.lifecycle.lifecycleScope
import com.mepassa.core.MePassaClientWrapper
import com.mepassa.service.MePassaService
import com.mepassa.ui.navigation.MePassaNavHost
import com.mepassa.ui.theme.MePassaTheme
import kotlinx.coroutines.launch

/**
 * MainActivity - Ponto de entrada do app
 *
 * Responsabilidades:
 * - Inicializar MePassaClient
 * - Solicitar permissões necessárias
 * - Iniciar MePassaService
 * - Configurar navegação Compose
 */
class MainActivity : ComponentActivity() {

    companion object {
        private const val TAG = "MainActivity"
    }

    private val pendingPeerIdState = mutableStateOf<String?>(null)

    // Launcher para solicitar permissão de notificação (Android 13+)
    private val notificationPermissionLauncher = registerForActivityResult(
        ActivityResultContracts.RequestPermission()
    ) { isGranted ->
        if (isGranted) {
            Log.i(TAG, "Notification permission granted")
            startService()
        } else {
            Log.w(TAG, "Notification permission denied")
            // Ainda assim iniciar service (notificação não aparecerá)
            startService()
        }
    }

    override fun onCreate(savedInstanceState: Bundle?) {
        super.onCreate(savedInstanceState)
        Log.i(TAG, "MainActivity created")

        // IDN-01: só inicializar automaticamente quando JÁ existe identidade.
        // Na primeira execução o Onboarding decide entre criar nova identidade
        // e restaurar um backup (o auto-init tornava o import impossível).
        lifecycleScope.launch {
            val hasIdentity =
                !com.mepassa.core.AndroidIdentityStore.loadIdentity(applicationContext)
                    .isNullOrBlank() ||
                    java.io.File(filesDir, "mepassa_data/identity.key").exists()
            if (hasIdentity) {
                val success = MePassaClientWrapper.initialize(applicationContext)
                if (!success) {
                    Log.e(TAG, "Failed to initialize MePassaClient")
                } else {
                    Log.i(TAG, "MePassaClient initialized successfully")
                }
            } else {
                Log.i(TAG, "No identity yet - onboarding will handle initialization")
            }
        }

        // Solicitar permissões e iniciar service
        requestPermissionsAndStartService()

        handleIntent(intent)

        // Setup UI
        setContent {
            MePassaTheme {
                Surface(
                    modifier = Modifier.fillMaxSize(),
                    color = MaterialTheme.colorScheme.background
                ) {
                    MePassaApp(
                        pendingPeerId = pendingPeerIdState.value,
                        onPeerIdConsumed = { pendingPeerIdState.value = null }
                    )
                }
            }
        }
    }

    /**
     * Solicita permissões necessárias e inicia service
     */
    private fun requestPermissionsAndStartService() {
        // Android 13+ requer permissão explícita para notificações
        if (Build.VERSION.SDK_INT >= Build.VERSION_CODES.TIRAMISU) {
            when {
                ContextCompat.checkSelfPermission(
                    this,
                    Manifest.permission.POST_NOTIFICATIONS
                ) == PackageManager.PERMISSION_GRANTED -> {
                    Log.i(TAG, "Notification permission already granted")
                    startService()
                }
                shouldShowRequestPermissionRationale(Manifest.permission.POST_NOTIFICATIONS) -> {
                    // TODO: Mostrar explicação ao usuário
                    Log.i(TAG, "Should show notification permission rationale")
                    notificationPermissionLauncher.launch(Manifest.permission.POST_NOTIFICATIONS)
                }
                else -> {
                    Log.i(TAG, "Requesting notification permission")
                    notificationPermissionLauncher.launch(Manifest.permission.POST_NOTIFICATIONS)
                }
            }
        } else {
            // Versões anteriores não precisam permissão explícita
            startService()
        }
    }

    /**
     * Inicia MePassaService
     */
    private fun startService() {
        Log.i(TAG, "Starting MePassaService")
        MePassaService.start(this)
    }

    override fun onNewIntent(intent: Intent?) {
        super.onNewIntent(intent)
        handleIntent(intent)
    }

    private fun handleIntent(intent: Intent?) {
        val peerId = intent?.getStringExtra("peer_id")
        if (!peerId.isNullOrBlank()) {
            Log.i(TAG, "Pending push navigation to peer: $peerId")
            pendingPeerIdState.value = peerId
        }
    }

    override fun onDestroy() {
        super.onDestroy()
        Log.i(TAG, "MainActivity destroyed")
        // NÃO parar o service aqui, pois queremos que continue em background
    }
}

/**
 * Composable principal do app
 */
@Composable
fun MePassaApp(
    pendingPeerId: String?,
    onPeerIdConsumed: () -> Unit
) {
    val isInitialized by MePassaClientWrapper.isInitialized.collectAsState()

    MePassaNavHost(
        isClientInitialized = isInitialized,
        pendingPeerId = pendingPeerId,
        onPeerIdConsumed = onPeerIdConsumed
    )
}
