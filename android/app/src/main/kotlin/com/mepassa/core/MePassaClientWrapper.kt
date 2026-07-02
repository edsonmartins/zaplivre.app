package com.mepassa.core

import android.content.Context
import android.util.Log
import android.system.Os
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.flow.MutableSharedFlow
import kotlinx.coroutines.flow.MutableStateFlow
import kotlinx.coroutines.flow.SharedFlow
import kotlinx.coroutines.flow.StateFlow
import kotlinx.coroutines.flow.asSharedFlow
import kotlinx.coroutines.flow.asStateFlow
import kotlinx.coroutines.withContext
import uniffi.mepassa.*
import java.io.File
import android.util.Base64
import com.mepassa.voip.VoipEventHandler

/**
 * Wrapper Singleton para MePassaClient do UniFFI
 *
 * Fornece:
 * - Inicialização lazy do client
 * - API coroutine-friendly
 * - Estado observável (Flows)
 * - Gerenciamento de ciclo de vida
 */
@OptIn(ExperimentalUnsignedTypes::class)
object MePassaClientWrapper {
    private const val TAG = "MePassaClientWrapper"
    private const val GROUP_SENDER_KEY_PREFIX = "mepassa-group-key:v1:"

    private var client: MePassaClient? = null
    private val _isInitialized = MutableStateFlow(false)
    val isInitialized: StateFlow<Boolean> = _isInitialized.asStateFlow()

    private val _localPeerId = MutableStateFlow<String?>(null)
    val localPeerId: StateFlow<String?> = _localPeerId.asStateFlow()

    // ─── Eventos de chamada (core -> UI) ───────────────────────────────────
    data class IncomingCallEvent(val callId: String, val callerPeerId: String)

    private val _incomingCall = MutableStateFlow<IncomingCallEvent?>(null)
    val incomingCall: StateFlow<IncomingCallEvent?> = _incomingCall.asStateFlow()

    private val _callState = MutableStateFlow<Pair<String, FfiCallState>?>(null)
    val callState: StateFlow<Pair<String, FfiCallState>?> = _callState.asStateFlow()

    private val _callEnded = MutableStateFlow<Pair<String, FfiCallEndReason>?>(null)
    val callEnded: StateFlow<Pair<String, FfiCallEndReason>?> = _callEnded.asStateFlow()

    /** Limpa o evento de chamada recebida após a UI navegar */
    fun consumeIncomingCall() {
        _incomingCall.value = null
    }

    // ─── Eventos de mensagem (core -> UI, substitui o polling) ─────────────
    sealed class MessageUiEvent {
        data class Received(val messageId: String, val fromPeerId: String) : MessageUiEvent()
        data class StatusChanged(
            val messageId: String,
            val status: MessageStatus,
            val peerId: String?
        ) : MessageUiEvent()
        data class Typing(val peerId: String, val isTyping: Boolean) : MessageUiEvent()
    }

    private val _messageEvents = MutableSharedFlow<MessageUiEvent>(extraBufferCapacity = 64)
    val messageEvents: SharedFlow<MessageUiEvent> = _messageEvents.asSharedFlow()

    private val messageEventCallback = object : FfiMessageEventCallback {
        override fun onMessageReceived(messageId: String, fromPeerId: String) {
            _messageEvents.tryEmit(MessageUiEvent.Received(messageId, fromPeerId))
        }

        override fun onMessageStatusChanged(
            messageId: String,
            status: MessageStatus,
            peerId: String?
        ) {
            _messageEvents.tryEmit(MessageUiEvent.StatusChanged(messageId, status, peerId))
        }

        override fun onTyping(peerId: String, isTyping: Boolean) {
            _messageEvents.tryEmit(MessageUiEvent.Typing(peerId, isTyping))
        }
    }

    private val callEventCallback = object : FfiCallEventCallback {
        override fun onIncomingCall(callId: String, fromPeerId: String) {
            Log.i(TAG, "📞 Incoming call $callId from $fromPeerId")
            _incomingCall.value = IncomingCallEvent(callId, fromPeerId)
        }

        override fun onCallStateChanged(callId: String, state: FfiCallState) {
            Log.i(TAG, "📞 Call $callId state: $state")
            _callState.value = callId to state
        }

        override fun onCallEnded(callId: String, reason: FfiCallEndReason) {
            Log.i(TAG, "📞 Call $callId ended: $reason")
            _callEnded.value = callId to reason
            if (_incomingCall.value?.callId == callId) {
                _incomingCall.value = null
            }
        }
    }

    /**
     * Inicializa o MePassaClient
     *
     * @param context Application context
     * @return true se inicializado com sucesso
     */
    suspend fun initialize(context: Context): Boolean = withContext(Dispatchers.IO) {
        if (client != null) {
            Log.w(TAG, "Client already initialized")
            return@withContext true
        }

        try {
            // Diretório de dados do app
            val dataDir = File(context.filesDir, "mepassa_data").apply {
                if (!exists()) {
                    mkdirs()
                }
            }

            Log.i(TAG, "Initializing MePassaClient with dataDir: ${dataDir.absolutePath}")

            // Load identity from secure storage and set env var for Rust core
            val secureIdentity = AndroidIdentityStore.loadIdentity(context)
            if (!secureIdentity.isNullOrBlank()) {
                Os.setenv("MEPASSA_IDENTITY_B64", secureIdentity, true)
            } else {
                Os.unsetenv("MEPASSA_IDENTITY_B64")
            }

            // Criar client via UniFFI
            client = MePassaClient(dataDir.absolutePath)

            // Obter peer ID local
            val peerId = client!!.localPeerId()
            _localPeerId.value = peerId
            _isInitialized.value = true

            // SEC-06: a identidade já foi consumida pelo core (localPeerId só
            // responde após o build) - remover a chave privada do ambiente
            try {
                Os.unsetenv("MEPASSA_IDENTITY_B64")
            } catch (e: Exception) {
                Log.w(TAG, "Failed to clear identity env var", e)
            }

            // Persist identity to secure storage if still on disk
            val keyFile = File(dataDir, "identity.key")
            if (AndroidIdentityStore.loadIdentity(context).isNullOrBlank() && keyFile.exists()) {
                val data = keyFile.readBytes()
                val b64 = Base64.encodeToString(data, Base64.NO_WRAP)
                AndroidIdentityStore.saveIdentity(context, b64)
                keyFile.delete()
            }

            // Register VoIP event callback (mute/speaker/camera)
            try {
                registerVoipEventCallback(VoipEventHandler(context))
            } catch (e: Exception) {
                Log.e(TAG, "Failed to register VoIP event callback", e)
            }

            // Register call lifecycle callback (incoming/state/ended) - sem isso
            // o callee nunca fica sabendo de uma chamada recebida
            try {
                client!!.registerCallEventCallback(callEventCallback)
            } catch (e: Exception) {
                Log.e(TAG, "Failed to register call event callback", e)
            }

            // Eventos de mensagem (EVT-01): substitui o polling das telas
            try {
                client!!.registerMessageEventCallback(messageEventCallback)
            } catch (e: Exception) {
                Log.e(TAG, "Failed to register message event callback", e)
            }

            Log.i(TAG, "Client initialized successfully. PeerId: $peerId")
            true
        } catch (e: Exception) {
            Log.e(TAG, "Failed to initialize client", e)
            _isInitialized.value = false
            false
        }
    }

    /**
     * Export identity keypair as Base64 string.
     * Returns null if the key does not exist.
     */
    suspend fun exportIdentity(context: Context): String? = withContext(Dispatchers.IO) {
        val stored = AndroidIdentityStore.loadIdentity(context)
        if (!stored.isNullOrBlank()) {
            return@withContext stored
        }
        val keyFile = File(context.filesDir, "mepassa_data/identity.key")
        if (!keyFile.exists()) {
            return@withContext null
        }
        val data = keyFile.readBytes()
        Base64.encodeToString(data, Base64.NO_WRAP)
    }

    /**
     * Import identity keypair from Base64 string.
     * Must be called before initialize(); requires app restart if already initialized.
     */
    suspend fun importIdentity(context: Context, backup: String): Boolean = withContext(Dispatchers.IO) {
        if (client != null) {
            Log.w(TAG, "Import requires app restart (client already initialized)")
            return@withContext false
        }

        val data = try {
            Base64.decode(backup.trim(), Base64.DEFAULT)
        } catch (e: IllegalArgumentException) {
            Log.e(TAG, "Invalid backup data", e)
            return@withContext false
        }

        val dataDir = File(context.filesDir, "mepassa_data").apply {
            if (!exists()) {
                mkdirs()
            }
        }
        val b64 = Base64.encodeToString(data, Base64.NO_WRAP)
        AndroidIdentityStore.saveIdentity(context, b64)
        val keyFile = File(dataDir, "identity.key")
        if (keyFile.exists()) {
            keyFile.delete()
        }

        val dbFile = File(dataDir, "mepassa.db")
        if (dbFile.exists()) {
            dbFile.delete()
        }

        true
    }

    /**
     * Export prekey bundle JSON for E2E setup.
     */
    suspend fun exportPrekeyBundleJson(): String? = withContext(Dispatchers.IO) {
        try {
            getClient().getPrekeyBundleJson()
        } catch (e: Exception) {
            Log.e(TAG, "Failed to export prekey bundle", e)
            null
        }
    }

    /**
     * Store a peer's prekey bundle for E2E.
     */
    suspend fun storePeerPrekeyBundle(peerId: String, bundleJson: String): Boolean = withContext(Dispatchers.IO) {
        try {
            getClient().setContactPrekeyBundle(peerId, bundleJson)
            true
        } catch (e: Exception) {
            Log.e(TAG, "Failed to store prekey bundle", e)
            false
        }
    }

    /**
     * Obtém o client (deve ser inicializado primeiro)
     */
    fun getClient(): MePassaClient {
        return client ?: throw IllegalStateException("Client not initialized. Call initialize() first.")
    }

    /**
     * Verifica se o client está inicializado
     */
    fun isClientReady(): Boolean = client != null

    /**
     * Lista todas as conversas
     */
    suspend fun listConversations(): List<FfiConversation> = withContext(Dispatchers.IO) {
        try {
            getClient().listConversations()
        } catch (e: Exception) {
            Log.e(TAG, "Failed to list conversations", e)
            emptyList()
        }
    }

    /**
     * Busca mensagens de uma conversa
     */
    suspend fun getConversationMessages(
        peerId: String,
        limit: UInt? = null,
        offset: UInt? = null
    ): List<FfiMessage> = withContext(Dispatchers.IO) {
        try {
            getClient().getConversationMessages(peerId, limit, offset)
        } catch (e: Exception) {
            Log.e(TAG, "Failed to get conversation messages", e)
            emptyList()
        }
    }

    /**
     * Envia mensagem de texto
     */
    suspend fun sendTextMessage(toPeerId: String, content: String): Result<String> =
        withContext(Dispatchers.IO) {
            try {
                val messageId = getClient().sendTextMessage(toPeerId, content)
                Result.success(messageId)
            } catch (e: Exception) {
                Log.e(TAG, "Failed to send text message", e)
                Result.failure(e)
            }
        }

    /**
     * Marca conversa como lida
     */
    suspend fun markConversationRead(peerId: String): Boolean = withContext(Dispatchers.IO) {
        try {
            getClient().markConversationRead(peerId)
            true
        } catch (e: Exception) {
            Log.e(TAG, "Failed to mark conversation as read", e)
            false
        }
    }

    /**
     * Busca mensagens (full-text search)
     */
    suspend fun searchMessages(query: String, limit: UInt? = null): List<FfiMessage> =
        withContext(Dispatchers.IO) {
            try {
                getClient().searchMessages(query, limit)
            } catch (e: Exception) {
                Log.e(TAG, "Failed to search messages", e)
                emptyList()
            }
        }

    /**
     * Conecta a um peer específico
     */
    suspend fun connectToPeer(peerId: String, multiaddr: String): Boolean =
        withContext(Dispatchers.IO) {
            try {
                getClient().connectToPeer(peerId, multiaddr)
                true
            } catch (e: Exception) {
                Log.e(TAG, "Failed to connect to peer", e)
                false
            }
        }

    /**
     * Inicia escuta em um endereço
     */
    suspend fun listenOn(multiaddr: String): Boolean = withContext(Dispatchers.IO) {
        try {
            getClient().listenOn(multiaddr)
            true
        } catch (e: Exception) {
            Log.e(TAG, "Failed to listen on address", e)
            false
        }
    }

    /**
     * Faz bootstrap (conecta aos bootstrap nodes)
     */
    suspend fun bootstrap(): Boolean = withContext(Dispatchers.IO) {
        try {
            getClient().bootstrap()
            true
        } catch (e: Exception) {
            Log.e(TAG, "Failed to bootstrap", e)
            false
        }
    }

    /**
     * Obtém contagem de peers conectados
     */
    suspend fun getConnectedPeersCount(): UInt = withContext(Dispatchers.IO) {
        try {
            getClient().connectedPeersCount()
        } catch (e: Exception) {
            Log.e(TAG, "Failed to get connected peers count", e)
            0u
        }
    }

    // ========== VoIP Methods ==========

    /**
     * Inicia uma chamada de voz para um peer
     *
     * @param toPeerId ID do peer de destino
     * @return Result com call_id se sucesso, ou Exception se falha
     */
    suspend fun startCall(toPeerId: String): Result<String> = withContext(Dispatchers.IO) {
        try {
            val callId = getClient().startCall(toPeerId)
            Log.i(TAG, "Call started successfully: $callId")
            Result.success(callId)
        } catch (e: Exception) {
            Log.e(TAG, "Failed to start call", e)
            Result.failure(e)
        }
    }

    /**
     * Aceita uma chamada recebida
     *
     * @param callId ID da chamada
     * @return true se sucesso
     */
    suspend fun acceptCall(callId: String): Boolean = withContext(Dispatchers.IO) {
        try {
            getClient().acceptCall(callId)
            Log.i(TAG, "Call accepted: $callId")
            true
        } catch (e: Exception) {
            Log.e(TAG, "Failed to accept call", e)
            false
        }
    }

    /**
     * Rejeita uma chamada recebida
     *
     * @param callId ID da chamada
     * @param reason Motivo da rejeição (opcional)
     * @return true se sucesso
     */
    suspend fun rejectCall(callId: String, reason: String? = null): Boolean =
        withContext(Dispatchers.IO) {
            try {
                getClient().rejectCall(callId, reason)
                Log.i(TAG, "Call rejected: $callId")
                true
            } catch (e: Exception) {
                Log.e(TAG, "Failed to reject call", e)
                false
            }
        }

    /**
     * Encerra uma chamada ativa
     *
     * @param callId ID da chamada
     * @return true se sucesso
     */
    suspend fun hangupCall(callId: String): Boolean = withContext(Dispatchers.IO) {
        try {
            getClient().hangupCall(callId)
            Log.i(TAG, "Call hung up: $callId")
            true
        } catch (e: Exception) {
            Log.e(TAG, "Failed to hangup call", e)
            false
        }
    }

    /**
     * Alterna mute do microfone
     *
     * @param callId ID da chamada
     * @return true se sucesso
     */
    suspend fun toggleMute(callId: String): Boolean = withContext(Dispatchers.IO) {
        try {
            getClient().toggleMute(callId)
            Log.i(TAG, "Mute toggled for call: $callId")
            true
        } catch (e: Exception) {
            Log.e(TAG, "Failed to toggle mute", e)
            false
        }
    }

    /**
     * Alterna speakerphone
     *
     * @param callId ID da chamada
     * @return true se sucesso
     */
    suspend fun toggleSpeakerphone(callId: String): Boolean = withContext(Dispatchers.IO) {
        try {
            getClient().toggleSpeakerphone(callId)
            Log.i(TAG, "Speakerphone toggled for call: $callId")
            true
        } catch (e: Exception) {
            Log.e(TAG, "Failed to toggle speakerphone", e)
            false
        }
    }

    // ========== Group Methods (FASE 15) ==========

    /**
     * Cria um novo grupo
     *
     * @param name Nome do grupo
     * @param description Descrição do grupo (opcional)
     * @return FfiGroup criado
     */
    suspend fun createGroup(name: String, description: String?): FfiGroup =
        withContext(Dispatchers.IO) {
            try {
                val group = getClient().createGroup(name, description)
                Log.i(TAG, "Group created successfully: ${group.id}")
                group
            } catch (e: Exception) {
                Log.e(TAG, "Failed to create group", e)
                throw e
            }
        }

    /**
     * Entra em um grupo existente
     *
     * @param groupId ID do grupo
     * @param groupName Nome do grupo
     */
    suspend fun joinGroup(groupId: String, groupName: String): Boolean =
        withContext(Dispatchers.IO) {
            try {
                getClient().joinGroup(groupId, groupName)
                Log.i(TAG, "Joined group successfully: $groupId")
                true
            } catch (e: Exception) {
                Log.e(TAG, "Failed to join group", e)
                false
            }
        }

    /**
     * Sai de um grupo
     *
     * @param groupId ID do grupo
     */
    suspend fun leaveGroup(groupId: String): Boolean = withContext(Dispatchers.IO) {
        try {
            getClient().leaveGroup(groupId)
            Log.i(TAG, "Left group successfully: $groupId")
            true
        } catch (e: Exception) {
            Log.e(TAG, "Failed to leave group", e)
            false
        }
    }

    /**
     * Adiciona um membro ao grupo (apenas admin)
     *
     * @param groupId ID do grupo
     * @param peerId ID do peer a adicionar
     */
    suspend fun addGroupMember(groupId: String, peerId: String): Boolean =
        withContext(Dispatchers.IO) {
            try {
                getClient().addGroupMember(groupId, peerId)
                Log.i(TAG, "Added member to group $groupId: $peerId")
                true
            } catch (e: Exception) {
                Log.e(TAG, "Failed to add group member", e)
                throw e
            }
        }

    /**
     * Remove um membro do grupo (apenas admin)
     *
     * @param groupId ID do grupo
     * @param peerId ID do peer a remover
     */
    suspend fun removeGroupMember(groupId: String, peerId: String): Boolean =
        withContext(Dispatchers.IO) {
            try {
                getClient().removeGroupMember(groupId, peerId)
                Log.i(TAG, "Removed member from group $groupId: $peerId")
                true
            } catch (e: Exception) {
                Log.e(TAG, "Failed to remove group member", e)
                false
            }
        }

    /**
     * Lista todos os grupos do usuário
     *
     * @return Lista de grupos
     */
    suspend fun getGroups(): List<FfiGroup> = withContext(Dispatchers.IO) {
        try {
            getClient().getGroups()
        } catch (e: Exception) {
            Log.e(TAG, "Failed to get groups", e)
            emptyList()
        }
    }

    /**
     * Retorna os peer IDs dos membros de um grupo
     */
    suspend fun getGroupMembers(groupId: String): List<String> = withContext(Dispatchers.IO) {
        try {
            getClient().getGroupMembers(groupId)
        } catch (e: Exception) {
            Log.e(TAG, "Failed to get group members", e)
            emptyList()
        }
    }

    /**
     * Atualiza nome/descrição de um grupo (admin only)
     */
    suspend fun updateGroup(
        groupId: String,
        name: String?,
        description: String?
    ): Boolean = withContext(Dispatchers.IO) {
        try {
            getClient().updateGroup(groupId, name, description)
            true
        } catch (e: Exception) {
            Log.e(TAG, "Failed to update group", e)
            false
        }
    }

    /**
     * Register callback for VoIP control events (mute/speaker/camera)
     */
    fun registerVoipEventCallback(callback: uniffi.mepassa.FfiVoipEventCallback) {
        try {
            getClient().registerVoipEventCallback(callback)
            Log.i(TAG, "VoIP event callback registered")
        } catch (e: Exception) {
            Log.e(TAG, "Failed to register VoIP event callback", e)
        }
    }

    /**
     * Retorna mensagens de um grupo
     */
    suspend fun getGroupMessages(
        groupId: String,
        limit: UInt? = null,
        offset: UInt? = null
    ): List<FfiMessage> = withContext(Dispatchers.IO) {
        try {
            getClient().getGroupMessages(groupId, limit, offset)
        } catch (e: Exception) {
            Log.e(TAG, "Failed to get group messages", e)
            emptyList()
        }
    }

    /**
     * Envia mensagem para um grupo
     */
    suspend fun sendGroupMessage(groupId: String, content: String): String =
        withContext(Dispatchers.IO) {
            try {
                getClient().sendGroupMessage(groupId, content)
            } catch (e: Exception) {
                Log.e(TAG, "Failed to send group message", e)
                throw e
            }
        }

    suspend fun getGroupSenderKeySeed(groupId: String): List<UByte>? =
        withContext(Dispatchers.IO) {
            try {
                getClient().getGroupSenderKeySeed(groupId)
            } catch (e: Exception) {
                Log.e(TAG, "Failed to get group sender key seed", e)
                null
            }
        }

    suspend fun addGroupSenderKey(
        groupId: String,
        senderPeerId: String,
        senderKeySeed: List<UByte>
    ): Boolean = withContext(Dispatchers.IO) {
        try {
            getClient().addGroupSenderKey(groupId, senderPeerId, senderKeySeed)
            true
        } catch (e: Exception) {
            Log.e(TAG, "Failed to add group sender key", e)
            false
        }
    }

    /**
     * Filtro de exibição para mensagens LEGADAS do hack antigo de distribuição
     * de sender key por texto. A distribuição agora é feita pelo core
     * (protocolo in-band, CORE-16) e não gera mais mensagens de chat.
     */
    fun isLegacyGroupKeyMessage(message: FfiMessage): Boolean =
        message.contentPlaintext?.startsWith(GROUP_SENDER_KEY_PREFIX) == true

    // ========== Video Methods (FASE 14) ==========

    /**
     * Enable video for an active call
     *
     * @param callId Call identifier
     * @param codec Video codec to use (H264, VP8, VP9)
     */
    suspend fun enableVideo(callId: String, codec: FfiVideoCodec = FfiVideoCodec.H264) = withContext(Dispatchers.IO) {
        try {
            getClient().enableVideo(callId, codec)
            Log.i(TAG, "Video enabled for call: $callId with codec: $codec")
        } catch (e: Exception) {
            Log.e(TAG, "Failed to enable video for call: $callId", e)
            throw e
        }
    }

    /**
     * Disable video for an active call
     *
     * @param callId Call identifier
     */
    suspend fun disableVideo(callId: String) = withContext(Dispatchers.IO) {
        try {
            getClient().disableVideo(callId)
            Log.i(TAG, "Video disabled for call: $callId")
        } catch (e: Exception) {
            Log.e(TAG, "Failed to disable video for call: $callId", e)
            throw e
        }
    }

    /**
     * Send video frame to remote peer
     *
     * @param callId Call identifier
     * @param frameData Raw frame data (pre-encoded H.264/VP8 NALUs)
     * @param width Frame width in pixels
     * @param height Frame height in pixels
     */
    suspend fun sendVideoFrame(
        callId: String,
        frameData: ByteArray,
        width: UInt,
        height: UInt
    ) = withContext(Dispatchers.IO) {
        try {
            getClient().sendVideoFrame(callId, frameData.toUByteArray().toList(), width, height)
        } catch (e: Exception) {
            Log.e(TAG, "Failed to send video frame for call: $callId", e)
            // Don't throw - frame drops are acceptable
        }
    }

    /**
     * Switch camera (front/back) during video call
     *
     * @param callId Call identifier
     */
    suspend fun switchCamera(callId: String) = withContext(Dispatchers.IO) {
        try {
            getClient().switchCamera(callId)
            Log.i(TAG, "Camera switched for call: $callId")
        } catch (e: Exception) {
            Log.e(TAG, "Failed to switch camera for call: $callId", e)
            throw e
        }
    }

    /**
     * Register callback for receiving remote video frames (FASE 14)
     *
     * @param callback Implementation of FfiVideoFrameCallback that will
     *                 receive decoded video frames for rendering
     */
    suspend fun registerVideoFrameCallback(callback: uniffi.mepassa.FfiVideoFrameCallback) = withContext(Dispatchers.IO) {
        try {
            getClient().registerVideoFrameCallback(callback)
            Log.i(TAG, "✅ Video frame callback registered")
        } catch (e: Exception) {
            Log.e(TAG, "❌ Failed to register video frame callback", e)
            throw e
        }
    }

    // ====== Media Messages ======

    /**
     * Send image message
     */
    suspend fun sendImageMessage(
        toPeerId: String,
        imageData: List<UByte>,
        fileName: String,
        quality: UInt = 85u
    ) = withContext(Dispatchers.IO) {
        try {
            val c = client ?: throw IllegalStateException("Client not initialized")
            c.sendImageMessage(toPeerId, imageData, fileName, quality)
            Log.d(TAG, "✅ Image message sent to $toPeerId")
        } catch (e: Exception) {
            Log.e(TAG, "❌ Failed to send image message", e)
            throw e
        }
    }

    /**
     * Send voice message
     */
    suspend fun sendVoiceMessage(
        toPeerId: String,
        audioData: List<UByte>,
        fileName: String,
        durationSeconds: Int
    ) = withContext(Dispatchers.IO) {
        try {
            val c = client ?: throw IllegalStateException("Client not initialized")
            c.sendVoiceMessage(toPeerId, audioData, fileName, durationSeconds)
            Log.d(TAG, "✅ Voice message sent to $toPeerId")
        } catch (e: Exception) {
            Log.e(TAG, "❌ Failed to send voice message", e)
            throw e
        }
    }

    /**
     * Send document/file message
     */
    suspend fun sendDocumentMessage(
        toPeerId: String,
        fileData: List<UByte>,
        fileName: String,
        mimeType: String
    ) = withContext(Dispatchers.IO) {
        try {
            val c = client ?: throw IllegalStateException("Client not initialized")
            c.sendDocumentMessage(toPeerId, fileData, fileName, mimeType)
            Log.d(TAG, "✅ Document message sent to $toPeerId: $fileName")
        } catch (e: Exception) {
            Log.e(TAG, "❌ Failed to send document message", e)
            throw e
        }
    }

    /**
     * Send video message
     */
    suspend fun sendVideoMessage(
        toPeerId: String,
        videoData: List<UByte>,
        fileName: String,
        width: Int? = null,
        height: Int? = null,
        durationSeconds: Int,
        thumbnailData: List<UByte>? = null
    ) = withContext(Dispatchers.IO) {
        try {
            val c = client ?: throw IllegalStateException("Client not initialized")
            c.sendVideoMessage(toPeerId, videoData, fileName, width, height, durationSeconds, thumbnailData)
            Log.d(TAG, "✅ Video message sent to $toPeerId")
        } catch (e: Exception) {
            Log.e(TAG, "❌ Failed to send video message", e)
            throw e
        }
    }

    // ====== Message Actions ======

    /**
     * Delete message
     */
    suspend fun deleteMessage(messageId: String) = withContext(Dispatchers.IO) {
        try {
            val c = client ?: throw IllegalStateException("Client not initialized")
            c.deleteMessage(messageId)
            Log.d(TAG, "✅ Message deleted: $messageId")
        } catch (e: Exception) {
            Log.e(TAG, "❌ Failed to delete message", e)
            throw e
        }
    }

    /**
     * Forward message
     */
    suspend fun forwardMessage(messageId: String, toPeerId: String) = withContext(Dispatchers.IO) {
        try {
            val c = client ?: throw IllegalStateException("Client not initialized")
            c.forwardMessage(messageId, toPeerId)
            Log.d(TAG, "✅ Message forwarded: $messageId to $toPeerId")
        } catch (e: Exception) {
            Log.e(TAG, "❌ Failed to forward message", e)
            throw e
        }
    }

    // ====== Reactions ======

    /**
     * Add reaction to message
     */
    suspend fun addReaction(messageId: String, emoji: String) = withContext(Dispatchers.IO) {
        try {
            val c = client ?: throw IllegalStateException("Client not initialized")
            c.addReaction(messageId, emoji)
            Log.d(TAG, "✅ Reaction added: $emoji to $messageId")
        } catch (e: Exception) {
            Log.e(TAG, "❌ Failed to add reaction", e)
            throw e
        }
    }

    /**
     * Remove reaction from message
     */
    suspend fun removeReaction(messageId: String, emoji: String) = withContext(Dispatchers.IO) {
        try {
            val c = client ?: throw IllegalStateException("Client not initialized")
            c.removeReaction(messageId, emoji)
            Log.d(TAG, "✅ Reaction removed: $emoji from $messageId")
        } catch (e: Exception) {
            Log.e(TAG, "❌ Failed to remove reaction", e)
            throw e
        }
    }

    /**
     * Get reactions for message
     */
    suspend fun getMessageReactions(messageId: String): List<uniffi.mepassa.FfiReaction> = withContext(Dispatchers.IO) {
        try {
            val c = client ?: throw IllegalStateException("Client not initialized")
            c.getMessageReactions(messageId)
        } catch (e: Exception) {
            Log.e(TAG, "❌ Failed to get message reactions", e)
            emptyList()
        }
    }

    // ====== Media Management ======

    /**
     * Get media from conversation
     */
    suspend fun getConversationMedia(
        conversationId: String,
        mediaType: uniffi.mepassa.FfiMediaType? = null,
        limit: UInt = 100u
    ): List<uniffi.mepassa.FfiMedia> = withContext(Dispatchers.IO) {
        try {
            val c = client ?: throw IllegalStateException("Client not initialized")
            c.getConversationMedia(conversationId, mediaType, limit)
        } catch (e: Exception) {
            Log.e(TAG, "❌ Failed to get conversation media", e)
            emptyList()
        }
    }

    /**
     * Download media by hash
     */
    suspend fun downloadMedia(mediaHash: String): ByteArray = withContext(Dispatchers.IO) {
        try {
            val c = client ?: throw IllegalStateException("Client not initialized")
            c.downloadMedia(mediaHash).map { it.toByte() }.toByteArray()
        } catch (e: Exception) {
            Log.e(TAG, "❌ Failed to download media", e)
            throw e
        }
    }

    /**
     * Shutdown do client (chame no onDestroy da Application)
     */
    fun shutdown() {
        try {
            client = null
            _isInitialized.value = false
            _localPeerId.value = null
            Log.i(TAG, "Client shutdown completed")
        } catch (e: Exception) {
            Log.e(TAG, "Error during shutdown", e)
        }
    }
}

