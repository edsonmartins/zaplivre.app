package com.zaplivre.ui.screens.settings

import androidx.compose.ui.test.assertIsOff
import androidx.compose.ui.test.assertIsOn
import androidx.compose.ui.test.junit4.createComposeRule
import androidx.compose.ui.test.onNodeWithTag
import androidx.compose.ui.test.performClick
import androidx.test.ext.junit.runners.AndroidJUnit4
import org.junit.Assert.assertFalse
import org.junit.Assert.assertTrue
import org.junit.Rule
import org.junit.Test
import org.junit.runner.RunWith/**
 * Teste instrumentado de UI (Compose) do conteúdo de Configurações (F1).
 * Valida que os toggles (com testTag, usados pelos flows Maestro) renderizam
 * com o estado recebido e que o clique dispara o callback correto.
 */
@RunWith(AndroidJUnit4::class)
class SettingsScreenTest {

    @get:Rule
    val composeRule = createComposeRule()

    private fun renderSettings(
        notificationsEnabled: Boolean = true,
        onNotificationsChange: (Boolean) -> Unit = {},
        onSoundChange: (Boolean) -> Unit = {},
        onVibrationChange: (Boolean) -> Unit = {},
    ) {
        composeRule.setContent {
            SettingsContent(
                peerId = "12D3KooWTest",
                name = "Teste",
                storageUsed = "0 B",
                appVersion = "0.1.0",
                notificationsEnabled = notificationsEnabled,
                soundEnabled = true,
                vibrationEnabled = true,
                readReceiptsEnabled = true,
                lastSeenEnabled = true,
                onNotificationsChange = onNotificationsChange,
                onSoundChange = onSoundChange,
                onVibrationChange = onVibrationChange,
                onReadReceiptsChange = {},
                onLastSeenChange = {},
                onExportBackup = {},
                onExportPrekeys = {},
                onImportPrekeys = {},
                onShowQrCode = {},
                onClearImageCache = {},
                onClearVideoCache = {},
                onLogout = {},
                onNavigateBack = {},
            )
        }
    }

    @Test
    fun notificationsToggleRendersCheckedWhenEnabled() {
        renderSettings(notificationsEnabled = true)
        composeRule.onNodeWithTag("settings_toggle_notifications").assertIsOn()
    }

    @Test
    fun notificationsToggleRendersUncheckedWhenDisabled() {
        renderSettings(notificationsEnabled = false)
        composeRule.onNodeWithTag("settings_toggle_notifications").assertIsOff()
    }

    @Test
    fun clickingNotificationsToggleInvokesCallback() {
        var captured: Boolean? = null
        renderSettings(
            notificationsEnabled = true,
            onNotificationsChange = { captured = it },
        )
        composeRule.onNodeWithTag("settings_toggle_notifications").performClick()
        assertTrue(captured == false)
    }

    @Test
    fun soundAndVibrationTogglesPresentAndClickable() {
        var soundCaptured: Boolean? = null
        var vibrationCaptured: Boolean? = null
        renderSettings(
            onSoundChange = { soundCaptured = it },
            onVibrationChange = { vibrationCaptured = it },
        )
        composeRule.onNodeWithTag("settings_toggle_sound").assertIsOn()
        composeRule.onNodeWithTag("settings_toggle_vibration").assertIsOn()

        composeRule.onNodeWithTag("settings_toggle_sound").performClick()
        composeRule.onNodeWithTag("settings_toggle_vibration").performClick()

        assertFalse(soundCaptured ?: true)
        assertFalse(vibrationCaptured ?: true)
    }
}
