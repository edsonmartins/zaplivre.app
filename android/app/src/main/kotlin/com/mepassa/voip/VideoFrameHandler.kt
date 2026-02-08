package com.mepassa.voip

import android.media.MediaCodec
import android.media.MediaCodecInfo
import android.media.MediaFormat
import android.util.Log
import android.view.Surface
import uniffi.mepassa.FfiVideoFrameCallback
import java.nio.ByteBuffer

/**
 * VideoFrameHandler - Implements FfiVideoFrameCallback for rendering remote video
 *
 * Receives H.264 encoded frames from FFI, decodes them using MediaCodec,
 * and renders to a Surface (from SurfaceView).
 */
class VideoFrameHandler(
    private val callId: String,
    private val onFrameRendered: ((width: Int, height: Int) -> Unit)? = null
) : FfiVideoFrameCallback {

    private var decoder: MediaCodec? = null
    private var surface: Surface? = null
    private var isDecoderConfigured = false
    private var currentWidth: Int = 640
    private var currentHeight: Int = 480

    companion object {
        private const val TAG = "VideoFrameHandler"
        private const val MIME_TYPE = MediaFormat.MIMETYPE_VIDEO_AVC // H.264
        private const val TIMEOUT_US = 10000L // 10ms
    }

    /**
     * Set the output surface for video rendering
     * Must be called before receiving frames
     */
    fun setSurface(surface: Surface) {
        this.surface = surface
        configureDecoder(currentWidth, currentHeight)
    }

    /**
     * Configure MediaCodec decoder
     */
    private fun configureDecoder(width: Int, height: Int) {
        try {
            // Release existing decoder if any
            decoder?.stop()
            decoder?.release()

            // Create H.264 decoder
            decoder = MediaCodec.createDecoderByType(MIME_TYPE)

            // Configure decoder
            val format = MediaFormat.createVideoFormat(MIME_TYPE, width, height).apply {
                // Set color format
                setInteger(
                    MediaFormat.KEY_COLOR_FORMAT,
                    MediaCodecInfo.CodecCapabilities.COLOR_FormatSurface
                )
            }

            decoder?.configure(format, surface, null, 0)
            decoder?.start()

            isDecoderConfigured = true
            Log.i(TAG, "✅ Decoder configured: ${width}x${height}")
        } catch (e: Exception) {
            Log.e(TAG, "❌ Failed to configure decoder", e)
            isDecoderConfigured = false
        }
    }

    /**
     * Called from FFI when a remote video frame is received
     *
     * @param callId Call identifier
     * @param frameData Raw H.264 frame data (NALUs)
     * @param width Frame width in pixels
     * @param height Frame height in pixels
     */
    override fun onVideoFrame(callId: String, frameData: List<UByte>, width: UInt, height: UInt) {
        // Ignore frames from other calls
        if (callId != this.callId) return

        val targetWidth = width.toInt()
        val targetHeight = height.toInt()
        if (targetWidth > 0 && targetHeight > 0 &&
            (targetWidth != currentWidth || targetHeight != currentHeight)
        ) {
            currentWidth = targetWidth
            currentHeight = targetHeight
            configureDecoder(currentWidth, currentHeight)
        }

        // Check if decoder is ready
        if (!isDecoderConfigured || decoder == null) {
            Log.w(TAG, "⚠️ Decoder not configured, skipping frame")
            return
        }

        try {
            // Convert UByte list to ByteArray
            val data = frameData.map { it.toByte() }.toByteArray()

            // Get input buffer
            val inputBufferIndex = decoder!!.dequeueInputBuffer(TIMEOUT_US)
            if (inputBufferIndex >= 0) {
                val inputBuffer = decoder!!.getInputBuffer(inputBufferIndex)
                inputBuffer?.clear()
                inputBuffer?.put(data)

                // Queue input buffer for decoding
                decoder!!.queueInputBuffer(
                    inputBufferIndex,
                    0,
                    data.size,
                    System.nanoTime() / 1000, // Presentation timestamp in microseconds
                    0 // Flags
                )
            }

            // Dequeue output buffer (decoded frame)
            val bufferInfo = MediaCodec.BufferInfo()
            val outputBufferIndex = decoder!!.dequeueOutputBuffer(bufferInfo, TIMEOUT_US)

            when {
                outputBufferIndex >= 0 -> {
                    // Frame decoded successfully, render to surface
                    decoder!!.releaseOutputBuffer(outputBufferIndex, true /* render */)

                    // Notify callback
                    onFrameRendered?.invoke(width.toInt(), height.toInt())
                }
                outputBufferIndex == MediaCodec.INFO_OUTPUT_FORMAT_CHANGED -> {
                    val newFormat = decoder!!.outputFormat
                    Log.d(TAG, "🔄 Output format changed: $newFormat")
                }
                outputBufferIndex == MediaCodec.INFO_TRY_AGAIN_LATER -> {
                    // No output available yet, this is normal
                }
            }
        } catch (e: Exception) {
            Log.e(TAG, "❌ Error processing video frame", e)
        }
    }

    /**
     * Release decoder resources
     */
    fun release() {
        try {
            decoder?.stop()
            decoder?.release()
            decoder = null
            isDecoderConfigured = false
            Log.i(TAG, "🧹 VideoFrameHandler released")
        } catch (e: Exception) {
            Log.e(TAG, "❌ Error releasing decoder", e)
        }
    }
}
