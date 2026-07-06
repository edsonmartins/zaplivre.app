package com.zaplivre.ui.preview

import androidx.compose.runtime.Composable
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.remember
import androidx.compose.runtime.setValue
import com.zaplivre.ui.screens.settings.SettingsContent

/**
 * Prévia visual (dados mock, sem client P2P) da tela de configurações. Usa o
 * mesmo [SettingsContent] stateless da tela real, com toggles funcionando via
 * estado local para permitir screenshotar/interagir offline.
 */
@Composable
fun SettingsPreviewContent() {
    var notifications by remember { mutableStateOf(true) }
    var sound by remember { mutableStateOf(true) }
    var vibration by remember { mutableStateOf(false) }
    var readReceipts by remember { mutableStateOf(true) }
    var lastSeen by remember { mutableStateOf(false) }

    SettingsContent(
        peerId = "12D3KooWMe4xPq7Zt9RabcDeFgHiJkLmNoPqRsTuVwXyZ",
        name = "Você",
        storageUsed = "128,4 MB",
        appVersion = "1.0.0",
        notificationsEnabled = notifications,
        soundEnabled = sound,
        vibrationEnabled = vibration,
        readReceiptsEnabled = readReceipts,
        lastSeenEnabled = lastSeen,
        onNotificationsChange = { notifications = it },
        onSoundChange = { sound = it },
        onVibrationChange = { vibration = it },
        onReadReceiptsChange = { readReceipts = it },
        onLastSeenChange = { lastSeen = it },
        onExportBackup = {},
        onExportPrekeys = {},
        onImportPrekeys = {},
        onClearImageCache = {},
        onClearVideoCache = {},
        onLogout = {},
        onNavigateBack = {},
    )
}
