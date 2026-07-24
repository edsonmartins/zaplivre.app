package com.zaplivre.service

import android.util.Log
import com.google.firebase.messaging.FirebaseMessagingService
import com.google.firebase.messaging.RemoteMessage
import com.zaplivre.core.AndroidPushTokenStore
import com.zaplivre.core.ZapLivreClientWrapper
import com.zaplivre.push.PushServerClient
import com.zaplivre.util.NotificationHelper
import kotlinx.coroutines.CoroutineScope
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.SupervisorJob
import kotlinx.coroutines.launch

/**
 * Firebase Cloud Messaging Service
 *
 * Handles incoming FCM messages and token refresh events.
 */
class ZapLivreFirebaseMessagingService : FirebaseMessagingService() {

    private val serviceScope = CoroutineScope(SupervisorJob() + Dispatchers.IO)
    private val pushClient by lazy { PushServerClient.create(applicationContext) }

    override fun onNewToken(token: String) {
        super.onNewToken(token)
        Log.d(TAG, "New FCM token received")

        AndroidPushTokenStore.saveToken(applicationContext, token)

        // Send token to Push Server asynchronously
        sendTokenToServer(token)
    }

    override fun onMessageReceived(message: RemoteMessage) {
        super.onMessageReceived(message)
        Log.d(TAG, "FCM message received from: ${message.from}")

        // Extract notification data
        val title = message.data["title"] ?: message.notification?.title ?: "Nova mensagem"
        val body = message.data["body"] ?: message.notification?.body ?: "Você recebeu uma mensagem"
        // sender_peer_id abre a conversa certa (peer_id é o destinatário - nós)
        val peerId = message.data["sender_peer_id"] ?: message.data["peer_id"]

        Log.d(TAG, "Push notification received")

        // Show notification
        NotificationHelper.showMessageNotification(
            context = this,
            title = title,
            body = body,
            peerId = peerId
        )

        // Wake up ZapLivreService to poll new messages
        try {
            ZapLivreService.start(this)
            Log.d(TAG, "ZapLivreService started to sync messages")
        } catch (e: Exception) {
            Log.e(TAG, "Failed to start ZapLivreService", e)
        }
    }

    private fun sendTokenToServer(token: String) {
        serviceScope.launch {
            try {
                // Wait for client to be initialized
                val peerId = ZapLivreClientWrapper.localPeerId.value
                if (peerId == null) {
                    Log.w(TAG, "⚠️ PeerId not available yet, token will be sent when client initializes")
                    AndroidPushTokenStore.saveToken(applicationContext, token)
                    // Token will be sent when ZapLivreService starts and client is initialized
                    return@launch
                }

                Log.d(TAG, "📤 Sending FCM token to Push Server...")
                val success = pushClient.registerToken(
                    peerId = peerId,
                    fcmToken = token,
                    deviceName = android.os.Build.MODEL,
                    appVersion = "0.1.0"
                )

                if (success) {
                    Log.i(TAG, "✅ FCM token successfully registered with Push Server")
                    AndroidPushTokenStore.clearToken(applicationContext)
                } else {
                    Log.e(TAG, "❌ Failed to register FCM token with Push Server")
                }
            } catch (e: Exception) {
                Log.e(TAG, "❌ Exception sending token to server", e)
            }
        }
    }

    companion object {
        private const val TAG = "FCM"
    }
}
