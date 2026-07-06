package com.zaplivre.ui.screens.chat

import androidx.lifecycle.ViewModel
import androidx.lifecycle.viewModelScope
import com.zaplivre.core.ZapLivreClientApi
import com.zaplivre.core.ZapLivreClientWrapper
import com.zaplivre.core.ServiceLocator
import kotlinx.coroutines.delay
import kotlinx.coroutines.flow.MutableSharedFlow
import kotlinx.coroutines.flow.MutableStateFlow
import kotlinx.coroutines.flow.SharedFlow
import kotlinx.coroutines.flow.StateFlow
import kotlinx.coroutines.flow.asSharedFlow
import kotlinx.coroutines.flow.asStateFlow
import kotlinx.coroutines.launch
import uniffi.zaplivre.FfiMessage

/**
 * ViewModel da ChatScreen (fluxos principais de texto).
 *
 * Responsável por: carregar mensagens da conversa (filtrando mensagens
 * legadas de sender key), marcar a conversa como lida ao abrir, enviar
 * texto e recarregar em eventos do core. Os fluxos de mídia/reações
 * continuam na tela e usam [refresh] para atualizar a lista.
 */
class ChatViewModel(
    private val peerId: String,
    private val api: ZapLivreClientApi = ServiceLocator.clientApi
) : ViewModel() {

    /** Resultado de um envio de texto (para haptics e restauração do input). */
    sealed class SendResult {
        data class Success(val messageId: String) : SendResult()

        /** Falha de envio: [content] preserva o texto para a tela restaurar. */
        data class Failure(val content: String, val error: Throwable?) : SendResult()
    }

    private val _messages = MutableStateFlow<List<FfiMessage>>(emptyList())
    val messages: StateFlow<List<FfiMessage>> = _messages.asStateFlow()

    private val _isSending = MutableStateFlow(false)
    val isSending: StateFlow<Boolean> = _isSending.asStateFlow()

    private val _sendResults = MutableSharedFlow<SendResult>(extraBufferCapacity = 8)
    val sendResults: SharedFlow<SendResult> = _sendResults.asSharedFlow()

    val localPeerId: StateFlow<String?> = api.localPeerId

    init {
        viewModelScope.launch {
            // Marca como lida ao abrir a conversa
            api.markConversationRead(peerId)
            refreshMessages()
        }

        // EVT-01: eventos do core substituem o polling
        viewModelScope.launch {
            api.messageEvents.collect { event ->
                val relevant = when (event) {
                    is ZapLivreClientWrapper.MessageUiEvent.Received ->
                        event.fromPeerId == peerId
                    is ZapLivreClientWrapper.MessageUiEvent.StatusChanged ->
                        event.peerId == null || event.peerId == peerId
                    is ZapLivreClientWrapper.MessageUiEvent.Typing -> false
                }
                if (relevant) {
                    refreshMessages()
                }
            }
        }

        // Safety net: refresh lento caso algum evento se perca
        viewModelScope.launch {
            while (true) {
                delay(REFRESH_INTERVAL_MS)
                refreshMessages()
            }
        }
    }

    /**
     * Envia mensagem de texto. Em caso de falha o conteúdo é preservado
     * em [SendResult.Failure] para a tela não perder a mensagem.
     */
    fun sendTextMessage(content: String) {
        val trimmed = content.trim()
        if (trimmed.isEmpty() || _isSending.value) return

        viewModelScope.launch {
            _isSending.value = true
            val result = api.sendTextMessage(peerId, trimmed)
            _isSending.value = false

            if (result.isSuccess) {
                _sendResults.emit(SendResult.Success(result.getOrDefault("")))
                refreshMessages()
            } else {
                _sendResults.emit(SendResult.Failure(trimmed, result.exceptionOrNull()))
            }
        }
    }

    /** Recarrega mensagens (usado também pelos fluxos de mídia da tela). */
    fun refresh() {
        viewModelScope.launch { refreshMessages() }
    }

    private suspend fun refreshMessages() {
        try {
            val fetched = api.getConversationMessages(peerId)
            _messages.value = fetched.filterNot { api.isLegacyGroupKeyMessage(it) }
        } catch (_: Exception) {
            // mantém a lista atual; o safety net tentará de novo
        }
    }

    private companion object {
        const val REFRESH_INTERVAL_MS = 30_000L
    }
}
