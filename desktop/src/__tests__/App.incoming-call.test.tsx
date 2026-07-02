/**
 * Teste anti-regressão do "callee cego" (DSK-05):
 * o App precisa registrar os listeners de chamada e RENDERIZAR o
 * IncomingCallModal quando o core emite voip:incoming_call.
 *
 * Antes da correção, o modal existia mas nunca era montado - quem recebia
 * uma chamada não via absolutamente nada. Este teste falha se esse fio
 * voltar a se soltar.
 */
import { render, screen, waitFor } from '@testing-library/react'
import { emit } from '@tauri-apps/api/event'
import { MemoryRouter } from 'react-router-dom'
import { describe, expect, it } from 'vitest'
import AppWithProviders from '../App'
import { setupTauri } from '../test/tauriMock'

function setupAppMocks() {
  return setupTauri({
    init_client: () => '12D3KooWLocalPeer',
    listen_on: () => null,
    bootstrap: () => null,
    list_conversations: () => [],
    register_video_frame_callback: () => null,
  })
}

describe('App - chamada recebida', () => {
  it('mostra o IncomingCallModal quando o core emite voip:incoming_call', async () => {
    setupAppMocks()

    render(
      <MemoryRouter initialEntries={['/conversations']}>
        <AppWithProviders />
      </MemoryRouter>
    )

    // Aguardar a inicialização do app (init_client -> conversations)
    await waitFor(() => {
      expect(screen.queryByText(/loading mepassa/i)).not.toBeInTheDocument()
    })

    // Core avisa: chamada chegando
    await emit('voip:incoming_call', {
      call_id: 'call-777',
      from_peer_id: '12D3KooWCallerPeer',
    })

    // O modal PRECISA aparecer, com os botões de atender/recusar
    expect(await screen.findByRole('button', { name: /atender/i })).toBeInTheDocument()
    expect(screen.getByRole('button', { name: /recusar/i })).toBeInTheDocument()
  })

  it('fecha o modal quando o caller cancela (voip:call_ended)', async () => {
    setupAppMocks()

    render(
      <MemoryRouter initialEntries={['/conversations']}>
        <AppWithProviders />
      </MemoryRouter>
    )

    await waitFor(() => {
      expect(screen.queryByText(/loading mepassa/i)).not.toBeInTheDocument()
    })

    await emit('voip:incoming_call', {
      call_id: 'call-777',
      from_peer_id: '12D3KooWCallerPeer',
    })
    expect(await screen.findByRole('button', { name: /atender/i })).toBeInTheDocument()

    // Caller desligou antes de atender
    await emit('voip:call_ended', { call_id: 'call-777', reason: 'CallerHungUp' })

    await waitFor(() => {
      expect(screen.queryByRole('button', { name: /atender/i })).not.toBeInTheDocument()
    })
  })
})
