package com.zaplivre.core

import kotlinx.coroutines.flow.SharedFlow
import kotlinx.coroutines.flow.StateFlow
import uniffi.zaplivre.FfiConversation
import uniffi.zaplivre.FfiMessage
import uniffi.zaplivre.FfiReaction

/**
 * Superfície mínima do cliente usada pelas telas de Conversas e Chat.
 *
 * Extraída de [ZapLivreClientWrapper] para permitir substituir o singleton
 * real por fakes/mocks em unit tests (via [ServiceLocator]).
 */
interface ZapLivreClientApi {

    /** Peer ID local (null enquanto o client não inicializa). */
    val localPeerId: StateFlow<String?>

    /** Eventos de mensagem vindos do core (EVT-01, substitui polling). */
    val messageEvents: SharedFlow<ZapLivreClientWrapper.MessageUiEvent>

    /** Lista todas as conversas. */
    suspend fun listConversations(): List<FfiConversation>

    /** Busca mensagens de uma conversa. */
    suspend fun getConversationMessages(
        peerId: String,
        limit: UInt? = null,
        offset: UInt? = null
    ): List<FfiMessage>

    /** Envia mensagem de texto. */
    suspend fun sendTextMessage(toPeerId: String, content: String): Result<String>

    /** Marca conversa como lida. */
    suspend fun markConversationRead(peerId: String): Boolean

    /** Filtro para mensagens legadas do hack antigo de sender key de grupo. */
    fun isLegacyGroupKeyMessage(message: FfiMessage): Boolean

    /** Envia mensagem de imagem. */
    suspend fun sendImageMessage(
        toPeerId: String,
        imageData: List<UByte>,
        fileName: String,
        quality: UInt = 85u
    )

    /** Envia mensagem de voz. */
    suspend fun sendVoiceMessage(
        toPeerId: String,
        audioData: List<UByte>,
        fileName: String,
        durationSeconds: Int
    )

    /** Envia documento/arquivo. */
    suspend fun sendDocumentMessage(
        toPeerId: String,
        fileData: List<UByte>,
        fileName: String,
        mimeType: String
    )

    /** Envia mensagem de vídeo. */
    suspend fun sendVideoMessage(
        toPeerId: String,
        videoData: List<UByte>,
        fileName: String,
        width: Int? = null,
        height: Int? = null,
        durationSeconds: Int,
        thumbnailData: List<UByte>? = null
    )

    /** Exclui uma mensagem. */
    suspend fun deleteMessage(messageId: String)

    /** Encaminha uma mensagem para outro peer. */
    suspend fun forwardMessage(messageId: String, toPeerId: String)

    /** Adiciona reação a uma mensagem. */
    suspend fun addReaction(messageId: String, emoji: String)

    /** Remove reação de uma mensagem. */
    suspend fun removeReaction(messageId: String, emoji: String)

    /** Retorna reações de uma mensagem. */
    suspend fun getMessageReactions(messageId: String): List<FfiReaction>
}
