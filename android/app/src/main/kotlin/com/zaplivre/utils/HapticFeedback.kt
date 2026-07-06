package com.zaplivre.utils

import android.view.HapticFeedbackConstants
import android.view.View
import androidx.compose.runtime.Composable
import androidx.compose.runtime.remember
import androidx.compose.ui.platform.LocalView

/**
 * HapticFeedback - Utility for providing haptic feedback
 */
object HapticFeedback {
    /**
     * Perform light haptic feedback
     */
    fun performLight(view: View) {
        view.performHapticFeedback(HapticFeedbackConstants.CLOCK_TICK)
    }

    /**
     * Perform medium haptic feedback
     */
    fun performMedium(view: View) {
        view.performHapticFeedback(HapticFeedbackConstants.CONTEXT_CLICK)
    }

    /**
     * Perform heavy haptic feedback
     */
    fun performHeavy(view: View) {
        view.performHapticFeedback(HapticFeedbackConstants.LONG_PRESS)
    }

    /**
     * Perform rejection haptic feedback (for errors)
     */
    fun performReject(view: View) {
        view.performHapticFeedback(HapticFeedbackConstants.REJECT)
    }
}

/**
 * Composable to get haptic feedback helper
 */
@Composable
fun rememberHapticFeedback(): HapticFeedbackHelper {
    val view = LocalView.current
    return remember(view) {
        HapticFeedbackHelper(view)
    }
}

/**
 * Helper class for haptic feedback in Compose
 */
class HapticFeedbackHelper(private val view: View) {
    fun light() = HapticFeedback.performLight(view)
    fun medium() = HapticFeedback.performMedium(view)
    fun heavy() = HapticFeedback.performHeavy(view)
    fun reject() = HapticFeedback.performReject(view)
}
