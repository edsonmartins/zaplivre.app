package com.mepassa

import android.app.Application
import android.system.Os
import android.util.Log

/**
 * MePassa Application class
 *
 * Responsável por:
 * - Carregar biblioteca nativa (libmepassa_core.so)
 * - Inicializar configurações globais
 */
class MePassaApplication : Application() {

    companion object {
        private const val TAG = "MePassaApplication"

        // Load native library
        init {
            try {
                System.loadLibrary("mepassa_core")
                Log.i(TAG, "Native library loaded successfully")
            } catch (e: UnsatisfiedLinkError) {
                Log.e(TAG, "Failed to load native library", e)
                throw RuntimeException("Failed to load mepassa_core native library", e)
            }
        }
    }

    override fun onCreate() {
        super.onCreate()
        Log.i(TAG, "MePassa Application created")

        // O core Rust lê MESSAGE_STORE_URL/SIGNALING_SERVER_URL do ambiente na
        // construção do client. Configurar aqui garante que qualquer caminho de
        // inicialização (MainActivity ou MePassaService) veja os valores.
        configureCoreEnvironment()
    }

    private fun configureCoreEnvironment() {
        val storeUrl = BuildConfig.MESSAGE_STORE_URL
        if (storeUrl.isNotBlank()) {
            try {
                Os.setenv("MESSAGE_STORE_URL", storeUrl, true)
            } catch (e: Exception) {
                Log.w(TAG, "Failed to set MESSAGE_STORE_URL env", e)
            }
        }
        val signalingUrl = BuildConfig.SIGNALING_SERVER_URL
        if (signalingUrl.isNotBlank()) {
            try {
                Os.setenv("SIGNALING_SERVER_URL", signalingUrl, true)
            } catch (e: Exception) {
                Log.w(TAG, "Failed to set SIGNALING_SERVER_URL env", e)
            }
        }
    }
}
