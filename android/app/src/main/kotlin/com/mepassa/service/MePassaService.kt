package com.mepassa.service

import android.app.*
import android.content.Context
import android.content.Intent
import android.os.Build
import android.os.IBinder
import android.util.Log
import androidx.core.app.NotificationCompat
import com.google.firebase.messaging.FirebaseMessaging
import com.mepassa.R
import com.mepassa.core.AndroidPushTokenStore
import com.mepassa.core.MePassaClientWrapper
import com.mepassa.push.PushServerClient
import kotlinx.coroutines.*
import kotlinx.coroutines.flow.collect
import kotlinx.coroutines.tasks.await

/**
 * Foreground Service para manter conexão P2P ativa
 *
 * Responsabilidades:
 * - Manter processo vivo em background
 * - Monitorar contagem de peers conectados
 * - Atualizar notificação com status
 * - Gerenciar lifecycle do MePassaClient
 */
class MePassaService : Service() {

    companion object {
        private const val TAG = "MePassaService"
        private const val NOTIFICATION_ID = 1
        private const val CHANNEL_ID = "mepassa_service_channel"

        fun start(context: Context) {
            val intent = Intent(context, MePassaService::class.java)
            if (Build.VERSION.SDK_INT >= Build.VERSION_CODES.O) {
                context.startForegroundService(intent)
            } else {
                context.startService(intent)
            }
        }

        fun stop(context: Context) {
            val intent = Intent(context, MePassaService::class.java)
            context.stopService(intent)
        }
    }

    private val serviceScope = CoroutineScope(Dispatchers.Default + SupervisorJob())
    private var monitoringJob: Job? = null
    private val pushClient by lazy { PushServerClient.create(applicationContext) }

    override fun onCreate() {
        super.onCreate()
        Log.i(TAG, "Service created")

        createNotificationChannel()
        startForeground(NOTIFICATION_ID, createNotification(0u))

        // Inicializar client se ainda não foi
        serviceScope.launch {
            if (!MePassaClientWrapper.isClientReady()) {
                // IDN-01: primeira execução (sem identidade) fica a cargo do
                // Onboarding - o service não pode criar uma identidade nova
                // enquanto o usuário decide entre criar e restaurar backup
                val hasIdentity = !com.mepassa.core.AndroidIdentityStore
                    .loadIdentity(applicationContext)
                    .isNullOrBlank() ||
                    java.io.File(filesDir, "mepassa_data/identity.key").exists()
                if (!hasIdentity) {
                    Log.i(TAG, "No identity yet - stopping service until onboarding completes")
                    stopSelf()
                    return@launch
                }

                Log.i(TAG, "Initializing MePassaClient from service")
                // MESSAGE_STORE_URL/SIGNALING_SERVER_URL são configuradas em
                // MePassaApplication.onCreate, antes de qualquer initialize()
                val success = MePassaClientWrapper.initialize(applicationContext)
                if (!success) {
                    Log.e(TAG, "Failed to initialize client, stopping service")
                    stopSelf()
                    return@launch
                }
            }

            // Registrar FCM token com Push Server (após client inicializado)
            registerPushToken()

            // Iniciar escuta P2P
            Log.i(TAG, "Starting P2P listener")
            MePassaClientWrapper.listenOn("/ip4/0.0.0.0/tcp/0")

            // Bootstrap (conectar a nodes conhecidos)
            Log.i(TAG, "Starting bootstrap")
            MePassaClientWrapper.bootstrap()

            // Iniciar monitoramento de peers
            startPeerMonitoring()
        }
    }

    override fun onStartCommand(intent: Intent?, flags: Int, startId: Int): Int {
        Log.i(TAG, "Service start command received")
        return START_STICKY // Service reinicia se o sistema matar
    }

    override fun onBind(intent: Intent?): IBinder? {
        return null // Unbound service
    }

    override fun onDestroy() {
        super.onDestroy()
        Log.i(TAG, "Service destroyed")

        monitoringJob?.cancel()
        serviceScope.cancel()

        // NÃO fazer shutdown do client aqui, pois pode ser usado pela UI
    }

    /**
     * Cria notification channel (Android O+)
     */
    private fun createNotificationChannel() {
        if (Build.VERSION.SDK_INT >= Build.VERSION_CODES.O) {
            val channel = NotificationChannel(
                CHANNEL_ID,
                getString(R.string.service_channel_name),
                NotificationManager.IMPORTANCE_LOW
            ).apply {
                description = getString(R.string.service_channel_description)
                setShowBadge(false)
            }

            val notificationManager = getSystemService(NotificationManager::class.java)
            notificationManager.createNotificationChannel(channel)
        }
    }

    /**
     * Cria notificação do foreground service
     */
    private fun createNotification(connectedPeers: UInt): Notification {
        // UInt não é aceito por String.format("%d") - converter para Int
        val contentText = getString(R.string.service_notification_text, connectedPeers.toInt())

        // Tocar na notificação abre o app (AND-14)
        val intent = Intent(this, com.mepassa.MainActivity::class.java).apply {
            flags = Intent.FLAG_ACTIVITY_NEW_TASK or Intent.FLAG_ACTIVITY_CLEAR_TOP
        }
        val pendingIntent = PendingIntent.getActivity(
            this,
            0,
            intent,
            PendingIntent.FLAG_UPDATE_CURRENT or PendingIntent.FLAG_IMMUTABLE
        )

        return NotificationCompat.Builder(this, CHANNEL_ID)
            .setContentTitle(getString(R.string.service_notification_title))
            .setContentText(contentText)
            .setSmallIcon(R.mipmap.ic_launcher)
            .setContentIntent(pendingIntent)
            .setOngoing(true)
            .setCategory(NotificationCompat.CATEGORY_SERVICE)
            .setPriority(NotificationCompat.PRIORITY_LOW)
            .build()
    }

    /**
     * Atualiza notificação com nova contagem de peers
     */
    private fun updateNotification(connectedPeers: UInt) {
        val notification = createNotification(connectedPeers)
        val notificationManager = getSystemService(NotificationManager::class.java)
        notificationManager.notify(NOTIFICATION_ID, notification)
    }

    /**
     * Inicia monitoramento periódico de peers conectados
     */
    private fun startPeerMonitoring() {
        monitoringJob = serviceScope.launch {
            while (isActive) {
                try {
                    val count = MePassaClientWrapper.getConnectedPeersCount()
                    Log.d(TAG, "Connected peers: $count")
                    updateNotification(count)
                } catch (e: Exception) {
                    Log.e(TAG, "Error monitoring peers", e)
                }

                delay(10_000) // Atualiza a cada 10 segundos
            }
        }
    }

    /**
     * Registra FCM token com Push Server
     *
     * Obtém o token FCM atual e registra com o Push Server para receber notificações.
     */
    private suspend fun registerPushToken() {
        try {
            // Obter peer ID do cliente
            val peerId = MePassaClientWrapper.localPeerId.value
            if (peerId == null) {
                Log.w(TAG, "⚠️ PeerId not available, skipping push token registration")
                return
            }

            val pendingToken = AndroidPushTokenStore.loadToken(applicationContext)
            Log.d(TAG, "🔐 Getting FCM token...")
            // Obter token FCM atual
            val token = pendingToken ?: FirebaseMessaging.getInstance().token.await()
            Log.d(TAG, "📱 FCM token obtained: ${token.take(20)}...")

            // Registrar com Push Server
            Log.d(TAG, "📤 Registering FCM token with Push Server...")
            val success = pushClient.registerToken(
                peerId = peerId,
                fcmToken = token,
                deviceName = Build.MODEL,
                appVersion = "0.1.0"
            )

            if (success) {
                Log.i(TAG, "✅ FCM token successfully registered with Push Server")
                AndroidPushTokenStore.clearToken(applicationContext)
            } else {
                Log.e(TAG, "❌ Failed to register FCM token with Push Server")
            }
        } catch (e: Exception) {
            Log.e(TAG, "❌ Exception registering push token", e)
        }
    }
}
