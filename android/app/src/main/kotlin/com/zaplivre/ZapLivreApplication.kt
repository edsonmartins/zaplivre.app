package com.zaplivre

import android.app.Application
import android.system.Os
import android.util.Log

/**
 * ZapLivre Application class
 *
 * Responsável por:
 * - Carregar biblioteca nativa (libzaplivre_core.so)
 * - Inicializar configurações globais
 */
class ZapLivreApplication : Application() {

    companion object {
        private const val TAG = "ZapLivreApplication"

        // Load native library
        init {
            try {
                System.loadLibrary("zaplivre_core")
                Log.i(TAG, "Native library loaded successfully")
            } catch (e: UnsatisfiedLinkError) {
                Log.e(TAG, "Failed to load native library", e)
                throw RuntimeException("Failed to load zaplivre_core native library", e)
            }
        }
    }

    override fun onCreate() {
        super.onCreate()
        Log.i(TAG, "ZapLivre Application created")

        // O core Rust lê MESSAGE_STORE_URL/SIGNALING_SERVER_URL do ambiente na
        // construção do client. Configurar aqui garante que qualquer caminho de
        // inicialização (MainActivity ou ZapLivreService) veja os valores.
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
