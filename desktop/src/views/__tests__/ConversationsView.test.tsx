import { render, screen, waitFor } from '@testing-library/react'
import userEvent from '@testing-library/user-event'
import { emit } from '@tauri-apps/api/event'
import { MemoryRouter } from 'react-router-dom'
import { describe, expect, it } from 'vitest'
import ConversationsView from '../ConversationsView'
import { conversationFixture, setupTauri } from '../../test/tauriMock'

function renderView() {
  return render(
    <MemoryRouter>
      <ConversationsView localPeerId="PEER_A" />
    </MemoryRouter>
  )
}

describe('ConversationsView', () => {
  it('renderiza as conversas vindas de list_conversations', async () => {
    setupTauri({
      list_conversations: () => [
        conversationFixture({ display_name: 'Alice' }),
        conversationFixture({ id: '1:1:PEER_C', peer_id: 'PEER_C', display_name: 'Bob' }),
      ],
    })

    renderView()

    await waitFor(() => {
      expect(screen.getByText(/Alice/)).toBeInTheDocument()
      expect(screen.getByText(/Bob/)).toBeInTheDocument()
    })
  })

  it('recarrega a lista quando o core emite message:received (EVT-03)', async () => {
    let conversations = [conversationFixture({ display_name: 'Alice' })]
    setupTauri({
      list_conversations: () => conversations,
    })

    renderView()
    await waitFor(() => expect(screen.getByText(/Alice/)).toBeInTheDocument())

    // Nova conversa aparece no backend; o evento deve disparar o reload
    conversations = [
      ...conversations,
      conversationFixture({ id: '1:1:PEER_C', peer_id: 'PEER_C', display_name: 'Bob' }),
    ]
    await emit('message:received', { message_id: 'm2', from_peer_id: 'PEER_C' })

    await waitFor(() => expect(screen.getByText(/Bob/)).toBeInTheDocument())
  })

  it('abre o diálogo de nova conversa', async () => {
    setupTauri({ list_conversations: () => [] })
    const user = userEvent.setup()

    renderView()

    const newChatButton = await screen.findByRole('button', { name: /new chat/i })
    await user.click(newChatButton)

    expect(await screen.findByPlaceholderText(/peer id/i)).toBeInTheDocument()
  })
})
