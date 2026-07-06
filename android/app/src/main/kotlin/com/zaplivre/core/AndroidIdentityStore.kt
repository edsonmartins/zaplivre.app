package com.zaplivre.core

import android.content.Context
import androidx.security.crypto.EncryptedSharedPreferences
import androidx.security.crypto.MasterKey

object AndroidIdentityStore {
    private const val PREFS_NAME = "zaplivre_secure"
    private const val KEY_IDENTITY = "identity_b64"

    private fun prefs(context: Context) = EncryptedSharedPreferences.create(
        context,
        PREFS_NAME,
        MasterKey.Builder(context)
            .setKeyScheme(MasterKey.KeyScheme.AES256_GCM)
            .build(),
        EncryptedSharedPreferences.PrefKeyEncryptionScheme.AES256_SIV,
        EncryptedSharedPreferences.PrefValueEncryptionScheme.AES256_GCM
    )

    fun loadIdentity(context: Context): String? {
        return prefs(context).getString(KEY_IDENTITY, null)
    }

    fun saveIdentity(context: Context, base64: String) {
        prefs(context).edit().putString(KEY_IDENTITY, base64).apply()
    }

    fun deleteIdentity(context: Context) {
        prefs(context).edit().remove(KEY_IDENTITY).apply()
    }
}
