package com.zaplivre.util

/**
 * Whether the UI is on screen.
 *
 * The service decides with this if an incoming message needs a system
 * notification: with the chat list visible it would only be noise.
 */
object AppVisibility {
    @Volatile
    var isForeground: Boolean = false
}
