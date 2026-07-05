package com.mepassa.viewmodel

import androidx.lifecycle.viewModelScope
import app.cash.turbine.test
import com.mepassa.MainDispatcherRule
import com.mepassa.core.MePassaClientApi
import com.mepassa.core.MePassaClientWrapper
import com.mepassa.ui.screens.chat.ChatViewModel
import io.mockk.coEvery
import io.mockk.coVerify
import io.mockk.every
import io.mockk.mockk
import kotlinx.coroutines.CompletableDeferred
import kotlinx.coroutines.ExperimentalCoroutinesApi
import kotlinx.coroutines.cancel
import kotlinx.coroutines.flow.MutableSharedFlow
import kotlinx.coroutines.flow.MutableStateFlow
import kotlinx.coroutines.test.TestScope
import kotlinx.coroutines.test.runCurrent
import kotlinx.coroutines.test.runTest
import org.junit.Assert.assertEquals
import org.junit.Assert.assertFalse
import org.junit.Assert.assertTrue
import org.junit.Before
import org.junit.Rule
import org.junit.Test
import uniffi.mepassa.FfiMessage
import uniffi.mepassa.MessageStatus

@OptIn(ExperimentalCoroutinesApi::class)
class ChatViewModelTest {

    @get:Rule
    val mainDispatcherRule = MainDispatcherRule()

    private val peerId = "peer-remote"
    private val localPeer = "peer-local"

    private lateinit var api: MePassaClientApi
    private lateinit var messageEvents: MutableSharedFlow<MePassaClientWrapper.MessageUiEvent>

    private val legacyPrefix = "mepassa-group-key:v1:"

    @Before
    fun setUp() {
        api = mockk()
        messageEvents = MutableSharedFlow(extraBufferCapacity = 16)
        every { api.messageEvents } returns messageEvents
        every { api.localPeerId } returns MutableStateFlow(localPeer)
        every { api.isLegacyGroupKeyMessage(any()) } answers {
            firstArg<FfiMessage>().contentPlaintext?.startsWith(legacyPrefix) == true
        }
        coEvery { api.markConversationRead(any()) } returns true
        coEvery { api.getConversationMessages(peerId, null, null) } returns emptyList()
    }

    /**
     * Cria o ViewModel e garante o cancelamento do viewModelScope ao final.
     * Sem isso o safety net (loop infinito de delay) impede o runTest de
     * concluir a limpeza do scheduler de tempo virtual.
     */
    private fun runVmTest(
        testBody: suspend TestScope.(ChatViewModel) -> Unit
    ) = runTest {
        val viewModel = ChatViewModel(peerId, api)
        try {
            testBody(viewModel)
        } finally {
            viewModel.viewModelScope.cancel()
        }
    }

    private fun message(
        id: String,
        content: String,
        sender: String = peerId
    ) = FfiMessage(
        messageId = id,
        conversationId = "conv-1",
        senderPeerId = sender,
        recipientPeerId = if (sender == peerId) localPeer else peerId,
        messageType = "text",
        contentPlaintext = content,
        createdAt = 1L,
        sentAt = 1L,
        receivedAt = null,
        readAt = null,
        status = MessageStatus.SENT,
        isDeleted = false
    )

    @Test
    fun `carrega mensagens da conversa ao abrir`() {
        val msgs = listOf(message("m1", "oi"), message("m2", "tudo bem?"))
        coEvery { api.getConversationMessages(peerId, null, null) } returns msgs

        runVmTest { viewModel ->
            runCurrent()
            assertEquals(msgs, viewModel.messages.value)
        }
    }

    @Test
    fun `filtra mensagens legadas de sender key de grupo`() {
        val normal = message("m1", "oi")
        val legacy = message("m2", legacyPrefix + "abc123")
        coEvery { api.getConversationMessages(peerId, null, null) } returns listOf(normal, legacy)

        runVmTest { viewModel ->
            runCurrent()
            assertEquals(listOf(normal), viewModel.messages.value)
        }
    }

    @Test
    fun `marca conversa como lida ao abrir`() {
        runVmTest { _ ->
            runCurrent()
            coVerify(exactly = 1) { api.markConversationRead(peerId) }
        }
    }

    @Test
    fun `envio feliz chama api com args corretos e atualiza estado`() {
        coEvery { api.sendTextMessage(peerId, "ola mundo") } returns Result.success("msg-id-1")

        runVmTest { viewModel ->
            runCurrent()

            val afterSend = listOf(message("msg-id-1", "ola mundo", sender = localPeer))
            coEvery { api.getConversationMessages(peerId, null, null) } returns afterSend

            viewModel.sendResults.test {
                viewModel.sendTextMessage("ola mundo")
                runCurrent()

                assertEquals(ChatViewModel.SendResult.Success("msg-id-1"), awaitItem())
            }

            coVerify(exactly = 1) { api.sendTextMessage(peerId, "ola mundo") }
            assertEquals(afterSend, viewModel.messages.value)
            assertFalse(viewModel.isSending.value)
        }
    }

    @Test
    fun `envio com falha expoe erro e preserva o conteudo da mensagem`() {
        val boom = RuntimeException("sem rota para o peer")
        coEvery { api.sendTextMessage(peerId, "nao vai") } returns Result.failure(boom)

        runVmTest { viewModel ->
            runCurrent()

            viewModel.sendResults.test {
                viewModel.sendTextMessage("nao vai")
                runCurrent()

                val result = awaitItem()
                assertTrue(result is ChatViewModel.SendResult.Failure)
                result as ChatViewModel.SendResult.Failure
                // O conteudo nao se perde: a tela restaura no input
                assertEquals("nao vai", result.content)
                assertEquals(boom, result.error)
            }

            // Lista de mensagens permanece a mesma (sem recarga apos falha)
            assertEquals(emptyList<FfiMessage>(), viewModel.messages.value)
            assertFalse(viewModel.isSending.value)
        }
    }

    @Test
    fun `isSending fica true durante o envio e volta a false ao final`() {
        val gate = CompletableDeferred<Unit>()
        coEvery { api.sendTextMessage(peerId, "lento") } coAnswers {
            gate.await()
            Result.success("msg-lento")
        }

        runVmTest { viewModel ->
            runCurrent()

            viewModel.sendTextMessage("lento")
            runCurrent()
            assertTrue(viewModel.isSending.value)

            gate.complete(Unit)
            runCurrent()
            assertFalse(viewModel.isSending.value)
        }
    }

    @Test
    fun `nao envia conteudo em branco`() {
        runVmTest { viewModel ->
            runCurrent()

            viewModel.sendTextMessage("   ")
            runCurrent()

            coVerify(exactly = 0) { api.sendTextMessage(any(), any()) }
        }
    }

    @Test
    fun `evento Received do peer da conversa recarrega mensagens`() {
        runVmTest { viewModel ->
            runCurrent()
            assertEquals(emptyList<FfiMessage>(), viewModel.messages.value)

            val incoming = listOf(message("m-new", "chegou!"))
            coEvery { api.getConversationMessages(peerId, null, null) } returns incoming

            messageEvents.tryEmit(
                MePassaClientWrapper.MessageUiEvent.Received("m-new", peerId)
            )
            runCurrent()

            assertEquals(incoming, viewModel.messages.value)
        }
    }

    @Test
    fun `evento Received de outro peer nao recarrega`() {
        runVmTest { _ ->
            runCurrent()

            messageEvents.tryEmit(
                MePassaClientWrapper.MessageUiEvent.Received("m-x", "outro-peer")
            )
            runCurrent()

            // Somente a carga inicial consultou a API
            coVerify(exactly = 1) { api.getConversationMessages(peerId, null, null) }
        }
    }

    @Test
    fun `evento StatusChanged global (peerId null) recarrega mensagens`() {
        runVmTest { viewModel ->
            runCurrent()

            val updated = listOf(message("m1", "oi"))
            coEvery { api.getConversationMessages(peerId, null, null) } returns updated

            messageEvents.tryEmit(
                MePassaClientWrapper.MessageUiEvent.StatusChanged(
                    "m1", MessageStatus.DELIVERED, null
                )
            )
            runCurrent()

            assertEquals(updated, viewModel.messages.value)
            coVerify(exactly = 2) { api.getConversationMessages(peerId, null, null) }
        }
    }

    @Test
    fun `evento Typing nao recarrega mensagens`() {
        runVmTest { _ ->
            runCurrent()

            messageEvents.tryEmit(
                MePassaClientWrapper.MessageUiEvent.Typing(peerId, true)
            )
            runCurrent()

            coVerify(exactly = 1) { api.getConversationMessages(peerId, null, null) }
        }
    }
}
