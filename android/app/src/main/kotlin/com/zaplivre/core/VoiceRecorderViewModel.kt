package com.zaplivre.core

import android.content.Context
import androidx.lifecycle.ViewModel
import androidx.lifecycle.viewModelScope
import kotlinx.coroutines.Job
import kotlinx.coroutines.delay
import kotlinx.coroutines.flow.MutableStateFlow
import kotlinx.coroutines.flow.StateFlow
import kotlinx.coroutines.flow.asStateFlow
import kotlinx.coroutines.launch
import java.io.File

/**
 * ViewModel for managing voice message recording
 */
class VoiceRecorderViewModel(context: Context) : ViewModel() {

    private val audioRecorder = AudioRecorder(context)

    private val _isRecording = MutableStateFlow(false)
    val isRecording: StateFlow<Boolean> = _isRecording.asStateFlow()

    private val _recordingDuration = MutableStateFlow(0L)
    val recordingDuration: StateFlow<Long> = _recordingDuration.asStateFlow()

    private val _recordingFile = MutableStateFlow<File?>(null)
    val recordingFile: StateFlow<File?> = _recordingFile.asStateFlow()

    private val _error = MutableStateFlow<String?>(null)
    val error: StateFlow<String?> = _error.asStateFlow()

    private var timerJob: Job? = null

    companion object {
        const val MAX_DURATION_MS = 60_000L // 60 seconds max
    }

    /**
     * Start recording audio
     */
    fun startRecording() {
        val result = audioRecorder.startRecording()

        result.onSuccess { file ->
            _isRecording.value = true
            _recordingFile.value = file
            _recordingDuration.value = 0
            _error.value = null

            startTimer()
        }.onFailure { error ->
            _error.value = error.message ?: "Failed to start recording"
        }
    }

    /**
     * Stop recording and return the audio file
     */
    fun stopRecording(): File? {
        stopTimer()

        val result = audioRecorder.stopRecording()
        _isRecording.value = false

        return result.getOrNull()
    }

    /**
     * Cancel recording
     */
    fun cancelRecording() {
        stopTimer()
        audioRecorder.cancelRecording()

        _isRecording.value = false
        _recordingFile.value = null
        _recordingDuration.value = 0
    }

    /**
     * Start the recording duration timer
     */
    private fun startTimer() {
        timerJob = viewModelScope.launch {
            while (_isRecording.value && _recordingDuration.value < MAX_DURATION_MS) {
                delay(100) // Update every 100ms
                _recordingDuration.value += 100

                // Auto-stop when max duration is reached
                if (_recordingDuration.value >= MAX_DURATION_MS) {
                    stopRecording()
                }
            }
        }
    }

    /**
     * Stop the timer
     */
    private fun stopTimer() {
        timerJob?.cancel()
        timerJob = null
    }

    /**
     * Format duration for display (MM:SS)
     */
    fun formatDuration(durationMs: Long): String {
        val totalSeconds = durationMs / 1000
        val minutes = totalSeconds / 60
        val seconds = totalSeconds % 60
        return String.format("%02d:%02d", minutes, seconds)
    }

    /**
     * Clear error message
     */
    fun clearError() {
        _error.value = null
    }

    override fun onCleared() {
        super.onCleared()
        audioRecorder.release()
        stopTimer()
    }
}
