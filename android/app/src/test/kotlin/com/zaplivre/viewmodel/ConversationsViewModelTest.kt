package com.zaplivre.viewmodel

import androidx.lifecycle.viewModelScope
import app.cash.turbine.test
import com.zaplivre.MainDispatcherRule
import com.zaplivre.core.ZapLivreClientApi
import com.zaplivre.core.ZapLivreClientWrapper
import com.zaplivre.ui.screens.conversations.ConversationsUiState
import com.zaplivre.ui.screens.conversations.ConversationsViewModel
import io.mockk.coEvery
import io.mockk.coVerify
import io.mockk.every
import io.mockk.mockk
import kotlinx.coroutines.ExperimentalCoroutinesApi
import kotlinx.coroutines.cancel
import kotlinx.coroutines.flow.MutableSharedFlow
import kotlinx.coroutines.flow.MutableStateFlow
import kotlinx.coroutines.test.TestScope
import kotlinx.coroutines.test.advanceTimeBy
import kotlinx.coroutines.test.runCurrent
import kotlinx.coroutines.test.runTest
import org.junit.Assert.assertEquals
import org.junit.Assert.assertTrue
import org.junit.Before
import org.junit.Rule
import org.junit.Test
import uniffi.zaplivre.FfiConversation

@OptIn(ExperimentalCoroutinesApi::class)
class ConversationsViewModelTest {

    @get:Rule
    val mainDispatcherRule = MainDispatcherRule()

    private lateinit var api: ZapLivreClientApi
    private lateinit var messageEvents: MutableSharedFlow<ZapLivreClientWrapper.MessageUiEvent>

    @Before
    fun setUp() {
        api = mockk()
        messageEvents = MutableSharedFlow(extraBufferCapacity = 16)
        every { api.messageEvents } returns messageEvents
        every { api.localPeerId } returns MutableStateFlow("local-peer")
    }

    /**
     * Cria o ViewModel e garante o cancelamento do viewModelScope ao final.
     * Sem isso o safety net (loop infinito de delay) impede o runTest de
     * concluir a limpeza do scheduler de tempo virtual.
     */
    private fun runVmTest(
        testBody: suspend TestScope.(ConversationsViewModel) -> Unit
    ) = runTest {
        val viewModel = ConversationsViewModel(api)
        try {
            testBody(viewModel)
        } finally {
            viewModel.viewModelScope.cancel()
        }
    }

    private fun conversation(
        id: String,
        peerId: String,
        lastMessageAt: Long?
    ) = FfiConversation(
        id = id,
        conversationType = "direct",
        peerId = peerId,
        displayName = "Peer $peerId",
        lastMessageId = null,
        lastMessageAt = lastMessageAt,
        unreadCount = 0,
        isMuted = false,
        isArchived = false,
        createdAt = 1L
    )

    @Test
    fun `estado inicial e Loading antes do carregamento completar`() {
        coEvery { api.listConversations() } returns emptyList()

        runVmTest { viewModel ->
            // Antes de o scheduler rodar, o estado deve ser Loading
            assertEquals(ConversationsUiState.Loading, viewModel.uiState.value)
        }
    }

    @Test
    fun `carregamento com sucesso emite Loading e depois Success`() {
        val conversations = listOf(
            conversation("c1", "peer-a", 100L),
            conversation("c2", "peer-b", 50L)
        )
        coEvery { api.listConversations() } returns conversations

        runVmTest { viewModel ->
            viewModel.uiState.test {
                assertEquals(ConversationsUiState.Loading, awaitItem())
                runCurrent()
                assertEquals(ConversationsUiState.Success(conversations), awaitItem())
            }
        }
    }

    @Test
    fun `falha no carregamento emite Error com a mensagem da excecao`() {
        coEvery { api.listConversations() } throws RuntimeException("core indisponivel")

        runVmTest { viewModel ->
            runCurrent()

            val state = viewModel.uiState.value
            assertTrue(state is ConversationsUiState.Error)
            assertEquals("core indisponivel", (state as ConversationsUiState.Error).message)
        }
    }

    @Test
    fun `preserva a ordem retornada pela API (core ja ordena por data)`() {
        // A tela nao reordena: confia na ordenacao do core (mais recente primeiro)
        val recent = conversation("c-recent", "peer-recent", 2000L)
        val older = conversation("c-older", "peer-older", 1000L)
        coEvery { api.listConversations() } returns listOf(recent, older)

        runVmTest { viewModel ->
            runCurrent()

            val state = viewModel.uiState.value as ConversationsUiState.Success
            assertEquals(listOf(recent, older), state.conversations)
        }
    }

    @Test
    fun `evento de mensagem recebida recarrega a lista`() {
        val initial = listOf(conversation("c1", "peer-a", 100L))
        coEvery { api.listConversations() } returns initial

        runVmTest { viewModel ->
            runCurrent()
            assertEquals(ConversationsUiState.Success(initial), viewModel.uiState.value)

            // Nova conversa aparece apos o evento
            val updated = initial + conversation("c2", "peer-b", 200L)
            coEvery { api.listConversations() } returns updated
            messageEvents.tryEmit(
                ZapLivreClientWrapper.MessageUiEvent.Received("msg-1", "peer-b")
            )
            runCurrent()

            assertEquals(ConversationsUiState.Success(updated), viewModel.uiState.value)
            coVerify(exactly = 2) { api.listConversations() }
        }
    }

    @Test
    fun `evento Typing nao recarrega a lista`() {
        val initial = listOf(conversation("c1", "peer-a", 100L))
        coEvery { api.listConversations() } returns initial

        runVmTest { viewModel ->
            runCurrent()

            coEvery { api.listConversations() } returns emptyList()
            messageEvents.tryEmit(
                ZapLivreClientWrapper.MessageUiEvent.Typing("peer-a", true)
            )
            runCurrent()

            // Estado inalterado e API nao foi chamada de novo
            assertEquals(ConversationsUiState.Success(initial), viewModel.uiState.value)
            coVerify(exactly = 1) { api.listConversations() }
        }
    }

    @Test
    fun `safety net recarrega apos o intervalo de 30s`() {
        val initial = listOf(conversation("c1", "peer-a", 100L))
        coEvery { api.listConversations() } returns initial

        runVmTest { viewModel ->
            runCurrent()

            val updated = listOf(conversation("c1", "peer-a", 999L))
            coEvery { api.listConversations() } returns updated

            advanceTimeBy(30_001L)
            runCurrent()

            assertEquals(ConversationsUiState.Success(updated), viewModel.uiState.value)
        }
    }
}
