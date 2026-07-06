package com.zaplivre.utils

import android.Manifest
import android.content.Context
import android.content.pm.PackageManager
import android.os.Build
import android.util.Log
import androidx.activity.ComponentActivity
import androidx.activity.result.ActivityResultLauncher
import androidx.activity.result.contract.ActivityResultContracts
import androidx.core.content.ContextCompat

/**
 * VoipPermissionManager - Gerencia permissões necessárias para VoIP
 *
 * Permissões necessárias:
 * - RECORD_AUDIO (obrigatória para chamadas)
 * - MODIFY_AUDIO_SETTINGS (para speaker/Bluetooth)
 * - BLUETOOTH_CONNECT (Android 12+ para Bluetooth)
 */
class VoipPermissionManager(private val activity: ComponentActivity) {

    companion object {
        private const val TAG = "VoipPermissionManager"

        /**
         * Permissões obrigatórias para VoIP
         */
        private val REQUIRED_PERMISSIONS = arrayOf(
            Manifest.permission.RECORD_AUDIO,
            Manifest.permission.MODIFY_AUDIO_SETTINGS
        )

        /**
         * Permissões adicionais para Bluetooth (Android 12+)
         */
        private val BLUETOOTH_PERMISSIONS = if (Build.VERSION.SDK_INT >= Build.VERSION_CODES.S) {
            arrayOf(Manifest.permission.BLUETOOTH_CONNECT)
        } else {
            emptyArray()
        }
    }

    private var onPermissionsGranted: (() -> Unit)? = null
    private var onPermissionsDenied: ((List<String>) -> Unit)? = null

    /**
     * Launcher para solicitar múltiplas permissões
     */
    private val permissionLauncher: ActivityResultLauncher<Array<String>> =
        activity.registerForActivityResult(
            ActivityResultContracts.RequestMultiplePermissions()
        ) { permissions ->
            val deniedPermissions = permissions.filterValues { !it }.keys.toList()

            if (deniedPermissions.isEmpty()) {
                Log.i(TAG, "All VoIP permissions granted")
                onPermissionsGranted?.invoke()
            } else {
                Log.w(TAG, "VoIP permissions denied: $deniedPermissions")
                onPermissionsDenied?.invoke(deniedPermissions)
            }
        }

    /**
     * Verifica se todas as permissões obrigatórias foram concedidas
     */
    fun hasRequiredPermissions(context: Context): Boolean {
        return REQUIRED_PERMISSIONS.all { permission ->
            ContextCompat.checkSelfPermission(context, permission) ==
                    PackageManager.PERMISSION_GRANTED
        }
    }

    /**
     * Verifica se permissões de Bluetooth foram concedidas
     */
    fun hasBluetoothPermissions(context: Context): Boolean {
        if (BLUETOOTH_PERMISSIONS.isEmpty()) return true

        return BLUETOOTH_PERMISSIONS.all { permission ->
            ContextCompat.checkSelfPermission(context, permission) ==
                    PackageManager.PERMISSION_GRANTED
        }
    }

    /**
     * Solicita permissões VoIP obrigatórias
     *
     * @param onGranted Callback quando todas as permissões são concedidas
     * @param onDenied Callback quando alguma permissão é negada (recebe lista de permissões negadas)
     */
    fun requestVoipPermissions(
        onGranted: () -> Unit,
        onDenied: (List<String>) -> Unit
    ) {
        this.onPermissionsGranted = onGranted
        this.onPermissionsDenied = onDenied

        // Verificar quais permissões faltam
        val missingPermissions = REQUIRED_PERMISSIONS.filter { permission ->
            ContextCompat.checkSelfPermission(activity, permission) !=
                    PackageManager.PERMISSION_GRANTED
        }

        if (missingPermissions.isEmpty()) {
            Log.i(TAG, "VoIP permissions already granted")
            onGranted()
            return
        }

        // Verificar se precisa mostrar explicação
        val shouldShowRationale = missingPermissions.any { permission ->
            activity.shouldShowRequestPermissionRationale(permission)
        }

        if (shouldShowRationale) {
            Log.i(TAG, "Should show permission rationale for: $missingPermissions")
            // Aqui poderíamos mostrar um dialog explicando
            // Por enquanto, apenas solicita
        }

        Log.i(TAG, "Requesting VoIP permissions: $missingPermissions")
        permissionLauncher.launch(missingPermissions.toTypedArray())
    }

    /**
     * Solicita permissões VoIP + Bluetooth
     */
    fun requestAllPermissions(
        onGranted: () -> Unit,
        onDenied: (List<String>) -> Unit
    ) {
        this.onPermissionsGranted = onGranted
        this.onPermissionsDenied = onDenied

        val allPermissions = REQUIRED_PERMISSIONS + BLUETOOTH_PERMISSIONS

        val missingPermissions = allPermissions.filter { permission ->
            ContextCompat.checkSelfPermission(activity, permission) !=
                    PackageManager.PERMISSION_GRANTED
        }

        if (missingPermissions.isEmpty()) {
            Log.i(TAG, "All VoIP + Bluetooth permissions already granted")
            onGranted()
            return
        }

        Log.i(TAG, "Requesting all VoIP permissions: $missingPermissions")
        permissionLauncher.launch(missingPermissions.toTypedArray())
    }

    /**
     * Retorna mensagem explicativa para o usuário
     */
    fun getPermissionRationale(permissions: List<String>): String {
        val messages = permissions.mapNotNull { permission ->
            when (permission) {
                Manifest.permission.RECORD_AUDIO ->
                    "Permissão de microfone é necessária para fazer chamadas de voz."
                Manifest.permission.MODIFY_AUDIO_SETTINGS ->
                    "Permissão de áudio é necessária para controlar o volume e alto-falante durante chamadas."
                Manifest.permission.BLUETOOTH_CONNECT ->
                    "Permissão de Bluetooth é necessária para usar fones de ouvido Bluetooth durante chamadas."
                else -> null
            }
        }

        return messages.joinToString("\n\n")
    }
}
