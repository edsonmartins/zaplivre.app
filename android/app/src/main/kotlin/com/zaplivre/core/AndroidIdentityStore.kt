package com.zaplivre.core

import android.content.Context
import androidx.security.crypto.EncryptedSharedPreferences
import androidx.security.crypto.MasterKey

object AndroidIdentityStore {
    private const val PREFS_NAME = "zaplivre_secure"
    private const val KEY_IDENTITY = "identity_b64"
    private const val KEY_USERNAME = "registered_username"
    private const val KEY_ONBOARDED = "onboarding_done"

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
        prefs(context).edit().remove(KEY_IDENTITY).remove(KEY_USERNAME).remove(KEY_ONBOARDED).apply()
    }

    fun loadUsername(context: Context): String? = prefs(context).getString(KEY_USERNAME, null)
    fun saveUsername(context: Context, username: String) { prefs(context).edit().putString(KEY_USERNAME, username).apply() }

    /** Onboarding concluído: com username registrado ou pulado de propósito. */
    fun isOnboarded(context: Context): Boolean = prefs(context).getBoolean(KEY_ONBOARDED, false)
    fun markOnboarded(context: Context) { prefs(context).edit().putBoolean(KEY_ONBOARDED, true).apply() }
}
