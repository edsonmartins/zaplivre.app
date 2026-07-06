package com.zaplivre.voip

import android.content.Context
import android.media.AudioDeviceInfo
import android.media.AudioManager
import android.os.Build
import android.util.Log
import androidx.annotation.RequiresApi

/**
 * CallAudioManager - Gerencia roteamento de áudio durante chamadas VoIP
 *
 * Responsabilidades:
 * - Configurar AudioManager para modo VoIP
 * - Gerenciar dispositivos de áudio (Speaker, Earpiece, Bluetooth)
 * - Request/Abandon audio focus
 * - Detectar e rotear para Bluetooth headsets
 */
class CallAudioManager(private val context: Context) {

    companion object {
        private const val TAG = "CallAudioManager"
    }

    private val audioManager: AudioManager =
        context.getSystemService(Context.AUDIO_SERVICE) as AudioManager

    private var savedAudioMode: Int = AudioManager.MODE_NORMAL
    private var savedSpeakerphoneOn: Boolean = false
    private var savedMicrophoneMute: Boolean = false
    private var audioFocusRequested: Boolean = false

    /**
     * Inicia gerenciamento de áudio para chamada
     */
    fun startCall() {
        Log.i(TAG, "Starting call audio management")

        // Salvar configurações atuais
        savedAudioMode = audioManager.mode
        savedSpeakerphoneOn = audioManager.isSpeakerphoneOn
        savedMicrophoneMute = audioManager.isMicrophoneMute

        // Configurar modo de comunicação (otimizado para voz)
        audioManager.mode = AudioManager.MODE_IN_COMMUNICATION

        // Request audio focus
        requestAudioFocus()

        // Verificar se há Bluetooth conectado
        if (hasBluetoothDevice()) {
            Log.i(TAG, "Bluetooth device detected, routing to Bluetooth")
            audioManager.isBluetoothScoOn = true
            audioManager.startBluetoothSco()
        } else {
            // Por padrão, usar earpiece (não speaker)
            audioManager.isSpeakerphoneOn = false
        }

        // Unmute por padrão
        audioManager.isMicrophoneMute = false

        Log.i(TAG, "Call audio started - Mode: ${audioManager.mode}, Speaker: ${audioManager.isSpeakerphoneOn}, BT: ${audioManager.isBluetoothScoOn}")
    }

    /**
     * Finaliza gerenciamento de áudio
     */
    fun stopCall() {
        Log.i(TAG, "Stopping call audio management")

        // Parar Bluetooth SCO se estava ativo
        if (audioManager.isBluetoothScoOn) {
            audioManager.stopBluetoothSco()
            audioManager.isBluetoothScoOn = false
        }

        // Restaurar configurações originais
        audioManager.mode = savedAudioMode
        audioManager.isSpeakerphoneOn = savedSpeakerphoneOn
        audioManager.isMicrophoneMute = savedMicrophoneMute

        // Abandon audio focus
        abandonAudioFocus()

        Log.i(TAG, "Call audio stopped")
    }

    /**
     * Toggle speakerphone (alto-falante)
     *
     * @return true se speakerphone está ativado após toggle
     */
    fun toggleSpeakerphone(): Boolean {
        val newState = !audioManager.isSpeakerphoneOn

        // Se ativar speakerphone, desativar Bluetooth
        if (newState && audioManager.isBluetoothScoOn) {
            audioManager.stopBluetoothSco()
            audioManager.isBluetoothScoOn = false
        }

        audioManager.isSpeakerphoneOn = newState
        Log.i(TAG, "Speakerphone toggled: $newState")

        return newState
    }

    /**
     * Force speakerphone state
     */
    fun setSpeakerphone(enabled: Boolean) {
        if (enabled && audioManager.isBluetoothScoOn) {
            audioManager.stopBluetoothSco()
            audioManager.isBluetoothScoOn = false
        }

        audioManager.isSpeakerphoneOn = enabled
        Log.i(TAG, "Speakerphone set to: $enabled")
    }

    /**
     * Toggle mute do microfone
     *
     * @return true se microfone está mutado após toggle
     */
    fun toggleMute(): Boolean {
        val newState = !audioManager.isMicrophoneMute
        audioManager.isMicrophoneMute = newState
        Log.i(TAG, "Microphone mute toggled: $newState")

        return newState
    }

    /**
     * Force mute state
     */
    fun setMuted(muted: Boolean) {
        audioManager.isMicrophoneMute = muted
        Log.i(TAG, "Microphone mute set to: $muted")
    }

    /**
     * Verifica se speakerphone está ativado
     */
    fun isSpeakerphoneOn(): Boolean = audioManager.isSpeakerphoneOn

    /**
     * Verifica se microfone está mutado
     */
    fun isMicrophoneMute(): Boolean = audioManager.isMicrophoneMute

    /**
     * Verifica se há dispositivo Bluetooth conectado
     */
    fun hasBluetoothDevice(): Boolean {
        return if (Build.VERSION.SDK_INT >= Build.VERSION_CODES.M) {
            hasBluetoothDeviceApi23()
        } else {
            // Fallback para APIs antigas
            audioManager.isBluetoothA2dpOn || audioManager.isBluetoothScoOn
        }
    }

    @RequiresApi(Build.VERSION_CODES.M)
    private fun hasBluetoothDeviceApi23(): Boolean {
        val devices = audioManager.getDevices(AudioManager.GET_DEVICES_OUTPUTS)
        return devices.any { device ->
            device.type == AudioDeviceInfo.TYPE_BLUETOOTH_SCO ||
            device.type == AudioDeviceInfo.TYPE_BLUETOOTH_A2DP
        }
    }

    /**
     * Rotear áudio para Bluetooth (se disponível)
     */
    fun routeToBluetoothIfAvailable(): Boolean {
        if (hasBluetoothDevice()) {
            audioManager.isSpeakerphoneOn = false
            audioManager.isBluetoothScoOn = true
            audioManager.startBluetoothSco()
            Log.i(TAG, "Audio routed to Bluetooth")
            return true
        } else {
            Log.w(TAG, "No Bluetooth device available")
            return false
        }
    }

    /**
     * Request audio focus (necessário para VoIP)
     */
    private fun requestAudioFocus() {
        if (audioFocusRequested) return

        val result = if (Build.VERSION.SDK_INT >= Build.VERSION_CODES.O) {
            requestAudioFocusApi26()
        } else {
            @Suppress("DEPRECATION")
            audioManager.requestAudioFocus(
                null,
                AudioManager.STREAM_VOICE_CALL,
                AudioManager.AUDIOFOCUS_GAIN_TRANSIENT
            )
        }

        audioFocusRequested = (result == AudioManager.AUDIOFOCUS_REQUEST_GRANTED)
        Log.i(TAG, "Audio focus requested: $audioFocusRequested")
    }

    @RequiresApi(Build.VERSION_CODES.O)
    private fun requestAudioFocusApi26(): Int {
        val focusRequest = android.media.AudioFocusRequest.Builder(
            AudioManager.AUDIOFOCUS_GAIN_TRANSIENT
        )
            .setAudioAttributes(
                android.media.AudioAttributes.Builder()
                    .setUsage(android.media.AudioAttributes.USAGE_VOICE_COMMUNICATION)
                    .setContentType(android.media.AudioAttributes.CONTENT_TYPE_SPEECH)
                    .build()
            )
            .build()

        return audioManager.requestAudioFocus(focusRequest)
    }

    /**
     * Abandon audio focus
     */
    private fun abandonAudioFocus() {
        if (!audioFocusRequested) return

        if (Build.VERSION.SDK_INT >= Build.VERSION_CODES.O) {
            // API 26+ não precisa fazer nada (focus request é local)
        } else {
            @Suppress("DEPRECATION")
            audioManager.abandonAudioFocus(null)
        }

        audioFocusRequested = false
        Log.i(TAG, "Audio focus abandoned")
    }

    /**
     * Retorna lista de dispositivos de áudio disponíveis
     */
    fun getAvailableDevices(): List<AudioDevice> {
        val devices = mutableListOf<AudioDevice>()

        // Earpiece sempre disponível
        devices.add(AudioDevice.EARPIECE)

        // Speaker sempre disponível
        devices.add(AudioDevice.SPEAKER)

        // Bluetooth se disponível
        if (hasBluetoothDevice()) {
            devices.add(AudioDevice.BLUETOOTH)
        }

        return devices
    }

    enum class AudioDevice {
        EARPIECE,
        SPEAKER,
        BLUETOOTH
    }
}
