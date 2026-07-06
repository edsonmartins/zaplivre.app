package com.zaplivre.voip

import android.content.Context
import android.util.Log
import uniffi.zaplivre.FfiVoipEventCallback

/**
 * VoipEventHandler - Receives VoIP control events from core and applies them on Android.
 */
class VoipEventHandler(context: Context) : FfiVoipEventCallback {
    private val audioManager = CallAudioManager(context.applicationContext)

    override fun onMuteChanged(callId: String, isMuted: Boolean) {
        audioManager.setMuted(isMuted)
        Log.i("VoipEventHandler", "Mute changed for $callId: $isMuted")
    }

    override fun onSpeakerphoneChanged(callId: String, enabled: Boolean) {
        audioManager.setSpeakerphone(enabled)
        Log.i("VoipEventHandler", "Speakerphone changed for $callId: $enabled")
    }

    override fun onCameraSwitchRequested(callId: String) {
        Log.i("VoipEventHandler", "Camera switch requested for $callId")
        // UI layer should handle actual camera switching.
    }
}
