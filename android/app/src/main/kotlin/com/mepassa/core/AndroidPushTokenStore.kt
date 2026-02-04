package com.mepassa.core

import android.content.Context
import androidx.security.crypto.EncryptedSharedPreferences
import androidx.security.crypto.MasterKey

object AndroidPushTokenStore {
    private const val PREFS_NAME = "mepassa_secure"
    private const val KEY_PUSH_TOKEN = "push_token"

    private fun prefs(context: Context) = EncryptedSharedPreferences.create(
        context,
        PREFS_NAME,
        MasterKey.Builder(context)
            .setKeyScheme(MasterKey.KeyScheme.AES256_GCM)
            .build(),
        EncryptedSharedPreferences.PrefKeyEncryptionScheme.AES256_SIV,
        EncryptedSharedPreferences.PrefValueEncryptionScheme.AES256_GCM
    )

    fun loadToken(context: Context): String? {
        return prefs(context).getString(KEY_PUSH_TOKEN, null)
    }

    fun saveToken(context: Context, token: String) {
        prefs(context).edit().putString(KEY_PUSH_TOKEN, token).apply()
    }

    fun clearToken(context: Context) {
        prefs(context).edit().remove(KEY_PUSH_TOKEN).apply()
    }
}
