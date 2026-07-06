package com.zaplivre.ui.screens.conversations

import androidx.lifecycle.ViewModel
import androidx.lifecycle.viewModelScope
import com.zaplivre.core.ZapLivreClientApi
import com.zaplivre.core.ZapLivreClientWrapper
import com.zaplivre.core.ServiceLocator
import kotlinx.coroutines.delay
import kotlinx.coroutines.flow.MutableStateFlow
import kotlinx.coroutines.flow.StateFlow
import kotlinx.coroutines.flow.asStateFlow
import kotlinx.coroutines.launch
import uniffi.zaplivre.FfiConversation

/**
 * Estado de UI da lista de conversas.
 */
sealed interface ConversationsUiState {
    object Loading : ConversationsUiState
    data class Success(val conversations: List<FfiConversation>) : ConversationsUiState
    data class Error(val message: String) : ConversationsUiState
}

/**
 * ViewModel da ConversationsScreen.
 *
 * Mantém o comportamento da tela original:
 * - carrega a lista ao abrir (o core já retorna ordenado por data);
 * - recarrega quando o core emite evento de mensagem (exceto Typing);
 * - safety net: refresh periódico caso algum evento se perca.
 */
class ConversationsViewModel(
    private val api: ZapLivreClientApi = ServiceLocator.clientApi
) : ViewModel() {

    private val _uiState = MutableStateFlow<ConversationsUiState>(ConversationsUiState.Loading)
    val uiState: StateFlow<ConversationsUiState> = _uiState.asStateFlow()

    init {
        load()

        // EVT-01: recarregar a lista quando o core avisa de mensagem nova
        viewModelScope.launch {
            api.messageEvents.collect { event ->
                if (event !is ZapLivreClientWrapper.MessageUiEvent.Typing) {
                    refresh()
                }
            }
        }

        // Safety net: refresh lento caso algum evento se perca
        viewModelScope.launch {
            while (true) {
                delay(REFRESH_INTERVAL_MS)
                refresh()
            }
        }
    }

    /** Carrega (ou recarrega) exibindo o estado Loading. */
    fun load() {
        _uiState.value = ConversationsUiState.Loading
        viewModelScope.launch {
            _uiState.value = try {
                ConversationsUiState.Success(api.listConversations())
            } catch (e: Exception) {
                ConversationsUiState.Error(e.message ?: "Erro ao carregar conversas")
            }
        }
    }

    /** Recarrega sem voltar para Loading (evita piscar a lista). */
    private suspend fun refresh() {
        try {
            _uiState.value = ConversationsUiState.Success(api.listConversations())
        } catch (_: Exception) {
            // mantém o estado atual; o safety net tentará de novo
        }
    }

    private companion object {
        const val REFRESH_INTERVAL_MS = 30_000L
    }
}
