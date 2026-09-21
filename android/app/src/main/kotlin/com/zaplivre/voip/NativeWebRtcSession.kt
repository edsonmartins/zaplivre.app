package com.zaplivre.voip

import android.content.Context
import android.util.Log
import com.zaplivre.core.ZapLivreClientWrapper
import kotlinx.coroutines.CoroutineScope
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.Job
import kotlinx.coroutines.SupervisorJob
import kotlinx.coroutines.cancel
import kotlinx.coroutines.flow.filter
import kotlinx.coroutines.launch
import kotlinx.coroutines.withTimeoutOrNull
import org.webrtc.AudioSource
import org.webrtc.AudioTrack
import org.webrtc.Camera2Enumerator
import org.webrtc.CameraVideoCapturer
import org.webrtc.DefaultVideoDecoderFactory
import org.webrtc.DefaultVideoEncoderFactory
import org.webrtc.EglBase
import org.webrtc.IceCandidate
import org.webrtc.MediaConstraints
import org.webrtc.MediaStream
import org.webrtc.PeerConnection
import org.webrtc.PeerConnectionFactory
import org.webrtc.RtpReceiver
import org.webrtc.RtpTransceiver
import org.webrtc.SdpObserver
import org.webrtc.SessionDescription
import org.webrtc.SurfaceTextureHelper
import org.webrtc.SurfaceViewRenderer
import org.webrtc.VideoSource
import org.webrtc.VideoTrack

/** Owns the complete Android WebRTC media path for one ZapLivre call. */
class NativeWebRtcSession(
    context: Context,
    private val callId: String
) {
    private val appContext = context.applicationContext
    private val scope = CoroutineScope(SupervisorJob() + Dispatchers.Main.immediate)
    private val eglBase = EglBase.create()
    private var signalJob: Job? = null
    private var setupJob: Job? = null
    private var peerConnection: PeerConnection? = null
    private var videoCapturer: CameraVideoCapturer? = null
    private var textureHelper: SurfaceTextureHelper? = null
    private var videoSource: VideoSource? = null
    private var audioSource: AudioSource? = null
    private var localVideoTrack: VideoTrack? = null
    private var localAudioTrack: AudioTrack? = null
    private var localRenderer: SurfaceViewRenderer? = null
    private var remoteRenderer: SurfaceViewRenderer? = null
    private val pendingIce = mutableListOf<IceCandidate>()

    fun start(
        localRenderer: SurfaceViewRenderer,
        remoteRenderer: SurfaceViewRenderer,
        createOffer: Boolean
    ) {
        this.localRenderer = localRenderer
        this.remoteRenderer = remoteRenderer
        localRenderer.init(eglBase.eglBaseContext, null)
        localRenderer.setMirror(true)
        localRenderer.setEnableHardwareScaler(true)
        remoteRenderer.init(eglBase.eglBaseContext, null)
        remoteRenderer.setMirror(false)
        remoteRenderer.setEnableHardwareScaler(true)

        // A PeerConnection nasce depois de obter os ICE servers. Sem eles só há
        // candidatos host e a chamada nunca conecta entre dois celulares em
        // rede móvel (CGNAT). `stop()` cancela o scope, e com ele este launch.
        setupJob = scope.launch {
            val iceServers = withTimeoutOrNull(ICE_SERVERS_TIMEOUT_MS) {
                ZapLivreClientWrapper.iceServers()
            }.orEmpty()
            if (iceServers.isEmpty()) {
                Log.w(TAG, "No ICE servers available: call limited to the local network")
            }
            connect(iceServers, createOffer)
        }
    }

    private fun connect(iceServers: List<PeerConnection.IceServer>, createOffer: Boolean) {
        val localRenderer = localRenderer ?: return
        ensureFactoryInitialized(appContext)
        val factory = PeerConnectionFactory.builder()
            .setVideoEncoderFactory(
                DefaultVideoEncoderFactory(eglBase.eglBaseContext, true, true)
            )
            .setVideoDecoderFactory(DefaultVideoDecoderFactory(eglBase.eglBaseContext))
            .createPeerConnectionFactory()

        peerConnection = factory.createPeerConnection(
            PeerConnection.RTCConfiguration(iceServers).apply {
                sdpSemantics = PeerConnection.SdpSemantics.UNIFIED_PLAN
                continualGatheringPolicy = PeerConnection.ContinualGatheringPolicy.GATHER_CONTINUALLY
            },
            peerObserver
        ) ?: error("Unable to create Android WebRTC PeerConnection")

        videoCapturer = createCameraCapturer()
        textureHelper = SurfaceTextureHelper.create("ZapLivreCamera", eglBase.eglBaseContext)
        videoSource = factory.createVideoSource(false).also { source ->
            videoCapturer!!.initialize(textureHelper, appContext, source.capturerObserver)
        }
        localVideoTrack = factory.createVideoTrack("video-$callId", videoSource).also {
            it.addSink(localRenderer)
            peerConnection!!.addTrack(it, listOf("stream-$callId"))
        }
        audioSource = factory.createAudioSource(MediaConstraints())
        localAudioTrack = factory.createAudioTrack("audio-$callId", audioSource).also {
            peerConnection!!.addTrack(it, listOf("stream-$callId"))
        }
        videoCapturer!!.startCapture(640, 480, 24)

        signalJob = scope.launch {
            ZapLivreClientWrapper.webRtcSignals
                .filter { it.callId == callId }
                .collect { signal -> handleSignal(signal) }
        }
        if (createOffer) createOffer()
    }

    fun setVideoEnabled(enabled: Boolean) {
        localVideoTrack?.setEnabled(enabled)
    }

    fun setAudioEnabled(enabled: Boolean) {
        localAudioTrack?.setEnabled(enabled)
    }

    fun switchCamera() {
        videoCapturer?.switchCamera(null)
    }

    fun stop() {
        setupJob?.cancel()
        signalJob?.cancel()
        try { videoCapturer?.stopCapture() } catch (_: InterruptedException) { }
        localVideoTrack?.dispose()
        localAudioTrack?.dispose()
        videoSource?.dispose()
        audioSource?.dispose()
        textureHelper?.dispose()
        videoCapturer?.dispose()
        peerConnection?.close()
        peerConnection?.dispose()
        localRenderer?.release()
        remoteRenderer?.release()
        eglBase.release()
        scope.cancel()
    }

    private fun createOffer() {
        peerConnection?.createOffer(object : SimpleSdpObserver() {
            override fun onCreateSuccess(description: SessionDescription) {
                setLocalAndSend(description) {
                    ZapLivreClientWrapper.sendWebRtcOffer(callId, description.description)
                }
            }
        }, offerConstraints())
    }

    private fun handleSignal(signal: ZapLivreClientWrapper.WebRtcSignal) {
        when (signal) {
            is ZapLivreClientWrapper.WebRtcSignal.Offer -> {
                setRemote(SessionDescription(SessionDescription.Type.OFFER, signal.sdp)) {
                    peerConnection?.createAnswer(object : SimpleSdpObserver() {
                        override fun onCreateSuccess(description: SessionDescription) {
                            setLocalAndSend(description) {
                                ZapLivreClientWrapper.sendWebRtcAnswer(callId, description.description)
                            }
                        }
                    }, offerConstraints())
                }
            }
            is ZapLivreClientWrapper.WebRtcSignal.Answer -> {
                setRemote(SessionDescription(SessionDescription.Type.ANSWER, signal.sdp))
            }
            is ZapLivreClientWrapper.WebRtcSignal.Ice -> {
                val candidate = IceCandidate(
                    signal.sdpMid,
                    signal.sdpMLineIndex?.toInt() ?: 0,
                    signal.candidate
                )
                if (peerConnection?.remoteDescription == null) pendingIce += candidate
                else peerConnection?.addIceCandidate(candidate)
            }
        }
    }

    private fun setRemote(description: SessionDescription, complete: () -> Unit = {}) {
        peerConnection?.setRemoteDescription(object : SimpleSdpObserver() {
            override fun onSetSuccess() {
                pendingIce.forEach { peerConnection?.addIceCandidate(it) }
                pendingIce.clear()
                complete()
            }
        }, description)
    }

    private fun setLocalAndSend(
        description: SessionDescription,
        send: suspend () -> Unit
    ) {
        peerConnection?.setLocalDescription(object : SimpleSdpObserver() {
            override fun onSetSuccess() {
                scope.launch {
                    runCatching { send() }
                        .onFailure { Log.e(TAG, "Failed to send SDP for $callId", it) }
                }
            }
        }, description)
    }

    private val peerObserver = object : PeerConnection.Observer {
        override fun onIceCandidate(candidate: IceCandidate) {
            scope.launch(Dispatchers.IO) {
                runCatching {
                    ZapLivreClientWrapper.sendWebRtcIceCandidate(
                        callId,
                        candidate.sdp,
                        candidate.sdpMid,
                        candidate.sdpMLineIndex.toUShort()
                    )
                }.onFailure { Log.e(TAG, "Failed to send ICE for $callId", it) }
            }
        }

        override fun onTrack(transceiver: RtpTransceiver) {
            (transceiver.receiver.track() as? VideoTrack)?.addSink(remoteRenderer)
        }

        override fun onAddTrack(receiver: RtpReceiver, mediaStreams: Array<out MediaStream>) {
            (receiver.track() as? VideoTrack)?.addSink(remoteRenderer)
        }

        override fun onIceConnectionChange(state: PeerConnection.IceConnectionState) {
            Log.i(TAG, "ICE $callId: $state")
        }

        override fun onSignalingChange(state: PeerConnection.SignalingState) = Unit
        override fun onIceConnectionReceivingChange(receiving: Boolean) = Unit
        override fun onIceGatheringChange(state: PeerConnection.IceGatheringState) = Unit
        override fun onIceCandidatesRemoved(candidates: Array<out IceCandidate>) = Unit
        override fun onAddStream(stream: MediaStream) = Unit
        override fun onRemoveStream(stream: MediaStream) = Unit
        override fun onDataChannel(channel: org.webrtc.DataChannel) = Unit
        override fun onRenegotiationNeeded() = Unit
    }

    private fun createCameraCapturer(): CameraVideoCapturer {
        val enumerator = Camera2Enumerator(appContext)
        val name = enumerator.deviceNames.firstOrNull(enumerator::isFrontFacing)
            ?: enumerator.deviceNames.firstOrNull()
            ?: error("No camera available")
        return enumerator.createCapturer(name, null)
            ?: error("Unable to open camera $name")
    }

    private fun offerConstraints() = MediaConstraints().apply {
        mandatory += MediaConstraints.KeyValuePair("OfferToReceiveAudio", "true")
        mandatory += MediaConstraints.KeyValuePair("OfferToReceiveVideo", "true")
    }

    private open class SimpleSdpObserver : SdpObserver {
        override fun onCreateSuccess(description: SessionDescription) = Unit
        override fun onSetSuccess() = Unit
        override fun onCreateFailure(error: String) {
            Log.e(TAG, "SDP create failed: $error")
        }
        override fun onSetFailure(error: String) {
            Log.e(TAG, "SDP set failed: $error")
        }
    }

    companion object {
        private const val TAG = "NativeWebRtcSession"

        /** Teto para obter os ICE servers antes de seguir só com a rede local. */
        private const val ICE_SERVERS_TIMEOUT_MS = 5_000L
        @Volatile private var initialized = false

        @Synchronized
        private fun ensureFactoryInitialized(context: Context) {
            if (initialized) return
            PeerConnectionFactory.initialize(
                PeerConnectionFactory.InitializationOptions.builder(context)
                    .setEnableInternalTracer(false)
                    .createInitializationOptions()
            )
            initialized = true
        }
    }
}
