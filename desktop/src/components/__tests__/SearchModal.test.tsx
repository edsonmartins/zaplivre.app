/**
 * UX-03: busca global de mensagens. O SearchModal chama search_messages
 * (FTS do core) e abre o chat do peer certo ao clicar num resultado.
 */
import { render, screen, waitFor } from '@testing-library/react'
import userEvent from '@testing-library/user-event'
import { describe, expect, it, vi } from 'vitest'
import SearchModal from '../SearchModal'
import { messageFixture, setupTauri } from '../../test/tauriMock'

const LOCAL_PEER = 'PEER_A'

function renderModal(onClose = vi.fn(), onOpenChat = vi.fn()) {
  render(
    <SearchModal localPeerId={LOCAL_PEER} onClose={onClose} onOpenChat={onOpenChat} />
  )
  return { onClose, onOpenChat }
}

async function search(user: ReturnType<typeof userEvent.setup>, query: string) {
  await user.type(screen.getByPlaceholderText(/buscar em todas as conversas/i), query)
  await user.click(screen.getByRole('button', { name: /buscar/i }))
}

describe('SearchModal', () => {
  it('digitar e buscar chama search_messages com a query', async () => {
    const { callsOf } = setupTauri({ search_messages: () => [] })
    const user = userEvent.setup()
    renderModal()

    await search(user, 'pix')

    await waitFor(() => {
      const calls = callsOf('search_messages')
      expect(calls).toHaveLength(1)
      expect(calls[0].args).toMatchObject({ query: 'pix', limit: 50 })
    })
  })

  it('renderiza os resultados retornados pelo core', async () => {
    setupTauri({
      search_messages: () => [messageFixture({ content: 'Combinado, te pago via pix' })],
    })
    const user = userEvent.setup()
    renderModal()

    await search(user, 'pix')

    expect(
      await screen.findByText('Combinado, te pago via pix')
    ).toBeInTheDocument()
    // O peer exibido é o "outro lado" da mensagem (sender != localPeerId)
    expect(screen.getByText(/PEER_B/)).toBeInTheDocument()
  })

  it('busca sem resultados mostra "Nenhuma mensagem encontrada."', async () => {
    setupTauri({ search_messages: () => [] })
    const user = userEvent.setup()
    renderModal()

    await search(user, 'inexistente')

    expect(
      await screen.findByText(/nenhuma mensagem encontrada/i)
    ).toBeInTheDocument()
  })

  it('erro do invoke não derruba a UI (cai no estado "sem resultados")', async () => {
    setupTauri({
      search_messages: () => {
        throw new Error('FTS indisponível')
      },
    })
    const user = userEvent.setup()
    renderModal()

    await search(user, 'pix')

    // O componente engole o erro e mostra o estado vazio
    expect(
      await screen.findByText(/nenhuma mensagem encontrada/i)
    ).toBeInTheDocument()
  })

  it('clicar num resultado chama onOpenChat com o peer da conversa', async () => {
    setupTauri({ search_messages: () => [messageFixture()] })
    const user = userEvent.setup()
    const { onOpenChat } = renderModal()

    await search(user, 'olá')
    const result = await screen.findByText('Olá!')

    await user.click(result)

    // sender é PEER_B (não sou eu), então o chat aberto é com PEER_B
    expect(onOpenChat).toHaveBeenCalledWith('PEER_B')
  })
})
