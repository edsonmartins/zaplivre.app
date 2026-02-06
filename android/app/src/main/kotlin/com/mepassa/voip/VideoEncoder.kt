package com.mepassa.voip

import android.media.MediaCodec
import android.media.MediaCodecInfo
import android.media.MediaFormat
import android.os.Build
import android.util.Log
import java.nio.ByteBuffer

/**
 * VideoEncoder - Encodes YUV frames to H.264 using MediaCodec.
 *
 * MVP: expects NV21 input and outputs H.264 Annex B NAL units.
 */
class VideoEncoder(
    private val width: Int,
    private val height: Int,
    private val onEncoded: (ByteArray, Boolean) -> Unit
) {
    private var encoder: MediaCodec? = null
    private var configData: ByteArray? = null

    companion object {
        private const val TAG = "VideoEncoder"
        private const val MIME_TYPE = MediaFormat.MIMETYPE_VIDEO_AVC
        private const val BITRATE = 800_000
        private const val FPS = 15
        private const val IFRAME_INTERVAL = 2
        private const val TIMEOUT_US = 10_000L
    }

    fun start() {
        if (encoder != null) return

        val format = MediaFormat.createVideoFormat(MIME_TYPE, width, height).apply {
            setInteger(MediaFormat.KEY_COLOR_FORMAT, MediaCodecInfo.CodecCapabilities.COLOR_FormatYUV420Flexible)
            setInteger(MediaFormat.KEY_BIT_RATE, BITRATE)
            setInteger(MediaFormat.KEY_FRAME_RATE, FPS)
            setInteger(MediaFormat.KEY_I_FRAME_INTERVAL, IFRAME_INTERVAL)
            if (Build.VERSION.SDK_INT >= 23) {
                setInteger(MediaFormat.KEY_PREPEND_HEADER_TO_SYNC_FRAME, 1)
            }
        }

        encoder = MediaCodec.createEncoderByType(MIME_TYPE).apply {
            configure(format, null, null, MediaCodec.CONFIGURE_FLAG_ENCODE)
            start()
        }

        Log.i(TAG, "✅ Video encoder started (${width}x${height})")
    }

    fun stop() {
        try {
            encoder?.stop()
            encoder?.release()
            encoder = null
            configData = null
            Log.i(TAG, "🛑 Video encoder stopped")
        } catch (e: Exception) {
            Log.e(TAG, "❌ Failed to stop encoder", e)
        }
    }

    fun encodeFrame(nv21: ByteArray, presentationTimeUs: Long) {
        val codec = encoder ?: return
        val inputIndex = codec.dequeueInputBuffer(TIMEOUT_US)
        if (inputIndex >= 0) {
            val inputBuffer = codec.getInputBuffer(inputIndex)
            inputBuffer?.clear()

            val nv12 = nv21ToNv12(nv21)
            inputBuffer?.put(nv12)

            codec.queueInputBuffer(
                inputIndex,
                0,
                nv12.size,
                presentationTimeUs,
                0
            )
        }

        drainOutput(codec)
    }

    private fun drainOutput(codec: MediaCodec) {
        val bufferInfo = MediaCodec.BufferInfo()
        var outputIndex = codec.dequeueOutputBuffer(bufferInfo, TIMEOUT_US)
        while (outputIndex >= 0) {
            val outputBuffer = codec.getOutputBuffer(outputIndex)
            val outData = ByteArray(bufferInfo.size)
            outputBuffer?.get(outData)

            val isConfig = bufferInfo.flags and MediaCodec.BUFFER_FLAG_CODEC_CONFIG != 0
            val isKeyFrame = bufferInfo.flags and MediaCodec.BUFFER_FLAG_KEY_FRAME != 0

            if (isConfig) {
                configData = outData
            } else {
                val payload = if (isKeyFrame && configData != null) {
                    configData!! + outData
                } else {
                    outData
                }
                onEncoded(payload, isKeyFrame)
            }

            codec.releaseOutputBuffer(outputIndex, false)
            outputIndex = codec.dequeueOutputBuffer(bufferInfo, 0)
        }
    }

    private fun nv21ToNv12(nv21: ByteArray): ByteArray {
        val nv12 = nv21.clone()
        var i = width * height
        while (i + 1 < nv12.size) {
            val v = nv12[i]
            nv12[i] = nv12[i + 1]
            nv12[i + 1] = v
            i += 2
        }
        return nv12
    }
}
