package com.zaplivre.ui.utils

import android.Manifest
import android.content.pm.PackageManager
import android.os.Build
import android.util.Log
import androidx.activity.compose.rememberLauncherForActivityResult
import androidx.activity.result.contract.ActivityResultContracts
import androidx.compose.runtime.*
import androidx.compose.ui.platform.LocalContext
import androidx.core.content.ContextCompat

/**
 * VoipPermissionsState - Estado de permissões VoIP
 */
data class VoipPermissionsState(
    val hasPermissions: Boolean,
    val requestPermissions: () -> Unit
)

/**
 * rememberVoipPermissions - Hook para gerenciar permissões VoIP em Compose
 *
 * @param onPermissionsGranted Callback quando todas as permissões são concedidas
 * @param onPermissionsDenied Callback quando alguma permissão é negada
 */
@Composable
fun rememberVoipPermissions(
    onPermissionsGranted: () -> Unit,
    onPermissionsDenied: (List<String>) -> Unit = {}
): VoipPermissionsState {
    val context = LocalContext.current

    // Permissões necessárias para VoIP
    val requiredPermissions = remember {
        buildList {
            add(Manifest.permission.RECORD_AUDIO)
            add(Manifest.permission.MODIFY_AUDIO_SETTINGS)

            // Android 12+ requer permissão específica para Bluetooth
            if (Build.VERSION.SDK_INT >= Build.VERSION_CODES.S) {
                add(Manifest.permission.BLUETOOTH_CONNECT)
            }
        }.toTypedArray()
    }

    // Estado de permissões
    var hasPermissions by remember {
        mutableStateOf(
            requiredPermissions.all { permission ->
                ContextCompat.checkSelfPermission(context, permission) ==
                        PackageManager.PERMISSION_GRANTED
            }
        )
    }

    // Launcher para solicitar permissões
    val permissionLauncher = rememberLauncherForActivityResult(
        contract = ActivityResultContracts.RequestMultiplePermissions()
    ) { permissions ->
        val deniedPermissions = permissions.filterValues { !it }.keys.toList()

        if (deniedPermissions.isEmpty()) {
            Log.i("VoipPermissions", "All VoIP permissions granted")
            hasPermissions = true
            onPermissionsGranted()
        } else {
            Log.w("VoipPermissions", "VoIP permissions denied: $deniedPermissions")
            hasPermissions = false
            onPermissionsDenied(deniedPermissions)
        }
    }

    // Função para solicitar permissões
    val requestPermissions = remember {
        {
            // Verificar quais permissões faltam
            val missingPermissions = requiredPermissions.filter { permission ->
                ContextCompat.checkSelfPermission(context, permission) !=
                        PackageManager.PERMISSION_GRANTED
            }

            if (missingPermissions.isEmpty()) {
                Log.i("VoipPermissions", "VoIP permissions already granted")
                hasPermissions = true
                onPermissionsGranted()
            } else {
                Log.i("VoipPermissions", "Requesting VoIP permissions: $missingPermissions")
                permissionLauncher.launch(requiredPermissions)
            }
        }
    }

    return VoipPermissionsState(
        hasPermissions = hasPermissions,
        requestPermissions = requestPermissions
    )
}

/**
 * Retorna mensagem explicativa para permissões negadas
 */
fun getPermissionDeniedMessage(deniedPermissions: List<String>): String {
    val messages = deniedPermissions.mapNotNull { permission ->
        when (permission) {
            Manifest.permission.RECORD_AUDIO ->
                "Permissão de microfone é necessária para fazer chamadas de voz."
            Manifest.permission.MODIFY_AUDIO_SETTINGS ->
                "Permissão de áudio é necessária para controlar o volume durante chamadas."
            Manifest.permission.BLUETOOTH_CONNECT ->
                "Permissão de Bluetooth permite usar fones de ouvido Bluetooth durante chamadas."
            else -> null
        }
    }

    return if (messages.isEmpty()) {
        "Permissões negadas. O app pode não funcionar corretamente."
    } else {
        messages.joinToString("\n\n")
    }
}
