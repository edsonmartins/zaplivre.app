package com.zaplivre.ui.components

import android.util.Log
import android.view.SurfaceHolder
import android.view.SurfaceView
import androidx.compose.foundation.background
import androidx.compose.runtime.Composable
import androidx.compose.runtime.DisposableEffect
import androidx.compose.runtime.remember
import androidx.compose.runtime.rememberCoroutineScope
import androidx.compose.ui.Modifier
import androidx.compose.ui.graphics.Color
import androidx.compose.ui.platform.LocalContext
import androidx.compose.ui.viewinterop.AndroidView
import com.zaplivre.core.ZapLivreClientWrapper
import kotlinx.coroutines.launch

/**
 * RemoteVideoView - Renders remote video using SurfaceView + MediaCodec
 *
 * This component creates a SurfaceView and registers a VideoFrameHandler
 * to receive and decode remote video frames from WebRTC.
 */
@Composable
fun RemoteVideoView(
    callId: String,
    modifier: Modifier = Modifier
) {
    val context = LocalContext.current
    val scope = rememberCoroutineScope()

    // Video frame handler (implements FfiVideoFrameCallback)
    val videoHandler = remember(callId) {
        com.zaplivre.voip.VideoFrameHandler(callId)
    }

    DisposableEffect(callId) {
        // Register callback when view appears
        scope.launch {
            try {
                ZapLivreClientWrapper.registerVideoFrameCallback(videoHandler)
                Log.d("RemoteVideoView", "✅ Video frame callback registered for call: $callId")
            } catch (e: Exception) {
                Log.e("RemoteVideoView", "❌ Failed to register video callback", e)
            }
        }

        onDispose {
            // Cleanup
            videoHandler.release()
        }
    }

    // SurfaceView for rendering decoded frames
    AndroidView(
        factory = { ctx ->
            SurfaceView(ctx).apply {
                holder.addCallback(object : SurfaceHolder.Callback {
                    override fun surfaceCreated(holder: SurfaceHolder) {
                        Log.d("RemoteVideoView", "📹 Surface created for call: $callId")
                        videoHandler.setSurface(holder.surface)
                    }

                    override fun surfaceChanged(
                        holder: SurfaceHolder,
                        format: Int,
                        width: Int,
                        height: Int
                    ) {
                        Log.d("RemoteVideoView", "🔄 Surface changed: ${width}x${height}")
                    }

                    override fun surfaceDestroyed(holder: SurfaceHolder) {
                        Log.d("RemoteVideoView", "🗑️ Surface destroyed")
                    }
                })
            }
        },
        modifier = modifier.background(Color.Black)
    )
}
