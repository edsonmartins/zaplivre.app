package com.zaplivre.push

import android.content.Context
import android.provider.Settings
import android.util.Log
import com.zaplivre.BuildConfig
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.withContext
import okhttp3.MediaType.Companion.toMediaType
import okhttp3.OkHttpClient
import okhttp3.Request
import okhttp3.RequestBody.Companion.toRequestBody
import okhttp3.logging.HttpLoggingInterceptor
import org.json.JSONObject
import java.util.concurrent.TimeUnit

/**
 * Cliente HTTP para comunicação com o ZapLivre Push Server
 *
 * Responsável por registrar/desregistrar tokens FCM e enviar notificações push.
 */
class PushServerClient(
    private val context: Context,
    private val baseUrl: String = BuildConfig.PUSH_SERVER_URL
) {
    private val client: OkHttpClient by lazy {
        val logging = HttpLoggingInterceptor { message ->
            Log.d(TAG, message)
        }.apply {
            level = HttpLoggingInterceptor.Level.BODY
        }

        OkHttpClient.Builder()
            .addInterceptor(logging)
            .connectTimeout(10, TimeUnit.SECONDS)
            .readTimeout(10, TimeUnit.SECONDS)
            .writeTimeout(10, TimeUnit.SECONDS)
            .build()
    }

    /**
     * Registra ou atualiza o token FCM do dispositivo no Push Server
     *
     * @param peerId ID do peer atual
     * @param fcmToken Token FCM do dispositivo
     * @param deviceName Nome do dispositivo (opcional)
     * @param appVersion Versão do app (opcional)
     * @return true se sucesso, false se falhou
     */
    suspend fun registerToken(
        peerId: String,
        fcmToken: String,
        deviceName: String? = null,
        appVersion: String = "0.1.0"
    ): Boolean = withContext(Dispatchers.IO) {
        try {
            val deviceId = Settings.Secure.getString(
                context.contentResolver,
                Settings.Secure.ANDROID_ID
            )

            val json = JSONObject().apply {
                put("peer_id", peerId)
                put("platform", "fcm")
                put("device_id", deviceId)
                put("token", fcmToken)
                put("device_name", deviceName ?: android.os.Build.MODEL)
                put("app_version", appVersion)
            }

            Log.d(TAG, "📤 Registering token - peer_id: $peerId, device_id: $deviceId")

            val requestBody = json.toString()
                .toRequestBody("application/json".toMediaType())

            val request = Request.Builder()
                .url("$baseUrl/api/v1/register")
                .post(requestBody)
                .build()

            client.newCall(request).execute().use { response ->
                if (response.isSuccessful) {
                    Log.i(TAG, "✅ Token registered successfully")
                    true
                } else {
                    Log.e(TAG, "❌ Failed to register token: ${response.code} - ${response.message}")
                    false
                }
            }
        } catch (e: Exception) {
            Log.e(TAG, "❌ Exception registering token", e)
            false
        }
    }

    /**
     * Desregistra (desativa) o token FCM do dispositivo no Push Server
     *
     * @param peerId ID do peer atual
     * @return true se sucesso, false se falhou
     */
    suspend fun unregisterToken(peerId: String): Boolean = withContext(Dispatchers.IO) {
        try {
            val deviceId = Settings.Secure.getString(
                context.contentResolver,
                Settings.Secure.ANDROID_ID
            )

            val json = JSONObject().apply {
                put("peer_id", peerId)
                put("device_id", deviceId)
            }

            Log.d(TAG, "📤 Unregistering token - peer_id: $peerId, device_id: $deviceId")

            val requestBody = json.toString()
                .toRequestBody("application/json".toMediaType())

            val request = Request.Builder()
                .url("$baseUrl/api/v1/unregister")
                .delete(requestBody)
                .build()

            client.newCall(request).execute().use { response ->
                if (response.isSuccessful) {
                    Log.i(TAG, "✅ Token unregistered successfully")
                    true
                } else {
                    Log.e(TAG, "❌ Failed to unregister token: ${response.code} - ${response.message}")
                    false
                }
            }
        } catch (e: Exception) {
            Log.e(TAG, "❌ Exception unregistering token", e)
            false
        }
    }

    /**
     * Testa conectividade com o Push Server
     *
     * @return true se o servidor está acessível, false caso contrário
     */
    suspend fun checkHealth(): Boolean = withContext(Dispatchers.IO) {
        try {
            Log.d(TAG, "🏥 Checking Push Server health...")

            val request = Request.Builder()
                .url("$baseUrl/health")
                .get()
                .build()

            client.newCall(request).execute().use { response ->
                val isHealthy = response.isSuccessful && response.body?.string() == "OK"
                if (isHealthy) {
                    Log.i(TAG, "✅ Push Server is healthy")
                } else {
                    Log.w(TAG, "⚠️ Push Server health check failed")
                }
                isHealthy
            }
        } catch (e: Exception) {
            Log.e(TAG, "❌ Exception checking health", e)
            false
        }
    }

    companion object {
        private const val TAG = "PushServerClient"

        /**
         * Cria uma instância do cliente com configuração customizada
         *
         * @param context Context da aplicação
         * @param pushServerUrl URL do Push Server (default: emulator localhost)
         * @return Nova instância de PushServerClient
         */
        fun create(context: Context, pushServerUrl: String? = null): PushServerClient {
            return PushServerClient(
                context = context.applicationContext,
                baseUrl = pushServerUrl ?: BuildConfig.PUSH_SERVER_URL
            )
        }
    }
}
